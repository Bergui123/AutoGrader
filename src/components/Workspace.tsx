import { useEffect, useState } from "react";
import * as api from "../lib/api";
import type { Class, LicenseStatus } from "../lib/types";
import ClassPanel from "./ClassPanel";
import StudentProfile from "./StudentProfile";
import GradeFlow from "./GradeFlow";

type View =
  | { kind: "class" }
  | { kind: "student"; studentId: number }
  | { kind: "grade"; studentId: number | null };

export default function Workspace({ status }: { status: LicenseStatus }) {
  const aiReady =
    status.state === "active" || status.state === "grace"
      ? status.ai_ready
      : false;

  const [classes, setClasses] = useState<Class[]>([]);
  const [selectedClassId, setSelectedClassId] = useState<number | null>(null);
  const [view, setView] = useState<View>({ kind: "class" });

  async function reloadClasses() {
    const list = await api.listClasses();
    setClasses(list);
    setSelectedClassId((prev) => prev ?? list[0]?.id ?? null);
  }
  useEffect(() => {
    void reloadClasses();
  }, []);

  function selectClass(id: number) {
    setSelectedClassId(id);
    setView({ kind: "class" });
  }

  return (
    <div className="flex h-full">
      <Sidebar
        classes={classes}
        selectedId={selectedClassId}
        onSelect={selectClass}
        onCreated={(c) => {
          setClasses((prev) => [...prev, c].sort((a, b) => a.name.localeCompare(b.name)));
          selectClass(c.id);
        }}
      />

      <div className="flex min-w-0 flex-1 flex-col">
        <Header aiReady={aiReady} />
        <main className="min-h-0 flex-1 overflow-auto p-6">
          {view.kind === "class" && (
            <ClassPanel
              classId={selectedClassId}
              onOpenStudent={(studentId) => setView({ kind: "student", studentId })}
              onGradeStudent={(studentId) => setView({ kind: "grade", studentId })}
            />
          )}
          {view.kind === "student" && (
            <StudentProfile
              studentId={view.studentId}
              onBack={() => setView({ kind: "class" })}
              onGrade={(studentId) => setView({ kind: "grade", studentId })}
            />
          )}
          {view.kind === "grade" && (
            <GradeFlow
              classId={selectedClassId}
              studentId={view.studentId}
              onDone={() =>
                setView(
                  view.studentId != null
                    ? { kind: "student", studentId: view.studentId }
                    : { kind: "class" },
                )
              }
            />
          )}
        </main>
      </div>
    </div>
  );
}

function Sidebar({
  classes,
  selectedId,
  onSelect,
  onCreated,
}: {
  classes: Class[];
  selectedId: number | null;
  onSelect: (id: number) => void;
  onCreated: (c: Class) => void;
}) {
  const [adding, setAdding] = useState(false);
  const [name, setName] = useState("");

  async function add(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    const c = await api.createClass(name.trim());
    setName("");
    setAdding(false);
    onCreated(c);
  }

  return (
    <aside className="flex w-64 shrink-0 flex-col border-r border-slate-200 bg-white/70">
      <div className="flex items-center gap-2 border-b border-slate-200 px-4 py-3">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-brand-500 to-violet-600 text-white">
          ✓
        </div>
        <span className="font-semibold text-slate-800">AI Grader</span>
      </div>

      <div className="flex items-center justify-between px-4 py-2">
        <span className="text-xs font-semibold uppercase tracking-wide text-slate-400">
          Classes
        </span>
        <button
          onClick={() => setAdding((v) => !v)}
          className="text-brand-600 hover:text-brand-700"
          title="New class"
        >
          ＋
        </button>
      </div>

      {adding && (
        <form onSubmit={add} className="px-3 pb-2">
          <input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Class name"
            className="input"
          />
        </form>
      )}

      <nav className="flex-1 overflow-auto px-2">
        {classes.length === 0 ? (
          <p className="px-2 py-4 text-sm text-slate-400">
            No classes yet. Click ＋ to add one.
          </p>
        ) : (
          classes.map((c) => (
            <button
              key={c.id}
              onClick={() => onSelect(c.id)}
              className={
                "mb-1 w-full truncate rounded-lg px-3 py-2 text-left text-sm transition " +
                (c.id === selectedId
                  ? "bg-brand-50 font-medium text-brand-700"
                  : "text-slate-600 hover:bg-slate-100")
              }
            >
              {c.name}
            </button>
          ))
        )}
      </nav>
    </aside>
  );
}

function Header({ aiReady }: { aiReady: boolean }) {
  return (
    <header className="flex items-center justify-end border-b border-slate-200 bg-white/70 px-6 py-3 backdrop-blur">
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
