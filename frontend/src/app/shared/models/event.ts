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

