use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::types::Json;
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
    pub venue_id: Option<Uuid>,
    pub venue_template_id: Option<Uuid>,
    pub seating_mode: Option<SeatingMode>,
    pub image_url: Option<String>,
    pub image_gallery: Json<Vec<String>>,
    pub venue_place_id: Option<String>,
    pub venue_latitude: Option<f64>,
    pub venue_longitude: Option<f64>,
    pub venue_locality: Option<String>,
    pub venue_city: Option<String>,
    pub venue_state: Option<String>,
    pub venue_pincode: Option<String>,
    pub venue_country: Option<String>,
    pub venue_landmark: Option<String>,
    pub venue_capacity: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct PublicEvent {
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
    pub venue_id: Option<Uuid>,
    pub venue_template_id: Option<Uuid>,
    pub seating_mode: Option<SeatingMode>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub image_url: Option<String>,
    pub image_gallery: Json<Vec<String>>,
    pub venue_place_id: Option<String>,
    pub venue_latitude: Option<f64>,
    pub venue_longitude: Option<f64>,
    pub venue_locality: Option<String>,
    pub venue_city: Option<String>,
    pub venue_state: Option<String>,
    pub venue_pincode: Option<String>,
    pub venue_country: Option<String>,
    pub venue_landmark: Option<String>,
    pub venue_capacity: Option<i32>,
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

// ─── Crowd Pulse DTOs ────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct EventPulseResponse {
    pub active_viewers: i64,
    pub recently_sold: i64,
    pub total_capacity: i64,
    pub sold_percentage: f64,
    pub sections: Vec<SectionPulse>,
}

#[derive(Debug, Serialize, Clone, FromRow)]
pub struct SectionPulse {
    pub section_name: String,
    pub status: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventDetailsResponse {
    pub event: Event,
    pub venue: Option<EventVenue>,
    pub images: EventImagesResponse,
    pub pricing: EventPricingResponse,
    pub availability: EventAvailabilityResponse,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventImagesResponse {
    pub hero: Option<String>,
    pub gallery: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventPricingResponse {
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub currency: String,
    pub tiers: Vec<EventTicketTier>,
    pub has_reserved_seating: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventAvailabilityResponse {
    pub total: i64,
    pub sold: i64,
    pub held: i64,
    pub available: i64,
    pub sold_percentage: f64,
    pub status: String,
}

// ─── Event request DTOs ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateEventRequest {
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub venue: Option<EventVenueInput>,
    pub venue_name: String,
    pub venue_address: String,
    pub start_time: DateTime<Utc>,
    /// Optional: attach a venue template to enable reserved seating
    pub venue_template_id: Option<Uuid>,
    pub seating_mode: Option<SeatingMode>,
    pub image_url: Option<String>,
    pub image_gallery: Option<Vec<String>>,
    pub venue_place_id: Option<String>,
    pub venue_latitude: Option<f64>,
    pub venue_longitude: Option<f64>,
    pub ticket_tiers: Option<Vec<CreateEventTicketTierRequest>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct EventVenue {
    pub id: Uuid,
    pub created_by: Uuid,
    pub name: String,
    pub place_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub address: String,
    pub locality: String,
    pub city: String,
    pub state: String,
    pub pincode: String,
    pub country: String,
    pub landmark: Option<String>,
    pub capacity: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EventVenueInput {
    pub id: Option<Uuid>,
    pub name: String,
    pub place_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub address: String,
    pub locality: String,
    pub city: String,
    pub state: String,
    pub pincode: String,
    pub country: String,
    pub landmark: Option<String>,
    pub capacity: Option<i32>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EventVenueMatchResponse {
    pub exact_match: Option<EventVenue>,
    pub similar_venues: Vec<EventVenue>,
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
pub struct EventTicketTier {
    pub id: Uuid,
    pub event_id: Uuid,
    pub name: String,
    pub price: f64,
    pub capacity: i32,
    pub color_hex: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct SeatHold {
    pub id: Uuid,
    pub event_id: Uuid,
    pub seat_id: Option<Uuid>,
    pub ticket_tier_id: Option<Uuid>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateEventTicketTierRequest {
    pub name: String,
    pub price: f64,
    pub capacity: i32,
    pub color_hex: Option<String>,
}

// ─── Seat Hold DTOs ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TicketTierSelection {
    pub ticket_tier_id: Uuid,
    pub quantity: i32,
}

#[derive(Debug, Deserialize)]
pub struct HoldSeatsRequest {
    #[serde(default)]
    pub seat_ids: Vec<Uuid>,
    #[serde(default)]
    pub ticket_tiers: Vec<TicketTierSelection>,
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
    pub seat_id: Option<Uuid>,
    pub ticket_tier_id: Option<Uuid>,
    pub price: f64,
}

#[derive(Debug, Deserialize)]
pub struct InitializeCheckoutRequest {
    pub hold_ids: Vec<Uuid>,
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

// ─── Split Checkout DTOs ─────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[sqlx(type_name = "split_status", rename_all = "PascalCase")]
pub enum SplitStatus {
    Pending,
    Completed,
    Expired,
    Refunded,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[sqlx(type_name = "split_type", rename_all = "PascalCase")]
pub enum SplitType {
    Even,
    Custom,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct SplitSession {
    pub id: Uuid,
    pub order_id: Uuid,
    pub total_amount: f64,
    pub split_type: SplitType,
    pub status: SplitStatus,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    #[sqlx(skip)]
    pub shares: Option<Vec<SplitShare>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct SplitShare {
    pub id: Uuid,
    pub split_session_id: Uuid,
    pub amount_due: f64,
    pub status: SplitStatus,
    pub is_host_share: bool,
    pub guest_name: Option<String>,
    pub guest_email: Option<String>,
    pub payment_token: Uuid,
    pub gateway_order_id: Option<String>,
    pub gateway_payment_id: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub claimed_by_user_id: Option<Uuid>,
    pub claimed_ticket_id: Option<Uuid>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub pending_manual_refund: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct SplitSharePublicDetail {
    pub id: Uuid,
    pub order_id: Uuid,
    pub amount_due: f64,
    pub status: SplitStatus,
    pub host_user_id: Uuid,
    pub is_host_share: bool,
    pub guest_name: Option<String>,
    pub guest_email: Option<String>,
    pub payment_token: Uuid,
    pub gateway_order_id: Option<String>,
    pub event_title: String,
    pub event_start_time: DateTime<Utc>,
    pub venue_name: String,
    pub host_name: String,
    pub claimed_ticket_id: Option<Uuid>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub session_expires_at: DateTime<Utc>,
    pub session_status: SplitStatus,
}

#[derive(Debug, Deserialize)]
pub struct VerifySplitShareRequest {
    pub razorpay_payment_id: String,
    pub razorpay_order_id: String,
}

#[derive(Debug, Serialize)]
pub struct SplitCheckoutInitialization {
    pub share_id: Uuid,
    pub split_session_id: Uuid,
    pub gateway: String,
    pub gateway_key_id: String,
    pub gateway_order_id: String,
    pub amount: i64,
    pub currency: String,
    pub customer_name: String,
    pub customer_email: String,
}

#[derive(Debug, Deserialize)]
pub struct CustomShareTicketTierRequest {
    pub ticket_tier_id: Uuid,
    pub quantity: i32,
}

#[derive(Debug, Deserialize)]
pub struct CustomShareRequest {
    pub guest_name: Option<String>,
    pub guest_email: Option<String>,
    pub seat_ids: Vec<Uuid>,
    pub ticket_tiers: Option<Vec<CustomShareTicketTierRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct InitializeSplitRequest {
    pub split_type: SplitType,
    pub num_shares: Option<i32>, // used for Even splits
    pub custom_shares: Option<Vec<CustomShareRequest>>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TicketTierInfo {
    pub ticket_tier_id: Uuid,
    pub name: String,
    pub quantity: i32,
    pub price: f64,
    pub color_hex: String,
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
    pub venue_address: String,
    pub seats: sqlx::types::Json<Vec<SeatInfo>>,
    pub tiers: sqlx::types::Json<Vec<TicketTierInfo>>,
    pub qr_payload: String,
    pub status: String,
    pub can_cancel: bool,
    pub refund_amount: Option<f64>,
    pub refund_status: Option<String>,
    pub refund_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct RefundRecord {
    pub id: Uuid,
    pub booking_id: Uuid,
    pub payment_id: Option<String>,
    pub amount: f64,
    pub refund_status: String,
    pub refund_reason: Option<String>,
    pub created_at: DateTime<Utc>,
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
