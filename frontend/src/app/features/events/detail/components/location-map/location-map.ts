import { CommonModule } from '@angular/common';
import { Component, Input, inject } from '@angular/core';
import { DomSanitizer, SafeResourceUrl } from '@angular/platform-browser';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';

import { EventVenue } from '../../../../../shared/models/event-venue';

@Component({
  selector: 'app-location-map',
  standalone: true,
  imports: [CommonModule, MatButtonModule, MatIconModule],
  templateUrl: './location-map.html',
  styleUrl: './location-map.scss',
})
export class LocationMap {
  @Input() venue: EventVenue | null = null;
  @Input() venueName = '';
  @Input() address = '';
  @Input() latitude: number | null | undefined = null;
  @Input() longitude: number | null | undefined = null;

  private readonly sanitizer = inject(DomSanitizer);

  get hasCoordinates(): boolean {
    return this.latitude != null && this.longitude != null;
  }

  get mapPreviewUrl(): SafeResourceUrl | null {
    if (!this.hasCoordinates) {
      return null;
    }

    const src = `https://www.google.com/maps?q=${this.latitude},${this.longitude}&z=15&output=embed`;
    return this.sanitizer.bypassSecurityTrustResourceUrl(src);
  }

  get mapsUrl(): string | null {
    if (this.hasCoordinates) {
      return `https://www.google.com/maps/search/?api=1&query=${this.latitude},${this.longitude}`;
    }

    const query = encodeURIComponent(`${this.venueName} ${this.address}`.trim());
    return query ? `https://www.google.com/maps/search/?api=1&query=${query}` : null;
  }
}
