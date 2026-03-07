import { Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, RouterModule, Router } from '@angular/router';
import { finalize } from 'rxjs';
import { ToastrService } from 'ngx-toastr';

import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatChipsModule } from '@angular/material/chips';

import { EventService } from '../../../core/services/event.service';
import { Event } from '../../../shared/models/event';

@Component({
  selector: 'app-event-detail',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    MatCardModule,
    MatButtonModule,
    MatIconModule,
    MatProgressSpinnerModule,
    MatChipsModule
  ],
  templateUrl: './event-detail.html',
  styleUrls: ['./event-detail.scss']
})
export class EventDetail implements OnInit {
  event: Event | null = null;
  loading = true;

  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly eventService = inject(EventService);
  private readonly toastr = inject(ToastrService);

  ngOnInit(): void {
    const eventId = this.route.snapshot.paramMap.get('id');

    if (!eventId) {
      this.toastr.error('No event ID provided.', 'Error');
      this.loading = false;
      return;
    }

    this.eventService.getEventById(eventId).pipe(
      finalize(() => (this.loading = false))
    ).subscribe({
      next: (event) => (this.event = event),
      error: () => this.toastr.error('Event not found or failed to load.', 'Error')
    });
  }

  goBack(): void {
    this.router.navigate(['/events']);
  }
}
