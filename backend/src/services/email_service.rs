use axum::http::StatusCode;
use chrono::Local;
use lettre::message::{Mailbox, header::ContentType};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, transport::smtp::authentication::Credentials};

use crate::{
    EmailConfig,
    db::models::{Order, TicketDetail, User},
};

pub struct EmailService;

pub struct BookingEmailData<'a> {
    pub user: &'a User,
    pub order: &'a Order,
    pub ticket: &'a TicketDetail,
}

pub struct CancellationEmailData<'a> {
    pub user: &'a User,
    pub order: &'a Order,
    pub ticket: &'a TicketDetail,
}

pub struct RefundEmailData<'a> {
    pub user: &'a User,
    pub order: &'a Order,
    pub ticket: &'a TicketDetail,
    pub refund_amount: f64,
    pub refund_status: &'a str,
}

impl EmailService {
    pub async fn send_booking_confirmation(
        config: Option<&EmailConfig>,
        data: BookingEmailData<'_>,
    ) -> Result<(), (StatusCode, String)> {
        let Some(config) = config else {
            return Ok(());
        };

        let subject = format!("Your Ticket is Confirmed - {}", data.ticket.event_title);
        let html = render_booking_html(&data);
        Self::send_email(config, &data.user.email, &subject, &html).await
    }

    pub async fn send_ticket_cancellation(
        config: Option<&EmailConfig>,
        data: CancellationEmailData<'_>,
    ) -> Result<(), (StatusCode, String)> {
        let Some(config) = config else {
            return Ok(());
        };

        let subject = "Your Ticket has been Cancelled";
        let html = render_cancellation_html(&data);
        Self::send_email(config, &data.user.email, subject, &html).await
    }

    pub async fn send_refund_status(
        config: Option<&EmailConfig>,
        data: RefundEmailData<'_>,
    ) -> Result<(), (StatusCode, String)> {
        let Some(config) = config else {
            return Ok(());
        };

        let subject = if data.refund_status == "Completed" {
            "Your Refund has been Processed"
        } else {
            "Your Refund is Being Processed"
        };
        let html = render_refund_html(&data);
        Self::send_email(config, &data.user.email, subject, &html).await
    }

    async fn send_email(
        config: &EmailConfig,
        to_email: &str,
        subject: &str,
        html: &str,
    ) -> Result<(), (StatusCode, String)> {
        let from = config
            .from_email
            .parse::<Mailbox>()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Invalid EMAIL_FROM value: {e}")))?;
        let to = to_email
            .parse::<Mailbox>()
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid recipient email address: {e}")))?;

        let email = Message::builder()
            .from(from)
            .to(to)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html.to_string())
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to build email message: {e}"),
                )
            })?;

        let credentials =
            Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to configure SMTP relay: {e}"),
                )
            })?
            .port(config.smtp_port)
            .credentials(credentials)
            .build();

        mailer.send(email).await.map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("SMTP provider error: {e}"),
            )
        })?;

        Ok(())
    }
}

fn render_booking_html(data: &BookingEmailData<'_>) -> String {
    let ticket_lines = render_ticket_lines(data.ticket);
    let total_tickets = total_ticket_count(data.ticket);
    let event_time = data
        .ticket
        .event_start_time
        .with_timezone(&Local)
        .format("%b %d, %Y at %I:%M %p");

    render_email_shell(
        "Booking Confirmed",
        "Your Ticket is Confirmed",
        &format!(
            "Hi {}, your booking is locked in. Keep this email handy at the venue entrance.",
            escape_html(&data.user.name)
        ),
        "#10b981",
        &format!(
            r#"
            {status_card}
            {section_open}
              <div style="font-size:22px;line-height:30px;font-weight:700;color:#111827;margin-bottom:6px;">{event_name}</div>
              <div style="font-size:14px;line-height:22px;color:#6b7280;">Presented by GrabAPass</div>
              <div style="height:20px;"></div>
              {detail_row}
              {detail_row}
              {detail_row}
              {detail_row}
            {section_close}

            {section_open}
              <div style="font-size:15px;line-height:24px;font-weight:700;color:#111827;margin-bottom:14px;">Ticket Summary</div>
              {summary_grid}
                {summary_item}
                {summary_item}
                {summary_item}
                {summary_item}
              {summary_grid_close}
            {section_close}

            <div style="background:linear-gradient(135deg,#111827 0%,#1f2937 100%);border-radius:20px;padding:22px 24px;margin-bottom:20px;">
              <div style="font-size:12px;line-height:18px;font-weight:700;letter-spacing:0.12em;text-transform:uppercase;color:#fbbf24;margin-bottom:10px;">Ticket Reference</div>
              <div style="font-size:14px;line-height:22px;color:#d1d5db;margin-bottom:12px;">Show this QR reference at entry. Your gate team can scan or verify it manually if needed.</div>
              <div style="font-size:15px;line-height:24px;font-weight:700;color:#ffffff;word-break:break-word;">{ticket_reference}</div>
            </div>

            <div style="background:#fff7ed;border:1px solid #fdba74;border-radius:18px;padding:20px;">
              <div style="font-size:15px;line-height:24px;font-weight:700;color:#9a3412;margin-bottom:12px;">Before You Go</div>
              <div style="font-size:14px;line-height:24px;color:#7c2d12;">Arrive early to avoid entry delays.</div>
              <div style="font-size:14px;line-height:24px;color:#7c2d12;">Carry a valid government ID proof.</div>
              <div style="font-size:14px;line-height:24px;color:#7c2d12;">Keep this email or your ticket screen ready at the gate.</div>
            </div>
            "#,
            status_card = render_status_card("Confirmed", "Your seats and payment have been verified successfully.", "#10b981", "#ecfdf5"),
            section_open = section_open(),
            section_close = section_close(),
            detail_row = render_detail_row("Venue", &escape_html(&data.ticket.venue_name), Some(&escape_html(&data.ticket.venue_address))),
            summary_grid = summary_grid_open(),
            summary_grid_close = summary_grid_close(),
            summary_item = render_summary_item("Booking ID", &data.order.id.to_string()),
            event_name = escape_html(&data.ticket.event_title),
            ticket_reference = escape_html(&data.ticket.qr_payload),
        )
        .replacen(
            &render_detail_row("Venue", &escape_html(&data.ticket.venue_name), Some(&escape_html(&data.ticket.venue_address))),
            &[
                render_detail_row("Venue", &escape_html(&data.ticket.venue_name), Some(&escape_html(&data.ticket.venue_address))),
                render_detail_row("Date & Time", &event_time.to_string(), None),
                render_detail_row("Booking ID", &data.order.id.to_string(), None),
                render_detail_row("Ticket ID", &data.ticket.id.to_string(), None),
            ]
            .join(""),
            1,
        )
        .replacen(
            &render_summary_item("Booking ID", &data.order.id.to_string()),
            &[
                render_summary_item("Tickets", &total_tickets.to_string()),
                render_summary_item("Details", &escape_html(&ticket_lines)),
                render_summary_item("Total Paid", &format!("INR {:.2}", data.order.total_amount)),
                render_summary_item("Payment", "Razorpay"),
            ]
            .join(""),
            1,
        ),
    )
}

fn render_cancellation_html(data: &CancellationEmailData<'_>) -> String {
    let ticket_lines = render_ticket_lines(data.ticket);
    render_email_shell(
        "Ticket Cancelled",
        "Your Ticket has been Cancelled",
        &format!(
            "Hi {}, your cancellation request has been completed successfully.",
            escape_html(&data.user.name)
        ),
        "#ef4444",
        &format!(
            r#"
            {status_card}
            {section_open}
              <div style="font-size:22px;line-height:30px;font-weight:700;color:#111827;margin-bottom:16px;">{event_name}</div>
              {detail_one}
              {detail_two}
              {detail_three}
            {section_close}
            <div style="background:#f9fafb;border:1px solid #e5e7eb;border-radius:18px;padding:20px;">
              <div style="font-size:15px;line-height:24px;font-weight:700;color:#111827;margin-bottom:10px;">Cancelled Ticket Details</div>
              <div style="font-size:14px;line-height:24px;color:#4b5563;">{ticket_lines}</div>
            </div>
            "#,
            status_card = render_status_card("Cancelled", "This ticket will no longer be valid for event entry.", "#ef4444", "#fef2f2"),
            section_open = section_open(),
            section_close = section_close(),
            event_name = escape_html(&data.ticket.event_title),
            detail_one = render_detail_row("Booking ID", &data.order.id.to_string(), None),
            detail_two = render_detail_row("Ticket ID", &data.ticket.id.to_string(), None),
            detail_three = render_detail_row("Venue", &escape_html(&data.ticket.venue_name), Some(&escape_html(&data.ticket.venue_address))),
            ticket_lines = escape_html(&ticket_lines),
        ),
    )
}

fn render_refund_html(data: &RefundEmailData<'_>) -> String {
    let (accent, surface, status_copy) = if data.refund_status == "Completed" {
        (
            "#2563eb",
            "#eff6ff",
            "The refund has been processed successfully and should reflect in your original payment method soon.",
        )
    } else {
        (
            "#f59e0b",
            "#fffbeb",
            "Your refund has been initiated. Banks usually take 5-7 business days to reflect the amount.",
        )
    };

    render_email_shell(
        "Refund Update",
        "Refund Status Update",
        &format!(
            "Hi {}, here is the latest update for your ticket refund.",
            escape_html(&data.user.name)
        ),
        accent,
        &format!(
            r#"
            {status_card}
            {section_open}
              <div style="font-size:22px;line-height:30px;font-weight:700;color:#111827;margin-bottom:16px;">{event_name}</div>
              {detail_one}
              {detail_two}
              {detail_three}
            {section_close}
            <div style="background:{surface};border:1px solid {accent};border-radius:18px;padding:20px;">
              <div style="font-size:15px;line-height:24px;font-weight:700;color:#111827;margin-bottom:10px;">What Happens Next</div>
              <div style="font-size:14px;line-height:24px;color:#4b5563;">{status_copy}</div>
            </div>
            "#,
            status_card = render_status_card(
                data.refund_status,
                status_copy,
                accent,
                surface,
            ),
            section_open = section_open(),
            section_close = section_close(),
            event_name = escape_html(&data.ticket.event_title),
            detail_one = render_detail_row("Booking ID", &data.order.id.to_string(), None),
            detail_two = render_detail_row("Refund Amount", &format!("INR {:.2}", data.refund_amount), None),
            detail_three = render_detail_row("Event", &escape_html(&data.ticket.event_title), Some(&escape_html(&data.ticket.venue_name))),
            surface = surface,
            accent = accent,
            status_copy = status_copy,
        ),
    )
}

fn render_email_shell(
    preheader: &str,
    title: &str,
    subtitle: &str,
    accent: &str,
    body: &str,
) -> String {
    format!(
        r#"
        <!DOCTYPE html>
        <html lang="en">
          <head>
            <meta charset="UTF-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <title>{title}</title>
          </head>
          <body style="margin:0;padding:0;background:#f3f4f6;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;color:#111827;">
            <div style="display:none;max-height:0;overflow:hidden;opacity:0;color:transparent;">{preheader}</div>
            <table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="background:#f3f4f6;padding:24px 12px;">
              <tr>
                <td align="center">
                  <table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="max-width:680px;">
                    <tr>
                      <td style="padding-bottom:16px;">
                        <div style="background:linear-gradient(135deg,#111827 0%,#1f2937 60%,{accent} 140%);border-radius:28px;padding:28px 28px 32px;color:#ffffff;">
                          <div style="font-size:14px;line-height:20px;font-weight:700;letter-spacing:0.08em;text-transform:uppercase;color:#fbbf24;margin-bottom:18px;">GrabAPass</div>
                          <div style="font-size:30px;line-height:38px;font-weight:800;margin-bottom:10px;">{title}</div>
                          <div style="font-size:15px;line-height:26px;color:#d1d5db;max-width:520px;">{subtitle}</div>
                        </div>
                      </td>
                    </tr>
                    <tr>
                      <td style="background:#ffffff;border:1px solid #e5e7eb;border-radius:28px;padding:24px;box-shadow:0 10px 30px rgba(17,24,39,0.08);">
                        {body}
                      </td>
                    </tr>
                    <tr>
                      <td style="padding:18px 8px 0;text-align:center;font-size:12px;line-height:20px;color:#6b7280;">
                        You received this email because of activity on your GrabAPass account.
                      </td>
                    </tr>
                  </table>
                </td>
              </tr>
            </table>
          </body>
        </html>
        "#,
        preheader = escape_html(preheader),
        title = escape_html(title),
        subtitle = escape_html(subtitle),
        accent = accent,
        body = body,
    )
}

fn render_status_card(label: &str, description: &str, accent: &str, surface: &str) -> String {
    format!(
        r#"
        <div style="background:{surface};border:1px solid {accent};border-radius:18px;padding:18px 20px;margin-bottom:20px;">
          <div style="font-size:12px;line-height:18px;font-weight:700;letter-spacing:0.12em;text-transform:uppercase;color:{accent};margin-bottom:8px;">{label}</div>
          <div style="font-size:14px;line-height:24px;color:#374151;">{description}</div>
        </div>
        "#,
        label = escape_html(label),
        description = escape_html(description),
        accent = accent,
        surface = surface,
    )
}

fn render_detail_row(label: &str, value: &str, helper: Option<&str>) -> String {
    let helper_markup = helper
        .map(|text| {
            format!(
                r#"<div style="font-size:13px;line-height:21px;color:#6b7280;margin-top:4px;">{}</div>"#,
                text
            )
        })
        .unwrap_or_default();

    format!(
        r#"
        <div style="padding:14px 0;border-top:1px solid #f3f4f6;">
          <div style="font-size:12px;line-height:18px;font-weight:700;letter-spacing:0.08em;text-transform:uppercase;color:#9ca3af;margin-bottom:4px;">{label}</div>
          <div style="font-size:15px;line-height:24px;color:#111827;font-weight:600;">{value}</div>
          {helper_markup}
        </div>
        "#,
        label = escape_html(label),
        value = value,
        helper_markup = helper_markup,
    )
}

fn render_summary_item(label: &str, value: &str) -> String {
    format!(
        r#"
        <div style="display:inline-block;vertical-align:top;width:calc(50% - 8px);margin:0 8px 12px 0;background:#f9fafb;border:1px solid #e5e7eb;border-radius:16px;padding:16px;box-sizing:border-box;">
          <div style="font-size:12px;line-height:18px;font-weight:700;letter-spacing:0.08em;text-transform:uppercase;color:#9ca3af;margin-bottom:6px;">{label}</div>
          <div style="font-size:15px;line-height:24px;color:#111827;font-weight:600;">{value}</div>
        </div>
        "#,
        label = escape_html(label),
        value = value,
    )
}

fn section_open() -> &'static str {
    r#"<div style="background:#ffffff;border:1px solid #e5e7eb;border-radius:20px;padding:22px 24px;margin-bottom:20px;">"#
}

fn section_close() -> &'static str {
    "</div>"
}

fn summary_grid_open() -> &'static str {
    r#"<div style="font-size:0;">"#
}

fn summary_grid_close() -> &'static str {
    "</div>"
}

fn render_ticket_lines(ticket: &TicketDetail) -> String {
    let seat_lines = ticket
        .seats
        .0
        .iter()
        .map(|seat| format!("{} ({})", seat.seat_label, seat.section_name))
        .collect::<Vec<_>>();
    let tier_lines = ticket
        .tiers
        .0
        .iter()
        .map(|tier| format!("{} x {}", tier.quantity, tier.name))
        .collect::<Vec<_>>();

    seat_lines
        .into_iter()
        .chain(tier_lines)
        .collect::<Vec<_>>()
        .join(", ")
}

fn total_ticket_count(ticket: &TicketDetail) -> i32 {
    let seat_count = ticket.seats.0.len() as i32;
    let tier_count = ticket.tiers.0.iter().map(|tier| tier.quantity).sum::<i32>();
    seat_count + tier_count
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
