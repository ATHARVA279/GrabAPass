import { CommonModule } from '@angular/common';
import { Component, EventEmitter, Input, Output } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';

import { EventVenue, VenueSearchResult } from '../../models/event-venue';

@Component({
  selector: 'app-venue-option-card',
  standalone: true,
  imports: [CommonModule, MatButtonModule, MatIconModule],
  templateUrl: './venue-option-card.html',
  styleUrl: './venue-option-card.scss',
})
export class VenueOptionCard {
  @Input() venue!: VenueSearchResult | EventVenue;
  @Input() actionLabel = 'Select';
  @Input() emphasisLabel: string | null = null;
  @Input() disabled = false;

  @Output() selected = new EventEmitter<VenueSearchResult | EventVenue>();

  get hasRating(): boolean {
    return typeof (this.venue as VenueSearchResult).rating === 'number';
  }

  selectVenue(): void {
    if (this.disabled) {
      return;
    }

    this.selected.emit(this.venue);
  }
}
