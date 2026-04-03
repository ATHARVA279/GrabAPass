import { CommonModule } from '@angular/common';
import { Component, EventEmitter, Input, Output } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatChipsModule } from '@angular/material/chips';
import { MatIconModule } from '@angular/material/icon';

import { Event } from '../../../../../shared/models/event';

@Component({
  selector: 'app-event-hero',
  standalone: true,
  imports: [CommonModule, MatButtonModule, MatChipsModule, MatIconModule],
  templateUrl: './event-hero.html',
  styleUrl: './event-hero.scss',
})
export class EventHero {
  @Input({ required: true }) event!: Event;
  @Input() imageUrl: string | null = null;

  @Output() share = new EventEmitter<void>();
}
