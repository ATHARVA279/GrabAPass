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

import { EventService } from '../../../core/services/event.service';
import { VenueService } from '../../../core/services/venue.service';
import { Event } from '../../../shared/models/event';
import { SeatLayoutResponse } from '../../../shared/models/venue';
import { SeatMapRenderer, SelectedSeat } from '../../../shared/components/seat-map-renderer/seat-map-renderer';

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
    SeatMapRenderer,
  ],
  templateUrl: './event-detail.html',
  styleUrls: ['./event-detail.scss']
})
export class EventDetail implements OnInit {
  event: Event | null = null;
  seatLayout: SeatLayoutResponse | null = null;
  loading = true;
  layoutLoading = false;

  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly eventService = inject(EventService);
  private readonly venueService = inject(VenueService);
  private readonly toastr = inject(ToastrService);

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
          this.loadSeatLayout(eventId);
        }
      },
      error: () => this.toastr.error('Event not found or failed to load.', 'Error')
    });
  }

  private loadSeatLayout(eventId: string): void {
    this.layoutLoading = true;
    this.venueService.getSeatLayout(eventId).pipe(
      finalize(() => (this.layoutLoading = false))
    ).subscribe({
      next: (layout) => (this.seatLayout = layout),
      error: () => this.toastr.error('Could not load seat layout.', 'Seating Error')
    });
  }

  onSeatSelected(seat: SelectedSeat): void {
    // Phase 4 will handle checkout; for now just show a toast
    this.toastr.info(
      `${seat.seatLabel} — ${seat.categoryName} $${seat.price.toFixed(2)}`,
      'Seat Selected'
    );
  }

  goBack(): void {
    this.router.navigate(['/events']);
  }
}
