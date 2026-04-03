import { Injectable } from '@angular/core';
import { getRuntimeConfigValue } from '../config/runtime-config';

@Injectable({
  providedIn: 'root',
})
export class GoogleMapsLoaderService {
  private loadPromise: Promise<void> | null = null;
  private readonly apiKey = getRuntimeConfigValue('GOOGLE_MAPS_API_KEY');

  get hasApiKey(): boolean {
    return !!this.apiKey;
  }

  get hasAuthFailure(): boolean {
    return !!window.__googleMapsAuthFailed;
  }

  async loadGoogleMaps(): Promise<void> {
    if (!this.hasApiKey) {
      throw new Error('Google Maps API key is not configured.');
    }

    if (this.hasAuthFailure) {
      throw new Error('Google Maps authentication failed.');
    }

    if (window.google?.maps?.places) {
      return;
    }

    if (!this.loadPromise) {
      this.loadPromise = new Promise<void>((resolve, reject) => {
        const existingScript = document.querySelector<HTMLScriptElement>(
          'script[data-google-maps="true"]',
        );
        if (existingScript) {
          existingScript.addEventListener('load', () => resolve(), { once: true });
          existingScript.addEventListener(
            'error',
            () => reject(new Error('Failed to load Google Maps.')),
            { once: true },
          );
          return;
        }

        const script = document.createElement('script');
        window.__googleMapsAuthFailed = false;
        window.gm_authFailure = () => {
          window.__googleMapsAuthFailed = true;
          this.loadPromise = null;
          reject(new Error('Google Maps authentication failed.'));
        };
        script.src = `https://maps.googleapis.com/maps/api/js?key=${encodeURIComponent(this.apiKey)}&libraries=places`;
        script.async = true;
        script.defer = true;
        script.dataset['googleMaps'] = 'true';
        script.onload = () => {
          if (window.__googleMapsAuthFailed) {
            this.loadPromise = null;
            reject(new Error('Google Maps authentication failed.'));
            return;
          }

          resolve();
        };
        script.onerror = () => {
          this.loadPromise = null;
          reject(new Error('Failed to load Google Maps.'));
        };
        document.body.appendChild(script);
      });
    }

    return this.loadPromise;
  }

  async loadPlacesLibrary(): Promise<void> {
    return this.loadGoogleMaps();
  }
}
