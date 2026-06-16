import { useEffect, useState } from "react";
import * as api from "../lib/api";
import { studentName, type Assignment, type Student } from "../lib/types";
import AssignmentForm from "./AssignmentForm";

export default function ClassPanel({
  classId,
  onOpenStudent,
  onGradeStudent,
}: {
  classId: number | null;
  onOpenStudent: (studentId: number) => void;
  onGradeStudent: (studentId: number) => void;
}) {
  const [students, setStudents] = useState<Student[]>([]);
  const [assignments, setAssignments] = useState<Assignment[]>([]);
  const [showAssignment, setShowAssignment] = useState(false);

  async function reload() {
    if (classId == null) {
      setStudents([]);
      setAssignments([]);
      return;
    }
    const [s, a] = await Promise.all([
      api.listStudents(classId),
      api.listAssignments(classId),
    ]);
    setStudents(s);
    setAssignments(a);
  }
  useEffect(() => {
    void reload();
  }, [classId]);

  if (classId == null) {
    return (
      <Empty>
        Create or select a class on the left to add students and assignments.
      </Empty>
    );
  }

  return (
    <div className="space-y-8">
      <section>
        <SectionHeader title="Students">
          <AddStudent classId={classId} onAdded={reload} />
        </SectionHeader>
        {students.length === 0 ? (
          <Empty>No students yet. Add one above.</Empty>
        ) : (
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {students.map((s) => (
              <div
                key={s.id}
                className="flex items-center justify-between rounded-xl bg-white p-4 shadow-sm ring-1 ring-slate-200"
              >
                <button
                  onClick={() => onOpenStudent(s.id)}
                  className="truncate text-left font-medium text-slate-700 hover:text-brand-700"
                >
                  {studentName(s)}
                </button>
                <button
                  onClick={() => onGradeStudent(s.id)}
                  className="btn-primary shrink-0 px-3 py-1.5 text-xs"
                >
                  Grade →
                </button>
              </div>
            ))}
          </div>
        )}
      </section>

      <section>
        <SectionHeader title="Assignments">
          <button
            onClick={() => setShowAssignment((v) => !v)}
            className="btn-primary px-3 py-1.5 text-xs"
          >
            ＋ New assignment
          </button>
        </SectionHeader>
        {showAssignment && (
          <div className="mb-4">
            <AssignmentForm
              classId={classId}
              onCreated={() => {
                setShowAssignment(false);
                void reload();
              }}
              onCancel={() => setShowAssignment(false)}
            />
          </div>
        )}
        {assignments.length === 0 ? (
          <Empty>No assignments yet.</Empty>
        ) : (
          <ul className="space-y-2">
            {assignments.map((a) => (
              <li
                key={a.id}
                className="flex items-center justify-between rounded-xl bg-white p-4 text-sm shadow-sm ring-1 ring-slate-200"
              >
                <span className="font-medium text-slate-700">{a.title}</span>
                <span className="flex items-center gap-2 text-slate-400">
                  <Tag>{a.subject}</Tag>
                  <Tag>{a.education_level}</Tag>
                  <Tag>/{a.max_score}</Tag>
                  {a.rubric_template.trim() && <Tag>rubric</Tag>}
                  {a.grading_prompt.trim() && <Tag>prompt</Tag>}
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

function AddStudent({
  classId,
  onAdded,
}: {
  classId: number;
  onAdded: () => void;
}) {
  const [first, setFirst] = useState("");
  const [last, setLast] = useState("");

  async function add(e: React.FormEvent) {
    e.preventDefault();
    if (!first.trim()) return;
    await api.createStudent(classId, first.trim(), last.trim());
    setFirst("");
    setLast("");
    onAdded();
  }

  return (
    <form onSubmit={add} className="flex items-center gap-2">
      <input
        value={first}
        onChange={(e) => setFirst(e.target.value)}
        placeholder="First name"
        className="input w-32"
      />
      <input
        value={last}
        onChange={(e) => setLast(e.target.value)}
        placeholder="Last name"
        className="input w-32"
      />
      <button type="submit" className="btn-primary px-3 py-2 text-xs">
        Add
      </button>
    </form>
  );
}

function SectionHeader({
  title,
  children,
}: {
  title: string;
  children?: React.ReactNode;
}) {
  return (
    <div className="mb-3 flex items-center justify-between">
      <h2 className="text-sm font-semibold uppercase tracking-wide text-slate-500">
        {title}
      </h2>
      {children}
    </div>
  );
}

function Tag({ children }: { children: React.ReactNode }) {
  return (
    <span className="rounded-full bg-slate-100 px-2 py-0.5 text-xs text-slate-500">
      {children}
    </span>
  );
}

function Empty({ children }: { children: React.ReactNode }) {
  return (
    <div className="rounded-xl border border-dashed border-slate-300 p-6 text-center text-sm text-slate-400">
      {children}
    </div>
  );
}
