import { Injectable } from '@angular/core';
import { Event } from '../../shared/models/event';
import { SelectedSeat } from '../../shared/components/seat-map-renderer/seat-map-renderer';

export interface BookingState {
  event: Event;
  selectedSeats: SelectedSeat[];
  heldSeatIds: string[];
  holdExpiresAt: Date | null;
  totalPrice: number;
}

interface StoredBookingState {
  event: Event;
  selectedSeats: SelectedSeat[];
  heldSeatIds: string[];
  holdExpiresAt: string | null;
  totalPrice: number;
}

@Injectable({ providedIn: 'root' })
export class BookingService {
  private static readonly storageKey = 'grabapass_booking_state';
  private state: BookingState | null = null;

  constructor() {
    this.restoreState();
  }

  setSelectedSeats(event: Event, seats: SelectedSeat[]): void {
    this.state = {
      event,
      selectedSeats: seats,
      heldSeatIds: [],
      holdExpiresAt: null,
      totalPrice: seats.reduce((sum, s) => sum + s.price, 0),
    };
    this.persistState();
  }

  setHoldData(seatIds: string[], expiresAt: Date): void {
    if (!this.state) return;
    this.state.heldSeatIds = seatIds;
    this.state.holdExpiresAt = expiresAt;
    this.persistState();
  }

  getState(): BookingState | null {
    return this.state;
  }

  clear(): void {
    this.state = null;
    this.clearPersistedState();
  }

  get hasHeldSeats(): boolean {
    return !!this.state && this.state.heldSeatIds.length > 0;
  }

  get hasSelectedSeats(): boolean {
    return !!this.state && this.state.selectedSeats.length > 0;
  }

  private persistState(): void {
    if (typeof window === 'undefined' || !this.state) return;

    const storedState: StoredBookingState = {
      event: this.state.event,
      selectedSeats: this.state.selectedSeats,
      heldSeatIds: this.state.heldSeatIds,
      holdExpiresAt: this.state.holdExpiresAt ? this.state.holdExpiresAt.toISOString() : null,
      totalPrice: this.state.totalPrice,
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
        heldSeatIds: parsed.heldSeatIds,
        holdExpiresAt,
        totalPrice: parsed.totalPrice,
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
