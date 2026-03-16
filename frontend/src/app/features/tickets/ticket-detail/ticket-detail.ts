import { Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { MatCardModule } from '@angular/material/card';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatIconModule } from '@angular/material/icon';
import { MatButtonModule } from '@angular/material/button';
import { QRCodeComponent } from 'angularx-qrcode';
import { ToastrService } from 'ngx-toastr';
import { TicketService, TicketDetail } from '../../../core/services/ticket.service';
import { finalize } from 'rxjs';
import { getTicketStatusClass } from '../../../shared/utils/ticket-status';

@Component({
  selector: 'app-ticket-detail',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    MatCardModule,
    MatProgressSpinnerModule,
    MatIconModule,
    MatButtonModule,
    QRCodeComponent,
  ],
  templateUrl: './ticket-detail.html',
  styleUrls: ['./ticket-detail.scss']
})
export class TicketDetailPage implements OnInit {
  ticket: TicketDetail | null = null;
  loading = true;
  cancelling = false;

  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly ticketService = inject(TicketService);
  private readonly toastr = inject(ToastrService);

  ngOnInit(): void {
    const ticketId = this.route.snapshot.paramMap.get('id');
    if (!ticketId) {
      this.toastr.error('No ticket ID provided.', 'Error');
      this.loading = false;
      return;
    }

    this.ticketService.getTicket(ticketId)
      .pipe(finalize(() => this.loading = false))
      .subscribe({
        next: (ticket) => this.ticket = ticket,
        error: () => {
          this.toastr.error('Ticket not found or failed to load.', 'Error');
        }
      });
  }

  getStatusClass(status: string): string {
    return getTicketStatusClass(status);
  }

  goBack(): void {
    this.router.navigate(['/tickets']);
  }

  cancelTicket(): void {
    if (!this.ticket || !this.ticket.can_cancel || this.cancelling) {
      return;
    }

    const confirmed = window.confirm(
      `Cancel "${this.ticket.event_title}" and invalidate this ticket?`
    );

    if (!confirmed) {
      return;
    }

    this.cancelling = true;
    this.ticketService.cancelTicket(this.ticket.id)
      .pipe(finalize(() => this.cancelling = false))
      .subscribe({
        next: (ticket) => {
          this.ticket = ticket;
          this.toastr.success('Ticket cancelled successfully.', 'Cancelled');
        },
        error: (err) => {
          const message = err.error?.message || 'Failed to cancel ticket.';
          this.toastr.error(message, 'Error');
        }
      });
  }
}
