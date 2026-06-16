import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import * as api from "../lib/api";
import type { Assignment, GradeResult, RoutedFile } from "../lib/types";
import AssignmentForm from "./AssignmentForm";
import PreFlight from "./PreFlight";
import ReviewPane from "./ReviewPane";

type Phase =
  | { step: "setup" }
  | { step: "extracting" }
  | { step: "preflight"; markdown: string }
  | { step: "grading"; markdown: string }
  | { step: "review"; result: GradeResult }
  | { step: "error"; message: string };

export default function GradeFlow({
  classId,
  studentId,
  onDone,
}: {
  classId: number | null;
  studentId: number | null;
  onDone: () => void;
}) {
  const [assignments, setAssignments] = useState<Assignment[]>([]);
  const [assignmentId, setAssignmentId] = useState<number | null>(null);
  const [showNew, setShowNew] = useState(false);
  const [files, setFiles] = useState<RoutedFile[]>([]);
  const [rejected, setRejected] = useState<string[]>([]);
  const [phase, setPhase] = useState<Phase>({ step: "setup" });
  const [dragging, setDragging] = useState(false);
  const [verifiedText, setVerifiedText] = useState("");
  const submissionRef = useRef<number | null>(null);

  async function reloadAssignments() {
    const list = await api.listAssignments(classId);
    setAssignments(list);
    setAssignmentId((prev) => prev ?? list[0]?.id ?? null);
  }
  useEffect(() => {
    void reloadAssignments();
  }, [classId]);

  // Native drag-and-drop while in setup.
  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (phase.step !== "setup") return;
      if (event.payload.type === "over" || event.payload.type === "enter") {
        setDragging(true);
      } else if (event.payload.type === "drop") {
        setDragging(false);
        void ingest(event.payload.paths);
      } else {
        setDragging(false);
      }
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, [phase.step]);

  async function ingest(paths: string[]) {
    const routed = await Promise.all(
      paths.map(async (p) => {
        const name = p.split(/[\\/]/).pop() ?? p;
        try {
          const info = await api.detectRoute(p);
          return { p, name, info };
        } catch {
          return { p, name, info: null };
        }
      }),
    );
    const ok: RoutedFile[] = [];
    const bad: string[] = [];
    for (const r of routed) {
      if (r.info && (r.info.route === "image" || r.info.route === "digital")) {
        ok.push({ path: r.p, name: r.name, route: r.info.route, mime: r.info.mime_type });
      } else bad.push(r.name);
    }
    if (ok.length) setFiles((prev) => [...prev, ...ok]);
    setRejected(bad);
  }

  async function pick() {
    const sel = await open({
      multiple: true,
      filters: [
        {
          name: "Assignments",
          extensions: ["jpg", "jpeg", "png", "heic", "pdf", "docx", "xlsx", "pptx", "txt"],
        },
      ],
    });
    if (!sel) return;
    await ingest(Array.isArray(sel) ? sel : [sel]);
  }

  async function processAndGrade() {
    if (assignmentId == null || files.length === 0) return;
    setPhase({ step: "extracting" });
    try {
      const route = files[0].route;
      const fileType = files[0].mime ?? files[0].name.split(".").pop() ?? "";
      const submission = await api.createSubmission({
        assignmentId,
        studentId,
        sourceRoute: route,
        fileType,
        pages: files.map((f) => f.path),
      });
      submissionRef.current = submission.id;
      const extraction = await api.extractSubmission(submission.id);
      setPhase({ step: "preflight", markdown: extraction.markdown });
    } catch (e) {
      setPhase({ step: "error", message: String(e) });
    }
  }

  async function confirmAndGrade(verified: string) {
    const id = submissionRef.current;
    if (id == null) return;
    setVerifiedText(verified);
    setPhase({ step: "grading", markdown: verified });
    try {
      await api.confirmVerifiedText(id, verified);
      const result = await api.gradeSubmission(id);
      setPhase({ step: "review", result });
    } catch (e) {
      setPhase({ step: "error", message: String(e) });
    }
  }

  async function finalize(edited: GradeResult) {
    const id = submissionRef.current;
    if (id == null) return;
    await api.finalizeGrade(id, edited);
    onDone();
  }

  const assignment = assignments.find((a) => a.id === assignmentId) ?? null;
  const imageCount = files.filter((f) => f.route === "image").length;
  const digitalCount = files.filter((f) => f.route === "digital").length;

  return (
    <div className="mx-auto max-w-4xl">
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-lg font-semibold text-slate-800">Grade submission</h1>
        <button onClick={onDone} className="text-sm text-slate-500 hover:text-slate-700">
          ✕ Close
        </button>
      </div>

      {phase.step === "setup" && (
        <div className="space-y-4">
          <div className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200">
            <label className="mb-1 block text-xs font-medium text-slate-500">
              Assignment
            </label>
            <div className="flex gap-2">
              <select
                value={assignmentId ?? ""}
                onChange={(e) => setAssignmentId(Number(e.target.value))}
                className="input"
              >
                {assignments.length === 0 && <option value="">No assignments yet</option>}
                {assignments.map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.title} — {a.subject} · {a.education_level}
                  </option>
                ))}
              </select>
              <button
                onClick={() => setShowNew((v) => !v)}
                className="shrink-0 rounded-lg border border-slate-300 px-3 text-sm text-slate-600 hover:bg-slate-50"
              >
                ＋ New
              </button>
            </div>
          </div>

          {showNew && (
            <AssignmentForm
              classId={classId}
              onCreated={(a) => {
                setShowNew(false);
                setAssignments((prev) => [a, ...prev]);
                setAssignmentId(a.id);
              }}
              onCancel={() => setShowNew(false)}
            />
          )}

          <button
            onClick={pick}
            className={
              "flex w-full flex-col items-center justify-center rounded-2xl border-2 border-dashed p-10 text-center transition " +
              (dragging
                ? "border-brand-500 bg-brand-50"
                : "border-slate-300 bg-white/60 hover:border-brand-400 hover:bg-brand-50/40")
            }
          >
            <div className="text-4xl">📥</div>
            <p className="mt-3 text-lg font-medium text-slate-700">
              {dragging ? "Release to add pages" : "Drop or choose the student's pages"}
            </p>
            <p className="mt-1 max-w-md text-sm text-slate-500">
              Add multiple photos for a multi-page exam, or a Word/Excel/PowerPoint
              file. We detect the type and route it automatically.
            </p>
          </button>

          {files.length > 0 && (
            <div className="rounded-2xl bg-white p-4 shadow-sm ring-1 ring-slate-200">
              <div className="mb-2 flex items-center justify-between text-sm">
                <span className="font-semibold text-slate-600">
                  {files.length} page{files.length === 1 ? "" : "s"} queued
                  {imageCount > 0 && digitalCount > 0 ? " (mixed types)" : ""}
                </span>
                <button
                  onClick={() => {
                    setFiles([]);
                    setRejected([]);
                  }}
                  className="text-xs text-slate-400 hover:text-slate-600"
                >
                  Clear
                </button>
              </div>
              <ul className="space-y-1.5 text-sm">
                {files.map((f, i) => (
                  <li key={i} className="flex items-center gap-2">
                    <span
                      className={
                        "rounded-full px-2 py-0.5 text-xs " +
                        (f.route === "image"
                          ? "bg-indigo-50 text-indigo-600"
                          : "bg-emerald-50 text-emerald-600")
                      }
                    >
                      {f.route === "image" ? "Vision" : "Digital"}
                    </span>
                    <span className="truncate text-slate-700">{f.name}</span>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {rejected.length > 0 && (
            <p className="text-xs text-red-500">Unsupported: {rejected.join(", ")}</p>
          )}

          <div className="flex justify-end">
            <button
              onClick={processAndGrade}
              disabled={assignmentId == null || files.length === 0}
              className="btn-primary"
            >
              Process &amp; Grade →
            </button>
          </div>
        </div>
      )}

      {phase.step === "extracting" && (
        <Centered>
          <Spinner />
          <p className="mt-3 text-sm text-slate-500">
            {files[0]?.route === "image"
              ? "Running multi-pass handwriting recognition…"
              : "Extracting document content…"}
          </p>
        </Centered>
      )}

      {(phase.step === "preflight" || phase.step === "grading") && (
        <PreFlight
          markdown={phase.markdown}
          busy={phase.step === "grading"}
          onBack={onDone}
          onConfirm={confirmAndGrade}
        />
      )}

      {phase.step === "review" && assignment && (
        <ReviewPane
          result={phase.result}
          maxScore={assignment.max_score}
          studentText={verifiedText}
          onFinalize={finalize}
        />
      )}

      {phase.step === "error" && (
        <Centered>
          <p className="max-w-md text-center text-sm text-red-600">{phase.message}</p>
          <button
            onClick={() => setPhase({ step: "setup" })}
            className="mt-4 rounded-lg border border-slate-300 px-4 py-2 text-sm text-slate-600 hover:bg-slate-50"
          >
            Back
          </button>
        </Centered>
      )}
    </div>
  );
}

function Centered({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center py-20">{children}</div>
  );
}

function Spinner() {
  return (
    <div className="h-8 w-8 animate-spin rounded-full border-2 border-slate-200 border-t-brand-500" />
  );
}
