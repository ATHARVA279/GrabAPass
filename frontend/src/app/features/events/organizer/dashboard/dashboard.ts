import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule } from '@angular/router';
import { Subject, Subscription, interval, startWith, switchMap, takeUntil, timeout } from 'rxjs';
import { ToastrService } from 'ngx-toastr';

import { MatTableModule } from '@angular/material/table';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';

import { AuthService, User } from '../../../../core/auth/auth';
import { EventService } from '../../../../core/services/event.service';
import { OrganizerDashboardSummaryResponse } from '../../../../shared/models/event';

@Component({
  selector: 'app-dashboard',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    MatTableModule,
    MatButtonModule,
    MatIconModule,
    MatProgressSpinnerModule
  ],
  templateUrl: './dashboard.html',
  styleUrls: ['./dashboard.scss']
})
export class Dashboard implements OnInit {
  user: User | null = null;
  summary: OrganizerDashboardSummaryResponse | null = null;
  displayedColumns: string[] = ['title', 'sales', 'inventory', 'scans', 'actions'];
  viewState: 'loading' | 'error' | 'success' = 'loading';
  lastUpdatedAt: Date | null = null;

  private readonly destroy$ = new Subject<void>();
  private refreshSubscription?: Subscription;

  private readonly authService = inject(AuthService);
  private readonly eventService = inject(EventService);
  private readonly router = inject(Router);
  private readonly toastr = inject(ToastrService);

  ngOnInit(): void {
    this.user = this.authService.currentUserValue;
    this.loadDashboard();
  }

  ngOnDestroy(): void {
    this.refreshSubscription?.unsubscribe();
    this.destroy$.next();
    this.destroy$.complete();
  }

  get loading(): boolean {
    return this.viewState === 'loading';
  }

  loadDashboard(): void {
    this.viewState = 'loading';

    if (!this.user) {
      this.router.navigate(['/login']);
      return;
    }

    this.refreshSubscription?.unsubscribe();

    this.refreshSubscription = interval(30000).pipe(
      startWith(0),
      switchMap(() => this.eventService.getOrganizerDashboardSummary().pipe(timeout(10000))),
      takeUntil(this.destroy$)
    ).subscribe({
      next: (events) => {
        this.summary = events;
        this.lastUpdatedAt = new Date();
        this.viewState = 'success';
      },
      error: (err) => {
        this.summary = null;

        if (err.name === 'TimeoutError') {
          this.toastr.error('Request timed out. Check if the backend is running.', 'Timeout');
        } else if (err.status === 401 || err.status === 403) {
          this.authService.logout();
          this.router.navigate(['/login']);
          return;
        } else {
          const msg = err instanceof Error
            ? err.message
            : typeof err.error === 'string'
              ? err.error
              : (err.error?.message ?? 'Failed to load your events.');
          this.toastr.error(msg, 'Error');
        }

        this.viewState = 'error';
      }
    });
  }

  get occupancyPercent(): number {
    if (!this.summary?.seats_total) return 0;
    return Math.round((this.summary.tickets_sold / this.summary.seats_total) * 100);
  }

  getEventOccupancyPercent(event: OrganizerDashboardSummaryResponse['events'][number]): number {
    if (!event.seats_total) return 0;
    return Math.round((event.tickets_sold / event.seats_total) * 100);
  }

  editEvent(eventId: string): void {
    this.router.navigate(['/organizer/events', eventId, 'edit']);
  }

  deleteEvent(eventId: string, title: string): void {
    const confirmed = window.confirm(`Delete "${title}"? This cannot be undone.`);
    if (!confirmed) return;

    this.eventService.deleteEvent(eventId).subscribe({
      next: () => {
        this.toastr.success('Event deleted successfully.', 'Deleted');
        this.loadDashboard();
      },
      error: (err) => {
        const msg = err.error?.message || 'Failed to delete event.';
        this.toastr.error(msg, 'Error');
      }
    });
  }

  logout(): void {
    this.authService.logout();
    this.router.navigate(['/login']);
  }
}
