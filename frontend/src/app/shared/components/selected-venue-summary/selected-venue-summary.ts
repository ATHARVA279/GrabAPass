import { CommonModule } from '@angular/common';
import { Component, EventEmitter, Input, Output, inject } from '@angular/core';
import { DomSanitizer, SafeResourceUrl } from '@angular/platform-browser';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';

import { EventVenue, EventVenueInput } from '../../models/event-venue';

@Component({
  selector: 'app-selected-venue-summary',
  standalone: true,
  imports: [CommonModule, MatButtonModule, MatIconModule],
  templateUrl: './selected-venue-summary.html',
  styleUrl: './selected-venue-summary.scss',
})
export class SelectedVenueSummary {
  @Input() venue: EventVenue | EventVenueInput | null = null;
  @Input() showAdjustAction = true;

  @Output() changeVenue = new EventEmitter<void>();
  @Output() adjustOnMap = new EventEmitter<void>();

  private readonly sanitizer = inject(DomSanitizer);

  get mapPreviewUrl(): SafeResourceUrl | null {
    if (!this.venue) {
      return null;
    }

    const mapUrl = `https://www.google.com/maps?q=${this.venue.latitude},${this.venue.longitude}&z=15&output=embed`;
    return this.sanitizer.bypassSecurityTrustResourceUrl(mapUrl);
  }

  get googleMapsUrl(): string | null {
    if (!this.venue) {
      return null;
    }

    return `https://www.google.com/maps/search/?api=1&query=${this.venue.latitude},${this.venue.longitude}`;
  }
}
