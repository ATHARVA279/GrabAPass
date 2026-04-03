import { CommonModule } from '@angular/common';
import { Component, EventEmitter, Input, Output } from '@angular/core';

import { SelectedTicketTier } from '../../../../../core/services/booking.service';
import {
  Event,
  EventAvailability,
  EventPricing,
  EventPulseResponse,
} from '../../../../../shared/models/event';
import { EventVenue } from '../../../../../shared/models/event-venue';
import { PricingCard } from '../pricing-card/pricing-card';
import { LocationMap } from '../location-map/location-map';
import { TicketTierSelectionViewModel } from '../../../../../shared/components/ticket-quantity-selector/ticket-quantity-selector';

@Component({
  selector: 'app-event-sidebar',
  standalone: true,
  imports: [CommonModule, PricingCard, LocationMap],
  templateUrl: './event-sidebar.html',
  styleUrl: './event-sidebar.scss',
})
export class EventSidebar {
  @Input({ required: true }) event!: Event;
  @Input({ required: true }) pricing!: EventPricing;
  @Input({ required: true }) availability!: EventAvailability;
  @Input() venue: EventVenue | null = null;
  @Input() isLoggedIn = false;
  @Input() isHoldingTiers = false;
  @Input() selectedTiers: SelectedTicketTier[] = [];
  @Input() pulseData: EventPulseResponse | null = null;

  @Output() selectSeats = new EventEmitter<void>();
  @Output() holdTickets = new EventEmitter<void>();
  @Output() selectionChanged = new EventEmitter<TicketTierSelectionViewModel[]>();
}
