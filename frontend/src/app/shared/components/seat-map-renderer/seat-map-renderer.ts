import {
  Component, Input, Output, EventEmitter,
  ChangeDetectionStrategy, OnChanges,
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { SeatLayout, SeatLayoutResponse, SeatStatus, SectionLayout } from '../../models/venue';

export interface SelectedSeat {
  seatId: string;
  seatLabel: string;
  sectionName: string;
  categoryName: string;
  price: number;
  colorHex: string;
}

@Component({
  selector: 'app-seat-map-renderer',
  standalone: true,
  imports: [CommonModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './seat-map-renderer.html',
  styleUrls: ['./seat-map-renderer.scss'],
})
export class SeatMapRenderer implements OnChanges {
  @Input() layout!: SeatLayoutResponse;
  @Output() seatSelected = new EventEmitter<SelectedSeat>();

  selectedSeatIds = new Set<string>();
  selectedSeats: SelectedSeat[] = [];

  ngOnChanges(): void {
    this.selectedSeatIds.clear();
    this.selectedSeats = [];
  }

  // ── Legend data ────────────────────────────────────────────────────────────

  get legendItems(): { label: string; color: string }[] {
    const items: { label: string; color: string }[] = [];
    for (const section of this.layout.sections) {
      if (section.category) {
        items.push({
          label: `${section.name} — ${section.category.name} ($${section.category.price.toFixed(2)})`,
          color: section.category.color_hex,
        });
      }
    }
    items.push({ label: 'Sold / Unavailable', color: '#3a3a3a' });
    items.push({ label: 'Selected', color: '#FFD60A' });
    return items;
  }

  // ── Seat interaction ───────────────────────────────────────────────────────

  onSeatClick(seat: SeatLayout, section: SectionLayout): void {
    if (!this.isSeatClickable(seat)) return;

    if (this.selectedSeatIds.has(seat.id)) {
      this.selectedSeatIds.delete(seat.id);
      this.selectedSeats = this.selectedSeats.filter((s) => s.seatId !== seat.id);
    } else {
      const selected: SelectedSeat = {
        seatId:       seat.id,
        seatLabel:    seat.seat_label,
        sectionName:  section.name,
        categoryName: section.category?.name ?? '',
        price:        section.category?.price ?? 0,
        colorHex:     section.category?.color_hex ?? '#4A90D9',
      };
      this.selectedSeatIds.add(seat.id);
      this.selectedSeats.push(selected);
      this.seatSelected.emit(selected);
    }
  }

  isSeatClickable(seat: SeatLayout): boolean {
    return seat.status === 'Available';
  }

  // ── Template helpers ───────────────────────────────────────────────────────

  seatColor(seat: SeatLayout, section: SectionLayout): string {
    if (this.selectedSeatIds.has(seat.id)) return '#FFD60A';
    if (seat.status === 'Sold')    return '#3a3a3a';
    if (seat.status === 'Held')    return '#6b6b00';
    if (seat.status === 'Blocked') return '#2a2a2a';
    return section.category?.color_hex ?? '#4A90D9';
  }

  seatTitle(seat: SeatLayout, section: SectionLayout): string {
    const parts: string[] = [seat.seat_label];
    if (section.category) parts.push(`${section.category.name} $${section.category.price.toFixed(2)}`);
    if (seat.is_accessible) parts.push('♿ Accessible');
    if (seat.is_vip)        parts.push('⭐ VIP');
    if (seat.is_aisle)      parts.push('Aisle');
    parts.push(seat.status);
    return parts.join(' · ');
  }

  trackSeat(_: number, seat: SeatLayout): string { return seat.id; }
  trackSection(_: number, section: SectionLayout): string { return section.id; }
}
