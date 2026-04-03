import { Injectable } from '@angular/core';
import { Event } from '../../shared/models/event';
import { SelectedSeat } from '../../shared/components/seat-map-renderer/seat-map-renderer';

export interface SelectedTicketTier {
  ticketTierId: string;
  name: string;
  quantity: number;
  unitPrice: number;
  colorHex: string;
}

export interface BookingState {
  event: Event;
  selectedSeats: SelectedSeat[];
  selectedTiers: SelectedTicketTier[];
  holdIds: string[];
  holdExpiresAt: Date | null;
  totalPrice: number;
  orderId?: string;
}

interface StoredBookingState {
  event: Event;
  selectedSeats: SelectedSeat[];
  selectedTiers: SelectedTicketTier[];
  holdIds: string[];
  holdExpiresAt: string | null;
  totalPrice: number;
  orderId?: string;
}

@Injectable({ providedIn: 'root' })
export class BookingService {
  private static readonly storageKey = 'grabapass_booking_state';
  private state: BookingState | null = null;

  constructor() {
    this.restoreState();
  }

  setSelection(event: Event, seats: SelectedSeat[], tiers: SelectedTicketTier[] = []): void {
    this.state = {
      event,
      selectedSeats: seats,
      selectedTiers: tiers,
      holdIds: [],
      holdExpiresAt: null,
      totalPrice:
        seats.reduce((sum, s) => sum + s.price, 0) +
        tiers.reduce((sum, tier) => sum + tier.unitPrice * tier.quantity, 0),
    };
    this.persistState();
  }

  setHoldData(holdIds: string[], expiresAt: Date): void {
    if (!this.state) return;
    this.state.holdIds = holdIds;
    this.state.holdExpiresAt = expiresAt;
    this.persistState();
  }

  setOrderId(orderId: string): void {
    if (!this.state) return;
    this.state.orderId = orderId;
    this.persistState();
  }

  getState(): BookingState | null {
    return this.state;
  }

  clear(): void {
    this.state = null;
    this.clearPersistedState();
  }

  private persistState(): void {
    if (typeof window === 'undefined' || !this.state) return;

    const storedState: StoredBookingState = {
      event: this.state.event,
      selectedSeats: this.state.selectedSeats,
      selectedTiers: this.state.selectedTiers,
      holdIds: this.state.holdIds,
      holdExpiresAt: this.state.holdExpiresAt ? this.state.holdExpiresAt.toISOString() : null,
      totalPrice: this.state.totalPrice,
      orderId: this.state.orderId,
    };

    window.sessionStorage.setItem(
      BookingService.storageKey,
      JSON.stringify(storedState)
    );
  }

  private restoreState(): void {
    if (typeof window === 'undefined') return;

    const rawState = window.sessionStorage.getItem(BookingService.storageKey);
    if (!rawState) return;

    try {
      const parsed = JSON.parse(rawState) as StoredBookingState;
      const holdExpiresAt = parsed.holdExpiresAt ? new Date(parsed.holdExpiresAt) : null;

      if (holdExpiresAt && Number.isNaN(holdExpiresAt.getTime())) {
        this.clearPersistedState();
        return;
      }

      if (holdExpiresAt && holdExpiresAt.getTime() <= Date.now()) {
        this.clearPersistedState();
        return;
      }

      this.state = {
        event: parsed.event,
        selectedSeats: parsed.selectedSeats,
        selectedTiers: parsed.selectedTiers ?? [],
        holdIds: parsed.holdIds ?? [],
        holdExpiresAt,
        totalPrice: parsed.totalPrice,
        orderId: parsed.orderId,
      };
    } catch {
      this.clearPersistedState();
    }
  }

  private clearPersistedState(): void {
    if (typeof window === 'undefined') return;
    window.sessionStorage.removeItem(BookingService.storageKey);
  }
}
