import { describe, expect, it } from "vitest";
import {
  buildDashboardCharts,
  getDashboardData,
  buildDashboardSummary,
  getDashboardSummary,
} from "./dashboard-summary";

describe("dashboard summary data", () => {
  it("counts KPI metrics from requests and authorities", () => {
    const summary = buildDashboardSummary(
      [
        { id: 1, title: "A", body: "Body", status: "draft" },
        { id: 2, title: "B", body: "Body", status: "waiting_response" },
        { id: 3, title: "C", body: "Body", status: "overdue" },
        { id: 4, title: "D", body: "Body", status: "successful" },
      ],
      [
        { slug: "ombudsman", name: "Ombudsman" },
        { slug: "dia", name: "Department of Internal Affairs" },
      ]
    );

    expect(summary).toEqual({
      totalRequests: 4,
      attentionNeeded: 2,
      overdue: 1,
      authoritiesCount: 2,
    });
  });

  it("fetches requests and authorities through an MCP-compatible client", async () => {
    const client = {
      listRequests: async () => [
        { id: 1, title: "A", body: "Body", status: "submitted" },
        { id: 2, title: "B", body: "Body", status: "completed" },
      ],
      listAuthorities: async () => [{ slug: "ombudsman", name: "Ombudsman" }],
    };

    await expect(getDashboardSummary(client)).resolves.toEqual({
      totalRequests: 2,
      attentionNeeded: 1,
      overdue: 0,
      authoritiesCount: 1,
    });
  });

  it("fetches combined dashboard summary and chart data", async () => {
    const client = {
      listRequests: async () => [
        {
          id: 1,
          title: "A",
          body: "Body",
          status: "submitted",
          created_at: "2026-04-01T00:00:00Z",
        },
      ],
      listAuthorities: async () => [{ slug: "ombudsman", name: "Ombudsman" }],
    };

    await expect(getDashboardData(client)).resolves.toEqual({
      summary: {
        totalRequests: 1,
        attentionNeeded: 1,
        overdue: 0,
        authoritiesCount: 1,
      },
      charts: {
        statusDistribution: [{ status: "submitted", count: 1 }],
        requestTimeline: [{ month: "2026-04", requests: 1 }],
        attentionTrend: [{ month: "2026-04", attentionNeeded: 1 }],
      },
    });
  });

  it("builds chart-ready status and timeline data", () => {
    const charts = buildDashboardCharts([
      {
        id: 1,
        title: "A",
        body: "Body",
        status: "draft",
        created_at: "2026-04-10T00:00:00Z",
      },
      {
        id: 2,
        title: "B",
        body: "Body",
        status: "submitted",
        created_at: "2026-04-22T00:00:00Z",
      },
      {
        id: 3,
        title: "C",
        body: "Body",
        status: "overdue",
        created_at: "2026-05-01T00:00:00Z",
      },
      {
        id: 4,
        title: "D",
        body: "Body",
        status: null,
        created_at: null,
      },
    ]);

    expect(charts.statusDistribution).toEqual([
      { status: "draft", count: 1 },
      { status: "overdue", count: 1 },
      { status: "submitted", count: 1 },
      { status: "unknown", count: 1 },
    ]);
    expect(charts.requestTimeline).toEqual([
      { month: "2026-04", requests: 2 },
      { month: "2026-05", requests: 1 },
      { month: "Unknown", requests: 1 },
    ]);
    expect(charts.attentionTrend).toEqual([
      { month: "2026-04", attentionNeeded: 1 },
      { month: "2026-05", attentionNeeded: 1 },
    ]);
  });
});
