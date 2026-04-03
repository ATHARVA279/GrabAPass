import { CommonModule } from '@angular/common';
import { Component, EventEmitter, Input, OnDestroy, OnInit, Output, inject } from '@angular/core';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { MatButtonModule } from '@angular/material/button';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { debounceTime, distinctUntilChanged, Subscription } from 'rxjs';

import { GooglePlacesService } from '../../../core/services/google-places.service';
import { VenueSearchResult } from '../../models/event-venue';
import { VenueOptionCard } from '../venue-option-card/venue-option-card';

@Component({
  selector: 'app-venue-search-select',
  standalone: true,
  imports: [
    CommonModule,
    ReactiveFormsModule,
    MatButtonModule,
    MatFormFieldModule,
    MatIconModule,
    MatInputModule,
    MatProgressSpinnerModule,
    VenueOptionCard,
  ],
  templateUrl: './venue-search-select.html',
  styleUrl: './venue-search-select.scss',
})
export class VenueSearchSelect implements OnInit, OnDestroy {
  @Input() placeholder = 'Search venue name, address, locality, or city';
  @Input() mapsEnabled = true;
  @Input() initialQuery = '';

  @Output() venuePicked = new EventEmitter<VenueSearchResult>();
  @Output() addVenue = new EventEmitter<string>();

  readonly searchControl = new FormControl('', { nonNullable: true });

  searchResults: VenueSearchResult[] = [];
  isLoading = false;
  errorMessage = '';
  hasSearched = false;

  private readonly googlePlaces = inject(GooglePlacesService);
  private subscription?: Subscription;

  ngOnInit(): void {
    this.searchControl.setValue(this.initialQuery, { emitEvent: false });
    this.subscription = this.searchControl.valueChanges
      .pipe(debounceTime(250), distinctUntilChanged())
      .subscribe((query) => void this.runSearch(query));

    if (this.initialQuery.trim().length >= 3) {
      void this.runSearch(this.initialQuery);
    }
  }

  ngOnDestroy(): void {
    this.subscription?.unsubscribe();
  }

  get normalizedQuery(): string {
    return this.searchControl.value.trim();
  }

  async runSearch(query: string): Promise<void> {
    const normalizedQuery = query.trim();
    this.errorMessage = '';
    this.hasSearched = normalizedQuery.length >= 3;

    if (!this.mapsEnabled) {
      this.searchResults = [];
      this.errorMessage = 'Google Maps search is unavailable right now.';
      return;
    }

    if (normalizedQuery.length < 3) {
      this.searchResults = [];
      this.isLoading = false;
      return;
    }

    try {
      this.isLoading = true;
      this.searchResults = await this.googlePlaces.searchVenues(normalizedQuery);
    } catch {
      this.errorMessage = 'Google Maps search failed. Please try again.';
      this.searchResults = [];
    } finally {
      this.isLoading = false;
    }
  }

  emitSelectedVenue(venue: VenueSearchResult): void {
    this.closeResults(venue.name);
    this.venuePicked.emit(venue);
  }

  openAddVenue(): void {
    this.closeResults(this.normalizedQuery);
    this.addVenue.emit(this.normalizedQuery);
  }

  clearSearch(): void {
    this.searchControl.setValue('');
    this.searchResults = [];
    this.hasSearched = false;
    this.errorMessage = '';
  }

  private closeResults(nextQuery: string): void {
    this.searchResults = [];
    this.hasSearched = false;
    this.errorMessage = '';
    this.searchControl.setValue(nextQuery, { emitEvent: false });

    const activeElement = document.activeElement;
    if (activeElement instanceof HTMLElement) {
      activeElement.blur();
    }
  }
}
