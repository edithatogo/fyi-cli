import { NextResponse } from "next/server";
import { getDashboardData } from "@/lib/dashboard-summary";
import { FyiMcpClient } from "@/lib/mcp-client";

export const dynamic = "force-dynamic";

export async function GET() {
  const client = new FyiMcpClient();

  try {
    const dashboardData = await getDashboardData(client);
    return NextResponse.json(dashboardData);
  } catch (error) {
    return NextResponse.json(
      {
        error: error instanceof Error ? error.message : "Unable to load dashboard data",
      },
      { status: 503 }
    );
  } finally {
    await client.close();
  }
}
