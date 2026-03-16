import { Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, RouterModule, Router } from '@angular/router';
import { finalize } from 'rxjs';
import { ToastrService } from 'ngx-toastr';

import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatChipsModule } from '@angular/material/chips';

import { PublicEventService } from '../../../core/services/public-event.service';
import { VenueService } from '../../../core/services/venue.service';
import { AuthService } from '../../../core/auth/auth';
import { Event } from '../../../shared/models/event';
import { SeatLayoutResponse } from '../../../shared/models/venue';

@Component({
  selector: 'app-event-detail',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    MatCardModule,
    MatButtonModule,
    MatIconModule,
    MatProgressSpinnerModule,
    MatChipsModule,
  ],
  templateUrl: './event-detail.html',
  styleUrls: ['./event-detail.scss']
})
export class EventDetail implements OnInit {
  event: Event | null = null;
  loading = true;
  priceRange: { min: number; max: number } | null = null;

  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly eventService = inject(PublicEventService);
  private readonly venueService = inject(VenueService);
  private readonly authService = inject(AuthService);
  private readonly toastr = inject(ToastrService);

  get isLoggedIn(): boolean {
    return !!this.authService.currentUserValue;
  }

  ngOnInit(): void {
    const eventId = this.route.snapshot.paramMap.get('id');

    if (!eventId) {
      this.toastr.error('No event ID provided.', 'Error');
      this.loading = false;
      return;
    }

    this.eventService.getEventById(eventId).pipe(
      finalize(() => (this.loading = false))
    ).subscribe({
      next: (event) => {
        this.event = event;
        if (event.venue_template_id) {
          this.loadPriceRange(eventId);
        }
      },
      error: () => this.toastr.error('Event not found or failed to load.', 'Error')
    });
  }

  private loadPriceRange(eventId: string): void {
    this.venueService.getSeatLayout(eventId).subscribe({
      next: (layout: SeatLayoutResponse) => {
        const prices = layout.sections
          .filter(s => s.category)
          .map(s => s.category!.price);
        if (prices.length > 0) {
          this.priceRange = { min: Math.min(...prices), max: Math.max(...prices) };
        }
      },
      error: () => {}
    });
  }

  goToSelectSeats(): void {
    if (this.event) {
      if (!this.isLoggedIn) {
        this.router.navigate(['/login'], {
          queryParams: { returnUrl: `/events/${this.event.id}/seats` }
        });
        return;
      }
      this.router.navigate(['/events', this.event.id, 'seats']);
    }
  }

  goBack(): void {
    this.router.navigate(['/events']);
  }
}
