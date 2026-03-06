import { ChangeDetectorRef, Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule } from '@angular/router';
import { finalize, timeout } from 'rxjs';

import { AuthService, User } from '../../../core/auth/auth';
import { EventService } from '../../../core/services/event.service';
import { Event } from '../../../shared/models/event';

@Component({
  selector: 'app-home',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './home.html',
  styleUrls: ['./home.scss']
})
export class Home implements OnInit {
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
    });

    this.fetchPublishedEvents();
  }

  fetchPublishedEvents(): void {
    this.loading = true;
    this.error = '';

    this.eventService.getPublishedEvents().pipe(
      timeout(10000),
      finalize(() => {
        this.loading = false;
      })
    ).subscribe({
      next: (events) => {
        this.events = events;
        this.cdr.detectChanges();
      },
      error: (err) => {
        this.error = err?.name === 'TimeoutError'
          ? 'Loading events timed out. Check backend server status.'
          : typeof err?.error === 'string'
            ? err.error
            : 'Failed to load events from the server.';
        this.cdr.detectChanges();
      }
    });
  }

  logout(): void {
    this.authService.logout();
    this.router.navigate(['/login']);
  }

  goToDashboard(): void {
    this.router.navigate(['/organizer']);
  }
}
