# Architecture

AI Grader is a **local-first desktop app**: a Tauri (Rust) shell hosting a React
+ TypeScript UI, with all data in a local SQLite database. Google Gemini is used
only as a stateless inference service (OCR + grading).

## Layers

```
┌─────────────────────────────────────────────────────────────┐
│ React + TypeScript (src/)                                     │
│   components/  — Workspace, ClassPanel, StudentProfile,       │
│                  GradeFlow, PreFlight, ReviewPane, licensing  │
│   lib/api.ts   — typed wrappers over every Tauri command      │
│   lib/types.ts — mirrors of the Rust models                   │
└───────────────▲───────────────────────────────────────────────┘
                │ Tauri command bridge (invoke)
┌───────────────┴───────────────────────────────────────────────┐
│ Rust backend (src-tauri/src/)                                  │
│   commands.rs — the command surface (the ONLY bridge)          │
│   db/         — Repository pattern over SQLite                 │
│   ai.rs       — Gemini client (Strategy: persona + grading)    │
│   extract.rs  — native .docx/.xlsx/.pptx/.txt parsing          │
│   gdpr.rs     — pre-upload scrub (EXIF / metadata)             │
│   export.rs   — Builder: Correction_*.md into student folders  │
│   licensing.rs— activation, heartbeat, grace, lock             │
│   state.rs    — DB handle + RAM-only ephemeral AI key          │
└───────────────▲───────────────────────────────────────────────┘
                │ HTTPS
        ┌───────┴────────┐         ┌──────────────────────────────┐
        │ Licensing       │         │ Gemini (generativelanguage   │
        │ Cloud Function  │         │ .googleapis.com)             │
        │ (returns API key)│        │  - OCR consensus             │
        └─────────────────┘         │  - structured grading        │
                                     └──────────────────────────────┘
```

## Data flow (a grading run)

1. **Setup** — teacher picks a Class → Student → Assignment (Persona Factory:
   subject, level, rubric template and/or grading prompt).
2. **Ingest** — files dropped into `GradeFlow`. `detect_route` classifies each by
   magic bytes + extension → `image` (vision) or `digital` (native parse).
3. **Submission** — `create_submission` stores the row + one `submission_files`
   row per page (multi-page).
4. **Extract** (`extract_submission`) — images are scrubbed (`gdpr`) and run
   through the 3-pass consensus OCR (`ai`); digital files are parsed (`extract`).
   Output saved to `submissions.extracted_markdown`.
5. **Pre-flight** — `PreFlight` shows the text; double-click a line to fix OCR
   errors; `confirm_verified_text` stores `verified_markdown`.
6. **Grade** (`grade_submission`) — `ai.grade` builds the persona + polymorphic
   grading strategy and forces the structured-JSON schema → `GradeResult`.
7. **Review** — `ReviewPane` shows student work | inline `-X pts` badges; the
   teacher double-clicks to override comments/points; score recomputed live.
8. **Finalize** (`finalize_grade`) — persists `evaluation_json` + `final_score`,
   sets status `graded`, and (`export`) writes
   `students/[Name]/Correction_[Assignment].md`.

## Design patterns

- **Repository** — `db/repo.rs`: all SQL behind plain functions over `&Connection`.
- **Strategy** — `ai::grading_strategy`: rubric-only / prompt-only / both / neither.
- **Builder/Composite** — `export::build_markdown` composes the correction doc;
  `submission_files` composes multi-page papers into one submission.

## Command bridge

`commands.rs` is the only place React reaches Rust. Each command returns
`Result<T, AppError>` (AppError serializes to a string). DB mutex guards are
always dropped before any `.await` so async commands stay `Send`.
