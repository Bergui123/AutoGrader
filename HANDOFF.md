# AI Grader — Handoff / Continuation Notes

> Read this first. It captures the exact state of the project, what's verified,
> what's left, and the non-obvious gotchas so you don't rediscover them.
> Last updated: 2026-06-15.

---

## 1. What this is

A **local-first, GDPR-compliant desktop app** for teachers to AI-grade handwritten
and digital student work. Greenfield build (the old repo was an unrelated MCP
starter — fully replaced).

**Stack:** Tauri 2 (Rust backend) + React 18 + TypeScript + Vite + TailwindCSS;
SQLite via `rusqlite` (bundled); Google Vertex AI (Gemini 2.5 Pro) for the AI.

---

## 2. Current status — FEATURE COMPLETE & VERIFIED

The full pipeline is implemented and wired end-to-end:
Persona Factory → auto-routing dropzone → extract → pre-flight verify → grade → results.

**Verified on 2026-06-15:**
- `cargo check` (in `src-tauri`) — compiles **clean, zero warnings**.
- `cargo test` — **8/8 unit tests pass**.
- `npm run build:vite` — frontend `tsc` + Vite build pass (43 modules).
- `npm run icon` — platform icons generated into `src-tauri/icons/`.

**NOT yet done:** nothing has been committed to git (still all working-tree
changes on `main`). The app has not been launched as a live window in this
session, and the real Vertex/licensing endpoints have never been exercised
(they don't exist yet — user will provide).

---

## 3. How to build / run / verify

Toolchain is installed but **`cargo` is NOT on the Git-Bash PATH**. It lives at
`~/.cargo/bin` (`C:\Users\berge\.cargo\bin`). Use **PowerShell** and prepend it:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo check --manifest-path D:\AutoGrader\src-tauri\Cargo.toml
cargo test  --manifest-path D:\AutoGrader\src-tauri\Cargo.toml
```

Rust toolchain: `stable-x86_64-pc-windows-msvc`, rustc/cargo 1.96.0. MSVC build
tools present. Node 22 / npm 11.

Frontend / app:
```powershell
npm install            # already done
npm run typecheck      # tsc --noEmit
npm run build:vite     # full frontend build
npm run dev            # launches the whole Tauri app (long-running GUI)
```

To actually use it: `copy .env.example .env` and fill in
`AIGRADER_LICENSE_ENDPOINT` + `AIGRADER_GCP_PROJECT`. With a blank `.env` the app
boots to the Activation Gate and activation fails by design (endpoint
unconfigured).

---

## 4. Architecture / file map

```
src/                              React frontend
  lib/types.ts                    Domain types (mirror Rust) + routeForFile() dropzone router
  lib/api.ts                      Typed wrappers over ALL 17 Tauri commands
  App.tsx                         Boot: fetch license status → route to a screen
  components/
    ActivationGate.tsx            §3 activation UI
    GraceBanner.tsx               §3 grace-period warning banner
    LockScreen.tsx                §3 "No Hostage" lock (Open Local Student Data)
    AppShell.tsx                  Persona Factory + assignment selector + dropzone
    GradingFlow.tsx               Orchestrates extract→preflight→grade→results for one file
    PreFlight.tsx                 §Phase 3: double-click a line to fix OCR misreads
    Results.tsx                   §Phase 4: score ring + summary + inline corrections

src-tauri/                        Rust backend
  Cargo.toml                      deps: tauri2, rusqlite(bundled), reqwest(rustls),
                                  image, zip, quick-xml, calamine, chrono, zeroize, dirs
  tauri.conf.json                 v2 config; bundle.icon → icons/*
  capabilities/default.json       v2 permissions (core/dialog/opener)
  src/
    main.rs                       thin → aigrader_lib::run()
    lib.rs                        builds app, manages AppState, spawns heartbeat, registers commands
    error.rs                      AppError (Serialize → string at the command boundary)
    config.rs                     AppConfig from env (+ minimal .env loader). Secrets NOT here.
    state.rs                      AppState: Mutex<Connection> + RwLock<Option<EphemeralCreds>>
                                  EphemeralCreds = RAM-only token, zeroized on drop
    db/
      mod.rs                      open() at Documents/AIGrader/aigrader.db, paths, in-memory test helper
      migrations.rs               forward-only, keyed on PRAGMA user_version
      models.rs                   serde structs (Student, Assignment, Submission, GradeResult, ...)
      repo.rs                     all CRUD + KV config store (has unit tests)
    licensing.rs                  activation, 24h heartbeat, 7-day grace, lock computation (has tests)
    gdpr.rs                       scrub_image (re-encode→PNG drops EXIF), scrub_text, base64 (has tests)
    ai.rs                         GeminiClient: multi-pass consensus OCR + structured grading + persona builder
    extract.rs                    native .docx/.xlsx/.pptx/.txt parsing (has tests)
    commands.rs                   the 17 #[tauri::command] functions
```

---

## 5. Critical invariants — DO NOT BREAK

1. **PII is local-only.** Student names, IDs, grades, teacher notes live ONLY in
   SQLite (`Documents/AIGrader/aigrader.db`). Never send them to the cloud.
2. **AI credentials are RAM-only.** The ephemeral Vertex token lives in
   `AppState.creds` (`RwLock<Option<EphemeralCreds>>`), is `zeroize`d on drop, and
   is NEVER written to disk/SQLite. The *activation code* IS stored locally (needed
   so the silent heartbeat can re-validate) — that's intentional and distinct.
3. **Scrub before upload.** Any bytes/text leaving the machine must pass through
   `gdpr::scrub_image_*` or `gdpr::scrub_text` first (strips EXIF/author metadata,
   zero-width tracking chars).
4. **Never hold the DB mutex guard across `.await`.** In async commands, lock →
   read into owned values → drop guard → await AI → lock again to write. The
   existing pipeline commands follow this; keep it that way (Connection isn't Sync).
5. **"No Hostage" lock.** When locked, grading is blocked but the SQLite DB stays
   intact and "Open Local Student Data" must still work.

### Licensing gateway contract (the Cloud Function the user will build)
```
POST {endpoint}        body {"activation_code":"..."}   → activate
POST {endpoint}/pulse  body {"activation_code":"..."}   → heartbeat
both return: {"status":"active"|"expired"|"invalid", "access_token":"...", "expires_in":3600}
```
On `active`: store token in RAM, set `activated=true`, clear grace clock.
On `expired`/`invalid`: start the 7-day grace clock.
On network failure: only start grace once stale (last good heartbeat older than
the heartbeat interval).

---

## 6. What's LEFT TO DO

### ✅ AI VERIFIED LIVE (2026-06-15)
The Gemini integration is confirmed working end-to-end against the real API,
including through the actual Rust client (`cargo test -- --ignored
live_structured_grading`):
  * Structured grading returns the exact `GradeResult` schema, correct logic.
  * Image OCR transcribes verbatim (errors preserved).
Notes:
  * **Model = `gemini-2.5-flash`** in `.env` — it works on the **free tier**.
    `gemini-2.5-pro` returns free-tier `limit: 0`; switch `AIGRADER_MODEL` to it
    once the project has **billing/credits** (one-line change, no code).
  * Auth = `x-goog-api-key` header on
    `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`.
  * Key lives in `.env` as `AIGRADER_DEV_API_KEY` (dev injection) — ROTATE it; it
    was shared in chat. In prod the key comes from the licensing gateway instead.

### DONE — full teacher app build (matches master spec)
- [x] **Classes** + **Student profiles** (first/last, per-student folder, teacher
      notes auto-save) + **Student ledger** (grade history, average %).
- [x] **Multi-view UI**: Sidebar(classes) → ClassPanel(students+assignments) →
      StudentProfile / GradeFlow. (`Workspace.tsx` and friends.)
- [x] **Polymorphic grading** (Strategy): rubric template / grading prompt / both /
      neither (`ai::grading_strategy`).
- [x] **Multi-page submissions** (`submission_files` table; consensus OCR over all pages).
- [x] **Split-pane review** with editable `-X pts` badges + **double-click override**
      (`ReviewPane.tsx`); score recomputed live.
- [x] **Finalize**: auto-sync `final_score` to the submission + **file injection**
      `students/[Name]/Correction_[Assignment].md` (`export.rs`).
- [x] **Real drag-and-drop**, **MIME routing** (`detect_route`), **PDF** scrub
      (`lopdf`), `.heic` friendly error.
- [x] **/documentation** (`architecture.md`, `security_gdpr.md`); Repository/Strategy/
      Builder patterns; 10 unit tests + 1 live integration test.

### Remaining polish (not blocking)
- [ ] **LaTeX rendering** in PreFlight/Review — math is preserved as `$…$`/`$$…$$`
      text but not rendered with KaTeX yet.
- [ ] **Search / filtering** across students & submissions.
- [ ] **HEIC decode** (currently a friendly "convert to JPG" message).
- [ ] **gemini-2.5-pro** in prod (needs billing; free tier uses `gemini-2.5-flash`).
- [ ] **Inline anchoring** — review is split-pane (work | corrections list); true
      per-location highlight overlay is a future enhancement.

### Medium
- [ ] **Commit a baseline.** Nothing is committed yet. Recommend committing the
      verified state before adding features. (User had not yet given the go-ahead.)
- [ ] **Live end-to-end test** against a real Vertex endpoint + Cloud Function once
      the user provides them. The wire format in `ai.rs` targets the standard Vertex
      `:generateContent` REST shape; verify response parsing against a real response.
- [ ] **Student linking UI.** `students` table + `create_student`/`list_students`
      commands exist, but the UI never assigns a student to a submission
      (`submission.student_id` is always null right now).
- [ ] **Grade history / review UI.** Grades persist (`grades` + `inline_corrections`
      tables) but there's no screen to browse past results.

### Lower
- [ ] PDF/image preview in the pre-flight step (currently text-only).
- [ ] Word-level (vs line-level) editing in PreFlight if desired (spec mentions word/line;
      line-level is implemented).
- [ ] Real heartbeat resilience testing (clock manipulation / offline scenarios).

---

## 7. Gotchas already discovered (so you don't repeat them)

- **Tauri v2, not v1.** Uses `capabilities/*.json` permissions, `tauri.conf.json`
  v2 schema, and plugins `tauri-plugin-dialog` / `tauri-plugin-opener`. Don't add a
  `tauri/custom-protocol` feature — it doesn't exist in v2 (removed; caused an error).
- **Icons are required to compile.** `generate_context!` embeds them. Run
  `npm run icon` (generates from `app-icon.png`) before `cargo check`, or it fails.
  `src-tauri/icons/` is gitignored — regenerate after a fresh clone.
- **`app-icon.png`** at repo root is a generated 512×512 source icon (committed-ish;
  it's the seed for `npm run icon`). Regenerate with the Node snippet in git history
  if lost.
- **calamine 0.26** uses `Data` (not `DataType`) and `Data` implements `Display` —
  that's how `extract.rs` formats cells. `Range::used_cells()` yields `(row, col, &Data)`.
- **quick-xml 0.36** — default config does NOT trim text (what we want); don't call a
  `trim_text` setter (API churned across versions). Match on `e.local_name().as_ref()`.
- **`if/else` returning `&str` vs `&String`** doesn't unify — use `.as_str()`.
- **PowerShell working dir persists** between tool calls but `cd` errors don't reset
  it; prefer `--manifest-path <absolute>` for cargo to avoid `src-tauri/src-tauri`.
- The `general.rs`-style base64 lives in `gdpr.rs` (`base64_encode` + `ScrubbedPayload::base64()`);
  there's intentionally no `base64` crate dependency.

---

## 8. Quick reference — the 17 Tauri commands

`get_license_status`, `activate`, `run_heartbeat`, `get_data_paths`,
`open_local_student_data`, `detect_route`, `create_student`, `list_students`,
`create_assignment`, `list_assignments`, `create_submission`, `save_extraction`,
`confirm_verified_text`, `extract_submission`, `grade_submission`,
`save_grade_result`, `scrub_image`, `scrub_text`. (18 total)

All are typed in `src/lib/api.ts` and registered in `src-tauri/src/lib.rs`.
