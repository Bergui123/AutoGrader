import { useCallback, useEffect, useState } from "react";
import * as api from "./lib/api";
import type { LicenseStatus } from "./lib/types";
import ActivationGate from "./components/ActivationGate";
import GraceBanner from "./components/GraceBanner";
import LockScreen from "./components/LockScreen";
import AppShell from "./components/AppShell";

type Boot =
  | { phase: "loading" }
  | { phase: "ready"; status: LicenseStatus }
  | { phase: "error"; message: string };

export default function App() {
  const [boot, setBoot] = useState<Boot>({ phase: "loading" });

  const refresh = useCallback(async () => {
    try {
      const status = await api.getLicenseStatus();
      setBoot({ phase: "ready", status });
    } catch (e) {
      setBoot({ phase: "error", message: String(e) });
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  if (boot.phase === "loading") {
    return <CenteredMessage title="Starting AI Grader…" />;
  }

  if (boot.phase === "error") {
    return (
      <CenteredMessage
        title="Could not start"
        subtitle={boot.message}
        tone="error"
      />
    );
  }

  const { status } = boot;

  switch (status.state) {
    case "unactivated":
      return <ActivationGate onActivated={refresh} />;
    case "locked":
      return <LockScreen onRetry={refresh} />;
    case "grace":
      return (
        <>
          <GraceBanner daysLeft={status.days_left} onRetry={refresh} />
          <AppShell status={status} />
        </>
      );
    case "active":
      return <AppShell status={status} />;
  }
}

function CenteredMessage({
  title,
  subtitle,
  tone = "neutral",
}: {
  title: string;
  subtitle?: string;
  tone?: "neutral" | "error";
}) {
  return (
    <div className="flex h-full items-center justify-center p-8">
      <div className="text-center">
        <h1
          className={
            "text-xl font-semibold " +
            (tone === "error" ? "text-red-600" : "text-slate-700")
          }
        >
          {title}
        </h1>
        {subtitle && (
          <p className="mt-2 max-w-md text-sm text-slate-500">{subtitle}</p>
        )}
      </div>
    </div>
  );
}
