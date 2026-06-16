# AI Grader

A local-first, GDPR-compliant desktop app that helps teachers grade handwritten
and digital student work with AI. Built with **Tauri (Rust)** + **React + TypeScript**.

> **Status:** Feature-complete end-to-end. Local data layer, licensing
> gatekeeper, GDPR scrub, the full auto-routed extraction pipelines (multi-pass
> vision consensus + native digital parsing), pre-flight verification, and the
> structured-output grading engine are all implemented and wired. The Rust
> backend compiles against the documented Vertex AI + Cloud Function endpoints
> you supply in `.env`.

## Architecture

```
src/                     React + TS + Tailwind frontend
  lib/types.ts           Domain types (mirror of Rust models) + dropzone router
  lib/api.ts             Typed wrappers over every Tauri command
  components/
    ActivationGate · GraceBanner · LockScreen   (licensing UI)
    AppShell                                      (persona factory + dropzone)
    GradingFlow · PreFlight · Results             (extract → verify → grade)
src-tauri/               Rust backend
  src/db/                SQLite: migrations, models, repository (all PII local)
  src/licensing.rs       Activation, 24h heartbeat, 7-day "No Hostage" grace
  src/gdpr.rs            Pre-upload EXIF/metadata scrub + base64
  src/ai.rs              Vertex AI client: multi-pass consensus OCR + grading
  src/extract.rs         Native .docx/.xlsx/.pptx/.txt parsing (on-device)
  src/state.rs           DB handle + RAM-only ephemeral AI credentials
  src/commands.rs        The React⇄Rust command surface (17 commands)
```

## End-to-end flow

1. **Persona Factory** — pick subject + level + rubric → dynamic system prompt.
2. **Dropzone** — auto-routes by extension: images/PDF → vision, Office/txt → digital.
3. **Extract** — images run 3 temperature passes + a consensus merge (LaTeX-aware,
   errors preserved); digital files are parsed natively to Markdown / cell refs /
   slide sections. Everything is GDPR-scrubbed before upload.
4. **Pre-Flight** — double-click any line to fix an OCR misread, then Confirm & Grade.
5. **Grade** — Gemini returns the forced JSON schema; the score is recomputed from
   deductions and persisted with inline corrections.

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
