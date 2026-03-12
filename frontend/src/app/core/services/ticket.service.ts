import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable } from 'rxjs';

export interface TicketDetail {
  id: string;
  order_id: string;
  event_id: string;
  event_title: string;
  event_start_time: string;
  venue_name: string;
  seats: { seat_id: string, seat_label: string, section_name: string }[];
  qr_payload: string;
  status: string;
  created_at: string;
  used_at: string | null;
}

@Injectable({
  providedIn: 'root'
})
export class TicketService {
  private readonly http = inject(HttpClient);
  private readonly apiUrl = '/api/tickets';

  getUserTickets(): Observable<TicketDetail[]> {
    return this.http.get<TicketDetail[]>(this.apiUrl);
  }

  getTicket(id: string): Observable<TicketDetail> {
    return this.http.get<TicketDetail>(`${this.apiUrl}/${id}`);
  }
}
