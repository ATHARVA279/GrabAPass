import { Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormBuilder, FormGroup, ReactiveFormsModule, Validators } from '@angular/forms';
import { Router, RouterModule } from '@angular/router';
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
import { EventService } from '../../../../core/services/event.service';
import { VenueService } from '../../../../core/services/venue.service';
import { CreateEventRequest } from '../../../../shared/models/event';
import { VenueTemplate } from '../../../../shared/models/venue';

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
  displayTime = '';
  venueTemplates: VenueTemplate[] = [];

  private readonly fb = inject(FormBuilder);
  private readonly eventService = inject(EventService);
  private readonly venueService = inject(VenueService);
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
      venue_template_id: [null]
    });
  }

  ngOnInit(): void {
    this.venueService.listVenueTemplates().subscribe({
      next: (templates) => (this.venueTemplates = templates),
      error: () => {} // non-fatal — organizer may have no templates yet
    });
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
    const { start_date, start_time_input, ...rest } = formValue;
    const payload: CreateEventRequest = {
      ...rest,
      start_time: date.toISOString(),
      venue_template_id: rest.venue_template_id || undefined,
      seating_mode: rest.venue_template_id ? 'Reserved' as const : undefined
    };

    this.eventService.createEvent(payload).pipe(
      finalize(() => (this.isSubmitting = false))
    ).subscribe({
      next: () => {
        this.toastr.success('Event created successfully!', 'Success');
        this.router.navigate(['/organizer']);
      },
      error: (err) => {
        const msg = typeof err.error === 'string'
          ? err.error
          : (err.error?.message ?? 'Failed to create event.');
        this.toastr.error(msg, 'Error');
      }
    });
  }
}
