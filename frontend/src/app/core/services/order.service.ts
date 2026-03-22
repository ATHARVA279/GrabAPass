import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable } from 'rxjs';
import { apiUrl } from '../api/api-url';

export interface Order {
  id: string;
  user_id: string;
  event_id: string;
  subtotal_amount: number;
  fee_amount: number;
  total_amount: number;
  currency: string;
  status: string;
  gateway?: string | null;
  gateway_order_id?: string | null;
  gateway_payment_id?: string | null;
  payment_signature?: string | null;
  payment_verified_at?: string | null;
  receipt?: string | null;
  failure_reason?: string | null;
  created_at: string;
}

@Injectable({
  providedIn: 'root'
})
export class OrderService {
  private readonly http = inject(HttpClient);
  private readonly apiUrl = apiUrl('/api/orders');

  getUserOrders(): Observable<Order[]> {
    return this.http.get<Order[]>(this.apiUrl);
  }
}
