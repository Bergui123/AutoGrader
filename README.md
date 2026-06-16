# AI Grader

A local-first, GDPR-compliant desktop app that helps teachers grade handwritten
and digital student work with AI. Built with **Tauri (Rust)** + **React + TypeScript**.

> **Status:** Milestone 1 — full project scaffold + local data layer + licensing
> gatekeeper + GDPR scrub plumbing. The AI extraction/grading pipelines are
> stubbed at the seams and wired to real endpoints next.

## Architecture

```
src/                     React + TS + Tailwind frontend
  lib/types.ts           Domain types (mirror of Rust models) + dropzone router
  lib/api.ts             Typed wrappers over every Tauri command
  components/            ActivationGate · GraceBanner · LockScreen · AppShell
src-tauri/               Rust backend
  src/db/                SQLite: migrations, models, repository (all PII local)
  src/licensing.rs       Activation, 24h heartbeat, 7-day "No Hostage" grace
  src/gdpr.rs            Pre-upload EXIF/metadata scrub
  src/state.rs           DB handle + RAM-only ephemeral AI credentials
  src/commands.rs        The React⇄Rust command surface
```

Data privacy by design (spec §4):
- All PII (student names, IDs, grades, notes) lives **only** in local SQLite at
  `Documents/AIGrader/aigrader.db`.
- Ephemeral AI credentials live **only in RAM** — never written to disk/SQLite,
  and zeroized on drop.
- Files are **scrubbed** (EXIF/author metadata stripped) before any upload.

## Prerequisites

- Node 18+ and npm
- Rust (stable, MSVC toolchain on Windows) — install via `rustup`
- Visual Studio Build Tools (C++ workload) on Windows

```powershell
winget install --id Rustlang.Rustup -e
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
rustup default stable-msvc
```

## Setup

```powershell
npm install
cp .env.example .env      # then fill in your license endpoint + GCP project
npm run icon              # generate app icons from app-icon.png (first run)
```

## Develop

```powershell
npm run dev               # launches Tauri + Vite with hot reload
```

Frontend only (no Rust): `npm run dev:vite`. Type-check: `npm run typecheck`.

## Build

```powershell
npm run build             # produces a native installer in src-tauri/target
```

## Configuration

See `.env.example`. The `AIGRADER_LICENSE_ENDPOINT` Cloud Function must accept:

- `POST {endpoint}` `{ "activation_code": "..." }` → activation
- `POST {endpoint}/pulse` `{ "activation_code": "..." }` → heartbeat

Both return `{ "status": "active"|"expired"|"invalid", "access_token": "...", "expires_in": 3600 }`.
