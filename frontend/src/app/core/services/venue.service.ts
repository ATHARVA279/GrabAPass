import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable } from 'rxjs';

import {
  AssignSeatCategoryRequest,
  CreateVenueRequest,
  SeatLayoutResponse,
  VenueTemplate,
} from '../../shared/models/venue';

@Injectable({ providedIn: 'root' })
export class VenueService {
  private readonly http = inject(HttpClient);
  private readonly venueApiUrl    = '/api/organizer/venues';
  private readonly eventApiUrl    = '/api/events';
  private readonly orgEventApiUrl = '/api/organizer/events';

  // ── Venue templates ────────────────────────────────────────────────────────

  createVenueTemplate(payload: CreateVenueRequest): Observable<VenueTemplate> {
    return this.http.post<VenueTemplate>(this.venueApiUrl, payload);
  }

  listVenueTemplates(): Observable<VenueTemplate[]> {
    return this.http.get<VenueTemplate[]>(this.venueApiUrl);
  }

  getVenueTemplate(id: string): Observable<VenueTemplate> {
    return this.http.get<VenueTemplate>(`${this.venueApiUrl}/${id}`);
  }

  // ── Seat categories ────────────────────────────────────────────────────────

  assignSeatCategories(
    eventId: string,
    categories: AssignSeatCategoryRequest[]
  ): Observable<void> {
    return this.http.post<void>(
      `${this.orgEventApiUrl}/${eventId}/seat-categories`,
      categories
    );
  }

  // ── Seat layout (public, used by booking page) ─────────────────────────────

  getSeatLayout(eventId: string): Observable<SeatLayoutResponse> {
    return this.http.get<SeatLayoutResponse>(
      `${this.eventApiUrl}/${eventId}/seat-layout`
    );
  }
}
