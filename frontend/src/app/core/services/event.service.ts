import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';

import { CreateEventRequest, Event } from '../../shared/models/event';
import { Order } from './order.service';

export interface CheckoutInitialization {
  order: Order;
  gateway: string;
  gateway_key_id: string;
  gateway_order_id: string;
  amount: number;
  currency: string;
  description: string;
  customer_name: string;
  customer_email: string;
  hold_expires_at: string;
}

export interface VerifyCheckoutPayload {
  order_id: string;
  razorpay_order_id: string;
  razorpay_payment_id: string;
  razorpay_signature: string;
}

export interface CheckoutFailurePayload {
  order_id: string;
  razorpay_order_id?: string;
  razorpay_payment_id?: string;
  reason?: string;
}

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

  holdSeats(eventId: string, seatIds: string[]): Observable<any[]> {
    return this.http.post<any[]>(`${this.publicApiUrl}/${eventId}/holds`, { seat_ids: seatIds });
  }

  initializeCheckout(eventId: string, seatIds: string[]): Observable<CheckoutInitialization> {
    return this.http.post<CheckoutInitialization>(
      `${this.publicApiUrl}/${eventId}/checkout/initialize`,
      { seat_ids: seatIds }
    );
  }

  verifyCheckout(eventId: string, payload: VerifyCheckoutPayload): Observable<Order> {
    return this.http.post<Order>(`${this.publicApiUrl}/${eventId}/checkout/verify`, payload);
  }

  recordCheckoutFailure(eventId: string, payload: CheckoutFailurePayload): Observable<void> {
    return this.http.post<void>(`${this.publicApiUrl}/${eventId}/checkout/failure`, payload);
  }
}
