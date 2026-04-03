import { CommonModule } from '@angular/common';
import { Component, EventEmitter, Input, Output } from '@angular/core';
import { MatIconModule } from '@angular/material/icon';

@Component({
  selector: 'app-event-info-section',
  standalone: true,
  imports: [CommonModule, MatIconModule],
  templateUrl: './event-info-section.html',
  styleUrl: './event-info-section.scss',
})
export class EventInfoSection {
  @Input() description: string | null | undefined = null;
  @Input() gallery: string[] = [];
  @Output() imageExpanded = new EventEmitter<string>();

  readonly highlights = [
    {
      icon: 'verified',
      title: 'Verified event',
      body: 'Hosted by a validated organizer with trusted venue information.',
    },
    {
      icon: 'flash_on',
      title: 'Instant confirmation',
      body: 'Your booking is confirmed right after payment is completed.',
    },
    {
      icon: 'shield',
      title: 'Secure checkout',
      body: 'Seats and ticket quantities are held for a short protected checkout window.',
    },
  ];
}
