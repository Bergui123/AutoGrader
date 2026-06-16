import { useMemo, useState } from "react";
import type { GradeResult, InlineCorrection } from "../lib/types";

/**
 * Grading Review UI (spec §Phase 5): student work on the left, the AI's inline
 * corrections with `-X pts` badges on the right. The teacher can double-click a
 * comment or a point value to override it before finalizing. The score is
 * recomputed live from the deductions.
 */
export default function ReviewPane({
  result,
  maxScore,
  studentText,
  onFinalize,
}: {
  result: GradeResult;
  maxScore: number;
  studentText: string | null;
  onFinalize: (edited: GradeResult) => Promise<void> | void;
}) {
  const [corrections, setCorrections] = useState<InlineCorrection[]>(
    result.inline_corrections,
  );
  const [feedback, setFeedback] = useState(result.summary.general_feedback);
  const [busy, setBusy] = useState(false);

  const total = useMemo(
    () => corrections.reduce((s, c) => s + (Number(c.points_deducted) || 0), 0),
    [corrections],
  );
  const finalScore = Math.max(0, maxScore + total);

  function update(i: number, patch: Partial<InlineCorrection>) {
    setCorrections((prev) => prev.map((c, idx) => (idx === i ? { ...c, ...patch } : c)));
  }
  function remove(i: number) {
    setCorrections((prev) => prev.filter((_, idx) => idx !== i));
  }
  function add() {
    setCorrections((prev) => [
      ...prev,
      { location_reference: "", correction_comment: "", points_deducted: 0 },
    ]);
  }

  async function finalize() {
    setBusy(true);
    try {
      await onFinalize({
        inline_corrections: corrections,
        summary: {
          general_feedback: feedback,
          total_points_deducted: total,
          final_score: finalScore,
        },
      });
    } finally {
      setBusy(false);
    }
  }

  const pct = maxScore > 0 ? Math.round((finalScore / maxScore) * 100) : 0;

  return (
    <div className="space-y-4">
      <div className="grid gap-4 lg:grid-cols-2">
        {/* Left: student work */}
        <section className="flex max-h-[60vh] flex-col rounded-2xl bg-white p-4 shadow-sm ring-1 ring-slate-200">
          <h3 className="mb-2 text-sm font-semibold uppercase tracking-wide text-slate-500">
            Student work
          </h3>
          <pre className="flex-1 overflow-auto whitespace-pre-wrap rounded-lg bg-slate-50 p-3 font-mono text-xs leading-relaxed text-slate-700">
            {studentText ?? "(text not shown)"}
          </pre>
        </section>

        {/* Right: corrections */}
        <section className="flex max-h-[60vh] flex-col rounded-2xl bg-white p-4 shadow-sm ring-1 ring-slate-200">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold uppercase tracking-wide text-slate-500">
              Corrections ({corrections.length})
            </h3>
            <button onClick={add} className="text-xs text-brand-600 hover:text-brand-700">
              ＋ Add
            </button>
          </div>
          <div className="flex-1 space-y-3 overflow-auto">
            {corrections.length === 0 && (
              <p className="text-sm text-emerald-600">No deductions.</p>
            )}
            {corrections.map((c, i) => (
              <CorrectionCard
                key={i}
                c={c}
                onChange={(patch) => update(i, patch)}
                onRemove={() => remove(i)}
              />
            ))}
          </div>
        </section>
      </div>

      {/* Summary */}
      <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200">
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1">
            <h3 className="mb-2 text-sm font-semibold uppercase tracking-wide text-slate-500">
              General feedback
            </h3>
            <textarea
              value={feedback}
              onChange={(e) => setFeedback(e.target.value)}
              rows={3}
              className="input resize-none"
            />
          </div>
          <div className="text-right">
            <div className="text-xs uppercase tracking-wide text-slate-400">Final score</div>
            <div className="text-3xl font-bold text-slate-800">
              {finalScore}
              <span className="ml-1 text-base font-normal text-slate-400">/ {maxScore}</span>
            </div>
            <div className="text-sm text-slate-500">{pct}%</div>
            <div className="mt-1 text-xs text-red-600">{total} pts deducted</div>
          </div>
        </div>
      </section>

      <div className="flex justify-end">
        <button onClick={finalize} disabled={busy} className="btn-primary">
          {busy ? "Saving…" : "Finalize & save correction"}
        </button>
      </div>
    </div>
  );
}

function CorrectionCard({
  c,
  onChange,
  onRemove,
}: {
  c: InlineCorrection;
  onChange: (patch: Partial<InlineCorrection>) => void;
  onRemove: () => void;
}) {
  const [editComment, setEditComment] = useState(false);
  const [editPoints, setEditPoints] = useState(false);

  return (
    <div className="rounded-lg border border-slate-200 p-3 text-sm">
      <div className="flex items-start justify-between gap-2">
        <input
          value={c.location_reference}
          onChange={(e) => onChange({ location_reference: e.target.value })}
          placeholder="location (quote / Cell C5 / Slide 3)"
          className="w-full rounded bg-slate-100 px-1.5 py-0.5 text-xs text-slate-700 outline-none focus:ring-1 focus:ring-brand-300"
        />
        {editPoints ? (
          <input
            type="number"
            step="0.5"
            autoFocus
            value={c.points_deducted}
            onChange={(e) => onChange({ points_deducted: Number(e.target.value) })}
            onBlur={() => setEditPoints(false)}
            className="w-16 shrink-0 rounded border border-brand-400 px-1 py-0.5 text-right text-xs outline-none"
          />
        ) : (
          <button
            onDoubleClick={() => setEditPoints(true)}
            title="Double-click to edit points"
            className="shrink-0 rounded-full bg-red-50 px-2 py-0.5 text-xs font-medium text-red-600"
          >
            {c.points_deducted} pts
          </button>
        )}
        <button
          onClick={onRemove}
          className="shrink-0 text-slate-300 hover:text-red-500"
          title="Remove"
        >
          ✕
        </button>
      </div>

      {editComment ? (
        <textarea
          autoFocus
          value={c.correction_comment}
          onChange={(e) => onChange({ correction_comment: e.target.value })}
          onBlur={() => setEditComment(false)}
          rows={2}
          className="mt-2 w-full resize-none rounded border border-brand-400 px-2 py-1 text-sm outline-none"
        />
      ) : (
        <p
          onDoubleClick={() => setEditComment(true)}
          title="Double-click to edit"
          className="mt-2 cursor-text rounded px-1 text-slate-600 hover:bg-amber-50"
        >
          {c.correction_comment || <span className="text-slate-300">(double-click to add a comment)</span>}
        </p>
      )}
    </div>
  );
}
