import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import * as api from "../lib/api";
import {
  routeForFile,
  type Assignment,
  type EducationLevel,
  type LicenseStatus,
  type SourceRoute,
} from "../lib/types";

const SUBJECTS = ["Math", "History", "French", "Science", "English", "Other"];
const LEVELS: EducationLevel[] = ["Elementary", "High School", "University"];

type RoutedFile = { path: string; name: string; route: SourceRoute };

export default function AppShell({ status }: { status: LicenseStatus }) {
  const aiReady = status.state === "active" || status.state === "grace"
    ? status.ai_ready
    : false;

  return (
    <div className="mx-auto flex h-full max-w-5xl flex-col">
      <Header aiReady={aiReady} />
      <main className="grid flex-1 gap-6 overflow-auto p-6 lg:grid-cols-[360px_1fr]">
        <PersonaFactory />
        <Dropzone />
      </main>
    </div>
  );
}

function Header({ aiReady }: { aiReady: boolean }) {
  return (
    <header className="flex items-center justify-between border-b border-slate-200 bg-white/70 px-6 py-3 backdrop-blur">
      <div className="flex items-center gap-2">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-brand-500 to-violet-600 text-white">
          ✓
        </div>
        <span className="font-semibold text-slate-800">AI Grader</span>
      </div>
      <span
        className={
          "rounded-full px-3 py-1 text-xs font-medium " +
          (aiReady
            ? "bg-emerald-50 text-emerald-700"
            : "bg-slate-100 text-slate-500")
        }
        title={aiReady ? "AI credentials loaded (in memory)" : "AI not connected"}
      >
        {aiReady ? "AI ready" : "AI offline"}
      </span>
    </header>
  );
}

/** Phase 1: Dynamic Persona Factory inputs that drive the System Instruction. */
function PersonaFactory() {
  const [title, setTitle] = useState("");
  const [subject, setSubject] = useState(SUBJECTS[0]);
  const [level, setLevel] = useState<EducationLevel>("High School");
  const [instructions, setInstructions] = useState("");
  const [assignments, setAssignments] = useState<Assignment[]>([]);
  const [error, setError] = useState<string | null>(null);

  async function reload() {
    try {
      setAssignments(await api.listAssignments());
    } catch (e) {
      setError(String(e));
    }
  }
  useEffect(() => {
    void reload();
  }, []);

  async function create(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    try {
      await api.createAssignment({
        title: title.trim() || `${subject} assignment`,
        subject,
        education_level: level,
        custom_instructions: instructions,
      });
      setTitle("");
      setInstructions("");
      await reload();
    } catch (e) {
      setError(String(e));
    }
  }

  const personaPreview = `You are a professional ${level} ${subject} teacher. Grade strictly based on the provided rubric.${
    instructions ? " " + instructions : ""
  }`;

  return (
    <section className="space-y-4">
      <div className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200">
        <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-slate-500">
          Grading persona
        </h2>
        <form onSubmit={create} className="space-y-3">
          <Field label="Assignment title">
            <input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="e.g. Algebra Quiz 3"
              className="input"
            />
          </Field>
          <div className="grid grid-cols-2 gap-3">
            <Field label="Subject">
              <select
                value={subject}
                onChange={(e) => setSubject(e.target.value)}
                className="input"
              >
                {SUBJECTS.map((s) => (
                  <option key={s}>{s}</option>
                ))}
              </select>
            </Field>
            <Field label="Level">
              <select
                value={level}
                onChange={(e) => setLevel(e.target.value as EducationLevel)}
                className="input"
              >
                {LEVELS.map((l) => (
                  <option key={l}>{l}</option>
                ))}
              </select>
            </Field>
          </div>
          <Field label="Custom instructions">
            <textarea
              value={instructions}
              onChange={(e) => setInstructions(e.target.value)}
              rows={3}
              placeholder="Optional grading rules…"
              className="input resize-none"
            />
          </Field>

          <div className="rounded-lg bg-slate-50 p-3 text-xs text-slate-500">
            <span className="font-medium text-slate-600">System prompt: </span>
            {personaPreview}
          </div>

          {error && <p className="text-sm text-red-600">{error}</p>}

          <button type="submit" className="btn-primary w-full">
            Save assignment
          </button>
        </form>
      </div>

      {assignments.length > 0 && (
        <div className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200">
          <h3 className="mb-2 text-sm font-semibold text-slate-600">
            Recent assignments
          </h3>
          <ul className="space-y-1 text-sm text-slate-600">
            {assignments.slice(0, 6).map((a) => (
              <li key={a.id} className="flex justify-between">
                <span className="truncate">{a.title}</span>
                <span className="ml-2 shrink-0 text-slate-400">
                  {a.subject} · {a.education_level}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}

/** Spec §5: intelligent dropzone that auto-routes by file type. */
function Dropzone() {
  const [files, setFiles] = useState<RoutedFile[]>([]);
  const [rejected, setRejected] = useState<string[]>([]);

  async function pickFiles() {
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: "Assignments",
          extensions: ["jpg", "jpeg", "png", "heic", "pdf", "docx", "xlsx", "pptx", "txt"],
        },
      ],
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    ingest(paths);
  }

  function ingest(paths: string[]) {
    const ok: RoutedFile[] = [];
    const bad: string[] = [];
    for (const p of paths) {
      const name = p.split(/[\\/]/).pop() ?? p;
      const route = routeForFile(name);
      if (route) ok.push({ path: p, name, route });
      else bad.push(name);
    }
    setFiles((prev) => [...prev, ...ok]);
    setRejected(bad);
  }

  const imageCount = files.filter((f) => f.route === "image").length;
  const digitalCount = files.filter((f) => f.route === "digital").length;

  return (
    <section className="flex flex-col gap-4">
      <button
        onClick={pickFiles}
        className="flex flex-1 flex-col items-center justify-center rounded-2xl border-2 border-dashed border-slate-300 bg-white/60 p-10 text-center transition hover:border-brand-400 hover:bg-brand-50/40"
      >
        <div className="text-4xl">📥</div>
        <p className="mt-3 text-lg font-medium text-slate-700">
          Drop or choose student work
        </p>
        <p className="mt-1 max-w-sm text-sm text-slate-500">
          Photos &amp; PDFs go to the handwriting vision pipeline. Word, Excel,
          PowerPoint &amp; text go to the digital extractor. We pick
          automatically — no settings needed.
        </p>
      </button>

      {(imageCount > 0 || digitalCount > 0) && (
        <div className="grid grid-cols-2 gap-3 text-sm">
          <RouteCard
            label="Handwritten / scanned"
            sub="Multi-pass vision pipeline"
            count={imageCount}
          />
          <RouteCard
            label="Digital documents"
            sub="Native extraction pipeline"
            count={digitalCount}
          />
        </div>
      )}

      {files.length > 0 && (
        <ul className="space-y-1 rounded-2xl bg-white p-4 text-sm shadow-sm ring-1 ring-slate-200">
          {files.map((f, i) => (
            <li key={i} className="flex items-center justify-between">
              <span className="truncate text-slate-700">{f.name}</span>
              <span
                className={
                  "ml-2 shrink-0 rounded-full px-2 py-0.5 text-xs " +
                  (f.route === "image"
                    ? "bg-indigo-50 text-indigo-600"
                    : "bg-emerald-50 text-emerald-600")
                }
              >
                {f.route === "image" ? "Vision" : "Digital"}
              </span>
            </li>
          ))}
        </ul>
      )}

      {rejected.length > 0 && (
        <p className="text-xs text-red-500">
          Unsupported: {rejected.join(", ")}
        </p>
      )}
    </section>
  );
}

function RouteCard({
  label,
  sub,
  count,
}: {
  label: string;
  sub: string;
  count: number;
}) {
  return (
    <div className="rounded-xl bg-white p-4 shadow-sm ring-1 ring-slate-200">
      <div className="text-2xl font-semibold text-slate-800">{count}</div>
      <div className="font-medium text-slate-600">{label}</div>
      <div className="text-xs text-slate-400">{sub}</div>
    </div>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <label className="block">
      <span className="mb-1 block text-xs font-medium text-slate-500">
        {label}
      </span>
      {children}
    </label>
  );
}
