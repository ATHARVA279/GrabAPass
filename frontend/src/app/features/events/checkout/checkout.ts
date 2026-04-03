import { Component, inject, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Router, RouterModule, ActivatedRoute } from '@angular/router';
import { finalize } from 'rxjs';
import { HttpErrorResponse } from '@angular/common/http';
import { ToastrService } from 'ngx-toastr';

import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';

import { BookingService, BookingState } from '../../../core/services/booking.service';
import {
  CheckoutService,
  CheckoutInitialization,
} from '../../../core/services/checkout.service';

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

interface SplitShareDraft {
  label: string;
  guest_name: string;
  guest_email: string;
}

let razorpayScriptPromise: Promise<void> | null = null;

@Component({
  selector: 'app-checkout',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    RouterModule,
    MatButtonModule,
    MatIconModule,
    MatProgressSpinnerModule,
  ],
  templateUrl: './checkout.html',
  styleUrls: ['./checkout.scss'],
})
export class Checkout implements OnInit, OnDestroy {
  private readonly shareAccentPalette = [
    '#ffd60a',
    '#4ade80',
    '#38bdf8',
    '#fb7185',
    '#c084fc',
    '#f59e0b',
    '#22c55e',
    '#f97316',
  ];

  state: BookingState | null = null;
  isProcessing = false;
  holdTimeRemaining = '';
  private timerInterval: any;

  private readonly router = inject(Router);
  private readonly route = inject(ActivatedRoute);
  private readonly bookingService = inject(BookingService);
  private readonly checkoutService = inject(CheckoutService);
  private readonly toastr = inject(ToastrService);

  ngOnInit(): void {
    this.state = this.bookingService.getState();
    if (!this.state || !this.state.holdExpiresAt) {
      this.toastr.error('No active booking session. Please select seats first.', 'Error');
      const eventId = this.route.snapshot.paramMap.get('id');
      this.router.navigate(eventId ? ['/events', eventId, 'seats'] : ['/events']);
      return;
    }

    if (this.state.orderId) {
      this.checkoutService.getSplitSession(this.state.orderId).subscribe({
        next: (session) => {
          if (session.status === 'Pending') {
            this.router.navigate(['/split-dashboard', this.state!.orderId]);
            return;
          }

          this.bookingService.clear();
          this.state = null;

          if (session.status === 'Completed') {
            this.toastr.info('Your last split checkout is already complete. Start a new selection to buy again.', 'Checkout Closed');
            this.router.navigate(['/tickets']);
            return;
          }

          const eventId = this.route.snapshot.paramMap.get('id');
          this.toastr.info('Your previous split checkout is no longer active. Please choose tickets again.', 'Checkout Reset');
          this.router.navigate(eventId ? ['/events', eventId] : ['/events']);
        },
        error: (_err: HttpErrorResponse) => {
          // 404 or any error: continue rendering normal checkout
          this.startTimer();
        }
      });
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

  get totalTicketCount(): number {
    const seatCount = this.state?.selectedSeats.length ?? 0;
    const tierCount = this.state?.selectedTiers.reduce((sum, tier) => sum + tier.quantity, 0) ?? 0;
    return seatCount + tierCount;
  }

  // --- Split & Pay ---
  isSplitEnabled = false;
  splitShares = 2;
  splitSession: any = null;
  shareDrafts: SplitShareDraft[] = [];
  activeShareIndex = 0;
  seatAssignments: Record<string, number> = {};
  tierAssignments: Record<string, number[]> = {};

  toggleSplit(event: any): void {
    if (!this.state || this.isProcessing) return;
    this.isSplitEnabled = event.target.checked;
    if (this.isSplitEnabled) {
      this.initializeSplitAssignments();
      return;
    }

    this.activeShareIndex = 0;
    this.seatAssignments = {};
    this.tierAssignments = {};
  }

  incrementShares(): void {
    if (this.splitShares < this.totalTicketCount) {
      this.splitShares++;
      this.syncShareDrafts();
      this.reconcileAssignmentsAfterShareCountChange();
      this.activeShareIndex = this.splitShares - 1;
      this.seedShare(this.activeShareIndex);
    }
  }

  decrementShares(): void {
    if (this.splitShares > 2) {
      this.splitShares--;
      this.syncShareDrafts();
      this.reconcileAssignmentsAfterShareCountChange();
      if (this.activeShareIndex >= this.splitShares) {
        this.activeShareIndex = this.splitShares - 1;
      }
    }
  }

  onConfirmPayment(): void {
    if (!this.state || this.isProcessing) return;
    this.isProcessing = true;

    this.checkoutService.initializeCheckout(this.state.event.id, this.state.holdIds).subscribe({
      next: async (session) => {
        this.bookingService.setHoldData(this.state!.holdIds, new Date(session.hold_expires_at));
        this.state = this.bookingService.getState();
        this.bookingService.setOrderId(session.order.id);

        if (this.isSplitEnabled) {
          if (!this.hasValidSplitAssignments()) {
            this.isProcessing = false;
            return;
          }

          this.checkoutService.initializeSplit(session.order.id, {
            split_type: 'Custom',
            custom_shares: this.buildCustomSharePayload(),
          }).subscribe({
            next: (splitSession) => {
              this.isProcessing = false;
              this.toastr.success('Split initiated!', 'Success');
              this.router.navigate(['/split-dashboard', session.order.id]);
            },
            error: (err) => {
              this.isProcessing = false;
              this.toastr.error(err.error?.message || 'Could not start split session.', 'Error');
            }
          });
          return;
        }

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
      if ((this.state?.selectedSeats.length ?? 0) > 0) {
        this.router.navigate(['/events', eventId, 'seats']);
      } else {
        this.router.navigate(['/events', eventId]);
      }
    }
  }

  ngOnDestroy(): void {
    if (this.timerInterval) {
      clearInterval(this.timerInterval);
    }
  }

  get shareIndices(): number[] {
    return Array.from({ length: this.splitShares }, (_, index) => index);
  }

  get hasReservedSeats(): boolean {
    return (this.state?.selectedSeats.length ?? 0) > 0;
  }

  get hasGeneralAdmission(): boolean {
    return (this.state?.selectedTiers.length ?? 0) > 0;
  }

  get activeShareLabel(): string {
    return this.shareDrafts[this.activeShareIndex]?.label ?? 'Selected Share';
  }

  getSplitReadyToSubmit(): boolean {
    return this.getSplitValidationError() === null;
  }

  getSplitAssignmentStatusMessage(): string {
    const validationError = this.getSplitValidationError();
    if (validationError) {
      return validationError;
    }

    return `Ready to create ${this.splitShares} secure payment link${this.splitShares === 1 ? '' : 's'}.`;
  }

  setActiveShare(index: number): void {
    this.activeShareIndex = index;
  }

  getShareAccent(index: number): string {
    return this.shareAccentPalette[index % this.shareAccentPalette.length];
  }

  getShareSeatCount(index: number): number {
    return this.state?.selectedSeats.filter((seat) => this.getSeatShareIndex(seat.seatId) === index).length ?? 0;
  }

  getShareTierCount(index: number): number {
    return this.state?.selectedTiers.reduce(
      (sum, tier) => sum + this.getTierQuantity(tier.ticketTierId, index),
      0
    ) ?? 0;
  }

  getShareTotal(index: number): number {
    const seatTotal = this.state?.selectedSeats.reduce(
      (sum, seat) => sum + (this.getSeatShareIndex(seat.seatId) === index ? seat.price : 0),
      0
    ) ?? 0;
    const tierTotal = this.state?.selectedTiers.reduce(
      (sum, tier) => sum + this.getTierQuantity(tier.ticketTierId, index) * tier.unitPrice,
      0
    ) ?? 0;

    return seatTotal + tierTotal;
  }

  getShareTicketCount(index: number): number {
    return this.getShareSeatCount(index) + this.getShareTierCount(index);
  }

  getShareEstimatedTotal(index: number): number {
    const subtotal = this.getShareTotal(index);
    if (!this.subtotal) {
      return 0;
    }

    return subtotal + (subtotal / this.subtotal) * this.convenienceFee;
  }

  getShareDescription(index: number): string {
    if (index === 0) {
      return 'Stays in your wallet until you choose to transfer it later.';
    }

    const email = this.shareDrafts[index]?.guest_email?.trim();
    return email ? `Claim link will be tied to ${email}.` : 'Add the guest email that should be allowed to claim this share.';
  }

  assignSeatToActiveShare(seatId: string): void {
    const currentShareIndex = this.getSeatShareIndex(seatId);
    if (currentShareIndex === this.activeShareIndex) {
      const nextAssignments = { ...this.seatAssignments };
      delete nextAssignments[seatId];
      this.seatAssignments = nextAssignments;
      return;
    }

    this.seatAssignments = {
      ...this.seatAssignments,
      [seatId]: this.activeShareIndex,
    };
  }

  isSeatAssignedToActiveShare(seatId: string): boolean {
    return this.getSeatShareIndex(seatId) === this.activeShareIndex;
  }

  getSeatShareIndex(seatId: string): number | null {
    return this.seatAssignments[seatId] ?? null;
  }

  getSeatAssignmentLabel(seatId: string): string {
    const shareIndex = this.getSeatShareIndex(seatId);
    return shareIndex === null ? 'Unassigned' : this.shareDrafts[shareIndex]?.label ?? 'Assigned';
  }

  getSeatAccent(seatId: string): string {
    const shareIndex = this.getSeatShareIndex(seatId);
    return shareIndex === null ? '#6b7280' : this.getShareAccent(shareIndex);
  }

  getTierQuantity(ticketTierId: string, shareIndex: number): number {
    return this.tierAssignments[ticketTierId]?.[shareIndex] ?? 0;
  }

  getTierRemainingQuantity(ticketTierId: string, totalQuantity: number): number {
    const assigned = (this.tierAssignments[ticketTierId] ?? []).reduce((sum, quantity) => sum + quantity, 0);
    return Math.max(0, totalQuantity - assigned);
  }

  incrementTierQuantity(ticketTierId: string, shareIndex: number, totalQuantity: number): void {
    const remaining = this.getTierRemainingQuantity(ticketTierId, totalQuantity);
    if (remaining <= 0) {
      return;
    }

    const next = [...this.getNormalizedTierQuantities(ticketTierId)];
    next[shareIndex] += 1;
    this.tierAssignments = {
      ...this.tierAssignments,
      [ticketTierId]: next,
    };
  }

  decrementTierQuantity(ticketTierId: string, shareIndex: number): void {
    const next = [...this.getNormalizedTierQuantities(ticketTierId)];
    if (next[shareIndex] <= 0) {
      return;
    }

    next[shareIndex] -= 1;
    this.tierAssignments = {
      ...this.tierAssignments,
      [ticketTierId]: next,
    };
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

  private initializeSplitAssignments(): void {
    if (!this.state) {
      return;
    }

    this.splitShares = Math.min(Math.max(2, this.splitShares), this.totalTicketCount);
    this.activeShareIndex = 0;
    this.syncShareDrafts();
    this.autoDistributeAssignments();
  }

  private syncShareDrafts(): void {
    const nextDrafts: SplitShareDraft[] = [];
    for (let index = 0; index < this.splitShares; index++) {
      const existing = this.shareDrafts[index];
      nextDrafts.push({
        label: index === 0 ? 'Host Share' : `Guest Share ${index + 1}`,
        guest_name: index === 0 ? '' : existing?.guest_name ?? '',
        guest_email: index === 0 ? '' : existing?.guest_email ?? '',
      });
    }
    this.shareDrafts = nextDrafts;
  }

  private reconcileAssignmentsAfterShareCountChange(): void {
    const nextSeatAssignments: Record<string, number> = {};
    for (const seat of this.state?.selectedSeats ?? []) {
      const currentIndex = this.seatAssignments[seat.seatId] ?? 0;
      nextSeatAssignments[seat.seatId] = currentIndex < this.splitShares ? currentIndex : 0;
    }
    this.seatAssignments = nextSeatAssignments;

    const nextTierAssignments: Record<string, number[]> = {};
    for (const tier of this.state?.selectedTiers ?? []) {
      const quantities = new Array(this.splitShares).fill(0);
      const existing = this.tierAssignments[tier.ticketTierId] ?? [];
      existing.forEach((quantity, index) => {
        const normalizedQuantity = Number.isFinite(quantity) ? Math.max(0, Math.floor(quantity)) : 0;
        const targetIndex = index < this.splitShares ? index : 0;
        quantities[targetIndex] += normalizedQuantity;
      });

      const assigned = quantities.reduce((sum, quantity) => sum + quantity, 0);
      if (assigned < tier.quantity) {
        quantities[0] += tier.quantity - assigned;
      } else if (assigned > tier.quantity) {
        let overflow = assigned - tier.quantity;
        for (let index = quantities.length - 1; index >= 0 && overflow > 0; index--) {
          const deducted = Math.min(quantities[index], overflow);
          quantities[index] -= deducted;
          overflow -= deducted;
        }
      }

      nextTierAssignments[tier.ticketTierId] = quantities;
    }
    this.tierAssignments = nextTierAssignments;
  }

  private hasValidSplitAssignments(): boolean {
    const validationError = this.getSplitValidationError();
    if (validationError) {
      this.toastr.error(validationError, 'Split Incomplete');
      return false;
    }

    return true;
  }

  private getSplitValidationError(): string | null {
    if (!this.isSplitEnabled || !this.state) {
      return null;
    }

    const unassignedSeat = this.state.selectedSeats.find((seat) => this.getSeatShareIndex(seat.seatId) === null);
    if (unassignedSeat) {
      return `Assign ${unassignedSeat.seatLabel} before continuing.`;
    }

    for (const index of this.shareIndices) {
      if (this.getShareTicketCount(index) === 0) {
        return `${this.shareDrafts[index]?.label || `Share ${index + 1}`} needs at least one ticket.`;
      }

      if (index > 0 && !this.shareDrafts[index]?.guest_email?.trim()) {
        return `Add a guest email for ${this.shareDrafts[index]?.label || `Share ${index + 1}`}.`;
      }
    }

    for (const tier of this.state.selectedTiers) {
      if (this.getTierRemainingQuantity(tier.ticketTierId, tier.quantity) !== 0) {
        return `Assign all ${tier.name} tickets before continuing.`;
      }
    }

    return null;
  }

  private buildCustomSharePayload() {
    return this.shareIndices.map((index) => {
      const tierCounts = this.state?.selectedTiers
        .map((tier) => ({
          ticket_tier_id: tier.ticketTierId,
          quantity: this.getTierQuantity(tier.ticketTierId, index),
        }))
        .filter((tier) => tier.quantity > 0) ?? [];

      return {
        guest_name: index === 0 ? undefined : this.shareDrafts[index]?.guest_name?.trim() || undefined,
        guest_email: index === 0 ? undefined : this.shareDrafts[index]?.guest_email?.trim() || undefined,
        seat_ids: this.state?.selectedSeats
          .filter((seat) => this.getSeatShareIndex(seat.seatId) === index)
          .map((seat) => seat.seatId) ?? [],
        ticket_tiers: tierCounts,
      };
    });
  }

  autoDistributeAssignments(): void {
    if (!this.state) {
      return;
    }

    const nextSeatAssignments: Record<string, number> = {};
    this.state.selectedSeats.forEach((seat, index) => {
      nextSeatAssignments[seat.seatId] = index % this.splitShares;
    });
    this.seatAssignments = nextSeatAssignments;

    const nextTierAssignments: Record<string, number[]> = {};
    for (const tier of this.state.selectedTiers) {
      const quantities = new Array(this.splitShares).fill(0);
      for (let index = 0; index < tier.quantity; index++) {
        quantities[index % this.splitShares] += 1;
      }
      nextTierAssignments[tier.ticketTierId] = quantities;
    }
    this.tierAssignments = nextTierAssignments;
  }

  resetAssignmentsToHost(): void {
    if (!this.state) {
      return;
    }

    const nextSeatAssignments: Record<string, number> = {};
    for (const seat of this.state.selectedSeats) {
      nextSeatAssignments[seat.seatId] = 0;
    }
    this.seatAssignments = nextSeatAssignments;

    const nextTierAssignments: Record<string, number[]> = {};
    for (const tier of this.state.selectedTiers) {
      const quantities = new Array(this.splitShares).fill(0);
      quantities[0] = tier.quantity;
      nextTierAssignments[tier.ticketTierId] = quantities;
    }
    this.tierAssignments = nextTierAssignments;
    this.activeShareIndex = 0;
  }

  private seedShare(index: number): void {
    const donorIndex = this.findBusiestShare(index);
    if (donorIndex === null) {
      return;
    }

    const donorSeat = this.state?.selectedSeats.find((seat) => this.getSeatShareIndex(seat.seatId) === donorIndex);
    if (donorSeat) {
      this.seatAssignments = {
        ...this.seatAssignments,
        [donorSeat.seatId]: index,
      };
      return;
    }

    for (const tier of this.state?.selectedTiers ?? []) {
      const quantities = [...this.getNormalizedTierQuantities(tier.ticketTierId)];
      if (quantities[donorIndex] > 0) {
        quantities[donorIndex] -= 1;
        quantities[index] += 1;
        this.tierAssignments = {
          ...this.tierAssignments,
          [tier.ticketTierId]: quantities,
        };
        return;
      }
    }
  }

  private findBusiestShare(excludedIndex: number): number | null {
    let busiestIndex: number | null = null;
    let busiestCount = 0;

    for (const index of this.shareIndices) {
      if (index === excludedIndex) {
        continue;
      }

      const ticketCount = this.getShareTicketCount(index);
      if (ticketCount > busiestCount) {
        busiestCount = ticketCount;
        busiestIndex = index;
      }
    }

    return busiestCount > 1 ? busiestIndex : null;
  }

  private getNormalizedTierQuantities(ticketTierId: string): number[] {
    const next = new Array(this.splitShares).fill(0);
    const existing = this.tierAssignments[ticketTierId] ?? [];
    for (let index = 0; index < Math.min(existing.length, this.splitShares); index++) {
      next[index] = Math.max(0, Math.floor(existing[index] ?? 0));
    }
    return next;
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
    this.checkoutService
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
          this.router.navigate(['/events', this.state!.event.id, 'booking-success'], { queryParams: { orderId: order.id } });
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
    this.checkoutService
      .recordCheckoutFailure(this.state!.event.id, {
        order_id: session.order.id,
        razorpay_order_id: session.gateway_order_id,
        razorpay_payment_id: paymentId,
        reason,
      })
      .subscribe({ error: () => undefined });
  }
}
