// Typed wrappers around Tauri commands. The React app never calls `invoke`
// directly — it goes through this module so the command names + signatures
// live in one place.

import { invoke } from "@tauri-apps/api/core";
import type {
  Assignment,
  DataPaths,
  GradeResult,
  LicenseStatus,
  NewAssignment,
  ScrubResult,
  Student,
  Submission,
} from "./types";

// ── Licensing / boot ──────────────────────────────────────────────────────
export const getLicenseStatus = () =>
  invoke<LicenseStatus>("get_license_status");

export const activate = (code: string) =>
  invoke<LicenseStatus>("activate", { code });

export const runHeartbeat = () => invoke<LicenseStatus>("run_heartbeat");

export const getDataPaths = () => invoke<DataPaths>("get_data_paths");

export const openLocalStudentData = () =>
  invoke<void>("open_local_student_data");

// ── Students ──────────────────────────────────────────────────────────────
export const createStudent = (display_name: string, external_ref?: string) =>
  invoke<Student>("create_student", { displayName: display_name, externalRef: external_ref ?? null });

export const listStudents = () => invoke<Student[]>("list_students");

// ── Assignments ─────────────────────────────────────────────────────────────
export const createAssignment = (assignment: NewAssignment) =>
  invoke<Assignment>("create_assignment", { assignment });

export const listAssignments = () => invoke<Assignment[]>("list_assignments");

// ── Submissions / extractions ───────────────────────────────────────────────
export const createSubmission = (args: {
  assignmentId: number;
  studentId?: number | null;
  sourceRoute: "image" | "digital";
  originalPath: string;
  mimeType?: string | null;
}) =>
  invoke<Submission>("create_submission", {
    assignmentId: args.assignmentId,
    studentId: args.studentId ?? null,
    sourceRoute: args.sourceRoute,
    originalPath: args.originalPath,
    mimeType: args.mimeType ?? null,
  });

export const saveExtraction = (
  submissionId: number,
  markdown: string,
  status: string,
) => invoke<number>("save_extraction", { submissionId, markdown, status });

export const confirmVerifiedText = (
  extractionId: number,
  verifiedMarkdown: string,
) =>
  invoke<void>("confirm_verified_text", { extractionId, verifiedMarkdown });

// ── Grading ──────────────────────────────────────────────────────────────────
export const saveGradeResult = (submissionId: number, result: GradeResult) =>
  invoke<number>("save_grade_result", { submissionId, result });

// ── GDPR scrub (pre-upload) ───────────────────────────────────────────────────
export const scrubImage = (path: string) =>
  invoke<ScrubResult>("scrub_image", { path });

export const scrubText = (input: string) =>
  invoke<string>("scrub_text", { input });
