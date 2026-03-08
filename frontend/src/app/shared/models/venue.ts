// ─── Enums ────────────────────────────────────────────────────────────────────

export type SeatingMode = 'Reserved' | 'GeneralAdmission' | 'Mixed';
export type SeatStatus  = 'Available' | 'Held' | 'Sold' | 'Blocked';
export type StageOrientation = 'North' | 'South' | 'East' | 'West';

// ─── Venue template (flat, organizer list/create) ─────────────────────────────

export interface VenueTemplate {
  id: string;
  organizer_id: string;
  name: string;
  description?: string | null;
  stage_label: string;
  orientation: StageOrientation;
  created_at: string;
}

// ─── Venue creation request ───────────────────────────────────────────────────

export interface CreateVenueRequest {
  name: string;
  description?: string;
  stage_label?: string;
  orientation?: StageOrientation;
  sections: CreateSectionRequest[];
}

export interface CreateSectionRequest {
  name: string;
  color_hex?: string;
  rows: CreateRowRequest[];
}

export interface CreateRowRequest {
  row_label: string;
  seat_count: number;
  seats?: CreateSeatRequest[];
}

export interface CreateSeatRequest {
  seat_number: number;
  is_accessible?: boolean;
  is_aisle?: boolean;
  is_vip?: boolean;
  is_companion?: boolean;
  blocked_default?: boolean;
}

// ─── Seat category assignment (organizer, per event) ─────────────────────────

export interface AssignSeatCategoryRequest {
  section_id: string;
  name: string;
  price: number;
  color_hex?: string;
}

// ─── Frontend-ready seat layout (from GET /api/events/:id/seat-layout) ────────

export interface SeatLayoutResponse {
  event_id: string;
  event_title: string;
  stage_label: string;
  orientation: StageOrientation;
  seating_mode?: SeatingMode | null;
  sections: SectionLayout[];
}

export interface SectionLayout {
  id: string;
  name: string;
  display_order: number;
  category?: CategoryInfo | null;
  rows: RowLayout[];
}

export interface CategoryInfo {
  name: string;
  price: number;
  color_hex: string;
}

export interface RowLayout {
  id: string;
  row_label: string;
  display_order: number;
  seats: SeatLayout[];
}

export interface SeatLayout {
  id: string;
  seat_number: number;
  seat_label: string;
  status: SeatStatus;
  is_accessible: boolean;
  is_aisle: boolean;
  is_vip: boolean;
  is_companion: boolean;
}
