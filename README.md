# Scholarly

Scholarly is the scholarship directory for `scholarly.titiknolenglish.com`, built in Rust with Axum, Askama, and SQLx against Postgres 17 on Neon. This folder covers milestones M1 to M3 of `docs/prd/PRD-rust.md`: the cargo project, the full schema and workbook import, the directory and detail pages, and the SEO behavior those pages need. M4 (snapshot tests, OG images, redirect maintenance) and M5 (deployment, TLS, cron, load test) are deferred.

## Environment

The server and every database-touching command read one required variable and three optional ones.

| Variable | Required | Default |
| --- | --- | --- |
| `DATABASE_URL` | yes | none, the program exits without it |
| `BIND` | no | `0.0.0.0` |
| `PORT` | no | `8080` |
| `BASE_URL` | no | `https://scholarly.titiknolenglish.com` |

`DATABASE_URL` must point at the Neon pooled endpoint. Never write it into a file inside this folder; it stays in the environment. Migrations are embedded and run automatically on startup and before every import and check.

## Toolchain

The crate builds on `stable-x86_64-pc-windows-msvc` with Visual Studio Build Tools 2022 installed. Cargo lives at `%USERPROFILE%\.cargo\bin`, so a fresh shell needs that on PATH:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path
```

## Commands

| Command | What it does |
| --- | --- |
| `cargo run -- serve` | Run migrations, then serve the site on `BIND:PORT` |
| `cargo run -- import --file ../processed/scholarship-master.xlsx` | Read the workbook, validate, and replace all data in one transaction |
| `cargo run -- import --file ../processed/scholarship-master.xlsx --dry-run` | Validate only, print the per-row report, write nothing |
| `cargo run -- check` | Re-run the same validation against the live database, exit code 1 on any hard error, for a cron job |

The workbook is the editing surface. One run deletes the previous dataset and inserts the new one, so removing a row from the workbook removes the page. A row is only published when every entry in the workbook's Checklist sheet reads `Lolos`. Import rejects any row carrying a `[contoh data]` or `CONTOH-` marker, and it fails the whole run on any hard error listed in PRD section 8.

## Tests

`cargo test` runs all unit tests offline with no database: status thresholds, countdown across midnight in Asia/Jakarta, slug validation, every import validation rule, filter parsing, title and meta boundaries, and JSON-LD escaping. The tests in `tests/routes.rs` are marked `#[ignore]` and need a seeded Neon branch: apply the migrations, run `tests/fixtures/seed.sql`, point `DATABASE_URL` at the branch, then run `cargo test -- --ignored`. `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` must all pass before every handoff.

## Static assets

`static/css/site.css` is the single stylesheet extracted from the four demo files in `docs/demo`, with the design tokens as CSS custom properties. `static/js/app.js` is progressive enhancement only: the carousel, the mobile filter sheet, the FAQ accordion, and the nav toggle. With JavaScript off, every page renders fully and the filter, sort, search, pagination, and FAQ still work through plain forms, links, and native details elements. Assets are served with immutable cache headers and no content hash, so rename them manually when they change.

## Deferred

OG images, snapshot tests with insta, the redirect admin path, Caddy or nginx TLS setup, systemd unit, the load smoke test, and `cargo audit` in CI belong to M4 and M5. The workbook has no column for rolling deadlines, so rolling records currently need a small SQL update after import, or a workbook column addition in `scripts/build_scholarship_master.py`.
