// TypeScript mirrors of the Rust domain models (src-tauri/src/db/models.rs)
// and the licensing status enum (src-tauri/src/licensing.rs). Keep in sync.

export type LicenseStatus =
  | { state: "unactivated" }
  | { state: "active"; ai_ready: boolean }
  | { state: "grace"; days_left: number; ai_ready: boolean }
  | { state: "locked" };

export interface DataPaths {
  data_root: string;
  students_dir: string;
  db_path: string;
}

export interface Student {
  id: number;
  display_name: string;
  external_ref: string | null;
  created_at: string;
}

export type EducationLevel = "Elementary" | "High School" | "University";

export interface NewAssignment {
  title: string;
  subject: string;
  education_level: EducationLevel | string;
  custom_instructions?: string;
  rubric?: string;
  max_score?: number;
}

export interface Assignment extends Required<NewAssignment> {
  id: number;
  created_at: string;
}

export type SourceRoute = "image" | "digital";

export interface Submission {
  id: number;
  assignment_id: number;
  student_id: number | null;
  source_route: SourceRoute;
  original_path: string;
  mime_type: string | null;
  created_at: string;
}

// The exact structured grading schema the AI must return (spec §Phase 4).
export interface InlineCorrection {
  id?: number;
  location_reference: string;
  correction_comment: string;
  points_deducted: number;
}

export interface GradeSummary {
  general_feedback: string;
  total_points_deducted: number;
  final_score: number;
}

export interface GradeResult {
  inline_corrections: InlineCorrection[];
  summary: GradeSummary;
}

export interface ScrubResult {
  base64: string;
  mime_type: string;
  byte_len: number;
}

// Dropzone auto-router (spec §5). Decides the pipeline from the extension.
const IMAGE_EXTS = ["jpg", "jpeg", "png", "heic", "pdf"];
const DIGITAL_EXTS = ["docx", "xlsx", "pptx", "txt"];

export function routeForFile(name: string): SourceRoute | null {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  if (IMAGE_EXTS.includes(ext)) return "image";
  if (DIGITAL_EXTS.includes(ext)) return "digital";
  return null;
}
