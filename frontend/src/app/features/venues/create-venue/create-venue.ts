import { Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule, ReactiveFormsModule, FormBuilder, FormGroup, Validators } from '@angular/forms';
import { Router, RouterModule } from '@angular/router';
import { finalize } from 'rxjs';
import { ToastrService } from 'ngx-toastr';

import { MatStepperModule, MatStepper } from '@angular/material/stepper';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatSelectModule } from '@angular/material/select';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';

import { VenueService } from '../../../core/services/venue.service';
import { CreateVenueRequest } from '../../../shared/models/venue';
import { SeatMapPreview } from '../../../shared/components/seat-map-preview/seat-map-preview';

// ─── Local draft types (UI state only) ───────────────────────────────────────

export interface DraftRow {
  row_label: string;
  seat_count: number;
}

export interface DraftSection {
  name: string;
  color_hex: string;
  rows: DraftRow[];
}

export interface DraftCategory {
  sectionIndex: number;
  sectionName: string;
  name: string;
  price: number;
  color_hex: string;
}

const SECTION_COLORS = [
  '#4A90D9', '#E94F37', '#44BBA4', '#F5A623',
  '#9B59B6', '#2ECC71', '#E74C3C', '#1ABC9C',
];

@Component({
  selector: 'app-create-venue',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    ReactiveFormsModule,
    RouterModule,
    MatStepperModule,
    MatFormFieldModule,
    MatInputModule,
    MatSelectModule,
    MatButtonModule,
    MatIconModule,
    MatProgressSpinnerModule,
    SeatMapPreview,
  ],
  templateUrl: './create-venue.html',
  styleUrls: ['./create-venue.scss'],
})
export class CreateVenue implements OnInit {
  readonly infoForm: FormGroup;

  sections: DraftSection[] = [];
  sectionCategories: DraftCategory[] = [];
  sectionError = '';
  categoryError = '';
  isSubmitting = false;

  private readonly fb = inject(FormBuilder);
  private readonly venueService = inject(VenueService);
  private readonly router = inject(Router);
  private readonly toastr = inject(ToastrService);

  constructor() {
    this.infoForm = this.fb.group({
      name:        ['', [Validators.required, Validators.maxLength(255)]],
      description: [''],
      stage_label: ['STAGE'],
      orientation: ['North'],
    });
  }

  ngOnInit(): void {
    // Start with one section, two rows
    this.sections = [this.blankSection(0)];
  }

  // ── Section helpers ────────────────────────────────────────────────────────

  private blankSection(index: number): DraftSection {
    return {
      name:      `Section ${index + 1}`,
      color_hex: SECTION_COLORS[index % SECTION_COLORS.length],
      rows:      [{ row_label: 'A', seat_count: 10 }, { row_label: 'B', seat_count: 10 }],
    };
  }

  addSection(): void {
    this.sections.push(this.blankSection(this.sections.length));
  }

  removeSection(index: number): void {
    this.sections.splice(index, 1);
  }

  // ── Row helpers ────────────────────────────────────────────────────────────

  addRow(section: DraftSection): void {
    const nextLabel = String.fromCharCode(
      65 + section.rows.length // A=65, so row n → char(65+n)
    );
    section.rows.push({ row_label: nextLabel, seat_count: 10 });
  }

  removeRow(section: DraftSection, rowIndex: number): void {
    section.rows.splice(rowIndex, 1);
  }

  incrementSeats(row: DraftRow): void {
    if (row.seat_count < 200) row.seat_count++;
  }

  decrementSeats(row: DraftRow): void {
    if (row.seat_count > 1) row.seat_count--;
  }

  // ── Step 2 validation → Step 3 ────────────────────────────────────────────

  validateSectionsAndNext(stepper: MatStepper): void {
    this.sectionError = '';

    for (const section of this.sections) {
      if (!section.name.trim()) {
        this.sectionError = 'All sections must have a name.';
        return;
      }
      for (const row of section.rows) {
        if (!row.row_label.trim()) {
          this.sectionError = `All rows in "${section.name}" must have a label.`;
          return;
        }
      }
    }

    // Sync category list when entering step 3
    this.sectionCategories = this.sections.map((s, i) => ({
      sectionIndex: i,
      sectionName:  s.name,
      name:         s.name,
      price:        0,
      color_hex:    s.color_hex,
    }));

    stepper.next();
  }

  // ── Step 3 validation → Step 4 ────────────────────────────────────────────

  validateCategoriesAndNext(stepper: MatStepper): void {
    this.categoryError = '';
    for (const cat of this.sectionCategories) {
      if (!cat.name.trim()) {
        this.categoryError = 'All sections need a category name.';
        return;
      }
      if (cat.price < 0) {
        this.categoryError = 'Prices cannot be negative.';
        return;
      }
    }
    // Sync category colors back to sections for preview
    this.sectionCategories.forEach((cat) => {
      this.sections[cat.sectionIndex].color_hex = cat.color_hex;
    });
    stepper.next();
  }

  // ── Preview helpers ────────────────────────────────────────────────────────

  get categoryColorMap(): Record<string, string> {
    return Object.fromEntries(
      this.sectionCategories.map((c) => [c.sectionName, c.color_hex])
    );
  }

  // ── Submit ─────────────────────────────────────────────────────────────────

  onSubmit(): void {
    if (this.isSubmitting) return;
    this.isSubmitting = true;

    const v = this.infoForm.getRawValue();

    const payload: CreateVenueRequest = {
      name:        v.name.trim(),
      description: v.description?.trim() || undefined,
      stage_label: v.stage_label?.trim() || 'STAGE',
      orientation: v.orientation,
      sections:    this.sections.map((s) => ({
        name:      s.name.trim(),
        color_hex: s.color_hex,
        rows:      s.rows.map((r) => ({
          row_label:  r.row_label.trim(),
          seat_count: r.seat_count,
        })),
      })),
    };

    this.venueService
      .createVenueTemplate(payload)
      .pipe(finalize(() => (this.isSubmitting = false)))
      .subscribe({
        next: (template) => {
          this.toastr.success(`"${template.name}" created!`, 'Venue Template Saved');
          this.router.navigate(['/organizer']);
        },
        error: (err) => {
          const msg =
            typeof err.error === 'string'
              ? err.error
              : (err.error?.message ?? 'Failed to create venue template.');
          this.toastr.error(msg, 'Error');
        },
      });
  }
}
