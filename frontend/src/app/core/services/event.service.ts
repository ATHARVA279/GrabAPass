import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';

import { CreateEventRequest, Event } from '../../shared/models/event';

@Injectable({
  providedIn: 'root'
})
export class EventService {
  private readonly http = inject(HttpClient);
  private readonly publicApiUrl = '/api/events';
  private readonly organizerApiUrl = '/api/organizer/events';

  getPublishedEvents(category?: string, search?: string): Observable<Event[]> {
    let params = new HttpParams();

    if (category) {
      params = params.set('category', category);
    }

    if (search) {
      params = params.set('search', search);
    }

    return this.http.get<Event[]>(this.publicApiUrl, { params });
  }

  getEventById(id: string): Observable<Event> {
    return this.http.get<Event>(`${this.publicApiUrl}/${id}`);
  }

  getOrganizerEvents(): Observable<Event[]> {
    return this.http.get<Event[]>(this.organizerApiUrl);
  }

  createEvent(payload: CreateEventRequest): Observable<Event> {
    return this.http.post<Event>(this.organizerApiUrl, payload);
  }
}
