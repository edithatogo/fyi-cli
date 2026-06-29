import type { FyiAuthority, FyiRequest } from "./mcp-client";

export interface DashboardSummaryData {
  totalRequests: number;
  attentionNeeded: number;
  overdue: number;
  authoritiesCount: number;
}

export interface DashboardChartData {
  statusDistribution: Array<{
    status: string;
    count: number;
  }>;
  requestTimeline: Array<{
    month: string;
    requests: number;
  }>;
}

export interface DashboardData {
  summary: DashboardSummaryData;
  charts: DashboardChartData;
}

type DashboardSummaryClient = {
  listRequests: (limit?: number) => Promise<FyiRequest[]>;
  listAuthorities: () => Promise<FyiAuthority[]>;
};

const ATTENTION_STATUSES = new Set([
  "submitted",
  "awaiting_response",
  "waiting_response",
  "waiting_clarification",
  "overdue",
  "partial",
  "partially_successful",
  "rejected",
]);

export function buildDashboardSummary(
  requests: FyiRequest[],
  authorities: FyiAuthority[]
): DashboardSummaryData {
  return {
    totalRequests: requests.length,
    attentionNeeded: requests.filter((request) =>
      ATTENTION_STATUSES.has((request.status ?? "").toLowerCase())
    ).length,
    overdue: requests.filter(
      (request) => (request.status ?? "").toLowerCase() === "overdue"
    ).length,
    authoritiesCount: authorities.length,
  };
}

export function buildDashboardCharts(requests: FyiRequest[]): DashboardChartData {
  const statusCounts = new Map<string, number>();
  const timelineCounts = new Map<string, number>();

  for (const request of requests) {
    const status = (request.status ?? "unknown").trim().toLowerCase() || "unknown";
    statusCounts.set(status, (statusCounts.get(status) ?? 0) + 1);

    const month = request.created_at?.slice(0, 7) || "Unknown";
    timelineCounts.set(month, (timelineCounts.get(month) ?? 0) + 1);
  }

  return {
    statusDistribution: Array.from(statusCounts.entries())
      .map(([status, count]) => ({ status, count }))
      .sort((left, right) => left.status.localeCompare(right.status)),
    requestTimeline: Array.from(timelineCounts.entries())
      .map(([month, requests]) => ({ month, requests }))
      .sort((left, right) => {
        if (left.month === "Unknown") {
          return 1;
        }
        if (right.month === "Unknown") {
          return -1;
        }
        return left.month.localeCompare(right.month);
      }),
  };
}

export async function getDashboardSummary(
  client: DashboardSummaryClient
): Promise<DashboardSummaryData> {
  const [requests, authorities] = await Promise.all([
    client.listRequests(500),
    client.listAuthorities(),
  ]);

  return buildDashboardSummary(requests, authorities);
}

export async function getDashboardData(
  client: DashboardSummaryClient
): Promise<DashboardData> {
  const [requests, authorities] = await Promise.all([
    client.listRequests(500),
    client.listAuthorities(),
  ]);

  return {
    summary: buildDashboardSummary(requests, authorities),
    charts: buildDashboardCharts(requests),
  };
}
