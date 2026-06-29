import { DashboardSummaryShell } from "@/components/DashboardSummaryShell";
import { getDashboardData, type DashboardData } from "@/lib/dashboard-summary";
import { FyiMcpClient } from "@/lib/mcp-client";

export const dynamic = "force-dynamic";

async function loadDashboardSummary(): Promise<{
  data?: DashboardData;
  error?: string;
}> {
  const client = new FyiMcpClient();

  try {
    return { data: await getDashboardData(client) };
  } catch (error) {
    return {
      error: error instanceof Error ? error.message : "Unable to load dashboard data",
    };
  } finally {
    await client.close();
  }
}

export default async function Home() {
  const { data, error } = await loadDashboardSummary();

  return (
    <DashboardSummaryShell
      initialSummary={data?.summary}
      initialCharts={data?.charts}
      initialError={error}
    />
  );
}
