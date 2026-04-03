import { Component, inject, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, RouterModule, Router } from '@angular/router';
import { finalize } from 'rxjs';
import { ToastrService } from 'ngx-toastr';

import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatChipsModule } from '@angular/material/chips';

import { PublicEventService } from '../../../core/services/public-event.service';
import { WsService } from '../../../core/services/ws.service';
import { VenueService } from '../../../core/services/venue.service';
import { CheckoutService } from '../../../core/services/checkout.service';
import { BookingService, SelectedTicketTier } from '../../../core/services/booking.service';
import { AuthService } from '../../../core/auth/auth';
import { Event, EventPulseResponse, EventTicketTier } from '../../../shared/models/event';
import { SeatLayoutResponse } from '../../../shared/models/venue';
import {
  TicketQuantitySelector,
  TicketTierSelectionViewModel,
} from '../../../shared/components/ticket-quantity-selector/ticket-quantity-selector';

@Component({
  selector: 'app-event-detail',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    MatCardModule,
    MatButtonModule,
    MatIconModule,
    MatProgressSpinnerModule,
    MatChipsModule,
    TicketQuantitySelector,
  ],
  templateUrl: './event-detail.html',
  styleUrls: ['./event-detail.scss']
})
export class EventDetail implements OnInit {
  event: Event | null = null;
  ticketTiers: EventTicketTier[] = [];
  selectedTiers: SelectedTicketTier[] = [];
  selectedGalleryImage: string | null = null;
  loading = true;
  tierLoading = false;
  isHoldingTiers = false;
  priceRange: { min: number; max: number } | null = null;
  pulseData: EventPulseResponse | null = null;
  private pulseTimer: any;

  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly eventService = inject(PublicEventService);
  private readonly wsService = inject(WsService);
  private readonly venueService = inject(VenueService);
  private readonly checkoutService = inject(CheckoutService);
  private readonly bookingService = inject(BookingService);
  private readonly authService = inject(AuthService);
  private readonly toastr = inject(ToastrService);

  get isLoggedIn(): boolean {
    return !!this.authService.currentUserValue;
  }

  ngOnInit(): void {
    const eventId = this.route.snapshot.paramMap.get('id');

    if (!eventId) {
      this.toastr.error('No event ID provided.', 'Error');
      this.loading = false;
      return;
    }

    this.eventService.getEventById(eventId).pipe(
      finalize(() => (this.loading = false))
    ).subscribe({
      next: (event) => {
        this.event = event;
        this.selectedGalleryImage = this.eventGallery[0] ?? event.image_url ?? null;
        if (event.venue_template_id) {
          this.loadPriceRange(eventId);
        }
        this.loadTicketTiers(eventId);
      },
      error: () => this.toastr.error('Event not found or failed to load.', 'Error')
    });

    this.loadPulseData(eventId);
    
    // Subscribe to real-time WebSockets
    this.pulseTimer = this.wsService.connectToEvent(eventId).subscribe(msg => {
      if (msg && msg.type === 'PULSE') {
        this.pulseData = msg.data;
      }
    });
  }

  ngOnDestroy(): void {
    if (this.pulseTimer) {
      this.pulseTimer.unsubscribe();
    }
  }

  private loadPulseData(eventId: string): void {
    this.eventService.getEventPulse(eventId).subscribe({
      next: (data) => this.pulseData = data,
      error: () => {}
    });
  }

  private loadPriceRange(eventId: string): void {
    this.venueService.getSeatLayout(eventId).subscribe({
      next: (layout: SeatLayoutResponse) => {
        const prices = layout.sections
          .filter(s => s.category)
          .map(s => s.category!.price);
        if (prices.length > 0) {
          this.priceRange = { min: Math.min(...prices), max: Math.max(...prices) };
        }
      },
      error: () => {}
    });
  }

  goToSelectSeats(): void {
    if (this.event) {
      if (!this.isLoggedIn) {
        this.router.navigate(['/login'], {
          queryParams: { returnUrl: `/events/${this.event.id}/seats` }
        });
        return;
      }
      this.router.navigate(['/events', this.event.id, 'seats']);
    }
  }

  onTierSelectionChanged(selection: TicketTierSelectionViewModel[]): void {
    this.selectedTiers = selection.map((tier) => ({
      ticketTierId: tier.ticketTierId,
      name: tier.name,
      quantity: tier.quantity,
      unitPrice: tier.unitPrice,
      colorHex: tier.colorHex,
    }));
  }

  get gaSelectionCount(): number {
    return this.selectedTiers.reduce((sum, tier) => sum + tier.quantity, 0);
  }

  holdGeneralAdmissionTickets(): void {
    if (!this.event || this.gaSelectionCount === 0 || this.isHoldingTiers) {
      return;
    }

    if (!this.isLoggedIn) {
      this.router.navigate(['/login'], {
        queryParams: { returnUrl: `/events/${this.event.id}` }
      });
      return;
    }

    this.isHoldingTiers = true;
    this.checkoutService.holdSeats(this.event.id, {
      ticket_tiers: this.selectedTiers.map((tier) => ({
        ticket_tier_id: tier.ticketTierId,
        quantity: tier.quantity,
      })),
    }).pipe(
      finalize(() => (this.isHoldingTiers = false))
    ).subscribe({
      next: (holds) => {
        this.bookingService.setSelection(this.event!, [], this.selectedTiers);
        this.bookingService.setHoldData(holds.map((hold) => hold.id), new Date(holds[0].expires_at));
        this.router.navigate(['/events', this.event!.id, 'checkout']);
      },
      error: (err) => {
        this.toastr.error(err.error?.message || 'Could not hold ticket quantities.', 'Hold Failed');
      },
    });
  }

  goBack(): void {
    this.router.navigate(['/events']);
  }

  get eventGallery(): string[] {
    if (!this.event) {
      return [];
    }

    const gallery = [this.event.image_url ?? '', ...(this.event.image_gallery ?? [])]
      .map((image) => image.trim())
      .filter((image) => !!image);

    return gallery.filter((image, index) => gallery.indexOf(image) === index);
  }

  get heroImage(): string | null {
    return this.selectedGalleryImage ?? this.eventGallery[0] ?? null;
  }

  selectGalleryImage(image: string): void {
    this.selectedGalleryImage = image;
  }

  shareEvent(): void {
    if (navigator.share) {
      navigator.share({
        title: this.event?.title,
        text: 'Check out this event on GrabAPass!',
        url: window.location.href
      }).catch(() => {});
    } else {
      navigator.clipboard.writeText(window.location.href);
      this.toastr.info('Link copied to clipboard', 'Shared');
    }
  }

  private loadTicketTiers(eventId: string): void {
    this.tierLoading = true;
    this.eventService.getEventTicketTiers(eventId).pipe(
      finalize(() => (this.tierLoading = false))
    ).subscribe({
      next: (tiers) => {
        this.ticketTiers = tiers;
        const prices = tiers.map((tier) => tier.price);
        if (prices.length > 0) {
          const min = Math.min(...prices);
          const max = Math.max(...prices);
          this.priceRange = this.priceRange
            ? { min: Math.min(this.priceRange.min, min), max: Math.max(this.priceRange.max, max) }
            : { min, max };
        }
      },
      error: () => {
        this.ticketTiers = [];
      },
    });
  }

}
