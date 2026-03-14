import { SeatingMode } from './venue';

export type EventStatus = 'Draft' | 'Published' | 'Cancelled';

export interface Event {
  id: string;
  organizer_id: string;
  title: string;
  description?: string | null;
  category: string;
  venue_name: string;
  venue_address: string;
  start_time: string;
  status: EventStatus;
  created_at: string;
  venue_template_id?: string | null;
  seating_mode?: SeatingMode | null;
}

export interface CreateEventRequest {
  title: string;
  description?: string;
  category: string;
  venue_name: string;
  venue_address: string;
  start_time: string;
  venue_template_id?: string;
  seating_mode?: SeatingMode;
}

export interface OrganizerEventDashboardSummary {
  event_id: string;
  title: string;
  category: string;
  venue_name: string;
  start_time: string;
  status: EventStatus;
  gross_revenue: number;
  orders_completed: number;
  tickets_sold: number;
  tickets_scanned: number;
  rejected_scans: number;
  seats_available: number;
  seats_held: number;
  seats_blocked: number;
  seats_total: number;
}

export interface OrganizerDashboardSummaryResponse {
  total_events: number;
  published_events: number;
  gross_revenue: number;
  tickets_sold: number;
  tickets_scanned: number;
  seats_available: number;
  seats_held: number;
  seats_blocked: number;
  seats_total: number;
  suspicious_alerts: number;
  recent_alerts: SuspiciousActivityEvent[];
  events: OrganizerEventDashboardSummary[];
}

export interface GateStaffSummary {
  id: string;
  email: string;
  name: string;
}

export interface SuspiciousActivityEvent {
  id: string;
  event_id: string;
  user_id?: string | null;
  ticket_id?: string | null;
  activity_type: string;
  severity: string;
  message: string;
  metadata: unknown;
  created_at: string;
}
