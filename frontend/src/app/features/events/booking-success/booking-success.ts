import { Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatCardModule } from '@angular/material/card';

@Component({
  selector: 'app-booking-success',
  standalone: true,
  imports: [CommonModule, RouterModule, MatButtonModule, MatIconModule, MatCardModule],
  templateUrl: './booking-success.html',
  styleUrls: ['./booking-success.scss']
})
export class BookingSuccess implements OnInit {
  orderId: string | null = null;
  eventId: string | null = null;
  confettiPieces = Array(15).fill(0);

  private route = inject(ActivatedRoute);
  private router = inject(Router);

  ngOnInit() {
    this.eventId = this.route.snapshot.paramMap.get('id');
    this.orderId = this.route.snapshot.queryParamMap.get('orderId');
    
    if (!this.orderId) {
      this.router.navigate(['/tickets']);
    }
  }

  viewTickets() {
    this.router.navigate(['/tickets']);
  }
}
