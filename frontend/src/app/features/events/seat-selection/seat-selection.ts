import { Component, inject, OnInit, OnDestroy, ChangeDetectorRef } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { finalize } from 'rxjs';
import { ToastrService } from 'ngx-toastr';

import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';

import { PublicEventService } from '../../../core/services/public-event.service';
import { CheckoutService } from '../../../core/services/checkout.service';
import { VenueService } from '../../../core/services/venue.service';
import { BookingService } from '../../../core/services/booking.service';
import { WsService } from '../../../core/services/ws.service';
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
export class SeatSelection implements OnInit, OnDestroy {
  event: Event | null = null;
  seatLayout: SeatLayoutResponse | null = null;
  loading = true;
  layoutLoading = false;
  isHolding = false;
  hasActiveHold = false;

  selectedSeats: SelectedSeat[] = [];
  private wsSubscription: any;

  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly eventService = inject(PublicEventService);
  private readonly checkoutService = inject(CheckoutService);
  private readonly venueService = inject(VenueService);
  private readonly bookingService = inject(BookingService);
  private readonly wsService = inject(WsService);
  private readonly authService = inject(AuthService);
  private readonly toastr = inject(ToastrService);
  private readonly cdr = inject(ChangeDetectorRef);

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
        this.restoreExistingBooking(event);
        if (event.venue_template_id) {
          this.loadSeatLayout(eventId);
        }
      },
      error: () => this.toastr.error('Event not found.', 'Error'),
    });

    this.wsSubscription = this.wsService.connectToEvent(eventId).subscribe(msg => {
      if (msg && msg.type === 'SEATS_UPDATED') {
        // Only refresh layout if there's no active local hold (we don't want to wipe the user's unsaved picks)
        // Wait, seat selected state is kept in `selectedSeats`, which is mapped to the layout
        this.loadSeatLayout(eventId, true);
      }
    });
  }

  ngOnDestroy(): void {
    if (this.wsSubscription) {
      this.wsSubscription.unsubscribe();
    }
  }

  private loadSeatLayout(eventId: string, isSilentRefresh: boolean = false): void {
    if (!isSilentRefresh) {
      this.layoutLoading = true;
    }
    this.venueService.getSeatLayout(eventId).pipe(
      finalize(() => {
        this.layoutLoading = false;
        this.cdr.markForCheck();
      })
    ).subscribe({
      next: (layout) => {
        this.seatLayout = layout;
        this.cdr.markForCheck();
      },
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

  get actionLabel(): string {
    return this.hasActiveHold ? 'Continue to Checkout' : 'Hold & Continue';
  }

  onHoldAndContinue(): void {
    if (!this.event || this.selectedSeats.length === 0) return;

    if (!this.isLoggedIn) {
      this.router.navigate(['/login'], {
        queryParams: { returnUrl: `/events/${this.event.id}/seats` }
      });
      return;
    }

    if (this.hasActiveHold) {
      this.router.navigate(['/events', this.event.id, 'checkout']);
      return;
    }

    this.isHolding = true;
    const seatIds = this.selectedSeats.map(s => s.seatId);

    this.checkoutService.holdSeats(this.event.id, { seat_ids: seatIds }).pipe(
      finalize(() => (this.isHolding = false))
    ).subscribe({
      next: (holds) => {
        this.toastr.success(`Held ${holds.length} seats!`, 'Seats Held');
        this.bookingService.setSelection(this.event!, this.selectedSeats, []);
        this.bookingService.setHoldData(holds.map((hold) => hold.id), new Date(holds[0].expires_at));
        this.router.navigate(['/events', this.event!.id, 'checkout']);
      },
      error: (err) => {
        this.toastr.error(err.error?.message || 'Failed to hold seats.', 'Hold Failed');
        this.hasActiveHold = false;
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

  private restoreExistingBooking(event: Event): void {
    const bookingState = this.bookingService.getState();
    if (!bookingState || bookingState.event.id !== event.id) {
      this.hasActiveHold = false;
      return;
    }

    const holdExpiresAt = bookingState.holdExpiresAt;
    const holdStillActive = !!holdExpiresAt && holdExpiresAt.getTime() > Date.now();

    this.selectedSeats = [...bookingState.selectedSeats];
    this.hasActiveHold = holdStillActive && bookingState.holdIds.length > 0;

    if (!holdStillActive) {
      this.hasActiveHold = false;
    }
  }
}
