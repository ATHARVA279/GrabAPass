import { Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule } from '@angular/router';
import { finalize, timeout, Subject, debounceTime, distinctUntilChanged, switchMap, of } from 'rxjs';
import { ToastrService } from 'ngx-toastr';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatSelectModule } from '@angular/material/select';
import { MatChipsModule } from '@angular/material/chips';

import { PublicEventService } from '../../../core/services/public-event.service';
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
    MatProgressSpinnerModule,
    MatFormFieldModule,
    MatSelectModule,
    MatChipsModule
  ],
  templateUrl: './home.html',
  styleUrls: ['./home.scss']
})
export class Home implements OnInit {
  events: Event[] = [];
  filteredEvents: Event[] = [];
  loading = true;
  searchQuery = '';
  categories: string[] = [];
  venues: string[] = [];
  selectedCategory = 'All';
  selectedVenue = 'All';
  selectedDate = 'Any time';
  selectedPrice = 'Any price';

  featuredEvents: Event[] = [];
  quickCategories = [
    { name: 'Music', icon: 'music_note' },
    { name: 'Comedy', icon: 'theater_comedy' },
    { name: 'Sports', icon: 'sports_soccer' },
    { name: 'Tech', icon: 'computer' },
    { name: 'Arts & Theater', icon: 'palette' }
  ];

  /* Skeleton simulation */
  skeletonArray = Array(6).fill(0);

  private readonly eventService = inject(PublicEventService);
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
      next: (events) => {
        this.events = events;
        // Mock featured events to be the top 3 published events with images
        this.featuredEvents = events.filter(e => e.image_url).slice(0, 3);
        if (this.featuredEvents.length === 0) {
          this.featuredEvents = events.slice(0, 3);
        }
        
        this.buildFilterOptions();
        this.applyFilters();
      },
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

  onFilterChange(): void {
    this.applyFilters();
  }

  private buildFilterOptions(): void {
    const categorySet = new Set<string>();
    const venueSet = new Set<string>();

    for (const event of this.events) {
      if (event.category) {
        categorySet.add(event.category);
      }
      if (event.venue_name) {
        venueSet.add(event.venue_name);
      }
    }

    this.categories = Array.from(categorySet).sort((a, b) => a.localeCompare(b));
    this.venues = Array.from(venueSet).sort((a, b) => a.localeCompare(b));

    if (this.selectedCategory !== 'All' && !this.categories.includes(this.selectedCategory)) {
      this.selectedCategory = 'All';
    }

    if (this.selectedVenue !== 'All' && !this.venues.includes(this.selectedVenue)) {
      this.selectedVenue = 'All';
    }
  }

  private applyFilters(): void {
    const now = new Date();
    const todayKey = now.toDateString();
    const weekAhead = new Date(now);
    weekAhead.setDate(now.getDate() + 7);

    this.filteredEvents = this.events.filter(event => {
      if (this.selectedCategory !== 'All' && event.category !== this.selectedCategory) {
        return false;
      }

      if (this.selectedVenue !== 'All' && event.venue_name !== this.selectedVenue) {
        return false;
      }

      if (this.selectedDate !== 'Any time') {
        const eventDate = new Date(event.start_time);
        if (Number.isNaN(eventDate.getTime())) {
          return false;
        }

        if (this.selectedDate === 'Today' && eventDate.toDateString() !== todayKey) {
          return false;
        }

        if (this.selectedDate === 'This Week' && (eventDate < now || eventDate > weekAhead)) {
          return false;
        }

        if (this.selectedDate === 'This Month') {
          if (eventDate.getMonth() !== now.getMonth() || eventDate.getFullYear() !== now.getFullYear()) {
            return false;
          }
        }

        if (this.selectedDate === 'Upcoming' && eventDate < now) {
          return false;
        }

        if (this.selectedDate === 'Past' && eventDate >= now) {
          return false;
        }
      }

      if (this.selectedPrice !== 'Any price') {
        const minPrice = event.min_price ?? event.max_price ?? 0;

        if (this.selectedPrice === 'Free' && minPrice > 0) {
          return false;
        }

        if (this.selectedPrice === 'Under ₹500' && minPrice >= 500) {
          return false;
        }

        if (this.selectedPrice === '₹500 - ₹1000' && (minPrice < 500 || minPrice > 1000)) {
          return false;
        }

        if (this.selectedPrice === '₹1000 - ₹2000' && (minPrice < 1000 || minPrice > 2000)) {
          return false;
        }

        if (this.selectedPrice === '₹2000+' && minPrice < 2000) {
          return false;
        }
      }

      return true;
    });
  }

  setQuickCategory(category: string): void {
    if (this.selectedCategory === category) {
      this.selectedCategory = 'All';
    } else {
      this.selectedCategory = category;
      this.onFilterChange();
    }
    this.applyFilters();
  }

  goToEvent(eventId: string): void {
    this.router.navigate(['/events', eventId]);
  }
}
