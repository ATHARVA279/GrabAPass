import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';

import { Event, EventTicketTier } from '../../shared/models/event';
import { apiUrl } from '../api/api-url';

@Injectable({
  providedIn: 'root'
})
export class PublicEventService {
  private readonly http = inject(HttpClient);
  private readonly apiUrl = apiUrl('/api/events');

  getPublishedEvents(category?: string, search?: string): Observable<Event[]> {
    let params = new HttpParams();

    if (category) {
      params = params.set('category', category);
    }

    if (search) {
      params = params.set('search', search);
    }

    return this.http.get<Event[]>(this.apiUrl, { params });
  }

  getEventById(id: string): Observable<Event> {
    return this.http.get<Event>(`${this.apiUrl}/${id}`);
  }

  getEventPulse(id: string): Observable<import('../../shared/models/event').EventPulseResponse> {
    return this.http.get<import('../../shared/models/event').EventPulseResponse>(`${this.apiUrl}/${id}/pulse`);
  }

  getEventTicketTiers(id: string): Observable<EventTicketTier[]> {
    return this.http.get<EventTicketTier[]>(`${this.apiUrl}/${id}/tiers`);
  }
}
