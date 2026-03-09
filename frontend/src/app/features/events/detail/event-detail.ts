import { Component, inject, OnInit, OnDestroy } from '@angular/core';
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
export class EventDetail implements OnInit, OnDestroy {
  event: Event | null = null;
  seatLayout: SeatLayoutResponse | null = null;
  loading = true;
  layoutLoading = false;
  
  // Seat hold state
  selectedSeats: SelectedSeat[] = [];
  isHolding = false;
  holdExpiresAt: Date | null = null;
  holdTimeRemaining = '';
  private timerInterval: any;

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

  onSelectedSeatsChanged(seats: SelectedSeat[]): void {
    this.selectedSeats = seats;
  }

  get totalSelectedPrice(): number {
    return this.selectedSeats.reduce((sum, seat) => sum + seat.price, 0);
  }

  onHoldSeats(): void {
    if (!this.event || this.selectedSeats.length === 0) return;

    this.isHolding = true;
    const seatIds = this.selectedSeats.map(s => s.seatId);

    this.eventService.holdSeats(this.event.id, seatIds).pipe(
      finalize(() => (this.isHolding = false))
    ).subscribe({
      next: (holds) => {
        this.toastr.success(`Successfully held ${holds.length} seats!`, 'Seats Held');
        this.selectedSeats = [];
        this.startHoldTimer(new Date(holds[0].expires_at));
        this.loadSeatLayout(this.event!.id);
      },
      error: (err) => {
        this.toastr.error(err.error?.message || 'Failed to hold seats. They may be unavailable.', 'Hold Failed');
        this.loadSeatLayout(this.event!.id);
      }
    });
  }

  startHoldTimer(expiresAt: Date): void {
    this.holdExpiresAt = expiresAt;
    this.updateTimerDisplay();
    
    if (this.timerInterval) {
      clearInterval(this.timerInterval);
    }
    
    this.timerInterval = setInterval(() => {
      this.updateTimerDisplay();
    }, 1000);
  }

  private updateTimerDisplay(): void {
    if (!this.holdExpiresAt) return;
    
    const now = new Date();
    const diff = this.holdExpiresAt.getTime() - now.getTime();
    
    if (diff <= 0) {
      this.holdTimeRemaining = '00:00';
      clearInterval(this.timerInterval);
      this.holdExpiresAt = null;
      this.toastr.warning('Your seat holds have expired.', 'Hold Expired');
      if (this.event) {
        this.loadSeatLayout(this.event.id);
      }
      return;
    }
    
    const minutes = Math.floor(diff / 60000);
    const seconds = Math.floor((diff % 60000) / 1000);
    this.holdTimeRemaining = `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
  }

  ngOnDestroy(): void {
    if (this.timerInterval) {
      clearInterval(this.timerInterval);
    }
  }

  goBack(): void {
    this.router.navigate(['/events']);
  }
}
