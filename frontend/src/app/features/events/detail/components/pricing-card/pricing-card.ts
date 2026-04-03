import { CommonModule } from '@angular/common';
import { Component, EventEmitter, Input, Output } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';

import { SelectedTicketTier } from '../../../../../core/services/booking.service';
import {
  TicketQuantitySelector,
  TicketTierSelectionViewModel,
} from '../../../../../shared/components/ticket-quantity-selector/ticket-quantity-selector';
import { EventAvailability, EventPricing } from '../../../../../shared/models/event';

@Component({
  selector: 'app-pricing-card',
  standalone: true,
  imports: [CommonModule, MatButtonModule, MatIconModule, TicketQuantitySelector],
  templateUrl: './pricing-card.html',
  styleUrl: './pricing-card.scss',
})
export class PricingCard {
  @Input({ required: true }) pricing!: EventPricing;
  @Input({ required: true }) availability!: EventAvailability;
  @Input() isLoggedIn = false;
  @Input() isHoldingTiers = false;
  @Input() selectedTiers: SelectedTicketTier[] = [];

  @Output() selectSeats = new EventEmitter<void>();
  @Output() holdTickets = new EventEmitter<void>();
  @Output() selectionChanged = new EventEmitter<TicketTierSelectionViewModel[]>();

  get hasTierSelection(): boolean {
    return this.pricing.tiers.length > 0;
  }

  get gaSelectionCount(): number {
    return this.selectedTiers.reduce((sum, tier) => sum + tier.quantity, 0);
  }
}
