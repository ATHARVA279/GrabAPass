import { Component, Input, ChangeDetectionStrategy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DraftSection } from '../../../features/venues/create-venue/create-venue';

/**
 * Lightweight preview rendered inside the create-venue stepper (step 4).
 * Uses the draft section data, not the API response.
 */
@Component({
  selector: 'app-seat-map-preview',
  standalone: true,
  imports: [CommonModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="preview-wrap">
      <!-- Stage bar -->
      <div class="stage-bar" [class]="'orient-' + orientation.toLowerCase()">
        {{ stageLabel }}
      </div>

      <!-- Sections -->
      <div class="sections-list">
        <div *ngFor="let section of sections" class="section-block">
          <div class="section-label" [style.color]="categoryColors[section.name] || section.color_hex">
            {{ section.name }}
          </div>
          <div class="rows-block">
            <div *ngFor="let row of section.rows" class="row-line">
              <span class="row-lbl">{{ row.row_label }}</span>
              <div class="seats-line">
                <div
                  *ngFor="let seat of seatRange(row.seat_count)"
                  class="seat-box"
                  [style.background]="categoryColors[section.name] || section.color_hex">
                </div>
              </div>
              <span class="row-lbl right">{{ row.row_label }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  `,
  styles: [`
    .preview-wrap {
      background: var(--black-soft);
      border: 1px solid var(--black-border);
      border-radius: 12px;
      padding: 1.25rem;
      overflow-x: auto;
    }
    .stage-bar {
      text-align: center;
      font-size: 0.72rem;
      font-weight: 700;
      letter-spacing: 0.12em;
      font-family: var(--font, 'Poppins', sans-serif);
      color: #000;
      background: var(--yellow);
      border-radius: 6px;
      padding: 6px 24px;
      margin-bottom: 1.5rem;
      width: fit-content;
      margin-inline: auto;
    }
    .sections-list {
      display: flex;
      flex-direction: column;
      gap: 1.25rem;
    }
    .section-block { display: flex; flex-direction: column; gap: 0.4rem; }
    .section-label {
      font-size: 0.75rem;
      font-weight: 700;
      font-family: var(--font, 'Poppins', sans-serif);
      letter-spacing: 0.06em;
      text-transform: uppercase;
      padding-left: 36px;
    }
    .rows-block { display: flex; flex-direction: column; gap: 3px; }
    .row-line {
      display: flex;
      align-items: center;
      gap: 6px;
    }
    .row-lbl {
      font-size: 0.68rem;
      font-weight: 600;
      font-family: var(--font, 'Poppins', sans-serif);
      color: var(--text-muted);
      width: 28px;
      text-align: center;
      flex-shrink: 0;
      &.right { text-align: left; }
    }
    .seats-line {
      display: flex;
      flex-wrap: wrap;
      gap: 2px;
    }
    .seat-box {
      width: 14px;
      height: 14px;
      border-radius: 3px;
      opacity: 0.75;
    }
  `]
})
export class SeatMapPreview {
  @Input() sections: DraftSection[] = [];
  @Input() categoryColors: Record<string, string> = {};
  @Input() stageLabel = 'STAGE';
  @Input() orientation = 'North';

  seatRange(count: number): number[] {
    return Array.from({ length: Math.min(count, 40) });
  }
}
