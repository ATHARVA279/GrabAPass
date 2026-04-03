import { CommonModule } from '@angular/common';
import { Component, Inject, OnInit, inject } from '@angular/core';
import { FormBuilder, ReactiveFormsModule, Validators } from '@angular/forms';
import { MAT_DIALOG_DATA, MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatButtonModule } from '@angular/material/button';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';

import { EventVenueService } from '../../../core/services/event-venue.service';
import { GoogleMapsLoaderService } from '../../../core/services/google-maps-loader.service';
import { EventVenue, EventVenueInput, VenueSearchResult } from '../../models/event-venue';
import { AddressFormGroup } from '../address-form-group/address-form-group';
import { MapSelector } from '../map-selector/map-selector';
import { VenueOptionCard } from '../venue-option-card/venue-option-card';
import { VenueSearchSelect } from '../venue-search-select/venue-search-select';

export interface AddVenueModalData {
  initialQuery?: string;
  seedVenue?: EventVenueInput | null;
}

@Component({
  selector: 'app-add-venue-modal',
  standalone: true,
  imports: [
    CommonModule,
    ReactiveFormsModule,
    MatButtonModule,
    MatDialogModule,
    MatFormFieldModule,
    MatIconModule,
    MatInputModule,
    MatProgressSpinnerModule,
    AddressFormGroup,
    MapSelector,
    VenueOptionCard,
    VenueSearchSelect,
  ],
  templateUrl: './add-venue-modal.html',
  styleUrl: './add-venue-modal.scss',
})
export class AddVenueModal implements OnInit {
  readonly venueForm = inject(FormBuilder).group({
    name: ['', [Validators.required, Validators.maxLength(255)]],
    landmark: [''],
    capacity: [null as number | null],
  });

  readonly mapsEnabled = inject(GoogleMapsLoaderService).hasApiKey;

  venueDraft: EventVenueInput | null = null;
  exactMatch: EventVenue | null = null;
  similarVenues: EventVenue[] = [];
  isCheckingDuplicates = false;
  isSaving = false;
  errorMessage = '';

  private readonly dialogRef = inject(MatDialogRef<AddVenueModal>);
  private readonly venueService = inject(EventVenueService);

  constructor(@Inject(MAT_DIALOG_DATA) public readonly data: AddVenueModalData) {}

  ngOnInit(): void {
    if (this.data.seedVenue) {
      this.applyVenueDraft(this.data.seedVenue);
      void this.checkDuplicates();
    }
  }

  onSearchSelection(result: VenueSearchResult): void {
    this.applyVenueDraft(result);
    void this.checkDuplicates();
  }

  onMapChange(result: EventVenueInput): void {
    this.applyVenueDraft(result, false);
    void this.checkDuplicates();
  }

  useSimilarVenue(venue: EventVenue): void {
    this.dialogRef.close(venue);
  }

  close(): void {
    this.dialogRef.close();
  }

  async saveVenue(): Promise<void> {
    this.errorMessage = '';

    if (!this.venueDraft) {
      this.errorMessage = 'Select a Google Maps place or drop a pin first.';
      return;
    }

    if (this.venueForm.invalid) {
      this.venueForm.markAllAsTouched();
      return;
    }

    const rawCapacity = this.venueForm.get('capacity')?.value;
    const capacity = rawCapacity === null || rawCapacity === undefined ? null : Number(rawCapacity);

    const payload: EventVenueInput = {
      ...this.venueDraft,
      name: this.venueForm.get('name')?.value?.trim() || this.venueDraft.name,
      landmark: this.venueForm.get('landmark')?.value?.trim() || null,
      capacity,
    };

    this.isSaving = true;
    this.venueService.saveVenue(payload).subscribe({
      next: (venue) => this.dialogRef.close(venue),
      error: (error) => {
        this.isSaving = false;
        this.errorMessage = error?.error ?? 'We could not save this venue right now.';
      },
      complete: () => {
        this.isSaving = false;
      },
    });
  }

  private applyVenueDraft(result: EventVenueInput, resetManualFields = true): void {
    const rawCapacity = this.venueForm.get('capacity')?.value;
    const manualCapacity =
      rawCapacity === null || rawCapacity === undefined ? null : Number(rawCapacity);

    this.venueDraft = {
      ...result,
      name: this.venueForm.get('name')?.value?.trim() || result.name,
      landmark: this.venueForm.get('landmark')?.value?.trim() || result.landmark || null,
      capacity: manualCapacity ?? result.capacity ?? null,
    };

    if (resetManualFields || !this.venueForm.get('name')?.value?.trim()) {
      this.venueForm.patchValue(
        {
          name: this.venueDraft.name,
          landmark: this.venueDraft.landmark ?? '',
          capacity: this.venueDraft.capacity,
        },
        { emitEvent: false },
      );
    }
  }

  private async checkDuplicates(): Promise<void> {
    if (!this.venueDraft?.placeId) {
      this.exactMatch = null;
      this.similarVenues = [];
      return;
    }

    this.isCheckingDuplicates = true;
    this.errorMessage = '';
    this.venueService.findMatches(this.venueDraft).subscribe({
      next: (response) => {
        this.exactMatch = response.exactMatch;
        this.similarVenues = response.similarVenues.filter(
          (venue) => venue.id !== response.exactMatch?.id,
        );
      },
      error: () => {
        this.exactMatch = null;
        this.similarVenues = [];
      },
      complete: () => {
        this.isCheckingDuplicates = false;
      },
    });
  }
}
