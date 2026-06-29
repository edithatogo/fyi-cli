import { NextResponse } from "next/server";
import { getDashboardSummary } from "@/lib/dashboard-summary";
import { FyiMcpClient } from "@/lib/mcp-client";

export const dynamic = "force-dynamic";

export async function GET() {
  const client = new FyiMcpClient();

  try {
    const summary = await getDashboardSummary(client);
    return NextResponse.json(summary);
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
