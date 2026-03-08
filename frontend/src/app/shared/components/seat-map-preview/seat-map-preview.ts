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
          <div class="section-header">
            <div class="section-label" [style.color]="categoryColors[section.name] || section.color_hex">
              {{ section.name }}
            </div>
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
      display: flex;
      flex-direction: column;
      gap: 2rem;
      background: var(--black-soft);
      border: 1px solid var(--black-border);
      border-radius: 16px;
      padding: 2rem;
      overflow-x: auto;
      align-items: center;
    }
    .stage-bar {
      text-align: center;
      font-size: 0.85rem;
      font-weight: 700;
      letter-spacing: 0.2em;
      text-transform: uppercase;
      font-family: var(--font, 'Poppins', sans-serif);
      
      background: linear-gradient(180deg, rgba(255, 214, 10, 0.2) 0%, rgba(255, 214, 10, 0.05) 100%);
      color: var(--yellow);
      border: 1px solid rgba(255, 214, 10, 0.4);
      box-shadow: 0 4px 24px rgba(255, 214, 10, 0.15);
      
      border-radius: 40px;
      padding: 12px 0;
      width: 60%;
      min-width: 300px;
      margin-inline: auto;
      margin-bottom: 2rem;
    }
    .sections-list {
      display: flex;
      flex-direction: column;
      gap: 3rem;
      align-items: center;
      width: 100%;
    }
    .section-block { 
      display: flex; 
      flex-direction: column; 
      gap: 1.25rem; 
      align-items: center;
    }
    .section-header {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      width: 100%;
      justify-content: center;
      padding-bottom: 0.5rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    }
    .section-label {
      font-size: 0.95rem;
      font-weight: 700;
      letter-spacing: 0.05em;
      text-transform: uppercase;
      font-family: var(--font);
    }
    .rows-block { 
      display: flex; 
      flex-direction: column; 
      gap: 8px; 
      align-items: center;
    }
    .row-line {
      display: flex;
      align-items: center;
      gap: 12px;
    }
    .row-lbl {
      font-size: 0.65rem;
      font-weight: 500;
      text-transform: uppercase;
      font-family: var(--font, 'Poppins', sans-serif);
      color: rgba(255,255,255,0.4);
      width: 24px;
      text-align: right;
      flex-shrink: 0;
      user-select: none;
      &.right { text-align: left; }
    }
    .seats-line {
      display: flex;
      flex-wrap: wrap;
      gap: 3px;
      justify-content: center;
    }
    .seat-box {
      width: 12px;
      height: 12px;
      border-radius: 50%;
      border: 1px solid rgba(255,255,255,0.15);
      box-shadow: 0 1px 2px rgba(0,0,0,0.2);
    }
    .seat-box:nth-child(5n):not(:last-child) {
      margin-right: 8px;
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
