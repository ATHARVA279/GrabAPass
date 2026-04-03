import { Component, inject, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router } from '@angular/router';
import { finalize } from 'rxjs';
import { ToastrService } from 'ngx-toastr';

import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { ClipboardModule, Clipboard } from '@angular/cdk/clipboard';

import { BookingService } from '../../core/services/booking.service';
import { CheckoutService, SplitSession, SplitShare } from '../../core/services/checkout.service';

@Component({
  selector: 'app-split-dashboard',
  standalone: true,
  imports: [CommonModule, MatButtonModule, MatIconModule, MatProgressSpinnerModule, ClipboardModule],
  templateUrl: './split-dashboard.html',
  styleUrls: ['./split-dashboard.scss']
})
export class SplitDashboardComponent implements OnInit, OnDestroy {
  session: SplitSession | null = null;
  isLoading = true;
  errorMsg = '';
  timerInterval: any;
  pollInterval: any;
  timeRemaining = '';
  private hasNavigatedOnComplete = false;

  private orderId = '';
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly bookingService = inject(BookingService);
  private readonly checkoutService = inject(CheckoutService);
  private readonly toastr = inject(ToastrService);
  private readonly clipboard = inject(Clipboard);

  ngOnInit(): void {
    this.orderId = this.route.snapshot.paramMap.get('id') || '';
    if (!this.orderId) {
      this.errorMsg = 'No Order ID provided.';
      this.isLoading = false;
      return;
    }
    this.loadSession();
  }

  private loadSession(): void {
    this.checkoutService.getSplitSession(this.orderId).subscribe({
      next: (session: SplitSession) => {
        this.session = session;
        this.isLoading = false;
        if (session.status === 'Completed') {
          this.handleCompletedSession();
          return;
        }
        if (session.status === 'Pending') {
          this.startTimer();
          this.startPolling();
        }
      },
      error: (err: any) => {
        this.errorMsg = err.error?.message || 'Could not fetch split session';
        this.isLoading = false;
      }
    });
  }

  private startPolling(): void {
    if (this.pollInterval) return;
    this.pollInterval = setInterval(() => {
      this.checkoutService.getSplitSession(this.orderId).subscribe({
        next: (session: SplitSession) => {
          this.session = session;
          if (session.status === 'Completed') {
            this.toastr.success('All shares paid. Opening your tickets now.', 'Complete');
            this.handleCompletedSession();
          } else if (session.status === 'Expired') {
            this.stopPolling();
          }
        },
        error: () => {} // silently ignore poll errors
      });
    }, 5000);
  }

  private stopPolling(): void {
    if (this.pollInterval) {
      clearInterval(this.pollInterval);
      this.pollInterval = null;
    }
  }

  ngOnDestroy(): void {
    if (this.timerInterval) clearInterval(this.timerInterval);
    this.stopPolling();
  }

  get completedShares(): number {
    return this.session?.shares?.filter((s: SplitShare) => s.status === 'Completed').length || 0;
  }

  get totalShares(): number {
    return this.session?.shares?.length || 0;
  }

  get isFullyPaid(): boolean {
    return this.completedShares === this.totalShares && this.totalShares > 0;
  }

  get isExpired(): boolean {
    if (!this.session) return false;
    return new Date(this.session.expires_at).getTime() < Date.now();
  }

  isHostShare(share: SplitShare): boolean {
    return share.is_host_share;
  }

  getShareLabel(share: SplitShare, index: number): string {
    if (this.isHostShare(share)) {
      return 'Host Share';
    }

    const guestName = share.guest_name?.trim();
    if (guestName) {
      return guestName;
    }

    return `Guest Share #${index + 1}`;
  }

  getShareShortId(share: SplitShare): string {
    return share.id.slice(0, 8).toUpperCase();
  }

  getClaimStatus(share: SplitShare, index: number): string | null {
    if (this.isHostShare(share)) {
      return 'Stays with host account';
    }

    if (share.claimed_at) {
      return 'Claimed';
    }

    if (share.status === 'Completed' && this.session?.status === 'Completed') {
      return 'Ready to claim';
    }

    return null;
  }

  canShowGuestLink(share: SplitShare, index: number): boolean {
    if (this.isHostShare(share)) {
      return false;
    }

    if (this.session?.status === 'Completed') {
      return true;
    }

    return share.status === 'Pending' && !this.isExpired;
  }

  getGuestLinkLabel(): string {
    return this.session?.status === 'Completed' ? 'Copy Claim Link' : 'Copy Payment Link';
  }

  private startTimer(): void {
    this.updateTimerDisplay();
    this.timerInterval = setInterval(() => this.updateTimerDisplay(), 1000);
  }

  private updateTimerDisplay(): void {
    if (!this.session?.expires_at) return;
    const diff = new Date(this.session.expires_at).getTime() - Date.now();
    
    if (diff <= 0) {
      this.timeRemaining = '00:00';
      clearInterval(this.timerInterval);
      if (!this.isFullyPaid) {
        this.session.status = 'Expired';
      }
      return;
    }
    
    const m = Math.floor(diff / 60000);
    const s = Math.floor((diff % 60000) / 1000);
    this.timeRemaining = `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  }

  getLinkForShare(token: string): string {
    return `${window.location.origin}/split/${token}`;
  }

  copyLink(token: string): void {
    const link = this.getLinkForShare(token);
    this.clipboard.copy(link);
    this.toastr.success('Magic link copied to clipboard!');
  }

  payMyShare(token: string): void {
    // Navigate to the public split-checkout screen for this token so the host can pay their share easily.
    this.router.navigate(['/split', token]);
  }

  private handleCompletedSession(): void {
    this.stopPolling();
    this.bookingService.clear();

    if (this.hasNavigatedOnComplete) {
      return;
    }

    this.hasNavigatedOnComplete = true;
    void this.router.navigate(['/tickets']);
  }
}
