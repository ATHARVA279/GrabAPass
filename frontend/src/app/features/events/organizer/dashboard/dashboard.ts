import { ChangeDetectorRef, Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule } from '@angular/router';
import { timeout } from 'rxjs';

import { AuthService, User } from '../../../../core/auth/auth';
import { EventService } from '../../../../core/services/event.service';
import { Event } from '../../../../shared/models/event';

@Component({
  selector: 'app-dashboard',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './dashboard.html',
  styleUrls: ['./dashboard.scss']
})
export class Dashboard implements OnInit {
  user: User | null = null;
  events: Event[] = [];
  loading = true;
  error = '';

  constructor(
    private readonly authService: AuthService,
    private readonly eventService: EventService,
    private readonly router: Router,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.authService.currentUser$.subscribe((user) => {
      this.user = user;
      this.cdr.detectChanges();

      if (user) {
        this.fetchMyEvents();
        return;
      }

      this.loading = false;
      this.error = 'No active organizer session. Please login again.';
      this.cdr.detectChanges();
    });
  }

  fetchMyEvents(): void {
    this.loading = true;
    this.error = '';

    this.eventService.getOrganizerEvents().pipe(
      timeout(10000)
    ).subscribe({
      next: (events) => {
        this.events = events;
        this.loading = false;
        this.cdr.detectChanges();
      },
      error: (err) => {
        this.loading = false;

        if (err.name === 'TimeoutError') {
          this.error = 'Request timed out while loading events. Check if backend is running on port 3000.';
          this.cdr.detectChanges();
          return;
        }

        if (err.status === 401 || err.status === 403) {
          this.error = 'Your session is invalid or expired. Please login again.';
          this.cdr.detectChanges();
          this.authService.logout();
          this.router.navigate(['/login']);
          return;
        }

        this.error = typeof err.error === 'string'
          ? err.error
          : err.error?.message || 'Failed to load organizer events.';
        this.cdr.detectChanges();
      }
    });
  }

  logout(): void {
    this.authService.logout();
    this.router.navigate(['/login']);
  }
}
