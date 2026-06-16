import { useEffect, useState } from "react";
import * as api from "../lib/api";
import { studentName, type Assignment, type Student, type Submission } from "../lib/types";

export default function StudentProfile({
  studentId,
  onBack,
  onGrade,
}: {
  studentId: number;
  onBack: () => void;
  onGrade: (studentId: number) => void;
}) {
  const [student, setStudent] = useState<Student | null>(null);
  const [submissions, setSubmissions] = useState<Submission[]>([]);
  const [titles, setTitles] = useState<Record<number, Assignment>>({});
  const [notes, setNotes] = useState("");
  const [savedNote, setSavedNote] = useState("");

  useEffect(() => {
    void (async () => {
      const s = await api.getStudent(studentId);
      setStudent(s);
      setNotes(s.teacher_notes);
      setSavedNote(s.teacher_notes);
      const subs = await api.listSubmissions({ studentId });
      setSubmissions(subs);
      const assigns = await api.listAssignments(null);
      setTitles(Object.fromEntries(assigns.map((a) => [a.id, a])));
    })();
  }, [studentId]);

  // Auto-save notes 800ms after the teacher stops typing.
  useEffect(() => {
    if (notes === savedNote) return;
    const t = setTimeout(async () => {
      await api.updateStudentNotes(studentId, notes);
      setSavedNote(notes);
    }, 800);
    return () => clearTimeout(t);
  }, [notes, savedNote, studentId]);

  if (!student) return <p className="text-sm text-slate-400">Loading…</p>;

  const graded = submissions.filter((s) => s.status === "graded");
  const avg =
    graded.length > 0
      ? graded.reduce((sum, s) => {
          const max = titles[s.assignment_id]?.max_score ?? 100;
          return sum + ((s.final_score ?? 0) / max) * 100;
        }, 0) / graded.length
      : null;

  return (
    <div className="mx-auto max-w-3xl space-y-6">
      <div className="flex items-center justify-between">
        <button onClick={onBack} className="text-sm text-slate-500 hover:text-slate-700">
          ← Back to class
        </button>
        <div className="flex gap-2">
          <button
            onClick={() => void api.openStudentFolder(studentId)}
            className="rounded-lg border border-slate-300 px-3 py-1.5 text-sm text-slate-600 hover:bg-slate-50"
          >
            📂 Open folder
          </button>
          <button onClick={() => onGrade(studentId)} className="btn-primary px-3 py-1.5 text-sm">
            Grade new submission
          </button>
        </div>
      </div>

      <div className="rounded-2xl bg-white p-6 shadow-sm ring-1 ring-slate-200">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-semibold text-slate-800">
            {studentName(student)}
          </h1>
          {avg != null && (
            <div className="text-right">
              <div className="text-xs uppercase tracking-wide text-slate-400">Average</div>
              <div className="text-2xl font-bold text-slate-800">{Math.round(avg)}%</div>
            </div>
          )}
        </div>
      </div>

      <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200">
        <h2 className="mb-2 text-sm font-semibold uppercase tracking-wide text-slate-500">
          Teacher notes
        </h2>
        <textarea
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          rows={3}
          placeholder="Private notes about this student…"
          className="input resize-none"
        />
        <p className="mt-1 text-right text-xs text-slate-400">
          {notes === savedNote ? "Saved" : "Saving…"}
        </p>
      </section>

      <section>
        <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-slate-500">
          Grade ledger
        </h2>
        {submissions.length === 0 ? (
          <div className="rounded-xl border border-dashed border-slate-300 p-6 text-center text-sm text-slate-400">
            No submissions yet.
          </div>
        ) : (
          <ul className="space-y-2">
            {submissions.map((s) => {
              const a = titles[s.assignment_id];
              const max = a?.max_score ?? 100;
              return (
                <li
                  key={s.id}
                  className="flex items-center justify-between rounded-xl bg-white p-4 text-sm shadow-sm ring-1 ring-slate-200"
                >
                  <div className="min-w-0">
                    <div className="truncate font-medium text-slate-700">
                      {a?.title ?? `Assignment #${s.assignment_id}`}
                    </div>
                    <div className="text-xs text-slate-400">
                      {s.file_type || s.source_route} · {new Date(s.created_at).toLocaleDateString()}
                    </div>
                  </div>
                  <div className="flex items-center gap-3">
                    <StatusBadge status={s.status} />
                    {s.final_score != null && (
                      <span className="font-semibold text-slate-800">
                        {s.final_score}/{max}
                      </span>
                    )}
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </section>
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const map: Record<string, string> = {
    graded: "bg-emerald-50 text-emerald-700",
    verified: "bg-amber-50 text-amber-700",
    ungraded: "bg-slate-100 text-slate-500",
  };
  return (
    <span className={"rounded-full px-2 py-0.5 text-xs " + (map[status] ?? map.ungraded)}>
      {status}
    </span>
  );
}
