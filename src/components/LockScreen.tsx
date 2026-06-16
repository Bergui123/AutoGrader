import { useState } from "react";
import * as api from "../lib/api";

/**
 * "No Hostage" lock state (spec §3). Grading is locked, but the teacher can
 * always open their local student data, and the SQLite DB is untouched.
 */
export default function LockScreen({ onRetry }: { onRetry: () => void }) {
  const [busy, setBusy] = useState(false);

  async function retry() {
    setBusy(true);
    try {
      await api.runHeartbeat();
    } finally {
      setBusy(false);
      onRetry();
    }
  }

  return (
    <div className="flex h-full items-center justify-center p-6">
      <div className="w-full max-w-lg rounded-2xl bg-white p-8 text-center shadow-xl ring-1 ring-slate-200">
        <div className="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-2xl bg-slate-100 text-2xl">
          🔒
        </div>
        <h1 className="text-2xl font-semibold text-slate-800">
          License needs renewal
        </h1>
        <p className="mx-auto mt-2 max-w-md text-sm text-slate-500">
          Your 7-day grace period has ended, so grading is paused. Nothing has
          been deleted — all of your students&apos; files and grades remain
          safely on this computer.
        </p>

        <button
          onClick={() => void api.openLocalStudentData()}
          className="mt-6 w-full rounded-xl bg-brand-600 px-4 py-3 font-medium text-white transition hover:bg-brand-700"
        >
          📂 Open Local Student Data
        </button>

        <button
          onClick={retry}
          disabled={busy}
          className="mt-3 w-full rounded-xl border border-slate-300 px-4 py-3 font-medium text-slate-700 transition hover:bg-slate-50 disabled:opacity-50"
        >
          {busy ? "Checking license…" : "I’ve renewed — check again"}
        </button>
      </div>
    </div>
  );
}
