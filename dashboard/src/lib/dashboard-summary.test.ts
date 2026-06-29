import { describe, expect, it } from "vitest";
import { buildDashboardSummary, getDashboardSummary } from "./dashboard-summary";

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
});
