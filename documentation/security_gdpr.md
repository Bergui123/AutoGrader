# Security & GDPR Compliance

AI Grader is GDPR-compliant **by design** through local data sovereignty and
anonymized cloud calls. This document explains the guarantees and where they are
enforced in code.

## 1. All PII stays local

Student names, IDs, teacher notes, grades, and folder paths exist **only** in:
- the local SQLite database at `Documents/AIGrader/aigrader.db`, and
- the local file system under `Documents/AIGrader/Students/[Name]/`.

The cloud never receives or stores any of it. Enforced by:
- `db/` — the sole persistence layer; no network writes.
- `commands.rs` — student/class/grade data is never placed in an AI payload.

## 2. The cloud sees only anonymized content

Before any bytes leave the machine they pass through `gdpr.rs`:
- **Images** are decoded and re-encoded to PNG (`scrub_image_bytes`), which
  inherently drops EXIF, XMP, ICC, and any tracking chunks — only pixels remain.
- **PDFs** have the `/Info` dictionary and XMP metadata stream removed
  (`scrub_pdf_file`) before being sent as `application/pdf`.
- **Text** has zero-width/tracking characters and control codes stripped
  (`scrub_text`).

The grading prompt contains the student's *work*, never their identity.

## 3. AI credentials live in RAM only

- The local DB stores an **Activation Code**, never the Gemini API key.
- On activation the licensing Cloud Function returns the key, held in
  `state::EphemeralCreds` — a `RwLock<Option<…>>` that is **never written to
  disk or logs** and is `zeroize`d on drop (`Drop for EphemeralCreds`).
- `Debug` for the credential is redacted.

## 4. "No Hostage" data survival

Licensing problems never lock a teacher out of their own data:
- A 24h heartbeat verifies the license; failure starts a **7-day grace period**
  (`grace_period_started_at` in SQLite) during which the app is fully functional.
- After 7 days the grading UI locks, **but** the SQLite DB and student folders
  remain un-encrypted and fully readable, and the lock screen exposes an
  **"Open Local Student Data"** button (`open_local_student_data`).

## 5. Transport

- All cloud calls are HTTPS (`reqwest` + rustls).
- The webview CSP (`tauri.conf.json`) restricts origins.

## Data subject rights

Because everything is local files + a single SQLite file, the teacher can fully
export, inspect, or **erase** a student's data by deleting their folder and row —
no cloud deletion request is required.
