import { useEffect, useRef, useState } from "react";

/**
 * Pre-Flight Verification Intercept (spec §Phase 3).
 * The extracted text is shown for review. Double-click any line to turn it
 * into an input and fix an OCR misread, then "Confirm & Grade".
 */
export default function PreFlight({
  markdown,
  busy,
  onConfirm,
  onBack,
}: {
  markdown: string;
  busy: boolean;
  onConfirm: (verified: string) => void;
  onBack: () => void;
}) {
  const [lines, setLines] = useState<string[]>(() => markdown.split("\n"));
  const [editing, setEditing] = useState<number | null>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    setLines(markdown.split("\n"));
  }, [markdown]);

  useEffect(() => {
    if (editing !== null) inputRef.current?.focus();
  }, [editing]);

  function updateLine(i: number, value: string) {
    setLines((prev) => prev.map((l, idx) => (idx === i ? value : l)));
  }

  return (
    <section className="flex h-full flex-col rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200">
      <header className="mb-3 flex items-center justify-between">
        <div>
          <h2 className="text-sm font-semibold uppercase tracking-wide text-slate-500">
            Verify extracted text
          </h2>
          <p className="text-xs text-slate-400">
            Double-click any line to correct a misread. Original errors are kept
            on purpose.
          </p>
        </div>
        <button
          onClick={onBack}
          disabled={busy}
          className="text-sm text-slate-500 hover:text-slate-700 disabled:opacity-50"
        >
          ← Back
        </button>
      </header>

      <div className="flex-1 overflow-auto rounded-lg border border-slate-200 bg-slate-50/50 p-3 font-mono text-sm leading-relaxed">
        {lines.map((line, i) =>
          editing === i ? (
            <textarea
              key={i}
              ref={inputRef}
              value={line}
              rows={1}
              onChange={(e) => updateLine(i, e.target.value)}
              onBlur={() => setEditing(null)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  setEditing(null);
                }
              }}
              className="my-0.5 w-full resize-none rounded border border-brand-400 bg-white px-2 py-1 outline-none focus:ring-2 focus:ring-brand-100"
            />
          ) : (
            <div
              key={i}
              onDoubleClick={() => setEditing(i)}
              title="Double-click to edit"
              className="cursor-text whitespace-pre-wrap rounded px-2 py-0.5 hover:bg-amber-50"
            >
              {line === "" ? " " : line}
            </div>
          ),
        )}
      </div>

      <footer className="mt-4 flex justify-end">
        <button
          onClick={() => onConfirm(lines.join("\n"))}
          disabled={busy}
          className="btn-primary"
        >
          {busy ? "Grading…" : "Confirm & Grade"}
        </button>
      </footer>
    </section>
  );
}
