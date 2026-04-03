import { Injectable, inject } from '@angular/core';

import { EventVenueInput, VenueSearchResult } from '../../shared/models/event-venue';
import { GoogleMapsLoaderService } from './google-maps-loader.service';

@Injectable({ providedIn: 'root' })
export class GooglePlacesService {
  private readonly loader = inject(GoogleMapsLoaderService);

  private autocompleteService: any | null = null;
  private placesService: any | null = null;
  private geocoder: any | null = null;

  async searchVenues(query: string): Promise<VenueSearchResult[]> {
    const normalizedQuery = query.trim();
    if (normalizedQuery.length < 3) {
      return [];
    }

    await this.ensureServices();

    const predictions = await new Promise<any[]>((resolve, reject) => {
      this.autocompleteService.getPlacePredictions(
        {
          input: normalizedQuery,
        },
        (results: any[] | null, status: string) => {
          const placesStatus = window.google?.maps?.places?.PlacesServiceStatus;
          if (status === placesStatus?.OK || status === placesStatus?.ZERO_RESULTS) {
            resolve(results ?? []);
            return;
          }

          reject(new Error('Google Places search failed.'));
        },
      );
    });

    const uniquePredictions = predictions.filter(
      (prediction, index, items) =>
        items.findIndex((item) => item.place_id === prediction.place_id) === index,
    );

    const details = await Promise.all(
      uniquePredictions.slice(0, 5).map(async (prediction) => {
        try {
          return await this.getPlaceDetails(prediction.place_id);
        } catch {
          return null;
        }
      }),
    );

    return details.filter((detail): detail is VenueSearchResult => !!detail);
  }

  async getPlaceDetails(placeId: string): Promise<VenueSearchResult> {
    await this.ensureServices();

    const place = await new Promise<any>((resolve, reject) => {
      this.placesService.getDetails(
        {
          placeId,
          fields: [
            'address_components',
            'formatted_address',
            'geometry',
            'name',
            'place_id',
            'rating',
          ],
        },
        (result: any, status: string) => {
          const placesStatus = window.google?.maps?.places?.PlacesServiceStatus;
          if (status === placesStatus?.OK && result) {
            resolve(result);
            return;
          }

          reject(new Error('Failed to load place details.'));
        },
      );
    });

    return this.mapGoogleResultToVenue(place, 'google');
  }

  async reverseGeocode(latitude: number, longitude: number): Promise<VenueSearchResult> {
    await this.ensureServices();

    const geocodeResult = await new Promise<any>((resolve, reject) => {
      this.geocoder.geocode(
        { location: { lat: latitude, lng: longitude } },
        (results: any[] | null, status: string) => {
          const geocoderStatus = window.google?.maps?.GeocoderStatus;
          if (status === geocoderStatus?.OK && results?.length) {
            resolve(results[0]);
            return;
          }

          reject(new Error('Failed to reverse geocode this location.'));
        },
      );
    });

    return this.mapGoogleResultToVenue(
      {
        ...geocodeResult,
        geometry: {
          location: {
            lat: () => latitude,
            lng: () => longitude,
          },
        },
      },
      'google',
    );
  }

  private async ensureServices(): Promise<void> {
    await this.loader.loadGoogleMaps();

    if (!this.autocompleteService) {
      this.autocompleteService = new window.google.maps.places.AutocompleteService();
    }

    if (!this.placesService) {
      this.placesService = new window.google.maps.places.PlacesService(
        document.createElement('div'),
      );
    }

    if (!this.geocoder) {
      this.geocoder = new window.google.maps.Geocoder();
    }
  }

  private mapGoogleResultToVenue(result: any, source: 'google' | 'existing'): VenueSearchResult {
    const latitude = result?.geometry?.location?.lat?.() ?? result?.geometry?.location?.lat ?? 0;
    const longitude = result?.geometry?.location?.lng?.() ?? result?.geometry?.location?.lng ?? 0;
    const addressComponents = this.parseAddressComponents(result?.address_components ?? []);

    return {
      id: null,
      name:
        result?.name?.trim() ||
        result?.formatted_address?.split(',')[0]?.trim() ||
        'Selected location',
      placeId: result?.place_id ?? '',
      latitude,
      longitude,
      address: result?.formatted_address ?? '',
      locality: addressComponents.locality,
      city: addressComponents.city,
      state: addressComponents.state,
      pincode: addressComponents.pincode,
      country: addressComponents.country,
      landmark: null,
      capacity: null,
      rating: typeof result?.rating === 'number' ? result.rating : null,
      source,
    };
  }

  private parseAddressComponents(
    components: any[],
  ): Pick<EventVenueInput, 'locality' | 'city' | 'state' | 'pincode' | 'country'> {
    const findByTypes = (...types: string[]): string => {
      const match = components.find((component) =>
        types.every((type) => component.types?.includes(type)),
      );

      return match?.long_name ?? '';
    };

    return {
      locality:
        findByTypes('sublocality_level_1') ||
        findByTypes('sublocality') ||
        findByTypes('neighborhood') ||
        findByTypes('locality'),
      city:
        findByTypes('locality') ||
        findByTypes('postal_town') ||
        findByTypes('administrative_area_level_2'),
      state: findByTypes('administrative_area_level_1'),
      pincode: findByTypes('postal_code'),
      country: findByTypes('country'),
    };
  }
}
