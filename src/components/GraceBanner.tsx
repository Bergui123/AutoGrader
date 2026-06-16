import { useState } from "react";
import * as api from "../lib/api";

/** Non-intrusive warning banner during the 7-day grace period (spec §3). */
export default function GraceBanner({
  daysLeft,
  onRetry,
}: {
  daysLeft: number;
  onRetry: () => void;
}) {
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
    <div className="flex items-center justify-between gap-4 border-b border-amber-200 bg-amber-50 px-4 py-2 text-sm text-amber-800">
      <div className="flex items-center gap-2">
        <span aria-hidden>⚠️</span>
        <span>
          We couldn&apos;t verify your license. The app stays fully functional
          for{" "}
          <strong>
            {daysLeft} more day{daysLeft === 1 ? "" : "s"}
          </strong>
          .
        </span>
      </div>
      <button
        onClick={retry}
        disabled={busy}
        className="rounded-lg border border-amber-300 bg-white px-3 py-1 font-medium text-amber-700 transition hover:bg-amber-100 disabled:opacity-50"
      >
        {busy ? "Checking…" : "Retry now"}
      </button>
    </div>
  );
}
