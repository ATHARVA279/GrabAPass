pub mod order_status {
    pub const PENDING: &str = "Pending";
    pub const COMPLETED: &str = "Completed";
    pub const FAILED: &str = "Failed";
    pub const MANUAL_REVIEW: &str = "ManualReview";
    pub const REFUNDED: &str = "Refunded";
    pub const CANCELLED: &str = "Cancelled";
}

pub mod ticket_status {
    pub const VALID: &str = "Valid";
    pub const USED: &str = "Used";
    pub const CANCELLED: &str = "Cancelled";
}

pub mod scan_result {
    pub const ADMITTED: &str = "Admitted";
    pub const REJECTED: &str = "Rejected";
}

pub mod scan_reason {
    pub const INVALID_QR_FORMAT: &str = "Invalid QR format";
    pub const INVALID_TICKET_ID: &str = "Invalid Ticket ID";
    pub const QR_SECRET_MISMATCH: &str = "QR Secret mismatch";
    pub const TICKET_NOT_FOUND: &str = "Ticket not found";
    pub const WRONG_EVENT: &str = "Wrong Event";
    pub const ALREADY_USED: &str = "Already Used";
    pub const CANCELLED: &str = "Cancelled";
    pub const VALID_ENTRY: &str = "Valid Entry";
}
