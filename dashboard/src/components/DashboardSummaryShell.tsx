"use client";

import { useEffect, useState } from "react";
import { DashboardSummary } from "@/components/DashboardSummary";
import type { DashboardSummaryData } from "@/lib/dashboard-summary";

interface DashboardSummaryShellProps {
  initialSummary?: DashboardSummaryData;
  initialError?: string;
  refreshIntervalMs?: number;
}

export function DashboardSummaryShell({
  initialSummary,
  initialError,
  refreshIntervalMs = 60_000,
}: DashboardSummaryShellProps) {
  const [summary, setSummary] = useState(initialSummary);
  const [error, setError] = useState(initialError);

  useEffect(() => {
    let active = true;

    async function refreshSummary() {
      try {
        const response = await fetch("/api/dashboard/summary", {
          cache: "no-store",
        });

        if (!response.ok) {
          throw new Error(`Dashboard refresh failed with ${response.status}`);
        }

        const nextSummary = (await response.json()) as DashboardSummaryData;
        if (active) {
          setSummary(nextSummary);
          setError(undefined);
        }
      } catch (refreshError) {
        if (active) {
          setError(
            refreshError instanceof Error
              ? refreshError.message
              : "Unable to refresh dashboard data"
          );
        }
      }
    }

    const interval = window.setInterval(refreshSummary, refreshIntervalMs);
    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, [refreshIntervalMs]);

  return <DashboardSummary summary={summary} error={error} />;
}
