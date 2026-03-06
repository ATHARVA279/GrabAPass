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
}

export interface CreateEventRequest {
  title: string;
  description?: string;
  category: string;
  venue_name: string;
  venue_address: string;
  start_time: string;
}
