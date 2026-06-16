import { useState } from "react";
import * as api from "../lib/api";

/** Clean activation gate shown when no valid activation flag exists (spec §3). */
export default function ActivationGate({
  onActivated,
}: {
  onActivated: () => void;
}) {
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      await api.activate(code);
      onActivated();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex h-full items-center justify-center p-6">
      <div className="w-full max-w-md rounded-2xl bg-white p-8 shadow-xl ring-1 ring-slate-200">
        <div className="mb-6 text-center">
          <div className="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-2xl bg-gradient-to-br from-brand-500 to-violet-600 text-2xl text-white shadow-lg">
            ✓
          </div>
          <h1 className="text-2xl font-semibold text-slate-800">
            Activate AI Grader
          </h1>
          <p className="mt-2 text-sm text-slate-500">
            Enter your activation code to get started. Your student data always
            stays on this computer.
          </p>
        </div>

        <form onSubmit={submit} className="space-y-4">
          <input
            autoFocus
            value={code}
            onChange={(e) => setCode(e.target.value)}
            placeholder="XXXX-XXXX-XXXX-XXXX"
            className="w-full rounded-xl border border-slate-300 px-4 py-3 text-center font-mono tracking-wider text-slate-800 outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100"
          />

          {error && (
            <p className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600">
              {error}
            </p>
          )}

          <button
            type="submit"
            disabled={busy || code.trim().length === 0}
            className="w-full rounded-xl bg-brand-600 px-4 py-3 font-medium text-white transition hover:bg-brand-700 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {busy ? "Activating…" : "Activate"}
          </button>
        </form>

        <p className="mt-6 text-center text-xs text-slate-400">
          Validated securely online. AI credentials are kept in memory only and
          never written to disk.
        </p>
      </div>
    </div>
  );
}
