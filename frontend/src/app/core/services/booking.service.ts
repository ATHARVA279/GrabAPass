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

@Injectable({ providedIn: 'root' })
export class BookingService {
  private state: BookingState | null = null;

  setSelectedSeats(event: Event, seats: SelectedSeat[]): void {
    this.state = {
      event,
      selectedSeats: seats,
      heldSeatIds: [],
      holdExpiresAt: null,
      totalPrice: seats.reduce((sum, s) => sum + s.price, 0),
    };
  }

  setHoldData(seatIds: string[], expiresAt: Date): void {
    if (!this.state) return;
    this.state.heldSeatIds = seatIds;
    this.state.holdExpiresAt = expiresAt;
  }

  getState(): BookingState | null {
    return this.state;
  }

  clear(): void {
    this.state = null;
  }

  get hasHeldSeats(): boolean {
    return !!this.state && this.state.heldSeatIds.length > 0;
  }

  get hasSelectedSeats(): boolean {
    return !!this.state && this.state.selectedSeats.length > 0;
  }
}
