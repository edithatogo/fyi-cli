import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DashboardSummaryShell } from "./DashboardSummaryShell";

describe("DashboardSummaryShell", () => {
  beforeEach(() => {
    global.ResizeObserver = class ResizeObserver {
      observe() {}
      unobserve() {}
      disconnect() {}
    };
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("polls the dashboard summary endpoint and updates KPI values", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        summary: {
          totalRequests: 9,
          attentionNeeded: 4,
          overdue: 1,
          authoritiesCount: 3,
        },
        charts: {
          statusDistribution: [{ status: "submitted", count: 9 }],
          requestTimeline: [],
          attentionTrend: [],
        },
      }),
    });
    vi.stubGlobal("fetch", fetchMock);

    render(
      <DashboardSummaryShell
        initialSummary={{
          totalRequests: 1,
          attentionNeeded: 0,
          overdue: 0,
          authoritiesCount: 1,
        }}
        initialCharts={{
          statusDistribution: [],
          requestTimeline: [],
          attentionTrend: [],
        }}
        refreshIntervalMs={1000}
      />
    );

    expect(screen.getAllByText("1")).toHaveLength(2);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });

    expect(fetchMock).toHaveBeenCalledWith("/api/dashboard/summary", {
      cache: "no-store",
    });
    expect(screen.getAllByText("9")).toHaveLength(2);
    expect(screen.getByText("4")).toBeDefined();
    expect(screen.getByText("3")).toBeDefined();
  });
});
