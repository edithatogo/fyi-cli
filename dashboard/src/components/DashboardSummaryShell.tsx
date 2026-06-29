"use client";

import { useEffect, useState } from "react";
import { DashboardSummary } from "@/components/DashboardSummary";
import type {
  DashboardChartData,
  DashboardData,
  DashboardSummaryData,
} from "@/lib/dashboard-summary";

interface DashboardSummaryShellProps {
  initialSummary?: DashboardSummaryData;
  initialCharts?: DashboardChartData;
  initialError?: string;
  refreshIntervalMs?: number;
}

export function DashboardSummaryShell({
  initialSummary,
  initialCharts,
  initialError,
  refreshIntervalMs = 60_000,
}: DashboardSummaryShellProps) {
  const [summary, setSummary] = useState(initialSummary);
  const [charts, setCharts] = useState(initialCharts);
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

        const dashboardData = (await response.json()) as DashboardData;
        if (active) {
          setSummary(dashboardData.summary);
          setCharts(dashboardData.charts);
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

  function exportJson() {
    const payload: DashboardData = {
      summary: summary ?? {
        totalRequests: 0,
        attentionNeeded: 0,
        overdue: 0,
        authoritiesCount: 0,
      },
      charts: charts ?? {
        statusDistribution: [],
        requestTimeline: [],
        attentionTrend: [],
      },
    };
    const blob = new Blob([JSON.stringify(payload, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `fyi-dashboard-${new Date().toISOString().slice(0, 10)}.json`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  }

  return (
    <DashboardSummary
      summary={summary}
      charts={charts}
      error={error}
      onExportJson={exportJson}
    />
  );
}
