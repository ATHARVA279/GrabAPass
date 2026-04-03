import { Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormBuilder, FormGroup, FormArray, ReactiveFormsModule, Validators } from '@angular/forms';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { finalize } from 'rxjs';
import { ToastrService } from 'ngx-toastr';

import { MatCardModule } from '@angular/material/card';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatButtonModule } from '@angular/material/button';
import { MatSelectModule } from '@angular/material/select';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatDatepickerModule } from '@angular/material/datepicker';
import { MatNativeDateModule } from '@angular/material/core';
import { MatDialogModule, MatDialog } from '@angular/material/dialog';

import { TimePickerDialog } from '../../../../shared/components/time-picker-dialog/time-picker-dialog';
import { OrganizerEventService } from '../../../../core/services/organizer-event.service';
import { PublicEventService } from '../../../../core/services/public-event.service';
import { VenueService } from '../../../../core/services/venue.service';
import {
  CreateEventRequest,
  CreateEventTicketTierRequest,
  GateStaffSummary,
} from '../../../../shared/models/event';
import { AssignSeatCategoryRequest, VenueTemplate } from '../../../../shared/models/venue';

const CLOUDINARY_CLOUD_NAME = 'dohkzgazq';
const CLOUDINARY_UPLOAD_PRESET = 'GrabAPass';
const CLOUDINARY_FOLDER = 'graba-pass/events';

@Component({
  selector: 'app-create-event',
  standalone: true,
  imports: [
    CommonModule,
    ReactiveFormsModule,
    RouterModule,
    MatCardModule,
    MatFormFieldModule,
    MatInputModule,
    MatButtonModule,
    MatSelectModule,
    MatIconModule,
    MatProgressSpinnerModule,
    MatDatepickerModule,
    MatNativeDateModule,
    MatDialogModule,
  ],
  templateUrl: './create-event.html',
  styleUrls: ['./create-event.scss']
})
export class CreateEvent implements OnInit {
  readonly eventForm: FormGroup;
  isSubmitting = false;
  isUploadingImage = false;
  isEditMode = false;
  editingEventId: string | null = null;
  loadingEvent = false;
  displayTime = '';
  venueTemplates: VenueTemplate[] = [];
  gateStaffUsers: GateStaffSummary[] = [];
  imagePreviewUrl: string | null = null;

  private readonly fb = inject(FormBuilder);
  private readonly eventService = inject(OrganizerEventService);
  private readonly publicEventService = inject(PublicEventService);
  private readonly venueService = inject(VenueService);
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly toastr = inject(ToastrService);
  private readonly dialog = inject(MatDialog);

  constructor() {
    this.eventForm = this.fb.group({
      title: ['', [Validators.required, Validators.maxLength(255)]],
      category: ['', Validators.required],
      venue_name: ['', Validators.required],
      venue_address: ['', Validators.required],
      start_date: [null, Validators.required],
      start_time_input: ['', [Validators.required, Validators.pattern(/^([01]\d|2[0-3]):([0-5]\d)$/)]],
      description: [''],
      image_url: [''],
      seating_mode: ['GeneralAdmission', Validators.required],
      venue_template_id: [null],
      gate_staff_ids: [[]],
      categories: this.fb.array([]),
      ticket_tiers: this.fb.array([])
    });
  }

  ngOnInit(): void {
    this.editingEventId = this.route.snapshot.paramMap.get('id');
    this.isEditMode = !!this.editingEventId;

    this.venueService.listVenueTemplates().subscribe({
      next: (templates) => (this.venueTemplates = templates),
      error: () => {} // non-fatal — organizer may have no templates yet
    });

    this.eventService.listGateStaffUsers().subscribe({
      next: (users) => (this.gateStaffUsers = users),
      error: () => {}
    });

    this.eventForm.get('venue_template_id')?.valueChanges.subscribe(templateId => {
      this.categories.clear();
      if (!templateId) return;
      
      this.venueService.listVenueTemplateSections(templateId).subscribe({
        next: (sections) => {
          sections.forEach(section => {
            this.categories.push(this.fb.group({
              section_id: [section.id],
              name: [section.name, Validators.required],
              price: [0, [Validators.required, Validators.min(0)]],
              color_hex: [section.color_hex || '#4A90D9']
            }));
          });
        }
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

  addTicketTier(): void {
    this.ticketTiers.push(this.fb.group({
      name: ['', Validators.required],
      price: [0, [Validators.required, Validators.min(0)]],
      capacity: [1, [Validators.required, Validators.min(1)]],
      color_hex: ['#4A90D9']
    }));
  }

  removeTicketTier(index: number): void {
    this.ticketTiers.removeAt(index);
  }

  openTimePicker(): void {
    const current = this.eventForm.get('start_time_input')?.value as string;
    let hour = 9, minute = 0;
    if (current) {
      const [h, m] = current.split(':').map(Number);
      hour = h; minute = m;
    }
    this.dialog.open(TimePickerDialog, {
      data: { hour, minute },
      panelClass: 'timepicker-panel',
      backdropClass: 'timepicker-backdrop'
    }).afterClosed().subscribe(result => {
      if (result !== null && result !== undefined) {
        const h = String(result.hour).padStart(2, '0');
        const m = String(result.minute).padStart(2, '0');
        this.eventForm.get('start_time_input')?.setValue(`${h}:${m}`);
        this.eventForm.get('start_time_input')?.markAsTouched();
        const isPM = result.hour >= 12;
        const h12 = result.hour % 12 === 0 ? 12 : result.hour % 12;
        this.displayTime = `${String(h12).padStart(2,'0')}:${m} ${isPM ? 'PM' : 'AM'}`;
      }
    });
  }

  onSubmit(): void {
    if (this.isUploadingImage) {
      this.toastr.info('Please wait for the image upload to finish.', 'Uploading');
      return;
    }

    if (this.eventForm.invalid) {
      this.eventForm.markAllAsTouched();
      this.toastr.warning('Please fill in all required fields.', 'Incomplete Form');
      return;
    }

    this.isSubmitting = true;

    const formValue = this.eventForm.getRawValue();
    const date: Date = new Date(formValue.start_date);
    const [hours, minutes] = (formValue.start_time_input as string).split(':').map(Number);
    date.setHours(hours, minutes, 0, 0);
    const {
      start_date,
      start_time_input,
      categories: categoriesData,
      gate_staff_ids,
      ticket_tiers,
      ...rest
    } = formValue;
    const payload: CreateEventRequest = {
      ...rest,
      start_time: date.toISOString(),
      venue_template_id: rest.venue_template_id || undefined,
      seating_mode: rest.seating_mode || undefined,
      image_url: rest.image_url ? rest.image_url : undefined,
      ticket_tiers: (ticket_tiers as CreateEventTicketTierRequest[])
        .filter((tier) => tier.name?.trim())
    };

    const categoryPayload: AssignSeatCategoryRequest[] = categoriesData || [];

    const request$ = this.isEditMode && this.editingEventId
      ? this.eventService.updateEvent(this.editingEventId, payload)
      : this.eventService.createEvent(payload);

    request$.subscribe({
      next: (event) => {
        // If there are categories, upload them now
        if (categoryPayload.length > 0 && event.id) {
          this.venueService.assignSeatCategories(event.id, categoryPayload).pipe(
            finalize(() => (this.isSubmitting = false))
          ).subscribe({
            next: () => {
              this.saveGateStaffAssignments(event.id, gate_staff_ids || []);
            },
            error: (err) => {
              this.toastr.error(
                this.isEditMode ? 'Event updated, but failed to save pricing.' : 'Event created, but failed to save pricing.',
                'Warning'
              );
              this.router.navigate(['/organizer']);
            }
          });
        } else {
          this.saveGateStaffAssignments(event.id, gate_staff_ids || []);
        }
      },
      error: (err) => {
        this.isSubmitting = false;
        const msg = typeof err.error === 'string'
          ? err.error
          : (err.error?.message ?? 'Failed to create event.');
        this.toastr.error(msg, 'Error');
      }
    });
  }

  private loadEventForEdit(eventId: string): void {
    this.loadingEvent = true;
    this.eventService.getOrganizerEventById(eventId).pipe(
      finalize(() => (this.loadingEvent = false))
    ).subscribe({
      next: (event) => {
        const startDate = new Date(event.start_time);
        const hours = startDate.getHours();
        const minutes = startDate.getMinutes();
        const hourText = String(hours).padStart(2, '0');
        const minuteText = String(minutes).padStart(2, '0');
        const isPM = hours >= 12;
        const h12 = hours % 12 === 0 ? 12 : hours % 12;
        this.displayTime = `${String(h12).padStart(2,'0')}:${minuteText} ${isPM ? 'PM' : 'AM'}`;

        this.eventForm.patchValue({
          title: event.title,
          category: event.category,
          venue_name: event.venue_name,
          venue_address: event.venue_address,
          start_date: startDate,
          start_time_input: `${hourText}:${minuteText}`,
          description: event.description ?? '',
          image_url: event.image_url ?? '',
          seating_mode: event.seating_mode ?? (event.venue_template_id ? 'Reserved' : 'GeneralAdmission'),
          venue_template_id: event.venue_template_id ?? null,
        });
        this.imagePreviewUrl = event.image_url ?? null;
        this.ticketTiers.clear();

        this.publicEventService.getEventTicketTiers(eventId).subscribe({
          next: (tiers) => {
            tiers.forEach((tier) => {
              this.ticketTiers.push(this.fb.group({
                name: [tier.name, Validators.required],
                price: [tier.price, [Validators.required, Validators.min(0)]],
                capacity: [tier.capacity, [Validators.required, Validators.min(1)]],
                color_hex: [tier.color_hex || '#4A90D9']
              }));
            });
          },
          error: () => {}
        });

        this.eventService.getAssignedGateStaff(eventId).subscribe({
          next: (assigned) => {
            this.eventForm.patchValue({
              gate_staff_ids: assigned.map((user) => user.id),
            });
          },
          error: () => {}
        });
      },
      error: () => {
        this.toastr.error('Failed to load event for editing.', 'Error');
        this.router.navigate(['/organizer']);
      }
    });
  }

  private saveGateStaffAssignments(eventId: string, gateStaffIds: string[]): void {
    this.eventService.assignGateStaff(eventId, gateStaffIds).pipe(
      finalize(() => (this.isSubmitting = false))
    ).subscribe({
      next: () => {
        this.toastr.success(
          this.isEditMode ? 'Event updated successfully!' : 'Event created successfully!',
          'Success'
        );
        this.router.navigate(['/organizer']);
      },
      error: () => {
        this.toastr.error('Event saved, but failed to assign gate staff.', 'Warning');
        this.router.navigate(['/organizer']);
      }
    });
  }

  async onImageSelected(event: Event): Promise<void> {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    if (!file.type.startsWith('image/')) {
      this.toastr.error('Please select an image file.', 'Invalid file');
      input.value = '';
      return;
    }

    if (file.size > 5 * 1024 * 1024) {
      this.toastr.error('Image must be smaller than 5MB.', 'File too large');
      input.value = '';
      return;
    }

    this.isUploadingImage = true;
    try {
      const imageUrl = await this.uploadToCloudinary(file);
      this.eventForm.get('image_url')?.setValue(imageUrl);
      this.imagePreviewUrl = imageUrl;
      this.toastr.success('Image uploaded successfully.', 'Uploaded');
    } catch (error) {
      this.toastr.error('Failed to upload image. Please try again.', 'Upload Error');
    } finally {
      this.isUploadingImage = false;
      input.value = '';
    }
  }

  removeImage(): void {
    this.eventForm.get('image_url')?.setValue('');
    this.imagePreviewUrl = null;
  }

  private async uploadToCloudinary(file: File): Promise<string> {
    const formData = new FormData();
    formData.append('file', file);
    formData.append('upload_preset', CLOUDINARY_UPLOAD_PRESET);
    formData.append('folder', CLOUDINARY_FOLDER);

    const response = await fetch(`https://api.cloudinary.com/v1_1/${CLOUDINARY_CLOUD_NAME}/image/upload`, {
      method: 'POST',
      body: formData
    });

    if (!response.ok) {
      throw new Error('Upload failed');
    }

    const data = await response.json() as { secure_url?: string };
    if (!data.secure_url) {
      throw new Error('Upload failed');
    }
    return data.secure_url;
  }
}
