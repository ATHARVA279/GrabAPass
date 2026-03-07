import { Component, Inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { MatDialogModule, MatDialogRef, MAT_DIALOG_DATA } from '@angular/material/dialog';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';

export interface TimePickerData {
  hour: number;   // 0–23
  minute: number; // 0–59
}

@Component({
  selector: 'app-time-picker-dialog',
  standalone: true,
  imports: [CommonModule, FormsModule, MatDialogModule, MatButtonModule, MatIconModule],
  template: `
    <div class="tp-wrapper">
      <div class="tp-header">
        <span class="tp-label">Select Time</span>
        <div class="tp-display">
          <span class="tp-digit" [class.active]="editing === 'hour'" (click)="editing = 'hour'">{{ pad(hour) }}</span>
          <span class="tp-sep">:</span>
          <span class="tp-digit" [class.active]="editing === 'minute'" (click)="editing = 'minute'">{{ pad(minute) }}</span>
          <div class="tp-ampm">
            <button class="ampm-btn" [class.active]="isPM === false" (click)="setAM()">AM</button>
            <button class="ampm-btn" [class.active]="isPM === true" (click)="setPM()">PM</button>
          </div>
        </div>
      </div>

      <div class="tp-clock">
        <div class="clock-face" (click)="onFaceClick($event)" #clockFace>
          <!-- tick marks -->
          <div class="center-dot"></div>
          <div class="hand" [style.transform]="'rotate(' + handDeg + 'deg)'"></div>
          <div
            *ngFor="let n of clockNumbers"
            class="clock-num"
            [class.selected]="isSelected(n)"
            [style]="numStyle(n)"
            (click)="$event.stopPropagation(); selectNumber(n)">
            {{ editing === 'hour' ? displayHour(n) : pad(n * minuteStep) }}
          </div>
        </div>
      </div>

      <div class="tp-actions">
        <button mat-button (click)="cancel()">Cancel</button>
        <button mat-flat-button class="ok-btn" (click)="confirm()">OK</button>
      </div>
    </div>
  `,
  styles: [`
    .tp-wrapper {
      background: #1a1a1a;
      border-radius: 16px;
      overflow: hidden;
      width: 300px;
      font-family: var(--font, 'Poppins', sans-serif);
    }
    .tp-header {
      background: #111;
      padding: 20px 24px 16px;
    }
    .tp-label {
      font-size: 0.72rem;
      letter-spacing: 0.1em;
      text-transform: uppercase;
      color: rgba(255,255,255,0.5);
      display: block;
      margin-bottom: 8px;
    }
    .tp-display {
      display: flex;
      align-items: center;
      gap: 2px;
    }
    .tp-digit {
      font-size: 3.2rem;
      font-weight: 700;
      color: rgba(255,255,255,0.35);
      cursor: pointer;
      padding: 0 4px;
      border-radius: 8px;
      transition: color 0.15s, background 0.15s;
      line-height: 1;
      &.active {
        color: #f5c400;
        background: rgba(245,196,0,0.1);
      }
    }
    .tp-sep {
      font-size: 3.2rem;
      font-weight: 700;
      color: rgba(255,255,255,0.35);
      line-height: 1;
      margin-bottom: 2px;
    }
    .tp-ampm {
      display: flex;
      flex-direction: column;
      gap: 4px;
      margin-left: 8px;
      .ampm-btn {
        background: none;
        border: none;
        font-size: 0.85rem;
        font-weight: 600;
        font-family: inherit;
        color: rgba(255,255,255,0.35);
        cursor: pointer;
        padding: 2px 6px;
        border-radius: 6px;
        transition: color 0.15s, background 0.15s;
        &.active {
          color: #f5c400;
          background: rgba(245,196,0,0.1);
        }
      }
    }
    .tp-clock {
      display: flex;
      justify-content: center;
      padding: 20px;
    }
    .clock-face {
      position: relative;
      width: 220px;
      height: 220px;
      border-radius: 50%;
      background: #111;
      cursor: pointer;
    }
    .center-dot {
      position: absolute;
      top: 50%; left: 50%;
      transform: translate(-50%, -50%);
      width: 8px; height: 8px;
      border-radius: 50%;
      background: #f5c400;
    }
    .hand {
      position: absolute;
      bottom: 50%; left: 50%;
      transform-origin: bottom center;
      width: 2px;
      height: 80px;
      background: #f5c400;
      margin-left: -1px;
      border-radius: 2px 2px 0 0;
      transition: transform 0.15s ease;
    }
    .clock-num {
      position: absolute;
      width: 36px;
      height: 36px;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 0.85rem;
      font-weight: 600;
      color: rgba(255,255,255,0.7);
      cursor: pointer;
      transition: background 0.12s, color 0.12s;
      transform: translate(-50%, -50%);
      &:hover { background: rgba(255,255,255,0.08); }
      &.selected {
        background: #f5c400;
        color: #000;
      }
    }
    .tp-actions {
      display: flex;
      justify-content: flex-end;
      gap: 8px;
      padding: 8px 16px 16px;
      button { font-family: inherit; font-weight: 600; }
      .ok-btn { background: #f5c400 !important; color: #000 !important; }
    }
  `]
})
export class TimePickerDialog {
  editing: 'hour' | 'minute' = 'hour';
  hour: number;
  minute: number;
  isPM: boolean;

  // For hour clock: positions 0–11 → displayed as 12,1..11 (AM) or 12,1..11 (PM)
  readonly clockNumbers = Array.from({ length: 12 }, (_, i) => i); // 0..11
  readonly minuteStep = 5;

  constructor(
    private dialogRef: MatDialogRef<TimePickerDialog>,
    @Inject(MAT_DIALOG_DATA) data: TimePickerData
  ) {
    // Normalize to 12h
    const h24 = data.hour ?? 9;
    this.minute = data.minute ?? 0;
    this.isPM = h24 >= 12;
    this.hour = h24 % 12 === 0 ? 12 : h24 % 12;
  }

  // ── Display helpers ──────────────────────────────────────────────────────────

  pad(n: number): string {
    return String(n).padStart(2, '0');
  }

  displayHour(index: number): string {
    // index 0 → "12", 1→"1" ... 11→"11"
    return String(index === 0 ? 12 : index);
  }

  // ── Hand angle ───────────────────────────────────────────────────────────────

  get handDeg(): number {
    if (this.editing === 'hour') {
      const h = this.hour % 12; // 12 → 0
      return h * 30;            // 360 / 12 = 30 deg per hour
    } else {
      return (this.minute / 60) * 360;
    }
  }

  // ── Clock number positions ───────────────────────────────────────────────────

  numStyle(index: number): object {
    const total = 12;
    const radius = 88; // px from center
    const angle = ((index / total) * 360 - 90) * (Math.PI / 180);
    const cx = 110 + radius * Math.cos(angle);
    const cy = 110 + radius * Math.sin(angle);
    return { left: cx + 'px', top: cy + 'px' };
  }

  isSelected(index: number): boolean {
    if (this.editing === 'hour') {
      const h = this.hour % 12; // 12 → 0
      return index === h;
    } else {
      // nearest 5-min slot
      const slot = Math.round(this.minute / 5) % 12;
      return index === slot;
    }
  }

  selectNumber(index: number): void {
    if (this.editing === 'hour') {
      this.hour = index === 0 ? 12 : index;
      // Auto-advance to minute
      setTimeout(() => { this.editing = 'minute'; }, 200);
    } else {
      this.minute = (index * this.minuteStep) % 60;
    }
  }

  onFaceClick(event: MouseEvent): void {
    const el = event.currentTarget as HTMLElement;
    const rect = el.getBoundingClientRect();
    const cx = rect.left + rect.width / 2;
    const cy = rect.top + rect.height / 2;
    const angle = Math.atan2(event.clientY - cy, event.clientX - cx) * (180 / Math.PI) + 90;
    const deg = (angle + 360) % 360;

    if (this.editing === 'hour') {
      let h = Math.round(deg / 30) % 12;
      this.hour = h === 0 ? 12 : h;
      setTimeout(() => { this.editing = 'minute'; }, 200);
    } else {
      const m = Math.round(deg / (360 / 60) / 5) * 5;
      this.minute = m % 60;
    }
  }

  setAM(): void { this.isPM = false; }
  setPM(): void { this.isPM = true; }

  cancel(): void { this.dialogRef.close(null); }

  confirm(): void {
    let h24 = this.hour % 12; // 12 → 0
    if (this.isPM) h24 += 12;
    this.dialogRef.close({ hour: h24, minute: this.minute } as TimePickerData);
  }
}
