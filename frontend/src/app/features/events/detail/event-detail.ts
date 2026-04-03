import { CommonModule } from '@angular/common';
import { Component, OnDestroy, OnInit, inject } from '@angular/core';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { finalize } from 'rxjs';
import { ToastrService } from 'ngx-toastr';

import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';

import { AuthService } from '../../../core/auth/auth';
import { BookingService, SelectedTicketTier } from '../../../core/services/booking.service';
import { CheckoutService } from '../../../core/services/checkout.service';
import { PublicEventService } from '../../../core/services/public-event.service';
import { WsService } from '../../../core/services/ws.service';
import {
  Event,
  EventAvailability,
  EventDetailsResponse,
  EventImages,
  EventPricing,
  EventPulseResponse,
} from '../../../shared/models/event';
import { EventVenue } from '../../../shared/models/event-venue';
import { EventHero } from './components/event-hero/event-hero';
import { EventInfoSection } from './components/event-info-section/event-info-section';
import { EventSidebar } from './components/event-sidebar/event-sidebar';
import { TicketTierSelectionViewModel } from '../../../shared/components/ticket-quantity-selector/ticket-quantity-selector';

@Component({
  selector: 'app-event-detail',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    MatButtonModule,
    MatIconModule,
    EventHero,
    EventInfoSection,
    EventSidebar,
  ],
  templateUrl: './event-detail.html',
  styleUrl: './event-detail.scss',
})
export class EventDetail implements OnInit, OnDestroy {
  event: Event | null = null;
  venue: EventVenue | null = null;
  images: EventImages = { hero: null, gallery: [] };
  pricing: EventPricing | null = null;
  availability: EventAvailability | null = null;
  selectedTiers: SelectedTicketTier[] = [];
  lightboxImage: string | null = null;
  pulseData: EventPulseResponse | null = null;
  loading = true;
  isHoldingTiers = false;

  private pulseSubscription: { unsubscribe(): void } | null = null;
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly eventService = inject(PublicEventService);
  private readonly wsService = inject(WsService);
  private readonly checkoutService = inject(CheckoutService);
  private readonly bookingService = inject(BookingService);
  private readonly authService = inject(AuthService);
  private readonly toastr = inject(ToastrService);

  get isLoggedIn(): boolean {
    return !!this.authService.currentUserValue;
  }

  get heroImage(): string | null {
    return this.images.hero ?? this.images.gallery[0] ?? this.fallbackImage;
  }

  get galleryImages(): string[] {
    const combined = [this.images.hero ?? '', ...this.images.gallery]
      .map((image) => image.trim())
      .filter(Boolean);

    return combined.filter((image, index) => combined.indexOf(image) === index);
  }

  get fallbackImage(): string {
    return `data:image/svg+xml;utf8,${encodeURIComponent(`
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1600 900">
        <defs>
          <linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="#1b1b1b" />
            <stop offset="100%" stop-color="#0c0c0c" />
          </linearGradient>
        </defs>
        <rect width="1600" height="900" fill="url(#bg)" />
        <circle cx="1200" cy="160" r="220" fill="rgba(255,214,10,0.18)" />
        <circle cx="260" cy="760" r="240" fill="rgba(255,214,10,0.08)" />
        <text x="90" y="760" fill="#ffffff" font-family="Poppins, Arial, sans-serif" font-size="86" font-weight="700">
          GrabAPass Event
        </text>
      </svg>
    `)}`;
  }

  ngOnInit(): void {
    const eventId = this.route.snapshot.paramMap.get('id');
    if (!eventId) {
      this.toastr.error('No event ID provided.', 'Error');
      this.loading = false;
      return;
    }

    this.eventService
      .getEventDetails(eventId)
      .pipe(finalize(() => (this.loading = false)))
      .subscribe({
        next: (details) => {
          this.applyDetails(details);
          this.connectPulse(eventId);
        },
        error: () => {
          this.toastr.error('Event details could not be loaded.', 'Error');
        },
      });
  }

  ngOnDestroy(): void {
    this.pulseSubscription?.unsubscribe();
  }

  goBack(): void {
    this.router.navigate(['/events']);
  }

  openImageViewer(image?: string | null): void {
    this.lightboxImage = image ?? this.heroImage;
  }

  closeImageViewer(): void {
    this.lightboxImage = null;
  }

  shareEvent(): void {
    if (navigator.share) {
      navigator
        .share({
          title: this.event?.title,
          text: 'Check out this event on GrabAPass!',
          url: window.location.href,
        })
        .catch(() => {});
      return;
    }

    navigator.clipboard.writeText(window.location.href);
    this.toastr.info('Link copied to clipboard', 'Shared');
  }

  goToSelectSeats(): void {
    if (!this.event) {
      return;
    }

    if (!this.isLoggedIn) {
      this.router.navigate(['/login'], {
        queryParams: { returnUrl: `/events/${this.event.id}/seats` },
      });
      return;
    }

    this.router.navigate(['/events', this.event.id, 'seats']);
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

  holdGeneralAdmissionTickets(): void {
    if (!this.event || this.selectedTiers.length === 0 || this.isHoldingTiers) {
      return;
    }

    if (!this.isLoggedIn) {
      this.router.navigate(['/login'], {
        queryParams: { returnUrl: `/events/${this.event.id}` },
      });
      return;
    }

    this.isHoldingTiers = true;
    this.checkoutService
      .holdSeats(this.event.id, {
        ticket_tiers: this.selectedTiers.map((tier) => ({
          ticket_tier_id: tier.ticketTierId,
          quantity: tier.quantity,
        })),
      })
      .pipe(finalize(() => (this.isHoldingTiers = false)))
      .subscribe({
        next: (holds) => {
          this.bookingService.setSelection(this.event!, [], this.selectedTiers);
          this.bookingService.setHoldData(
            holds.map((hold) => hold.id),
            new Date(holds[0].expires_at),
          );
          this.router.navigate(['/events', this.event!.id, 'checkout']);
        },
        error: (err) => {
          this.toastr.error(err.error?.message || 'Could not hold ticket quantities.', 'Hold Failed');
        },
      });
  }

  private applyDetails(details: EventDetailsResponse): void {
    this.event = details.event;
    this.venue = details.venue;
    this.images = details.images;
    this.pricing = details.pricing;
    this.availability = details.availability;
  }

  private connectPulse(eventId: string): void {
    this.pulseSubscription = this.wsService.connectToEvent(eventId).subscribe((message) => {
      if (message?.type === 'PULSE') {
        this.pulseData = message.data;
      }
    });
  }
}
