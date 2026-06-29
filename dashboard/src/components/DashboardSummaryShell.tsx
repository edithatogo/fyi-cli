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

const emptySummary: DashboardSummaryData = {
  totalRequests: 0,
  attentionNeeded: 0,
  overdue: 0,
  authoritiesCount: 0,
};

const emptyCharts: DashboardChartData = {
  statusDistribution: [],
  requestTimeline: [],
  attentionTrend: [],
};

function csvCell(value: string | number) {
  const text = String(value);
  if (!/[",\r\n]/.test(text)) {
    return text;
  }
  return `"${text.replaceAll("\"", "\"\"")}"`;
}

function buildDashboardCsv(data: DashboardData) {
  const rows: Array<[string, string, number]> = [
    ["summary", "totalRequests", data.summary.totalRequests],
    ["summary", "attentionNeeded", data.summary.attentionNeeded],
    ["summary", "overdue", data.summary.overdue],
    ["summary", "authoritiesCount", data.summary.authoritiesCount],
    ...data.charts.statusDistribution.map(
      (entry): [string, string, number] => [
        "statusDistribution",
        entry.status,
        entry.count,
      ]
    ),
    ...data.charts.requestTimeline.map(
      (entry): [string, string, number] => [
        "requestTimeline",
        entry.month,
        entry.requests,
      ]
    ),
    ...data.charts.attentionTrend.map(
      (entry): [string, string, number] => [
        "attentionTrend",
        entry.month,
        entry.attentionNeeded,
      ]
    ),
  ];

  return [
    ["section", "label", "value"],
    ...rows,
  ]
    .map((row) => row.map(csvCell).join(","))
    .join("\n");
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

  const dashboardData: DashboardData = {
    summary: summary ?? emptySummary,
    charts: charts ?? emptyCharts,
  };

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

  function downloadBlob(blob: Blob, extension: "csv" | "json") {
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `fyi-dashboard-${new Date().toISOString().slice(0, 10)}.${extension}`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  }

  function exportJson() {
    downloadBlob(
      new Blob([JSON.stringify(dashboardData, null, 2)], {
        type: "application/json",
      }),
      "json"
    );
  }

  function exportCsv() {
    downloadBlob(
      new Blob([buildDashboardCsv(dashboardData)], {
        type: "text/csv",
      }),
      "csv"
    );
  }

  return (
    <DashboardSummary
      summary={summary}
      charts={charts}
      error={error}
      onExportJson={exportJson}
      onExportCsv={exportCsv}
    />
  );
}
