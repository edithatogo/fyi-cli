import type { FyiAuthority, FyiRequest } from "./mcp-client";

export interface DashboardSummaryData {
  totalRequests: number;
  attentionNeeded: number;
  overdue: number;
  authoritiesCount: number;
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

export async function getDashboardSummary(
  client: DashboardSummaryClient
): Promise<DashboardSummaryData> {
  const [requests, authorities] = await Promise.all([
    client.listRequests(500),
    client.listAuthorities(),
  ]);

  return buildDashboardSummary(requests, authorities);
}
