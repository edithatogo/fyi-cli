import { DashboardSummaryShell } from "@/components/DashboardSummaryShell";
import { getDashboardSummary, type DashboardSummaryData } from "@/lib/dashboard-summary";
import { FyiMcpClient } from "@/lib/mcp-client";

export const dynamic = "force-dynamic";

async function loadDashboardSummary(): Promise<{
  summary?: DashboardSummaryData;
  error?: string;
}> {
  const client = new FyiMcpClient();

  try {
    return { summary: await getDashboardSummary(client) };
  } catch (error) {
    return {
      error: error instanceof Error ? error.message : "Unable to load dashboard data",
    };
  } finally {
    await client.close();
  }
}

export default async function Home() {
  const { summary, error } = await loadDashboardSummary();

  return <DashboardSummaryShell initialSummary={summary} initialError={error} />;
}
