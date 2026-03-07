import { Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule } from '@angular/router';
import { finalize, timeout } from 'rxjs';
import { ToastrService } from 'ngx-toastr';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';

import { EventService } from '../../../core/services/event.service';
import { Event } from '../../../shared/models/event';

@Component({
  selector: 'app-home',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    MatCardModule,
    MatButtonModule,
    MatIconModule,
    MatProgressSpinnerModule
  ],
  templateUrl: './home.html',
  styleUrls: ['./home.scss']
})
export class Home implements OnInit {
  events: Event[] = [];
  loading = true;

  private readonly eventService = inject(EventService);
  private readonly router = inject(Router);
  private readonly toastr = inject(ToastrService);

  ngOnInit(): void {
    this.fetchPublishedEvents();
  }

  fetchPublishedEvents(): void {
    this.loading = true;

    this.eventService.getPublishedEvents().pipe(
      timeout(10000),
      finalize(() => (this.loading = false))
    ).subscribe({
      next: (events) => (this.events = events),
      error: (err) => {
        const msg = err?.name === 'TimeoutError'
          ? 'Loading events timed out. Check backend server status.'
          : typeof err?.error === 'string'
            ? err.error
            : 'Failed to load events from the server.';
        this.toastr.error(msg, 'Error');
      }
    });
  }

  goToEvent(eventId: string): void {
    this.router.navigate(['/events', eventId]);
  }
}
