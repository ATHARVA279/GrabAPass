import { Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import { MatCardModule } from '@angular/material/card';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatIconModule } from '@angular/material/icon';
import { MatButtonModule } from '@angular/material/button';
import { MatChipsModule } from '@angular/material/chips';
import { TicketService, TicketDetail } from '../../../core/services/ticket.service';
import { finalize } from 'rxjs';
import { getTicketStatusClass } from '../../../shared/utils/ticket-status';

@Component({
  selector: 'app-ticket-wallet',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    MatCardModule,
    MatProgressSpinnerModule,
    MatIconModule,
    MatButtonModule,
    MatChipsModule,
  ],
  templateUrl: './ticket-wallet.html',
  styleUrls: ['./ticket-wallet.scss']
})
export class TicketWallet implements OnInit {
  tickets: TicketDetail[] = [];
  loading = true;
  error = false;

  private ticketService = inject(TicketService);

  ngOnInit(): void {
    this.ticketService.getUserTickets()
      .pipe(finalize(() => this.loading = false))
      .subscribe({
        next: (tickets) => this.tickets = tickets,
        error: () => this.error = true
      });
  }

  getStatusClass(status: string): string {
    return getTicketStatusClass(status);
  }

  getTicketCount(ticket: TicketDetail): number {
    const tierCount = ticket.tiers.reduce((sum, tier) => sum + tier.quantity, 0);
    return ticket.seats.length + tierCount;
  }
}
