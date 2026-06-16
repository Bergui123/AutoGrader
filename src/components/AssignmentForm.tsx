import { useState } from "react";
import * as api from "../lib/api";
import type { Assignment, EducationLevel } from "../lib/types";

const SUBJECTS = ["Math", "History", "French", "Science", "English", "Other"];
const LEVELS: EducationLevel[] = ["Elementary", "High School", "University"];

/**
 * Dynamic Persona Factory (§Phase 1) + Polymorphic grading inputs (§Phase 4):
 * the teacher can supply a rubric template, a grading prompt, or both.
 */
export default function AssignmentForm({
  classId,
  onCreated,
  onCancel,
}: {
  classId: number | null;
  onCreated: (a: Assignment) => void;
  onCancel?: () => void;
}) {
  const [title, setTitle] = useState("");
  const [subject, setSubject] = useState(SUBJECTS[0]);
  const [level, setLevel] = useState<EducationLevel>("High School");
  const [persona, setPersona] = useState("");
  const [rubric, setRubric] = useState("");
  const [prompt, setPrompt] = useState("");
  const [maxScore, setMaxScore] = useState(100);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const a = await api.createAssignment({
        class_id: classId,
        title: title.trim() || `${subject} assignment`,
        subject,
        education_level: level,
        custom_persona: persona,
        rubric_template: rubric,
        grading_prompt: prompt,
        max_score: Number(maxScore) || 100,
      });
      onCreated(a);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  const personaPreview = `You are a professional ${level} ${subject} teacher.${
    persona ? " " + persona : ""
  }`;

  return (
    <form
      onSubmit={submit}
      className="space-y-3 rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200"
    >
      <div className="grid gap-3 sm:grid-cols-2">
        <Field label="Assignment title">
          <input
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="e.g. Algebra Midterm"
            className="input"
          />
        </Field>
        <Field label="Max score">
          <input
            type="number"
            value={maxScore}
            onChange={(e) => setMaxScore(Number(e.target.value))}
            className="input"
          />
        </Field>
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
        <Field label="Education level">
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

      <Field label="Custom persona notes (optional)">
        <input
          value={persona}
          onChange={(e) => setPersona(e.target.value)}
          placeholder="e.g. Be encouraging but precise."
          className="input"
        />
      </Field>

      <div className="grid gap-3 sm:grid-cols-2">
        <Field label="Rubric / point template (optional)">
          <textarea
            value={rubric}
            onChange={(e) => setRubric(e.target.value)}
            rows={4}
            placeholder={"Q1 (5 pts): ...\nQ2 (10 pts): ..."}
            className="input resize-none font-mono text-xs"
          />
        </Field>
        <Field label="Grading prompt (optional)">
          <textarea
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            rows={4}
            placeholder="e.g. Penalize missing steps; be lenient on arithmetic slips."
            className="input resize-none text-xs"
          />
        </Field>
      </div>

      <p className="rounded-lg bg-slate-50 p-2 text-xs text-slate-500">
        <span className="font-medium text-slate-600">Persona: </span>
        {personaPreview}
      </p>
      <p className="text-xs text-slate-400">
        Provide a rubric, a prompt, or both — the grader adapts automatically.
      </p>

      {error && <p className="text-sm text-red-600">{error}</p>}

      <div className="flex justify-end gap-2">
        {onCancel && (
          <button
            type="button"
            onClick={onCancel}
            className="rounded-xl border border-slate-300 px-4 py-2 text-sm text-slate-600 hover:bg-slate-50"
          >
            Cancel
          </button>
        )}
        <button type="submit" disabled={busy} className="btn-primary">
          {busy ? "Saving…" : "Save assignment"}
        </button>
      </div>
    </form>
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
