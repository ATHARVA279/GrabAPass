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

#[derive(Debug, Serialize, Clone, FromRow)]
pub struct GateStaffSummary {
    pub id: Uuid,
    pub email: String,
    pub name: String,
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

#[derive(Debug, Serialize, Clone, FromRow)]
pub struct OrganizerEventDashboardSummary {
    pub event_id: Uuid,
    pub title: String,
    pub category: String,
    pub venue_name: String,
    pub start_time: DateTime<Utc>,
    pub status: EventStatus,
    pub gross_revenue: f64,
    pub orders_completed: i64,
    pub tickets_sold: i64,
    pub tickets_scanned: i64,
    pub rejected_scans: i64,
    pub seats_available: i64,
    pub seats_held: i64,
    pub seats_blocked: i64,
    pub seats_total: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct OrganizerDashboardSummaryResponse {
    pub total_events: i64,
    pub published_events: i64,
    pub gross_revenue: f64,
    pub tickets_sold: i64,
    pub tickets_scanned: i64,
    pub seats_available: i64,
    pub seats_held: i64,
    pub seats_blocked: i64,
    pub seats_total: i64,
    pub suspicious_alerts: i64,
    pub recent_alerts: Vec<SuspiciousActivityEvent>,
    pub events: Vec<OrganizerEventDashboardSummary>,
}

#[derive(Debug, Serialize, Clone, FromRow)]
pub struct SuspiciousActivityEvent {
    pub id: Uuid,
    pub event_id: Uuid,
    pub user_id: Option<Uuid>,
    pub ticket_id: Option<Uuid>,
    pub activity_type: String,
    pub severity: String,
    pub message: String,
    pub metadata: sqlx::types::Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AssignGateStaffRequest {
    pub gate_staff_ids: Vec<Uuid>,
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

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct SeatHold {
    pub id: Uuid,
    pub event_id: Uuid,
    pub seat_id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
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

// ─── Seat Hold DTOs ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct HoldSeatsRequest {
    pub seat_ids: Vec<Uuid>,
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

// ─── Order and Checkout DTOs ─────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub event_id: Uuid,
    pub subtotal_amount: f64,
    pub fee_amount: f64,
    pub total_amount: f64,
    pub currency: String,
    pub status: String,
    pub gateway: Option<String>,
    pub gateway_order_id: Option<String>,
    pub gateway_payment_id: Option<String>,
    pub payment_signature: Option<String>,
    pub payment_verified_at: Option<DateTime<Utc>>,
    pub receipt: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct OrderItem {
    pub id: Uuid,
    pub order_id: Uuid,
    pub seat_id: Uuid,
    pub price: f64,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutRequest {
    pub seat_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct InitializeCheckoutRequest {
    pub seat_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct InitializeCheckoutResponse {
    pub order: Order,
    pub gateway: String,
    pub gateway_key_id: String,
    pub gateway_order_id: String,
    pub amount: i64,
    pub currency: String,
    pub description: String,
    pub customer_name: String,
    pub customer_email: String,
    pub hold_expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyCheckoutRequest {
    pub order_id: Uuid,
    pub razorpay_order_id: String,
    pub razorpay_payment_id: String,
    pub razorpay_signature: String,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutFailureRequest {
    pub order_id: Uuid,
    pub razorpay_order_id: Option<String>,
    pub razorpay_payment_id: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RazorpayWebhookPayload {
    pub event: String,
    pub payload: RazorpayWebhookData,
}

#[derive(Debug, Deserialize)]
pub struct RazorpayWebhookData {
    pub payment: Option<RazorpayWebhookPaymentContainer>,
}

#[derive(Debug, Deserialize)]
pub struct RazorpayWebhookPaymentContainer {
    pub entity: RazorpayWebhookPaymentEntity,
}

#[derive(Debug, Deserialize)]
pub struct RazorpayWebhookPaymentEntity {
    pub id: String,
    pub order_id: Option<String>,
    pub status: String,
    pub amount: i64,
    pub currency: String,
}

// ─── Ticket models ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct Ticket {
    pub id: Uuid,
    pub order_id: Uuid,
    pub event_id: Uuid,
    pub user_id: Uuid,
    pub qr_secret: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SeatInfo {
    pub seat_id: Uuid,
    pub seat_label: String,
    pub section_name: String,
}

/// Rich ticket detail joining ticket + event + seat info for the frontend.
#[derive(Debug, Serialize, Clone, FromRow)]
pub struct TicketDetail {
    pub id: Uuid,
    pub order_id: Uuid,
    pub event_id: Uuid,
    pub event_title: String,
    pub event_start_time: DateTime<Utc>,
    pub venue_name: String,
    pub seats: sqlx::types::Json<Vec<SeatInfo>>,
    pub qr_payload: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}
// ─── Gate Validation DTOs ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ValidateTicketRequest {
    pub qr_payload: String,
    pub event_id: Uuid,
}

#[derive(Debug, Serialize, Clone, FromRow)]
pub struct ScanLog {
    pub id: Uuid,
    pub ticket_id: Option<Uuid>,
    pub event_id: Uuid,
    pub scanned_by: Uuid,
    pub result: String,
    pub reason: Option<String>,
    pub scanned_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ScanResultResponse {
    pub success: bool,
    pub message: String,
    pub ticket_detail: Option<TicketDetail>,
}
