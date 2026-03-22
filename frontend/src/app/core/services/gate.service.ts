import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable } from 'rxjs';
import { TicketDetail } from './ticket.service';
import { Event } from '../../shared/models/event';
import { apiUrl } from '../api/api-url';

export interface ScanResultResponse {
  success: boolean;
  message: string;
  ticket_detail: TicketDetail | null;
}

export interface ScanLog {
  id: string;
  ticket_id: string | null;
  event_id: string;
  scanned_by: string;
  result: string;
  reason: string | null;
  scanned_at: string;
}

@Injectable({ providedIn: 'root' })
export class GateService {
  private readonly http = inject(HttpClient);
  private readonly apiUrl = apiUrl('/api/gate');

  validateTicket(qrPayload: string, eventId: string): Observable<ScanResultResponse> {
    return this.http.post<ScanResultResponse>(`${this.apiUrl}/validate`, {
      qr_payload: qrPayload,
      event_id: eventId
    });
  }

  getScanHistory(eventId: string): Observable<ScanLog[]> {
    return this.http.get<ScanLog[]>(`${this.apiUrl}/events/${eventId}/scans`);
  }

  getAssignedEvents(): Observable<Event[]> {
    return this.http.get<Event[]>(`${this.apiUrl}/events`);
  }
}
