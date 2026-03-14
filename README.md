# GrabAPass

GrabAPass is an Angular + Rust event ticketing application backed by cloud PostgreSQL.

Current foundation scope:

- Angular frontend for event listing, event detail, organizer dashboard, login, and registration
- Rust backend with health check, auth, create event, and list events APIs
- Cloud PostgreSQL configuration through environment variables only

This repository does not require Docker for the current development phase.

## Razorpay Sandbox Checkout

GrabAPass now uses a production-style Razorpay payment flow for checkout:

- seat holds are validated before payment starts
- a pending local order is created first
- a Razorpay order is created in test mode
- the frontend opens the real Razorpay Checkout modal
- the backend verifies the Razorpay signature and payment status before marking seats as sold
- tickets are generated only after verified payment

### Required backend environment variables

Set these in the backend environment before starting the Rust API:

```env
RAZORPAY_KEY_ID=rzp_test_your_key_id
RAZORPAY_KEY_SECRET=your_test_secret
RAZORPAY_WEBHOOK_SECRET=your_webhook_secret
RAZORPAY_CHECKOUT_NAME=GrabAPass
```

### Database migration

Apply the new migration before testing checkout:

```bash
cd backend
sqlx migrate run
```

### Sandbox testing

Use Razorpay Test Mode in the dashboard. The checkout page now launches the Razorpay-hosted payment modal and supports sandbox cards and test UPI flows.

### Razorpay webhook

Configure a Razorpay webhook pointing to:

```text
POST /api/payments/razorpay/webhook
```

Subscribe at minimum to:

- `payment.authorized`
- `payment.captured`
- `payment.failed`

This lets GrabAPass reconcile payments even if the browser callback is interrupted or retried.
