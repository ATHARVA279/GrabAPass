import { Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { finalize } from 'rxjs';
import { ToastrService } from 'ngx-toastr';

import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';

import { EventService } from '../../../core/services/event.service';
import { VenueService } from '../../../core/services/venue.service';
import { BookingService } from '../../../core/services/booking.service';
import { AuthService } from '../../../core/auth/auth';
import { Event } from '../../../shared/models/event';
import { SeatLayoutResponse } from '../../../shared/models/venue';
import { SeatMapRenderer, SelectedSeat } from '../../../shared/components/seat-map-renderer/seat-map-renderer';

@Component({
  selector: 'app-seat-selection',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    MatButtonModule,
    MatIconModule,
    MatProgressSpinnerModule,
    SeatMapRenderer,
  ],
  templateUrl: './seat-selection.html',
  styleUrls: ['./seat-selection.scss'],
})
export class SeatSelection implements OnInit {
  event: Event | null = null;
  seatLayout: SeatLayoutResponse | null = null;
  loading = true;
  layoutLoading = false;
  isHolding = false;

  selectedSeats: SelectedSeat[] = [];

  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly eventService = inject(EventService);
  private readonly venueService = inject(VenueService);
  private readonly bookingService = inject(BookingService);
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
          this.loadSeatLayout(eventId);
        }
      },
      error: () => this.toastr.error('Event not found.', 'Error'),
    });
  }

  private loadSeatLayout(eventId: string): void {
    this.layoutLoading = true;
    this.venueService.getSeatLayout(eventId).pipe(
      finalize(() => (this.layoutLoading = false))
    ).subscribe({
      next: (layout) => (this.seatLayout = layout),
      error: () => this.toastr.error('Could not load seat layout.', 'Error'),
    });
  }

  onSelectedSeatsChanged(seats: SelectedSeat[]): void {
    if (!this.isLoggedIn) {
      this.selectedSeats = [];
      return;
    }
    this.selectedSeats = seats;
  }

  get totalPrice(): number {
    return this.selectedSeats.reduce((sum, s) => sum + s.price, 0);
  }

  onHoldAndContinue(): void {
    if (!this.event || this.selectedSeats.length === 0) return;

    if (!this.isLoggedIn) {
      this.router.navigate(['/login'], {
        queryParams: { returnUrl: `/events/${this.event.id}/seats` }
      });
      return;
    }

    this.isHolding = true;
    const seatIds = this.selectedSeats.map(s => s.seatId);

    this.eventService.holdSeats(this.event.id, seatIds).pipe(
      finalize(() => (this.isHolding = false))
    ).subscribe({
      next: (holds) => {
        this.toastr.success(`Held ${holds.length} seats!`, 'Seats Held');
        this.bookingService.setSelectedSeats(this.event!, this.selectedSeats);
        this.bookingService.setHoldData(seatIds, new Date(holds[0].expires_at));
        this.router.navigate(['/events', this.event!.id, 'checkout']);
      },
      error: (err) => {
        this.toastr.error(err.error?.message || 'Failed to hold seats.', 'Hold Failed');
        if (this.event) this.loadSeatLayout(this.event.id);
      },
    });
  }

  goBack(): void {
    if (this.event) {
      this.router.navigate(['/events', this.event.id]);
    } else {
      this.router.navigate(['/events']);
    }
  }
}
