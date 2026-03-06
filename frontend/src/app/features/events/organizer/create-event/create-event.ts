import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormBuilder, FormGroup, ReactiveFormsModule, Validators } from '@angular/forms';
import { Router, RouterModule } from '@angular/router';

import { EventService } from '../../../../core/services/event.service';
import { CreateEventRequest } from '../../../../shared/models/event';

@Component({
  selector: 'app-create-event',
  standalone: true,
  imports: [CommonModule, ReactiveFormsModule, RouterModule],
  templateUrl: './create-event.html',
  styleUrls: ['./create-event.scss']
})
export class CreateEvent {
  eventForm: FormGroup;
  isSubmitting = false;
  errorMessage = '';

  constructor(
    private readonly fb: FormBuilder,
    private readonly eventService: EventService,
    private readonly router: Router
  ) {
    this.eventForm = this.fb.group({
      title: ['', [Validators.required, Validators.maxLength(255)]],
      category: ['', Validators.required],
      venue_name: ['', Validators.required],
      venue_address: ['', Validators.required],
      start_time: ['', Validators.required],
      description: ['']
    });
  }

  onSubmit(): void {
    if (this.eventForm.invalid) {
      this.eventForm.markAllAsTouched();
      return;
    }

    this.isSubmitting = true;
    this.errorMessage = '';

    const formValue = this.eventForm.getRawValue();
    const payload: CreateEventRequest = {
      ...formValue,
      start_time: new Date(formValue.start_time).toISOString()
    };

    this.eventService.createEvent(payload).subscribe({
      next: () => {
        this.router.navigate(['/organizer']);
      },
      error: (err) => {
        this.isSubmitting = false;
        this.errorMessage = typeof err.error === 'string'
          ? err.error
          : err.error?.message || 'Failed to create event.';
      }
    });
  }
}
