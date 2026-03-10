import { Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule } from '@angular/router';
import { finalize, timeout, Subject, debounceTime, distinctUntilChanged, switchMap, of } from 'rxjs';
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
  searchQuery = '';

  private readonly eventService = inject(EventService);
  private readonly router = inject(Router);
  private readonly toastr = inject(ToastrService);
  private searchSubject = new Subject<string>();

  ngOnInit(): void {
    this.fetchPublishedEvents();

    this.searchSubject.pipe(
      debounceTime(400),
      distinctUntilChanged(),
    ).subscribe(query => {
      this.searchQuery = query;
      this.fetchPublishedEvents(query);
    });
  }

  fetchPublishedEvents(search?: string): void {
    this.loading = true;

    this.eventService.getPublishedEvents(undefined, search).pipe(
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

  onSearch(event: globalThis.Event): void {
    const value = (event.target as HTMLInputElement).value;
    this.searchSubject.next(value);
  }

  goToEvent(eventId: string): void {
    this.router.navigate(['/events', eventId]);
  }
}
