import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable } from 'rxjs';

import {
  CreateEventRequest,
  Event,
  GateStaffSummary,
  OrganizerDashboardSummaryResponse,
} from '../../shared/models/event';
import { apiUrl } from '../api/api-url';

@Injectable({
  providedIn: 'root'
})
export class OrganizerEventService {
  private readonly http = inject(HttpClient);
  private readonly apiUrl = apiUrl('/api/organizer/events');

  getOrganizerEvents(): Observable<Event[]> {
    return this.http.get<Event[]>(this.apiUrl);
  }

  getOrganizerEventById(id: string): Observable<Event> {
    return this.http.get<Event>(`${this.apiUrl}/${id}`);
  }

  getOrganizerDashboardSummary(): Observable<OrganizerDashboardSummaryResponse> {
    return this.http.get<OrganizerDashboardSummaryResponse>(`${this.apiUrl}/dashboard/summary`);
  }

  createEvent(payload: CreateEventRequest): Observable<Event> {
    return this.http.post<Event>(this.apiUrl, payload);
  }

  updateEvent(eventId: string, payload: CreateEventRequest): Observable<Event> {
    return this.http.put<Event>(`${this.apiUrl}/${eventId}`, payload);
  }

  deleteEvent(eventId: string): Observable<void> {
    return this.http.delete<void>(`${this.apiUrl}/${eventId}`);
  }

  cancelEvent(eventId: string): Observable<Event> {
    return this.http.put<Event>(`${this.apiUrl}/${eventId}/cancel`, {});
  }

  listGateStaffUsers(): Observable<GateStaffSummary[]> {
    return this.http.get<GateStaffSummary[]>(`${this.apiUrl}/gate-staff/users`);
  }

  getAssignedGateStaff(eventId: string): Observable<GateStaffSummary[]> {
    return this.http.get<GateStaffSummary[]>(`${this.apiUrl}/${eventId}/gate-staff`);
  }

  assignGateStaff(eventId: string, gateStaffIds: string[]): Observable<void> {
    return this.http.put<void>(`${this.apiUrl}/${eventId}/gate-staff`, {
      gate_staff_ids: gateStaffIds,
    });
  }
}
