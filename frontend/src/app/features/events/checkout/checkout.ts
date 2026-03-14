import { Component, inject, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule, ActivatedRoute } from '@angular/router';
import { finalize } from 'rxjs';
import { ToastrService } from 'ngx-toastr';

import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';

import { BookingService, BookingState } from '../../../core/services/booking.service';
import {
  CheckoutInitialization,
  EventService,
} from '../../../core/services/event.service';

interface RazorpaySuccessResponse {
  razorpay_payment_id: string;
  razorpay_order_id: string;
  razorpay_signature: string;
}

interface RazorpayFailureResponse {
  error?: {
    description?: string;
    reason?: string;
    metadata?: {
      payment_id?: string;
      order_id?: string;
    };
  };
}

interface RazorpayInstance {
  open(): void;
  on(eventName: 'payment.failed', handler: (response: RazorpayFailureResponse) => void): void;
}

interface RazorpayOptions {
  key: string;
  amount: number;
  currency: string;
  name: string;
  description: string;
  order_id: string;
  prefill?: {
    name?: string;
    email?: string;
  };
  notes?: Record<string, string>;
  theme?: {
    color?: string;
  };
  modal?: {
    ondismiss?: () => void;
  };
  handler: (response: RazorpaySuccessResponse) => void;
}

declare global {
  interface Window {
    Razorpay: new (options: RazorpayOptions) => RazorpayInstance;
  }
}

let razorpayScriptPromise: Promise<void> | null = null;

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
    if (!this.state || this.isProcessing) return;
    this.isProcessing = true;

    this.eventService.initializeCheckout(this.state.event.id, this.state.heldSeatIds).subscribe({
      next: async (session) => {
        this.bookingService.setHoldData(this.state!.heldSeatIds, new Date(session.hold_expires_at));
        this.state = this.bookingService.getState();
        try {
          await this.loadRazorpayScript();
          this.openRazorpayCheckout(session);
        } catch (error) {
          this.isProcessing = false;
          this.reportCheckoutFailure(
            session,
            'Unable to load the secure payment form. Please try again.'
          );
          this.toastr.error('Unable to load Razorpay checkout.', 'Payment Unavailable');
        }
      },
      error: (err) => {
        this.isProcessing = false;
        this.toastr.error(err.error?.message || 'Could not start checkout.', 'Error');
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

  private async loadRazorpayScript(): Promise<void> {
    if (typeof window !== 'undefined' && window.Razorpay) {
      return;
    }

    if (!razorpayScriptPromise) {
      razorpayScriptPromise = new Promise<void>((resolve, reject) => {
        const existingScript = document.querySelector<HTMLScriptElement>(
          'script[data-razorpay-checkout="true"]'
        );
        if (existingScript) {
          existingScript.addEventListener('load', () => resolve(), { once: true });
          existingScript.addEventListener(
            'error',
            () => reject(new Error('Failed to load Razorpay checkout script.')),
            { once: true }
          );
          return;
        }

        const script = document.createElement('script');
        script.src = 'https://checkout.razorpay.com/v1/checkout.js';
        script.async = true;
        script.dataset['razorpayCheckout'] = 'true';
        script.onload = () => resolve();
        script.onerror = () => reject(new Error('Failed to load Razorpay checkout script.'));
        document.body.appendChild(script);
      }).catch((error) => {
        razorpayScriptPromise = null;
        throw error;
      });
    }

    return razorpayScriptPromise;
  }

  private openRazorpayCheckout(session: CheckoutInitialization): void {
    let handled = false;

    const checkout = new window.Razorpay({
      key: session.gateway_key_id,
      amount: session.amount,
      currency: session.currency,
      name: 'GrabAPass',
      description: session.description,
      order_id: session.gateway_order_id,
      prefill: {
        name: session.customer_name,
        email: session.customer_email,
      },
      notes: {
        local_order_id: session.order.id,
      },
      theme: {
        color: '#ffd60a',
      },
      modal: {
        ondismiss: () => {
          if (handled) return;
          handled = true;
          this.isProcessing = false;
          this.reportCheckoutFailure(session, 'Payment window was closed before completion.');
          this.toastr.info('Payment was not completed.', 'Checkout Closed');
        },
      },
      handler: (response) => {
        if (handled) return;
        handled = true;
        this.verifyPayment(session, response);
      },
    });

    checkout.on('payment.failed', (response) => {
      if (handled) return;
      handled = true;
      this.isProcessing = false;
      this.reportCheckoutFailure(
        session,
        response.error?.description || response.error?.reason || 'Payment failed.',
        response.error?.metadata?.payment_id
      );
      this.toastr.error(
        response.error?.description || 'Payment failed before verification.',
        'Payment Failed'
      );
    });

    checkout.open();
  }

  private verifyPayment(
    session: CheckoutInitialization,
    response: RazorpaySuccessResponse
  ): void {
    this.eventService
      .verifyCheckout(this.state!.event.id, {
        order_id: session.order.id,
        razorpay_order_id: response.razorpay_order_id,
        razorpay_payment_id: response.razorpay_payment_id,
        razorpay_signature: response.razorpay_signature,
      })
      .pipe(finalize(() => (this.isProcessing = false)))
      .subscribe({
        next: (order) => {
          this.toastr.success('Payment successful and tickets confirmed.', 'Order Complete');
          clearInterval(this.timerInterval);
          this.bookingService.clear();
          this.router.navigate(['/tickets']);
        },
        error: (err) => {
          const message =
            err.error?.message ||
            'Payment was authorized, but ticket confirmation failed. Please contact support.';
          this.toastr.error(message, 'Verification Failed');
          this.router.navigate(['/tickets']);
        },
      });
  }

  private reportCheckoutFailure(
    session: CheckoutInitialization,
    reason: string,
    paymentId?: string
  ): void {
    this.eventService
      .recordCheckoutFailure(this.state!.event.id, {
        order_id: session.order.id,
        razorpay_order_id: session.gateway_order_id,
        razorpay_payment_id: paymentId,
        reason,
      })
      .subscribe({ error: () => undefined });
  }
}
