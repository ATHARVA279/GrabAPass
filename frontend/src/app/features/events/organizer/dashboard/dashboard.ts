import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule } from '@angular/router';
import { timeout } from 'rxjs';
import { ToastrService } from 'ngx-toastr';

import { MatTableModule } from '@angular/material/table';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';

import { AuthService, User } from '../../../../core/auth/auth';
import { EventService } from '../../../../core/services/event.service';
import { Event } from '../../../../shared/models/event';

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
  events: Event[] = [];
  displayedColumns: string[] = ['title', 'category', 'status', 'start_time', 'actions'];
  viewState: 'loading' | 'error' | 'success' = 'loading';

  private readonly authService = inject(AuthService);
  private readonly eventService = inject(EventService);
  private readonly router = inject(Router);
  private readonly toastr = inject(ToastrService);

  ngOnInit(): void {
    this.user = this.authService.currentUserValue;
    this.loadDashboard();
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

    this.eventService.getOrganizerEvents().pipe(
      timeout(10000)
    ).subscribe({
      next: (events) => {
        this.events = events;
        this.viewState = 'success';
      },
      error: (err) => {
        this.events = [];

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

  logout(): void {
    this.authService.logout();
    this.router.navigate(['/login']);
  }
}
