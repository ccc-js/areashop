# AGENTS.md — areashop

## Commands (use these, don't guess)
- Dev (SQLite, zero-dep): `./run.sh --seed` → backend `:8080`, frontend `:5173`. `--empty` = wipe DB, `--reseed` = wipe + reseed, `--pg` = Postgres via docker, `--prod` = `docker compose up --build` (web `:3000`), `--stop` = kill all.
- Full verify: `./test.sh` (unit `cargo test` + build + e2e on temp SQLite via port `18090`). `./test.sh --pg` also re-runs e2e on Postgres (needs docker).
- CI (`.github/workflows/ci.yml`): backend `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test`; frontend `npm run build`. Run fmt/clippy before pushing backend changes.
- Frontend: `cd frontend && npm run dev` (`/api` proxies to `:8080` per `vite.config.ts`), `npm run lint` (oxlint), `npm run build` (`tsc -b && vite build`).

## Architecture (non-obvious)
- No migrations. Backend auto-creates schema at startup from SeaORM entities (`backend/src/schema.rs:setup_schema`) + adds missing columns via `ALTER TABLE ... ADD COLUMN` (duplicate-column errors swallowed). Same binary serves SQLite and Postgres — `DATABASE_URL` scheme is the only switch (`backend/src/config.rs`).
- Env: `DATABASE_URL` (default `sqlite://./areashop.db?mode=rwc`), `JWT_SECRET`, `PORT` (default 8080), `SEED=0/false/no/empty` skips seeding. `main.rs` copies `JWT_SECRET` into env for the `AuthUser` extractor — don't read auth secret any other way.
- Seed (`backend/src/seed.rs:seed_if_empty`) is idempotent: skips if `areas` non-empty. Test accounts (all `password123`): `0900000001` admin, `0900000002` seller, `0900000003` buyer. Reseed only via wipe (`./run.sh --reseed` or delete sqlite file / `DROP SCHEMA public`).
- Backend layout: `src/main.rs` → `routes/v1_router()` (`src/routes/mod.rs`), handlers per resource in `src/routes/*.rs`, entities in `src/entities/*`.
- Unified model（賣東西＋賣服務同一種，不分兩類）: `items`（`stock` NULL=不限量，`bookable`=可選日期，兩者可並存）＋ `availability_rules` (weekly template, weekday 0=Sun..6=Sat, windows are `{start,end}` free-text pairs, display-only; 24h = `00:00-24:00`) + `availability_exceptions` (per-day open/full override — all seller edits go here, never touch template) + `orders` (`date`/`window` NULL = 直接買，有值 = 預約）. Availability resolution is pure `booking::resolve()` (exception > rule > closed) with unit tests; a day is bookable iff open && !full. Order state machine `pending → confirmed → ready → completed | cancelled | noshow` in `entities/order.rs` (confirmed 可跳過 ready 直接 complete； noshow 限店主）; cancel restocks (only stocked items), over-sell rejected, cross-shop order in one request rejected, duplicate booking (same buyer/item/date) rejected in one transaction; cancel policy (dated orders only) `now < day 00:00 UTC − cancel_hours` else 422.
- ⚠️ Breaking: old `products`/`services`/`appointments` tables are orphaned (not migrated). After pulling this change, existing dev DBs need `./run.sh --reseed`.
- Frontend: React Router SPA (`src/App.tsx`, pages in `src/pages/`: Home/ItemDetail/Orders/Seller/Provider/Auth), API client + auth/area context in `src/lib/{api,auth,area}.tsx`. Prices are `price_cents` (int) end-to-end. ItemDetail (`/i/:id`) merges buy box + month-grid booking; Orders shows date/window + seller sees buyer/items; Provider = per-item week template + day toggle + day detail.

## Gotchas
- `test.sh` kills any `target/debug/areashop-api` process and uses its own temp DB/port — don't run `./run.sh` dev server on `18090` concurrently.
- Frontend `dist/` and `backend/target/` are build artifacts; sqlite `*.db` files are local state — never commit.
