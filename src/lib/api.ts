// Typed wrappers around Tauri commands. The React app never calls `invoke`
// directly — it goes through this module so the command names + signatures
// live in one place.

import { invoke } from "@tauri-apps/api/core";
import type {
  Assignment,
  Class,
  DataPaths,
  ExtractionOutput,
  FinalizeOutput,
  GradeResult,
  LicenseStatus,
  NewAssignment,
  RouteInfo,
  Student,
  Submission,
  SubmissionStatus,
} from "./types";

type TauriWindow = Window & {
  __TAURI_INTERNALS__?: unknown;
};

function requireTauriRuntime() {
  if (
    typeof window === "undefined" ||
    !(window as TauriWindow).__TAURI_INTERNALS__
  ) {
    throw new Error(
      "This app must be opened through the Tauri desktop shell. Run `npm run dev` and use the native app window, not http://localhost:5173 in a browser.",
    );
  }
}

function tauriInvoke<T>(command: string, args?: Record<string, unknown>) {
  requireTauriRuntime();
  return invoke<T>(command, args);
}

// ── Licensing / boot ──────────────────────────────────────────────────────
export const getLicenseStatus = () =>
  tauriInvoke<LicenseStatus>("get_license_status");
export const activate = (code: string) =>
  tauriInvoke<LicenseStatus>("activate", { code });
export const runHeartbeat = () => tauriInvoke<LicenseStatus>("run_heartbeat");
export const getDataPaths = () => tauriInvoke<DataPaths>("get_data_paths");
export const openLocalStudentData = () =>
  tauriInvoke<void>("open_local_student_data");
export const openStudentFolder = (studentId: number) =>
  tauriInvoke<void>("open_student_folder", { studentId });

// ── Routing ──────────────────────────────────────────────────────────────
export const detectRoute = (path: string) =>
  tauriInvoke<RouteInfo>("detect_route", { path });

// ── Classes ────────────────────────────────────────────────────────────────
export const createClass = (name: string) =>
  tauriInvoke<Class>("create_class", { name });
export const listClasses = () => tauriInvoke<Class[]>("list_classes");
export const deleteClass = (id: number) =>
  tauriInvoke<void>("delete_class", { id });

// ── Students ──────────────────────────────────────────────────────────────
export const createStudent = (
  classId: number | null,
  firstName: string,
  lastName: string,
) =>
  tauriInvoke<Student>("create_student", {
    classId,
    firstName,
    lastName,
  });
export const listStudents = (classId?: number | null) =>
  tauriInvoke<Student[]>("list_students", { classId: classId ?? null });
export const getStudent = (id: number) =>
  tauriInvoke<Student>("get_student", { id });
export const updateStudentNotes = (id: number, notes: string) =>
  tauriInvoke<void>("update_student_notes", { id, notes });
export const deleteStudent = (id: number) =>
  tauriInvoke<void>("delete_student", { id });

// ── Assignments ─────────────────────────────────────────────────────────────
export const createAssignment = (assignment: NewAssignment) =>
  tauriInvoke<Assignment>("create_assignment", { assignment });
export const listAssignments = (classId?: number | null) =>
  tauriInvoke<Assignment[]>("list_assignments", { classId: classId ?? null });

// ── Submissions ───────────────────────────────────────────────────────────
export const createSubmission = (args: {
  assignmentId: number;
  studentId?: number | null;
  sourceRoute: "image" | "digital";
  fileType: string;
  pages: string[];
}) =>
  tauriInvoke<Submission>("create_submission", {
    assignmentId: args.assignmentId,
    studentId: args.studentId ?? null,
    sourceRoute: args.sourceRoute,
    fileType: args.fileType,
    pages: args.pages,
  });

export const listSubmissions = (args: {
  assignmentId?: number | null;
  studentId?: number | null;
  status?: SubmissionStatus | null;
}) =>
  tauriInvoke<Submission[]>("list_submissions", {
    assignmentId: args.assignmentId ?? null,
    studentId: args.studentId ?? null,
    status: args.status ?? null,
  });

export const getSubmission = (id: number) =>
  tauriInvoke<Submission>("get_submission", { id });

export const confirmVerifiedText = (
  submissionId: number,
  verifiedMarkdown: string,
) =>
  tauriInvoke<void>("confirm_verified_text", { submissionId, verifiedMarkdown });

// ── Pipelines ────────────────────────────────────────────────────────────────
export const extractSubmission = (submissionId: number) =>
  tauriInvoke<ExtractionOutput>("extract_submission", { submissionId });

export const gradeSubmission = (submissionId: number) =>
  tauriInvoke<GradeResult>("grade_submission", { submissionId });

export const finalizeGrade = (submissionId: number, result: GradeResult) =>
  tauriInvoke<FinalizeOutput>("finalize_grade", { submissionId, result });
