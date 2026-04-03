import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormArray, FormBuilder, FormGroup, ReactiveFormsModule, Validators } from '@angular/forms';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { finalize } from 'rxjs';
import { ToastrService } from 'ngx-toastr';

import { MatButtonModule } from '@angular/material/button';
import { MatCardModule } from '@angular/material/card';
import { MatDatepickerModule } from '@angular/material/datepicker';
import { MatDialog, MatDialogModule } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { MatNativeDateModule } from '@angular/material/core';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatSelectModule } from '@angular/material/select';

import { EventVenueService } from '../../../../core/services/event-venue.service';
import { GoogleMapsLoaderService } from '../../../../core/services/google-maps-loader.service';
import { OrganizerEventService } from '../../../../core/services/organizer-event.service';
import { PublicEventService } from '../../../../core/services/public-event.service';
import { VenueService } from '../../../../core/services/venue.service';
import { AddVenueModal } from '../../../../shared/components/add-venue-modal/add-venue-modal';
import { MapSelector } from '../../../../shared/components/map-selector/map-selector';
import { SelectedVenueSummary } from '../../../../shared/components/selected-venue-summary/selected-venue-summary';
import { TimePickerDialog } from '../../../../shared/components/time-picker-dialog/time-picker-dialog';
import { VenueOptionCard } from '../../../../shared/components/venue-option-card/venue-option-card';
import { VenueSearchSelect } from '../../../../shared/components/venue-search-select/venue-search-select';
import {
  CreateEventRequest,
  CreateEventTicketTierRequest,
  Event as OrganizerEventModel,
  GateStaffSummary,
} from '../../../../shared/models/event';
import {
  EventVenue,
  EventVenueInput,
  VenueSearchResult,
} from '../../../../shared/models/event-venue';
import { AssignSeatCategoryRequest, VenueTemplate } from '../../../../shared/models/venue';

const CLOUDINARY_CLOUD_NAME = 'dohkzgazq';
const CLOUDINARY_UPLOAD_PRESET = 'GrabAPass';
const CLOUDINARY_FOLDER = 'graba-pass/events';

type VenueFlowState = 'search' | 'review' | 'confirmed';

@Component({
  selector: 'app-create-event',
  standalone: true,
  imports: [
    CommonModule,
    ReactiveFormsModule,
    RouterModule,
    MatButtonModule,
    MatCardModule,
    MatDatepickerModule,
    MatDialogModule,
    MatFormFieldModule,
    MatIconModule,
    MatInputModule,
    MatNativeDateModule,
    MatProgressSpinnerModule,
    MatSelectModule,
    MapSelector,
    SelectedVenueSummary,
    VenueOptionCard,
    VenueSearchSelect,
  ],
  templateUrl: './create-event.html',
  styleUrls: ['./create-event.scss'],
})
export class CreateEvent implements OnInit {
  readonly maxGalleryImages = 8;
  readonly eventForm: FormGroup;
  readonly mapsEnabled = inject(GoogleMapsLoaderService).hasApiKey;

  isSubmitting = false;
  isUploadingImages = false;
  isEditMode = false;
  editingEventId: string | null = null;
  loadingEvent = false;
  displayTime = '';
  venueTemplates: VenueTemplate[] = [];
  gateStaffUsers: GateStaffSummary[] = [];
  galleryImages: string[] = [];
  activeGalleryIndex = 0;
  venueFlowState: VenueFlowState = 'search';
  isVenueLookupLoading = false;
  venueLookupError = '';
  exactVenueMatch: EventVenue | null = null;
  similarVenueMatches: EventVenue[] = [];
  selectedVenue: EventVenue | EventVenueInput | null = null;
  pendingVenueCandidate: EventVenueInput | null = null;

  private readonly fb = inject(FormBuilder);
  private readonly dialog = inject(MatDialog);
  private readonly eventService = inject(OrganizerEventService);
  private readonly eventVenueService = inject(EventVenueService);
  private readonly publicEventService = inject(PublicEventService);
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly toastr = inject(ToastrService);
  private readonly venueService = inject(VenueService);

  constructor() {
    this.eventForm = this.fb.group({
      title: ['', [Validators.required, Validators.maxLength(255)]],
      category: ['', Validators.required],
      venue_payload: [null as EventVenueInput | null, Validators.required],
      start_date: [null, Validators.required],
      start_time_input: [
        '',
        [Validators.required, Validators.pattern(/^([01]\d|2[0-3]):([0-5]\d)$/)],
      ],
      description: [''],
      image_url: [''],
      image_gallery: [[]],
      seating_mode: ['GeneralAdmission', Validators.required],
      venue_template_id: [null],
      gate_staff_ids: [[]],
      categories: this.fb.array([]),
      ticket_tiers: this.fb.array([]),
    });
  }

  ngOnInit(): void {
    this.editingEventId = this.route.snapshot.paramMap.get('id');
    this.isEditMode = !!this.editingEventId;

    this.venueService.listVenueTemplates().subscribe({
      next: (templates) => (this.venueTemplates = templates),
      error: () => {},
    });

    this.eventService.listGateStaffUsers().subscribe({
      next: (users) => (this.gateStaffUsers = users),
      error: () => {},
    });

    this.eventForm.get('venue_template_id')?.valueChanges.subscribe((templateId) => {
      this.categories.clear();
      if (!templateId) {
        return;
      }

      this.venueService.listVenueTemplateSections(templateId).subscribe({
        next: (sections) => {
          sections.forEach((section) => {
            this.categories.push(
              this.fb.group({
                section_id: [section.id],
                name: [section.name, Validators.required],
                price: [0, [Validators.required, Validators.min(0)]],
                color_hex: [section.color_hex || '#4A90D9'],
              }),
            );
          });
        },
      });
    });

    if (this.editingEventId) {
      this.loadEventForEdit(this.editingEventId);
    }
  }

  get categories(): FormArray {
    return this.eventForm.get('categories') as FormArray;
  }

  get ticketTiers(): FormArray {
    return this.eventForm.get('ticket_tiers') as FormArray;
  }

  get venueFlowHeading(): string {
    if (this.venueFlowState === 'review') {
      return 'Review the Google Maps match and confirm the exact pin.';
    }

    if (this.venueFlowState === 'confirmed') {
      return 'The venue is locked in with a verified Place ID and coordinates.';
    }

    return 'Search with Google Maps first so similar venue names stay unambiguous.';
  }

  get hasConfirmedVenue(): boolean {
    return !!this.selectedVenue;
  }

  get activeGalleryImage(): string | null {
    return this.galleryImages[this.activeGalleryIndex] ?? null;
  }

  addTicketTier(): void {
    this.ticketTiers.push(
      this.fb.group({
        name: ['', Validators.required],
        price: [0, [Validators.required, Validators.min(0)]],
        capacity: [1, [Validators.required, Validators.min(1)]],
        color_hex: ['#4A90D9'],
      }),
    );
  }

  removeTicketTier(index: number): void {
    this.ticketTiers.removeAt(index);
  }

  openTimePicker(): void {
    const current = this.eventForm.get('start_time_input')?.value as string;
    let hour = 9;
    let minute = 0;

    if (current) {
      const [parsedHour, parsedMinute] = current.split(':').map(Number);
      hour = parsedHour;
      minute = parsedMinute;
    }

    this.dialog
      .open(TimePickerDialog, {
        data: { hour, minute },
        panelClass: 'timepicker-panel',
        backdropClass: 'timepicker-backdrop',
      })
      .afterClosed()
      .subscribe((result) => {
        if (result !== null && result !== undefined) {
          const h = String(result.hour).padStart(2, '0');
          const m = String(result.minute).padStart(2, '0');
          this.eventForm.get('start_time_input')?.setValue(`${h}:${m}`);
          this.eventForm.get('start_time_input')?.markAsTouched();
          const isPM = result.hour >= 12;
          const h12 = result.hour % 12 === 0 ? 12 : result.hour % 12;
          this.displayTime = `${String(h12).padStart(2, '0')}:${m} ${isPM ? 'PM' : 'AM'}`;
        }
      });
  }

  onVenueSearchSelected(result: VenueSearchResult): void {
    this.pendingVenueCandidate = { ...result };
    this.venueFlowState = 'review';
    this.selectedVenue = null;
    this.eventForm.get('venue_payload')?.setValue(null);
    void this.lookupVenueMatches(this.pendingVenueCandidate);
  }

  onVenueMapAdjusted(updatedVenue: EventVenueInput): void {
    this.pendingVenueCandidate = {
      ...updatedVenue,
      name: this.pendingVenueCandidate?.name?.trim() || updatedVenue.name,
      landmark: this.pendingVenueCandidate?.landmark ?? updatedVenue.landmark ?? null,
      capacity: this.pendingVenueCandidate?.capacity ?? updatedVenue.capacity ?? null,
    };
    void this.lookupVenueMatches(this.pendingVenueCandidate);
  }

  confirmPendingVenue(): void {
    if (!this.pendingVenueCandidate?.placeId) {
      this.toastr.warning(
        'Pick a Google Maps place and confirm the marker first.',
        'Venue required',
      );
      return;
    }

    const confirmedVenue = this.exactVenueMatch
      ? {
          ...this.exactVenueMatch,
          name: this.pendingVenueCandidate.name || this.exactVenueMatch.name,
          landmark: this.pendingVenueCandidate.landmark ?? this.exactVenueMatch.landmark ?? null,
          capacity: this.pendingVenueCandidate.capacity ?? this.exactVenueMatch.capacity ?? null,
          latitude: this.pendingVenueCandidate.latitude,
          longitude: this.pendingVenueCandidate.longitude,
          address: this.pendingVenueCandidate.address,
          locality: this.pendingVenueCandidate.locality,
          city: this.pendingVenueCandidate.city,
          state: this.pendingVenueCandidate.state,
          pincode: this.pendingVenueCandidate.pincode,
          country: this.pendingVenueCandidate.country,
          placeId: this.pendingVenueCandidate.placeId,
        }
      : { ...this.pendingVenueCandidate };

    this.selectedVenue = confirmedVenue;
    this.venueFlowState = 'confirmed';
    this.eventForm.get('venue_payload')?.setValue(this.toVenueInput(confirmedVenue));
    this.toastr.success(
      this.exactVenueMatch ? 'Existing venue matched and confirmed.' : 'Venue confirmed.',
      'Location locked',
    );
  }

  useExistingVenueMatch(venue: EventVenue): void {
    this.selectedVenue = venue;
    this.pendingVenueCandidate = this.toVenueInput(venue);
    this.venueFlowState = 'confirmed';
    this.eventForm.get('venue_payload')?.setValue(this.toVenueInput(venue));
  }

  changeVenue(): void {
    this.selectedVenue = null;
    this.pendingVenueCandidate = null;
    this.exactVenueMatch = null;
    this.similarVenueMatches = [];
    this.venueLookupError = '';
    this.venueFlowState = 'search';
    this.eventForm.get('venue_payload')?.setValue(null);
  }

  reopenMapReview(): void {
    if (!this.selectedVenue) {
      return;
    }

    this.pendingVenueCandidate = this.toVenueInput(this.selectedVenue);
    this.selectedVenue = null;
    this.venueFlowState = 'review';
    this.eventForm.get('venue_payload')?.setValue(null);
    void this.lookupVenueMatches(this.pendingVenueCandidate);
  }

  openAddVenueModal(initialQuery = ''): void {
    this.dialog
      .open(AddVenueModal, {
        data: {
          initialQuery,
          seedVenue: this.pendingVenueCandidate,
        },
        panelClass: 'venue-modal-panel',
        backdropClass: 'venue-modal-backdrop',
        width: 'min(960px, 96vw)',
        maxWidth: '96vw',
        maxHeight: '92vh',
        autoFocus: false,
      })
      .afterClosed()
      .subscribe((venue: EventVenue | undefined) => {
        if (!venue) {
          return;
        }

        this.selectedVenue = venue;
        this.pendingVenueCandidate = this.toVenueInput(venue);
        this.venueFlowState = 'confirmed';
        this.eventForm.get('venue_payload')?.setValue(this.toVenueInput(venue));
        this.exactVenueMatch = venue;
        this.similarVenueMatches = [];
        this.toastr.success('Venue saved and attached to this event.', 'Venue ready');
      });
  }

  onSubmit(): void {
    if (this.isUploadingImages) {
      this.toastr.info('Please wait for the image uploads to finish.', 'Uploading');
      return;
    }

    if (!this.selectedVenue) {
      this.eventForm.get('venue_payload')?.markAsTouched();
      this.toastr.warning('Confirm the venue on the map before publishing.', 'Venue not confirmed');
      return;
    }

    if (this.eventForm.invalid) {
      this.eventForm.markAllAsTouched();
      this.toastr.warning('Please fill in all required fields.', 'Incomplete form');
      return;
    }

    this.isSubmitting = true;

    const formValue = this.eventForm.getRawValue();
    const date = new Date(formValue.start_date);
    const [hours, minutes] = (formValue.start_time_input as string).split(':').map(Number);
    date.setHours(hours, minutes, 0, 0);

    const {
      start_date,
      start_time_input,
      venue_payload,
      categories: categoriesData,
      gate_staff_ids,
      ticket_tiers,
      ...rest
    } = formValue;

    const venuePayload = this.toVenueInput(this.selectedVenue);

    const payload: CreateEventRequest = {
      ...rest,
      start_time: date.toISOString(),
      venue: venuePayload,
      venue_name: venuePayload.name,
      venue_address: venuePayload.address,
      venue_template_id: rest.venue_template_id || undefined,
      seating_mode: rest.seating_mode || undefined,
      image_url: rest.image_url || undefined,
      image_gallery: this.galleryImages,
      venue_place_id: venuePayload.placeId,
      venue_latitude: venuePayload.latitude,
      venue_longitude: venuePayload.longitude,
      ticket_tiers: (ticket_tiers as CreateEventTicketTierRequest[]).filter((tier) =>
        tier.name?.trim(),
      ),
    };

    const categoryPayload: AssignSeatCategoryRequest[] = categoriesData || [];
    const request$ =
      this.isEditMode && this.editingEventId
        ? this.eventService.updateEvent(this.editingEventId, payload)
        : this.eventService.createEvent(payload);

    request$.subscribe({
      next: (event) => {
        if (categoryPayload.length > 0 && event.id) {
          this.venueService
            .assignSeatCategories(event.id, categoryPayload)
            .pipe(finalize(() => (this.isSubmitting = false)))
            .subscribe({
              next: () => this.saveGateStaffAssignments(event.id, gate_staff_ids || []),
              error: () => {
                this.toastr.error(
                  this.isEditMode
                    ? 'Event updated, but failed to save pricing.'
                    : 'Event created, but failed to save pricing.',
                  'Warning',
                );
                void this.router.navigate(['/organizer']);
              },
            });
        } else {
          this.saveGateStaffAssignments(event.id, gate_staff_ids || []);
        }
      },
      error: (err) => {
        this.isSubmitting = false;
        const message =
          typeof err.error === 'string'
            ? err.error
            : (err.error?.message ?? 'Failed to save event.');
        this.toastr.error(message, 'Error');
      },
    });
  }

  async onImagesSelected(event: globalThis.Event): Promise<void> {
    const input = event.target as HTMLInputElement;
    const files = Array.from(input.files ?? []);
    if (!files.length) {
      return;
    }

    if (this.galleryImages.length + files.length > this.maxGalleryImages) {
      this.toastr.error(
        `You can upload up to ${this.maxGalleryImages} event images.`,
        'Too many images',
      );
      input.value = '';
      return;
    }

    for (const file of files) {
      if (!file.type.startsWith('image/')) {
        this.toastr.error('Please select image files only.', 'Invalid file');
        input.value = '';
        return;
      }

      if (file.size > 5 * 1024 * 1024) {
        this.toastr.error('Each image must be smaller than 5MB.', 'File too large');
        input.value = '';
        return;
      }
    }

    this.isUploadingImages = true;
    try {
      for (const file of files) {
        const imageUrl = await this.uploadToCloudinary(file);
        this.galleryImages = [...this.galleryImages, imageUrl];
      }

      this.syncImageControls();
      this.toastr.success(
        `${files.length} image${files.length === 1 ? '' : 's'} uploaded successfully.`,
        'Uploaded',
      );
    } catch {
      this.toastr.error('Failed to upload one or more images. Please try again.', 'Upload error');
    } finally {
      this.isUploadingImages = false;
      input.value = '';
    }
  }

  setPrimaryImage(index: number): void {
    if (index <= 0 || index >= this.galleryImages.length) {
      return;
    }

    const reordered = [...this.galleryImages];
    const [selected] = reordered.splice(index, 1);
    reordered.unshift(selected);
    this.galleryImages = reordered;
    this.activeGalleryIndex = 0;
    this.syncImageControls();
  }

  removeGalleryImage(index: number): void {
    this.galleryImages = this.galleryImages.filter((_, currentIndex) => currentIndex !== index);
    this.activeGalleryIndex = Math.min(
      this.activeGalleryIndex,
      Math.max(this.galleryImages.length - 1, 0),
    );
    this.syncImageControls();
  }

  selectGalleryImage(index: number): void {
    this.activeGalleryIndex = index;
  }

  private async lookupVenueMatches(venue: EventVenueInput): Promise<void> {
    this.isVenueLookupLoading = true;
    this.venueLookupError = '';
    this.eventVenueService.findMatches(venue).subscribe({
      next: (response) => {
        this.exactVenueMatch = response.exactMatch;
        this.similarVenueMatches = response.similarVenues.filter(
          (match) => match.id !== response.exactMatch?.id,
        );
      },
      error: () => {
        this.exactVenueMatch = null;
        this.similarVenueMatches = [];
        this.venueLookupError =
          'We could not check saved venues right now, but Google Maps confirmation still works.';
      },
      complete: () => {
        this.isVenueLookupLoading = false;
      },
    });
  }

  private loadEventForEdit(eventId: string): void {
    this.loadingEvent = true;
    this.eventService
      .getOrganizerEventById(eventId)
      .pipe(finalize(() => (this.loadingEvent = false)))
      .subscribe({
        next: (event) => {
          const startDate = new Date(event.start_time);
          const hours = startDate.getHours();
          const minutes = startDate.getMinutes();
          const hourText = String(hours).padStart(2, '0');
          const minuteText = String(minutes).padStart(2, '0');
          const isPM = hours >= 12;
          const h12 = hours % 12 === 0 ? 12 : hours % 12;
          this.displayTime = `${String(h12).padStart(2, '0')}:${minuteText} ${isPM ? 'PM' : 'AM'}`;

          this.eventForm.patchValue({
            title: event.title,
            category: event.category,
            start_date: startDate,
            start_time_input: `${hourText}:${minuteText}`,
            description: event.description ?? '',
            image_url: event.image_url ?? '',
            image_gallery: event.image_gallery ?? [],
            seating_mode:
              event.seating_mode ?? (event.venue_template_id ? 'Reserved' : 'GeneralAdmission'),
            venue_template_id: event.venue_template_id ?? null,
          });

          const mappedVenue = this.mapEventToVenue(event);
          if (mappedVenue) {
            this.selectedVenue = mappedVenue;
            this.pendingVenueCandidate = this.toVenueInput(mappedVenue);
            this.eventForm.get('venue_payload')?.setValue(this.toVenueInput(mappedVenue));
            this.venueFlowState = 'confirmed';
          }

          this.galleryImages = this.resolveGalleryImages(
            event.image_url ?? null,
            event.image_gallery ?? [],
          );
          this.syncImageControls();
          this.ticketTiers.clear();

          this.publicEventService.getEventTicketTiers(eventId).subscribe({
            next: (tiers) => {
              tiers.forEach((tier) => {
                this.ticketTiers.push(
                  this.fb.group({
                    name: [tier.name, Validators.required],
                    price: [tier.price, [Validators.required, Validators.min(0)]],
                    capacity: [tier.capacity, [Validators.required, Validators.min(1)]],
                    color_hex: [tier.color_hex || '#4A90D9'],
                  }),
                );
              });
            },
            error: () => {},
          });

          this.eventService.getAssignedGateStaff(eventId).subscribe({
            next: (assigned) => {
              this.eventForm.patchValue({
                gate_staff_ids: assigned.map((user) => user.id),
              });
            },
            error: () => {},
          });
        },
        error: () => {
          this.toastr.error('Failed to load event for editing.', 'Error');
          void this.router.navigate(['/organizer']);
        },
      });
  }

  private saveGateStaffAssignments(eventId: string, gateStaffIds: string[]): void {
    this.eventService
      .assignGateStaff(eventId, gateStaffIds)
      .pipe(finalize(() => (this.isSubmitting = false)))
      .subscribe({
        next: () => {
          this.toastr.success(
            this.isEditMode ? 'Event updated successfully!' : 'Event created successfully!',
            'Success',
          );
          void this.router.navigate(['/organizer']);
        },
        error: () => {
          this.toastr.error('Event saved, but failed to assign gate staff.', 'Warning');
          void this.router.navigate(['/organizer']);
        },
      });
  }

  private async uploadToCloudinary(file: File): Promise<string> {
    const formData = new FormData();
    formData.append('file', file);
    formData.append('upload_preset', CLOUDINARY_UPLOAD_PRESET);
    formData.append('folder', CLOUDINARY_FOLDER);

    const response = await fetch(
      `https://api.cloudinary.com/v1_1/${CLOUDINARY_CLOUD_NAME}/image/upload`,
      {
        method: 'POST',
        body: formData,
      },
    );

    if (!response.ok) {
      throw new Error('Upload failed');
    }

    const data = (await response.json()) as { secure_url?: string };
    if (!data.secure_url) {
      throw new Error('Upload failed');
    }

    return data.secure_url;
  }

  private syncImageControls(): void {
    this.eventForm.patchValue(
      {
        image_url: this.galleryImages[0] ?? '',
        image_gallery: this.galleryImages,
      },
      { emitEvent: false },
    );
  }

  private resolveGalleryImages(primaryImage: string | null, gallery: string[]): string[] {
    const normalized = [primaryImage ?? '', ...gallery]
      .map((image) => image.trim())
      .filter((image) => !!image);

    return normalized.filter((image, index) => normalized.indexOf(image) === index);
  }

  private toVenueInput(venue: EventVenue | EventVenueInput): EventVenueInput {
    return {
      id: venue.id ?? null,
      name: venue.name,
      placeId: venue.placeId,
      latitude: venue.latitude,
      longitude: venue.longitude,
      address: venue.address,
      locality: venue.locality,
      city: venue.city,
      state: venue.state,
      pincode: venue.pincode,
      country: venue.country,
      landmark: venue.landmark ?? null,
      capacity: venue.capacity ?? null,
    };
  }

  private mapEventToVenue(event: OrganizerEventModel): EventVenue | null {
    const placeId = event.venue_place_id ?? '';
    const latitude = event.venue_latitude;
    const longitude = event.venue_longitude;

    if (
      !event.venue_name ||
      !event.venue_address ||
      latitude == null ||
      longitude == null ||
      !placeId
    ) {
      return null;
    }

    return {
      id: event.venue_id ?? '',
      name: event.venue_name,
      placeId,
      latitude,
      longitude,
      address: event.venue_address,
      locality: event.venue_locality ?? '',
      city: event.venue_city ?? '',
      state: event.venue_state ?? '',
      pincode: event.venue_pincode ?? '',
      country: event.venue_country ?? '',
      landmark: event.venue_landmark ?? null,
      capacity: event.venue_capacity ?? null,
    };
  }
}
