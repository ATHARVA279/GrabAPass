import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable } from 'rxjs';

import { Order } from './order.service';
import { apiUrl } from '../api/api-url';

export interface TicketTierSelectionPayload {
  ticket_tier_id: string;
  quantity: number;
}

export interface HoldSelectionPayload {
  seat_ids?: string[];
  ticket_tiers?: TicketTierSelectionPayload[];
}

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

export interface SplitShare {
  id: string;
  split_session_id: string;
  amount_due: number;
  status: 'Pending' | 'Completed' | 'Expired' | 'Refunded';
  is_host_share: boolean;
  guest_name?: string;
  guest_email?: string;
  payment_token: string;
  claimed_ticket_id?: string;
  claimed_at?: string;
  paid_at?: string;
  pending_manual_refund?: boolean;
}

export interface SplitSession {
  id: string;
  order_id: string;
  total_amount: number;
  split_type: 'Even' | 'Custom';
  status: 'Pending' | 'Completed' | 'Expired' | 'Refunded';
  expires_at: string;
  shares?: SplitShare[];
}

export interface InitializeSplitPayload {
  split_type: 'Even' | 'Custom';
  num_shares?: number;
  custom_shares?: {
    guest_name?: string;
    guest_email?: string;
    seat_ids: string[];
    ticket_tiers?: {
      ticket_tier_id: string;
      quantity: number;
    }[];
  }[];
}

@Injectable({
  providedIn: 'root'
})
export class CheckoutService {
  private readonly http = inject(HttpClient);
  private readonly publicApiUrl = apiUrl('/api/events');
  private readonly ordersApiUrl = apiUrl('/api/orders');

  holdSeats(eventId: string, payload: HoldSelectionPayload): Observable<any[]> {
    return this.http.post<any[]>(`${this.publicApiUrl}/${eventId}/holds`, payload);
  }

  initializeCheckout(eventId: string, holdIds: string[]): Observable<CheckoutInitialization> {
    return this.http.post<CheckoutInitialization>(
      `${this.publicApiUrl}/${eventId}/checkout/initialize`,
      { hold_ids: holdIds }
    );
  }

  initializeSplit(orderId: string, payload: InitializeSplitPayload): Observable<SplitSession> {
    return this.http.post<SplitSession>(
      `${this.ordersApiUrl}/${orderId}/split`,
      payload
    );
  }

  getSplitSession(orderId: string): Observable<SplitSession> {
    return this.http.get<SplitSession>(
      `${this.ordersApiUrl}/${orderId}/split`
    );
  }

  verifyCheckout(eventId: string, payload: VerifyCheckoutPayload): Observable<Order> {
    return this.http.post<Order>(`${this.publicApiUrl}/${eventId}/checkout/verify`, payload);
  }

  recordCheckoutFailure(eventId: string, payload: CheckoutFailurePayload): Observable<void> {
    return this.http.post<void>(`${this.publicApiUrl}/${eventId}/checkout/failure`, payload);
  }
}
