export interface EventVenue {
  id: string;
  name: string;
  placeId: string;
  latitude: number;
  longitude: number;
  address: string;
  locality: string;
  city: string;
  state: string;
  pincode: string;
  country: string;
  landmark?: string | null;
  capacity?: number | null;
}

export interface EventVenueInput {
  id?: string | null;
  name: string;
  placeId: string;
  latitude: number;
  longitude: number;
  address: string;
  locality: string;
  city: string;
  state: string;
  pincode: string;
  country: string;
  landmark?: string | null;
  capacity?: number | null;
}

export interface VenueSearchResult extends EventVenueInput {
  rating?: number | null;
  source: 'google' | 'existing';
  matchReason?: string | null;
}

export interface EventVenueMatchResponse {
  exactMatch: EventVenue | null;
  similarVenues: EventVenue[];
}
