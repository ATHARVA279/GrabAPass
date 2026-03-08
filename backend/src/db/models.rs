use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ─── Enums ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[sqlx(type_name = "user_role", rename_all = "PascalCase")]
pub enum UserRole {
    Customer,
    Organizer,
    GateStaff,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[sqlx(type_name = "event_status", rename_all = "PascalCase")]
pub enum EventStatus {
    Draft,
    Published,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[sqlx(type_name = "seating_mode", rename_all = "PascalCase")]
pub enum SeatingMode {
    Reserved,
    GeneralAdmission,
    Mixed,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[sqlx(type_name = "seat_status", rename_all = "PascalCase")]
pub enum SeatStatus {
    Available,
    Held,
    Sold,
    Blocked,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[sqlx(type_name = "stage_orientation", rename_all = "PascalCase")]
pub enum StageOrientation {
    North,
    South,
    East,
    West,
}

// ─── Core app models ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub role: UserRole,
    pub name: String,
    pub phone_number: Option<String>,
    pub organizer_company: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct Event {
    pub id: Uuid,
    pub organizer_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub venue_name: String,
    pub venue_address: String,
    pub start_time: DateTime<Utc>,
    pub status: EventStatus,
    pub created_at: DateTime<Utc>,
    pub venue_template_id: Option<Uuid>,
    pub seating_mode: Option<SeatingMode>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub role: UserRole,
    pub name: String,
    pub exp: usize,
}

// ─── Event request DTOs ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateEventRequest {
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub venue_name: String,
    pub venue_address: String,
    pub start_time: DateTime<Utc>,
    /// Optional: attach a venue template to enable reserved seating
    pub venue_template_id: Option<Uuid>,
    pub seating_mode: Option<SeatingMode>,
}

// ─── Venue template raw DB rows ───────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct VenueTemplate {
    pub id: Uuid,
    pub organizer_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub stage_label: String,
    pub orientation: StageOrientation,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct VenueSection {
    pub id: Uuid,
    pub venue_template_id: Uuid,
    pub name: String,
    pub display_order: i32,
    pub color_hex: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct VenueRow {
    pub id: Uuid,
    pub section_id: Uuid,
    pub row_label: String,
    pub seat_count: i32,
    pub display_order: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct VenueSeat {
    pub id: Uuid,
    pub row_id: Uuid,
    pub seat_number: i32,
    pub seat_label: String,
    pub is_accessible: bool,
    pub is_aisle: bool,
    pub is_vip: bool,
    pub is_companion: bool,
    pub blocked_default: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct EventSeatCategory {
    pub id: Uuid,
    pub event_id: Uuid,
    pub section_id: Uuid,
    pub name: String,
    pub price: f64,
    pub color_hex: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct EventSeatInventory {
    pub id: Uuid,
    pub event_id: Uuid,
    pub seat_id: Uuid,
    pub status: SeatStatus,
}

// ─── Venue creation request DTOs ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateVenueRequest {
    pub name: String,
    pub description: Option<String>,
    pub stage_label: Option<String>,
    pub orientation: Option<StageOrientation>,
    pub sections: Vec<CreateSectionRequest>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSectionRequest {
    pub name: String,
    pub color_hex: Option<String>,
    pub rows: Vec<CreateRowRequest>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRowRequest {
    pub row_label: String,
    pub seat_count: i32,
    /// Optional per-seat metadata. If absent every seat gets default flags.
    pub seats: Option<Vec<CreateSeatRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSeatRequest {
    pub seat_number: i32,
    pub is_accessible: Option<bool>,
    pub is_aisle: Option<bool>,
    pub is_vip: Option<bool>,
    pub is_companion: Option<bool>,
    pub blocked_default: Option<bool>,
}

// ─── Event seat category assignment DTOs ─────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AssignSeatCategoryRequest {
    pub section_id: Uuid,
    pub name: String,
    pub price: f64,
    pub color_hex: Option<String>,
}

// ─── Frontend-ready seat layout response ─────────────────────────────────────
// This is what GET /api/events/:id/seat-layout returns.

#[derive(Debug, Serialize, Clone)]
pub struct SeatLayoutResponse {
    pub event_id: Uuid,
    pub event_title: String,
    pub stage_label: String,
    pub orientation: StageOrientation,
    pub seating_mode: Option<SeatingMode>,
    pub sections: Vec<SectionLayout>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SectionLayout {
    pub id: Uuid,
    pub name: String,
    pub display_order: i32,
    pub category: Option<CategoryInfo>,
    pub rows: Vec<RowLayout>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CategoryInfo {
    pub name: String,
    pub price: f64,
    pub color_hex: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct RowLayout {
    pub id: Uuid,
    pub row_label: String,
    pub display_order: i32,
    pub seats: Vec<SeatLayout>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SeatLayout {
    pub id: Uuid,
    pub seat_number: i32,
    pub seat_label: String,
    pub status: SeatStatus,
    pub is_accessible: bool,
    pub is_aisle: bool,
    pub is_vip: bool,
    pub is_companion: bool,
}

