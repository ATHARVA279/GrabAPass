import { CommonModule } from '@angular/common';
import { Component, EventEmitter, Input, OnChanges, Output, SimpleChanges } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';

import { EventTicketTier } from '../../models/event';

export interface TicketTierSelectionViewModel {
  ticketTierId: string;
  name: string;
  quantity: number;
  unitPrice: number;
  colorHex: string;
}

@Component({
  selector: 'app-ticket-quantity-selector',
  standalone: true,
  imports: [CommonModule, MatButtonModule, MatIconModule],
  templateUrl: './ticket-quantity-selector.html',
  styleUrls: ['./ticket-quantity-selector.scss'],
})
export class TicketQuantitySelector implements OnChanges {
  @Input() tiers: EventTicketTier[] = [];
  @Input() disabled = false;
  @Output() selectionChange = new EventEmitter<TicketTierSelectionViewModel[]>();

  quantities: Record<string, number> = {};

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['tiers']) {
      const nextQuantities: Record<string, number> = {};
      for (const tier of this.tiers) {
        nextQuantities[tier.id] = this.quantities[tier.id] ?? 0;
      }
      this.quantities = nextQuantities;
      this.emitSelection();
    }
  }

  increment(tier: EventTicketTier): void {
    if (this.disabled) return;
    this.quantities[tier.id] = (this.quantities[tier.id] ?? 0) + 1;
    this.emitSelection();
  }

  decrement(tier: EventTicketTier): void {
    if (this.disabled) return;
    this.quantities[tier.id] = Math.max(0, (this.quantities[tier.id] ?? 0) - 1);
    this.emitSelection();
  }

  getQuantity(tierId: string): number {
    return this.quantities[tierId] ?? 0;
  }

  get totalCount(): number {
    return this.selectedTiers.reduce((sum, tier) => sum + tier.quantity, 0);
  }

  get subtotal(): number {
    return this.selectedTiers.reduce((sum, tier) => sum + tier.quantity * tier.unitPrice, 0);
  }

  private get selectedTiers(): TicketTierSelectionViewModel[] {
    return this.tiers
      .map((tier) => ({
        ticketTierId: tier.id,
        name: tier.name,
        quantity: this.getQuantity(tier.id),
        unitPrice: tier.price,
        colorHex: tier.color_hex,
      }))
      .filter((tier) => tier.quantity > 0);
  }

  private emitSelection(): void {
    this.selectionChange.emit(this.selectedTiers);
  }
}
