// TypeScript mirrors of the Rust domain models (src-tauri/src/db/models.rs)
// and command outputs. Keep in sync with the backend.

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

export interface Class {
  id: number;
  name: string;
  created_at: string;
}

export interface Student {
  id: number;
  class_id: number | null;
  first_name: string;
  last_name: string;
  local_folder_path: string;
  teacher_notes: string;
  created_at: string;
}

export function studentName(s: Student): string {
  return `${s.first_name} ${s.last_name}`.trim();
}

export type EducationLevel = "Elementary" | "High School" | "University";

export interface NewAssignment {
  class_id: number | null;
  title: string;
  subject: string;
  education_level: EducationLevel | string;
  custom_persona?: string;
  rubric_template?: string;
  grading_prompt?: string;
  max_score?: number;
}

export interface Assignment {
  id: number;
  class_id: number | null;
  title: string;
  subject: string;
  education_level: string;
  custom_persona: string;
  rubric_template: string;
  grading_prompt: string;
  max_score: number;
  created_at: string;
}

export type SourceRoute = "image" | "digital";

export interface RoutedFile {
  path: string;
  name: string;
  route: SourceRoute;
  mime?: string;
}

export interface RouteInfo {
  route: SourceRoute | "unsupported";
  mime_type: string;
}

export type SubmissionStatus = "ungraded" | "verified" | "graded";

export interface Submission {
  id: number;
  assignment_id: number;
  student_id: number | null;
  source_route: SourceRoute;
  file_type: string;
  status: SubmissionStatus;
  extracted_markdown: string;
  verified_markdown: string | null;
  evaluation_json: string | null;
  final_score: number | null;
  local_output_path: string | null;
  created_at: string;
}

// Structured grading schema (spec §Phase 4).
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

export interface ExtractionOutput {
  markdown: string;
  route: SourceRoute;
}

export interface FinalizeOutput {
  final_score: number;
  output_path: string | null;
}

// Fallback extension router (the backend detect_route is authoritative).
const IMAGE_EXTS = ["jpg", "jpeg", "png", "heic", "pdf"];
const DIGITAL_EXTS = ["docx", "xlsx", "pptx", "txt"];

export function routeForFile(name: string): SourceRoute | null {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  if (IMAGE_EXTS.includes(ext)) return "image";
  if (DIGITAL_EXTS.includes(ext)) return "digital";
  return null;
}
