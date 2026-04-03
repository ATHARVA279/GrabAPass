import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable } from 'rxjs';
import { apiUrl } from '../api/api-url';

export interface TicketDetail {
  id: string;
  order_id: string;
  event_id: string;
  event_title: string;
  event_start_time: string;
  venue_name: string;
  seats: { seat_id: string, seat_label: string, section_name: string }[];
  tiers: { ticket_tier_id: string, name: string, quantity: number, price: number, color_hex: string }[];
  qr_payload: string;
  status: string;
  can_cancel: boolean;
  refund_amount?: number | null;
  refund_status?: 'Pending' | 'Processed' | 'Failed' | null;
  refund_reason?: string | null;
  created_at: string;
  used_at: string | null;
}

@Injectable({
  providedIn: 'root'
})
export class TicketService {
  private readonly http = inject(HttpClient);
  private readonly apiUrl = apiUrl('/api/tickets');

  getUserTickets(): Observable<TicketDetail[]> {
    return this.http.get<TicketDetail[]>(this.apiUrl);
  }

  getTicket(id: string): Observable<TicketDetail> {
    return this.http.get<TicketDetail>(`${this.apiUrl}/${id}`);
  }

  cancelTicket(id: string): Observable<TicketDetail> {
    return this.http.post<TicketDetail>(`${this.apiUrl}/${id}/cancel`, {});
  }
}
