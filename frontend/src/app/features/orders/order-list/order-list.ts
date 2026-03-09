import { Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import { MatCardModule } from '@angular/material/card';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatIconModule } from '@angular/material/icon';
import { MatButtonModule } from '@angular/material/button';
import { OrderService, Order } from '../../../core/services/order.service';
import { finalize } from 'rxjs';

@Component({
  selector: 'app-order-list',
  standalone: true,
  imports: [CommonModule, RouterModule, MatCardModule, MatProgressSpinnerModule, MatIconModule, MatButtonModule],
  templateUrl: './order-list.html',
  styleUrls: ['./order-list.scss']
})
export class OrderList implements OnInit {
  orders: Order[] = [];
  loading = true;
  error = false;

  private orderService = inject(OrderService);

  ngOnInit(): void {
    this.orderService.getUserOrders()
      .pipe(finalize(() => this.loading = false))
      .subscribe({
        next: (orders) => this.orders = orders,
        error: () => this.error = true
      });
  }
}
