import { CommonModule } from '@angular/common';
import {
  AfterViewInit,
  Component,
  ElementRef,
  EventEmitter,
  Input,
  OnChanges,
  Output,
  SimpleChanges,
  ViewChild,
  inject,
} from '@angular/core';
import { MatIconModule } from '@angular/material/icon';

import { EventVenueInput, VenueSearchResult } from '../../models/event-venue';
import { GoogleMapsLoaderService } from '../../../core/services/google-maps-loader.service';
import { GooglePlacesService } from '../../../core/services/google-places.service';

@Component({
  selector: 'app-map-selector',
  standalone: true,
  imports: [CommonModule, MatIconModule],
  templateUrl: './map-selector.html',
  styleUrl: './map-selector.scss',
})
export class MapSelector implements AfterViewInit, OnChanges {
  @ViewChild('mapCanvas') mapCanvas?: ElementRef<HTMLDivElement>;

  @Input() venue: EventVenueInput | null = null;
  @Input() interactive = true;
  @Input() height = 320;
  @Input() helperText = 'Drag the marker or tap the map to fine-tune the exact venue pin.';
  @Input() fallbackCenter: { lat: number; lng: number } = { lat: 20.5937, lng: 78.9629 };

  @Output() venueChange = new EventEmitter<EventVenueInput>();
  @Output() mapError = new EventEmitter<string>();

  isLoading = true;
  isResolvingLocation = false;
  errorMessage = '';
  canRenderMap = false;

  private readonly mapsLoader = inject(GoogleMapsLoaderService);
  private readonly googlePlaces = inject(GooglePlacesService);
  private map: any | null = null;
  private marker: any | null = null;
  private mapClickListener: any | null = null;

  async ngAfterViewInit(): Promise<void> {
    await this.initializeMap();
  }

  async ngOnChanges(changes: SimpleChanges): Promise<void> {
    if (!changes['venue'] || !this.map) {
      return;
    }

    this.syncMarkerToVenue();
  }

  get mapHeight(): string {
    return `${this.height}px`;
  }

  private async initializeMap(): Promise<void> {
    if (!this.mapCanvas) {
      return;
    }

    try {
      this.isLoading = true;
      this.errorMessage = '';
      await this.mapsLoader.loadGoogleMaps();
      if (this.mapsLoader.hasAuthFailure) {
        throw new Error('Google Maps authentication failed.');
      }

      const center = this.venue
        ? { lat: this.venue.latitude, lng: this.venue.longitude }
        : this.fallbackCenter;

      this.canRenderMap = true;
      this.map = new window.google.maps.Map(this.mapCanvas.nativeElement, {
        center,
        zoom: this.venue ? 16 : 5,
        disableDefaultUI: true,
        zoomControl: true,
        mapTypeControl: false,
        streetViewControl: false,
        fullscreenControl: false,
        gestureHandling: this.interactive ? 'greedy' : 'cooperative',
      });

      this.marker = new window.google.maps.Marker({
        map: this.map,
        position: center,
        draggable: this.interactive,
        visible: !!this.venue,
      });

      this.attachInteractions();
      this.syncMarkerToVenue();
    } catch {
      this.canRenderMap = false;
      this.errorMessage = this.mapsLoader.hasAuthFailure
        ? 'Google Maps is configured incorrectly. Enable Maps JavaScript API, Places API, and billing for this key.'
        : 'Google Maps failed to load.';
      this.mapError.emit(this.errorMessage);
    } finally {
      this.isLoading = false;
    }
  }

  private attachInteractions(): void {
    if (!this.map || !this.marker || !this.interactive) {
      return;
    }

    this.marker.addListener('dragend', (event: any) => {
      this.handleCoordinateUpdate(event?.latLng?.lat?.(), event?.latLng?.lng?.());
    });

    this.mapClickListener = this.map.addListener('click', (event: any) => {
      this.handleCoordinateUpdate(event?.latLng?.lat?.(), event?.latLng?.lng?.());
    });
  }

  private syncMarkerToVenue(): void {
    if (!this.map || !this.marker) {
      return;
    }

    const position = this.venue
      ? { lat: this.venue.latitude, lng: this.venue.longitude }
      : this.fallbackCenter;

    this.marker.setPosition(position);
    this.marker.setVisible(!!this.venue || this.interactive);
    this.marker.setDraggable(this.interactive);
    this.map.panTo(position);
    this.map.setZoom(this.venue ? 16 : 5);
  }

  private async handleCoordinateUpdate(latitude?: number, longitude?: number): Promise<void> {
    if (typeof latitude !== 'number' || typeof longitude !== 'number') {
      return;
    }

    try {
      this.isResolvingLocation = true;
      this.errorMessage = '';
      const reversedVenue = await this.googlePlaces.reverseGeocode(latitude, longitude);
      const nextVenue: EventVenueInput = {
        ...this.venue,
        ...reversedVenue,
        latitude,
        longitude,
        name: this.venue?.name?.trim() || reversedVenue.name || 'Selected location',
      };

      this.marker?.setPosition({ lat: latitude, lng: longitude });
      this.venueChange.emit(nextVenue);
    } catch {
      this.errorMessage = 'We could not confirm this pin. Please try another spot.';
      this.mapError.emit(this.errorMessage);
    } finally {
      this.isResolvingLocation = false;
    }
  }
}
