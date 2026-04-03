import { Component, OnDestroy, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { HttpClient } from '@angular/common/http';
import { ToastrService } from 'ngx-toastr';

import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { apiUrl } from '../../core/api/api-url';
import { AuthService } from '../../core/auth/auth';
import { BookingService } from '../../core/services/booking.service';
import { CheckoutService } from '../../core/services/checkout.service';
import { TicketDetail } from '../../core/services/ticket.service';

type SplitSessionStatus = 'Pending' | 'Completed' | 'Expired' | 'Refunded';

export interface SplitSharePublicDetail {
  id: string;
  order_id: string;
  amount_due: number;
  status: SplitSessionStatus;
  host_user_id: string;
  is_host_share: boolean;
  guest_name?: string;
  guest_email?: string;
  payment_token: string;
  claimed_ticket_id?: string;
  claimed_at?: string;
  event_title: string;
  event_start_time: string;
  venue_name: string;
  host_name: string;
  session_expires_at: string;
  session_status: SplitSessionStatus;
}

let razorpayScriptPromise: Promise<void> | null = null;

@Component({
  selector: 'app-split-checkout',
  standalone: true,
  imports: [CommonModule, MatButtonModule, MatIconModule, MatProgressSpinnerModule, RouterModule],
  templateUrl: './split-checkout.html',
  styleUrls: ['./split-checkout.scss']
})
export class SplitCheckoutComponent implements OnInit, OnDestroy {
  shareDetail: SplitSharePublicDetail | null = null;
  isLoading = true;
  isProcessing = false;
  errorMsg = '';

  private token = '';
  private pollInterval: ReturnType<typeof setInterval> | null = null;

  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly http = inject(HttpClient);
  private readonly toastr = inject(ToastrService);
  private readonly authService = inject(AuthService);
  private readonly bookingService = inject(BookingService);
  private readonly checkoutService = inject(CheckoutService);

  private readonly baseApiUrl = apiUrl('/api/split');

  ngOnInit(): void {
    this.token = this.route.snapshot.paramMap.get('token') || '';
    if (!this.token) {
      this.errorMsg = 'No split token provided.';
      this.isLoading = false;
      return;
    }

    this.loadShareDetail(true);
    this.loadRazorpayScript().catch(() => {
      this.toastr.error('Could not load payment gateway.', 'Error');
    });
  }

  ngOnDestroy(): void {
    this.stopPolling();
  }

  get isExpired(): boolean {
    if (!this.shareDetail) {
      return true;
    }
    return new Date(this.shareDetail.session_expires_at).getTime() < Date.now();
  }

  get currentUser() {
    return this.authService.currentUserValue;
  }

  get isClaimReady(): boolean {
    return !!this.shareDetail
      && this.shareDetail.status === 'Completed'
      && this.shareDetail.session_status === 'Completed'
      && !this.shareDetail.is_host_share;
  }

  get isBookingOwnerViewing(): boolean {
    return !!this.shareDetail
      && !!this.currentUser
      && this.currentUser.id === this.shareDetail.host_user_id;
  }

  get canShowClaimButton(): boolean {
    return this.isClaimReady && !this.isAlreadyClaimed && !this.isBookingOwnerViewing;
  }

  get shouldShowClaimPanel(): boolean {
    return this.isClaimReady;
  }

  get isAlreadyClaimed(): boolean {
    return !!this.shareDetail?.claimed_ticket_id;
  }

  get loginUrl(): string {
    return `/login?returnUrl=${encodeURIComponent(this.getClaimReturnUrl())}`;
  }

  get registerUrl(): string {
    return `/register?returnUrl=${encodeURIComponent(this.getClaimReturnUrl())}`;
  }

  get claimReturnUrl(): string {
    return this.getClaimReturnUrl();
  }

  onPay(): void {
    if (!this.shareDetail || this.isProcessing) {
      return;
    }

    this.isProcessing = true;

    this.http.post<any>(`${this.baseApiUrl}/${this.shareDetail.payment_token}/checkout`, {}).subscribe({
      next: (session) => {
        this.openRazorpayCheckout(session);
      },
      error: (err) => {
        this.isProcessing = false;
        this.toastr.error(err.error?.message || 'Unable to start checkout.', 'Error');
      }
    });
  }

  getLinkForShare(token: string): string {
    return `${window.location.origin}/split/${token}`;
  }

  claimTickets(): void {
    if (
      !this.shareDetail ||
      !this.currentUser ||
      !this.canShowClaimButton ||
      this.isProcessing
    ) {
      return;
    }

    this.isProcessing = true;

    this.http.post<TicketDetail>(`${this.baseApiUrl}/${this.shareDetail.payment_token}/claim`, {}).subscribe({
      next: (ticket) => {
        this.isProcessing = false;
        if (this.shareDetail) {
          this.shareDetail.claimed_ticket_id = ticket.id;
          this.shareDetail.claimed_at = new Date().toISOString();
        }
        this.toastr.success('Your ticket has been claimed into your account.', 'Claimed');
        void this.router.navigate(['/tickets', ticket.id]);
      },
      error: (err) => {
        this.isProcessing = false;
        const message = err.error?.message || 'Unable to claim this split ticket right now.';
        this.toastr.error(message, 'Claim Failed');
        this.loadShareDetail(false);
      }
    });
  }

  private loadShareDetail(showSpinner: boolean): void {
    if (showSpinner) {
      this.isLoading = true;
    }

    this.http.get<SplitSharePublicDetail>(`${this.baseApiUrl}/${this.token}`).subscribe({
      next: (detail) => {
        this.shareDetail = detail;
        this.errorMsg = '';
        this.isLoading = false;
        this.syncPolling();
        this.tryAutoClaim();
      },
      error: (err) => {
        this.errorMsg = err.error?.message || 'This split link is invalid or has expired.';
        this.isLoading = false;
        this.stopPolling();
      }
    });
  }

  private syncPolling(): void {
    if (!this.shareDetail) {
      this.stopPolling();
      return;
    }

    if (this.shareDetail.session_status === 'Pending' && !this.isExpired) {
      this.startPolling();
      return;
    }

    this.stopPolling();
  }

  private startPolling(): void {
    if (this.pollInterval) {
      return;
    }

    this.pollInterval = setInterval(() => {
      this.loadShareDetail(false);
    }, 5000);
  }

  private stopPolling(): void {
    if (!this.pollInterval) {
      return;
    }

    clearInterval(this.pollInterval);
    this.pollInterval = null;
  }

  private openRazorpayCheckout(session: any): void {
    let handled = false;

    const checkout = new window.Razorpay({
      key: session.gateway_key_id,
      amount: session.amount,
      currency: session.currency,
      name: 'GrabAPass',
      description: `Share of Order from ${this.shareDetail?.host_name}`,
      order_id: session.gateway_order_id,
      prefill: {
        name: session.customer_name,
        email: session.customer_email,
      },
      theme: { color: '#ffd60a' },
      modal: {
        ondismiss: () => {
          if (handled) {
            return;
          }
          handled = true;
          this.isProcessing = false;
          this.toastr.info('Payment window closed.', 'Cancelled');
        }
      },
      handler: (response: any) => {
        if (handled) {
          return;
        }

        handled = true;
        this.isProcessing = true;
        this.toastr.success('Payment received! Confirming...', 'Success');

        this.http.post<{ session_status: SplitSessionStatus; order_id: string }>(
          `${this.baseApiUrl}/${this.shareDetail!.payment_token}/verify`,
          {
            razorpay_payment_id: response.razorpay_payment_id,
            razorpay_order_id: response.razorpay_order_id,
          }
        ).subscribe({
          next: (result) => {
            this.isProcessing = false;

            if (this.shareDetail) {
              this.shareDetail.status = 'Completed';
              this.shareDetail.session_status = result.session_status;
            }

            if (result.session_status === 'Completed') {
              this.toastr.success('All shares are paid. Finalizing tickets...', 'Complete');
              this.stopPolling();
              if (this.shareDetail?.is_host_share) {
                this.bookingService.clear();
                this.tryHostNavigation(result.order_id, 'tickets');
              } else if (this.currentUser && !this.isBookingOwnerViewing) {
                this.claimTickets();
              } else {
                this.loadShareDetail(false);
              }
              return;
            }

            this.toastr.info('Your share is paid. Waiting for the remaining payments.', 'Paid');
            this.tryHostNavigation(result.order_id, 'dashboard', false);
            this.loadShareDetail(false);
          },
          error: (err) => {
            this.isProcessing = false;
            this.toastr.info(
              err.error?.message || 'Payment was received. Refreshing the split status now.',
              'Refreshing'
            );
            this.loadShareDetail(false);
          }
        });
      }
    });

    checkout.open();
  }

  private tryHostNavigation(
    orderId: string,
    destination: 'dashboard' | 'tickets',
    showFallbackToast = true
  ): void {
    if (!this.authService.currentUserValue) {
      if (showFallbackToast) {
        this.loadShareDetail(false);
      }
      return;
    }

    this.checkoutService.getSplitSession(orderId).subscribe({
      next: () => {
        const target = destination === 'tickets' ? ['/tickets'] : ['/split-dashboard', orderId];
        void this.router.navigate(target);
      },
      error: () => {
        if (showFallbackToast) {
          this.loadShareDetail(false);
        }
      }
    });
  }

  private tryAutoClaim(): void {
    if (!this.route.snapshot.queryParamMap.has('claim')) {
      return;
    }

    if (!this.currentUser || !this.isClaimReady || this.isAlreadyClaimed || this.isProcessing) {
      return;
    }

    this.claimTickets();
  }

  private getClaimReturnUrl(): string {
    const tree = this.router.createUrlTree(['/split', this.token], {
      queryParams: { claim: 1 }
    });
    return this.router.serializeUrl(tree);
  }

  private async loadRazorpayScript(): Promise<void> {
    if (typeof window !== 'undefined' && window.Razorpay) {
      return;
    }

    if (!razorpayScriptPromise) {
      razorpayScriptPromise = new Promise<void>((resolve, reject) => {
        const script = document.createElement('script');
        script.src = 'https://checkout.razorpay.com/v1/checkout.js';
        script.async = true;
        script.onload = () => resolve();
        script.onerror = () => reject(new Error('Failed to load Razorpay checkout script.'));
        document.body.appendChild(script);
      });
    }

    return razorpayScriptPromise;
  }
}
