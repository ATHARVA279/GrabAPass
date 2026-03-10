import { Component, inject, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule, ActivatedRoute } from '@angular/router';
import { finalize } from 'rxjs';
import { ToastrService } from 'ngx-toastr';

import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';

import { BookingService, BookingState } from '../../../core/services/booking.service';
import { EventService } from '../../../core/services/event.service';

@Component({
  selector: 'app-checkout',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    MatButtonModule,
    MatIconModule,
    MatProgressSpinnerModule,
  ],
  templateUrl: './checkout.html',
  styleUrls: ['./checkout.scss'],
})
export class Checkout implements OnInit, OnDestroy {
  state: BookingState | null = null;
  isProcessing = false;
  holdTimeRemaining = '';
  private timerInterval: any;

  private readonly router = inject(Router);
  private readonly route = inject(ActivatedRoute);
  private readonly bookingService = inject(BookingService);
  private readonly eventService = inject(EventService);
  private readonly toastr = inject(ToastrService);

  ngOnInit(): void {
    this.state = this.bookingService.getState();
    if (!this.state || !this.state.holdExpiresAt) {
      this.toastr.error('No active booking session. Please select seats first.', 'Error');
      const eventId = this.route.snapshot.paramMap.get('id');
      this.router.navigate(eventId ? ['/events', eventId, 'seats'] : ['/events']);
      return;
    }
    this.startTimer();
  }

  private startTimer(): void {
    this.updateTimerDisplay();
    this.timerInterval = setInterval(() => this.updateTimerDisplay(), 1000);
  }

  private updateTimerDisplay(): void {
    if (!this.state?.holdExpiresAt) return;
    const diff = this.state.holdExpiresAt.getTime() - Date.now();
    if (diff <= 0) {
      this.holdTimeRemaining = '00:00';
      clearInterval(this.timerInterval);
      this.toastr.warning('Your seat holds have expired.', 'Hold Expired');
      this.bookingService.clear();
      const eventId = this.route.snapshot.paramMap.get('id');
      this.router.navigate(eventId ? ['/events', eventId] : ['/events']);
      return;
    }
    const m = Math.floor(diff / 60000);
    const s = Math.floor((diff % 60000) / 1000);
    this.holdTimeRemaining = `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  }

  get subtotal(): number {
    return this.state?.totalPrice ?? 0;
  }

  get convenienceFee(): number {
    return Math.round(this.subtotal * 0.02 * 100) / 100;
  }

  get grandTotal(): number {
    return this.subtotal + this.convenienceFee;
  }

  onConfirmPayment(): void {
    if (!this.state) return;
    this.isProcessing = true;

    this.eventService.checkout(this.state.event.id, this.state.heldSeatIds).pipe(
      finalize(() => (this.isProcessing = false))
    ).subscribe({
      next: (order) => {
        this.toastr.success('Payment successful!', 'Order Complete');
        clearInterval(this.timerInterval);
        const orderId = order.id || order.order_id || 'success';
        this.bookingService.clear();
        this.router.navigate(['/orders', orderId, 'confirmation']);
      },
      error: (err) => {
        this.toastr.error(err.error?.message || 'Checkout failed.', 'Error');
      },
    });
  }

  goBack(): void {
    const eventId = this.route.snapshot.paramMap.get('id');
    if (eventId) {
      this.router.navigate(['/events', eventId, 'seats']);
    }
  }

  ngOnDestroy(): void {
    if (this.timerInterval) {
      clearInterval(this.timerInterval);
    }
  }
}
