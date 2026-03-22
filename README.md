# GrabAPass

GrabAPass is an event ticketing application built with:

- Angular frontend
- Rust backend using Axum
- PostgreSQL database

This README covers:

- what you need to install first
- how to set up environment variables
- how to run the project without Docker
- how to run the project with Docker
- app-specific notes for payments, API routing, and database usage

## Project structure

```text
GrabAPass/
├── frontend/   # Angular app
├── backend/    # Rust + Axum API
└── docker-compose.yml
```

## Prerequisites

Before running this project on your laptop, install these tools.

### 1. Install Git

Download and install Git:

- https://git-scm.com/downloads

Check installation:

```bash
git --version
```

### 2. Install Node.js and npm

The Angular frontend uses Node.js and npm.

Recommended:

- Install Node.js 22 LTS or a recent Node 22 release

Download:

- https://nodejs.org/

Check installation:

```bash
node -v
npm -v
```

### 3. Install Rust

The backend is written in Rust, so you need Rust installed locally if you want to run the backend without Docker.

Install with `rustup`:

- https://rustup.rs/

Recommended command:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, restart your terminal and verify:

```bash
rustc --version
cargo --version
```

### 4. Install PostgreSQL

You only need this if you want to run the database locally without Docker.

Download:

- https://www.postgresql.org/download/

Check installation:

```bash
psql --version
```

### 5. Install Docker Desktop

You only need this if you want to run the full project with Docker.

Download:

- https://www.docker.com/products/docker-desktop/

Check installation:

```bash
docker --version
docker compose version
```

## Clone the repository

```bash
git clone <your-repo-url>
cd GrabAPass
```

## Environment variables

The backend uses environment variables for database access, JWT auth, frontend origin, and Razorpay.

### Backend env files

Create:

```text
backend/.env
```

You can start from one of these templates:

- [backend/.env.example](/Users/Atharva/Desktop/GrabAPass/backend/.env.example) for cloud Postgres or Supabase
- [backend/.env.local.example](/Users/Atharva/Desktop/GrabAPass/backend/.env.local.example) for locally installed PostgreSQL
- [backend/.env.docker.example](/Users/Atharva/Desktop/GrabAPass/backend/.env.docker.example) as the Docker reference

There is also a dedicated Docker env file in the repo:

- [backend/.env.docker](/Users/Atharva/Desktop/GrabAPass/backend/.env.docker)

Example:

```env
DATABASE_URL=postgres://<user>:<password>@<host>:<port>/<database>?sslmode=require
JWT_SECRET=<replace-with-a-long-random-secret>
FRONTEND_URL=http://localhost:4200
PORT=3000
```

### Required backend variables

`DATABASE_URL`

- PostgreSQL connection string
- Non-Docker local mode can point to Supabase, Neon, Railway, or local Postgres
- Docker mode overrides this to the local `postgres` container

`JWT_SECRET`

- used to sign and verify JWT tokens
- use a long random string in real usage

`FRONTEND_URL`

- used by backend CORS
- for local frontend development this should usually be:

```env
FRONTEND_URL=http://localhost:4200
```

`PORT`

- backend server port
- default used by this app:

```env
PORT=3000
```

### Razorpay variables

These are required if you want the payment flow to work:

```env
RAZORPAY_KEY_ID=rzp_test_your_key_id
RAZORPAY_KEY_SECRET=your_test_secret
RAZORPAY_WEBHOOK_SECRET=your_webhook_secret
RAZORPAY_CHECKOUT_NAME=GrabAPass
```

If `RAZORPAY_KEY_ID` and `RAZORPAY_KEY_SECRET` are missing, the backend still starts, but checkout initialization will fail with:

```text
Payment gateway is not configured.
```

## Install dependencies for local development

### Frontend

```bash
cd frontend
npm install
cd ..
```

### Backend

Rust dependencies are handled automatically by Cargo when you build or run the project.

## Running the project without Docker

This is the normal local development workflow.

### Step 1. Start PostgreSQL

Make sure your database is running.

You have two choices:

- use a cloud database like Supabase through `backend/.env`
- use a local PostgreSQL instance and point `DATABASE_URL` to it

Recommended shortcuts:

- copy [backend/.env.example](/Users/Atharva/Desktop/GrabAPass/backend/.env.example) to `backend/.env` if using Supabase or another managed Postgres
- copy [backend/.env.local.example](/Users/Atharva/Desktop/GrabAPass/backend/.env.local.example) to `backend/.env` if using local Postgres

Example local connection string:

```env
DATABASE_URL=postgres://postgres:password@localhost:5432/grabapass
```

### Step 2. Start the backend

Open a terminal:

```bash
cd backend
cargo run
```

The backend will start on:

```text
http://localhost:3000
```

Health endpoint:

```text
http://localhost:3000/health
```

Important app behavior:

- the backend binds to `0.0.0.0`
- SQLx migrations run automatically on startup

### Step 3. Start the frontend

Open another terminal:

```bash
cd frontend
npm start
```

The frontend will start on:

```text
http://localhost:4200
```

### Local frontend API behavior

In local non-Docker development, Angular uses:

- [frontend/proxy.conf.json](/Users/Atharva/Desktop/GrabAPass/frontend/proxy.conf.json)

That proxy forwards `/api` calls from the Angular dev server to:

```text
http://localhost:3000
```

This means the frontend code can use relative API paths like:

```text
/api/auth
/api/events
/api/organizer/events
```

and still work locally.

### Frontend API base URL behavior

The frontend uses Angular environment files to decide where API requests go.

- local development uses [frontend/src/environments/environment.ts](/Users/Atharva/Desktop/GrabAPass/frontend/src/environments/environment.ts)
- in local development, `apiBaseUrl` is empty, so requests stay relative and keep using [frontend/proxy.conf.json](/Users/Atharva/Desktop/GrabAPass/frontend/proxy.conf.json)
- production builds use [frontend/src/environments/environment.prod.ts](/Users/Atharva/Desktop/GrabAPass/frontend/src/environments/environment.prod.ts)

Current production API base:

```text
https://grabapass.onrender.com
```

That means the deployed frontend calls endpoints like:

```text
https://grabapass.onrender.com/api/events
https://grabapass.onrender.com/api/auth/login
```

If the backend URL changes later, update [frontend/src/environments/environment.prod.ts](/Users/Atharva/Desktop/GrabAPass/frontend/src/environments/environment.prod.ts).

## Running the project with Docker

Docker runs the full stack together:

- frontend
- backend
- postgres

### What Docker does in this repo

- frontend is built as an Angular production build
- frontend is served by Nginx
- Nginx proxies `/api/*` to the backend container
- backend connects to the `postgres` service using Docker internal networking
- Postgres data is stored in a persistent Docker volume

### Start everything

From the repository root:

```bash
docker compose up --build
```

### Open the app

- Frontend: `http://localhost:4200`
- Backend API: `http://localhost:3000`
- Backend health: `http://localhost:3000/health`
- Postgres: `localhost:5432`

### Stop everything

```bash
docker compose down
```

### Stop and remove containers and DB volume

```bash
docker compose down -v
```

## Docker environment behavior

The backend service in Docker loads `backend/.env.docker`.

Reference file:

- [backend/.env.docker.example](/Users/Atharva/Desktop/GrabAPass/backend/.env.docker.example)
- actual Compose-loaded file: [backend/.env.docker](/Users/Atharva/Desktop/GrabAPass/backend/.env.docker)

### Docker backend values

Inside Docker, the backend uses:

```env
DATABASE_URL=postgresql://grabapass:grabapass@postgres:5432/grabapass
JWT_SECRET=change-me-for-real-use
FRONTEND_URL=http://localhost:4200
PORT=3000
RAZORPAY_CHECKOUT_NAME=GrabAPass
```

### Important note about Supabase vs Docker Postgres

When running with Docker:

- the backend does **not** use Supabase for `DATABASE_URL`
- it uses the local Docker Postgres container
- the hostname is `postgres`, which is the Compose service name

When running without Docker:

- the backend uses whatever `DATABASE_URL` is set in `backend/.env`
- that can be Supabase, local Postgres, or any other PostgreSQL instance

### Razorpay in Docker

If your [backend/.env.docker](/Users/Atharva/Desktop/GrabAPass/backend/.env.docker) contains:

```env
RAZORPAY_KEY_ID=...
RAZORPAY_KEY_SECRET=...
RAZORPAY_WEBHOOK_SECRET=...
```

then the backend container will receive those values too, and the payment flow can work in Docker.

If those values are missing, checkout will fail with:

```text
Payment gateway is not configured.
```

## App-specific technical notes

### Frontend API configuration

- the Angular app already uses relative API URLs like `/api/...`
- that is compatible with both local and Docker workflows
- in local development, Angular uses `proxy.conf.json`
- in Docker, Nginx replaces the Angular dev proxy and forwards `/api` to the backend container

### Backend bind address

The backend already binds to:

```text
0.0.0.0:3000
```

That is correct for both local access and Docker containers.

### Database migrations

SQLx migrations run automatically during backend startup.

That means:

- local `cargo run` applies migrations automatically
- Docker backend startup also applies migrations automatically

### CORS

The backend uses `FRONTEND_URL` for CORS.

Recommended value for local usage:

```env
FRONTEND_URL=http://localhost:4200
```

In Docker, the browser usually talks to the frontend container at `http://localhost:4200`, and Nginx forwards API traffic internally, so this setup remains compatible.

## Deploying Backend To Render

Deploy the Rust API as a Render web service directly from the [backend](/Users/Atharva/Desktop/GrabAPass/backend) directory.

### Render service settings

- Root Directory: `backend`
- Environment: `Rust`
- Build Command: `cargo build --release --locked`
- Start Command: `./target/release/backend`

### Render backend environment variables

- `DATABASE_URL`
- `JWT_SECRET`
- `FRONTEND_URL`
- `RAZORPAY_KEY_ID` if payments are enabled
- `RAZORPAY_KEY_SECRET` if payments are enabled
- `RAZORPAY_WEBHOOK_SECRET` if webhook verification is enabled
- `RAZORPAY_CHECKOUT_NAME` optional, defaults to `GrabAPass`

Render injects `PORT` automatically. Locally, the backend falls back to `3000`.

### Frontend and backend pairing

Your current deployed backend base URL is:

```text
https://grabapass.onrender.com
```

Set backend `FRONTEND_URL` to the frontend site origin, not to the backend URL.

Example:

```env
FRONTEND_URL=https://your-frontend-site.com
```

## Common commands

### Frontend

Install dependencies:

```bash
cd frontend
npm install
```

Run dev server:

```bash
npm start
```

Build:

```bash
npm run build
```

### Backend

Run backend:

```bash
cd backend
cargo run
```

Check code:

```bash
cargo check
```

### Docker

Start:

```bash
docker compose up --build
```

Stop:

```bash
docker compose down
```

Remove volume too:

```bash
docker compose down -v
```

## Troubleshooting

### 1. Frontend starts but API calls fail

Check:

- backend is running on `http://localhost:3000`
- local frontend uses `proxy.conf.json`
- Docker frontend uses Nginx proxy

### 2. Login works but protected pages fail

Check:

- `JWT_SECRET` is set
- backend is reachable
- token exists in browser local storage

### 3. Payment fails with `Payment gateway is not configured.`

Check:

- `RAZORPAY_KEY_ID` exists
- `RAZORPAY_KEY_SECRET` exists
- backend was restarted after env changes
- if using Docker, confirm `backend/.env` contains the Razorpay variables

### 4. Database connection fails

Check:

- `DATABASE_URL` is correct
- PostgreSQL is running
- if using Docker, the backend should use `postgres` as hostname, not `localhost`

## Notes

- Docker support does not break the non-Docker workflow
- non-Docker mode can still use Supabase through `backend/.env`
- Docker mode uses local Postgres by default
- Postgres data persists in the `postgres_data` Docker volume

## Security note

Do not commit real secrets to the repository.

That includes:

- database passwords
- JWT secrets
- Razorpay keys
- webhook secrets

Use local env files for development and rotate any real secrets that were accidentally committed.
