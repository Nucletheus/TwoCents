# TwoCents

A finance and budgeting app for couples to track expenses, manage split expenses, work towards savings goals, and gain insights into their financial habits.

## Tech Stack

- **Frontend**: Next.js 14+ (Web/Desktop)
- **Backend**: Supabase (PostgreSQL + Auth + Storage + Real-time)
- **Monorepo**: Turborepo

## Getting Started

### Prerequisites

- Node.js 18+
- npm 9+
- Docker Desktop
- Supabase CLI

### Installation

- Install dependencies:

  ```bash
  npm install
  ```

- Set up environment variables (see `apps/web/.env.example`)

- Start development server:

  ```bash
  npm run dev
  ```

### Local Supabase (Recommended)

- Start local Supabase services:

  ```bash
  npm run supabase:start
  ```

- Reset/apply all local migrations and seed data:

  ```bash
  npm run supabase:reset
  ```

- Get local API URL and anon key:

  ```bash
  npm run supabase:status
  ```

- Update `apps/web/.env.local` with the local values, then run:

  ```bash
  npm run dev:web
  ```

### Standalone (local) mode

**Default is on** (no sign-in screen): unless you set `NEXT_PUBLIC_STANDALONE_MODE=false`, the app skips the marketing page and `/auth/*` and uses a **single local Supabase user**; the server signs in with `STANDALONE_AUTH_EMAIL` / `STANDALONE_AUTH_PASSWORD` in `.env.local` (not sent to the browser).

1. In Supabase Studio (local), open **Authentication → Users** and add a user with the same email and password as `STANDALONE_AUTH_EMAIL` and `STANDALONE_AUTH_PASSWORD` in `apps/web/.env.local` (see `apps/web/.env.example`).

The database still uses Supabase Auth under the hood (JWT in cookies) so row-level security keeps working. For a hosted or multi-user deploy, set `NEXT_PUBLIC_STANDALONE_MODE=false` to enable `/auth/login` and `/auth/signup`.

Private-mode defaults:

- Supabase signup is disabled in `supabase/config.toml`.
- Web signup UI is disabled unless `NEXT_PUBLIC_ENABLE_SIGNUP=true`.

## Project Structure

- `apps/web` - Next.js web application
- `packages/shared` - Shared business logic and utilities
- `packages/supabase` - Supabase client configuration and types
- `supabase/` - Database migrations and configuration

## Development

- `npm run dev` - Start all configured dev tasks (web-focused)
- `npm run dev:web` - Start only the web application
- `npm run supabase:start` - Start local Supabase stack
- `npm run supabase:stop` - Stop local Supabase stack
- `npm run supabase:status` - Show local Supabase URLs and keys
- `npm run supabase:reset` - Reset local DB with migrations and seed
- `npm run build` - Build all apps
- `npm run lint` - Lint all apps
- `npm run format` - Format code with Prettier
