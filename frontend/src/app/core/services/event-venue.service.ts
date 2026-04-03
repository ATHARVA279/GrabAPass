import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable } from 'rxjs';

import {
  EventVenue,
  EventVenueInput,
  EventVenueMatchResponse,
} from '../../shared/models/event-venue';
import { apiUrl } from '../api/api-url';

@Injectable({ providedIn: 'root' })
export class EventVenueService {
  private readonly http = inject(HttpClient);
  private readonly baseUrl = apiUrl('/api/organizer/event-venues');

  saveVenue(payload: EventVenueInput): Observable<EventVenue> {
    return this.http.post<EventVenue>(this.baseUrl, payload);
  }

  getVenue(id: string): Observable<EventVenue> {
    return this.http.get<EventVenue>(`${this.baseUrl}/${id}`);
  }

  findMatches(payload: EventVenueInput): Observable<EventVenueMatchResponse> {
    return this.http.post<EventVenueMatchResponse>(`${this.baseUrl}/match`, payload);
  }
}
