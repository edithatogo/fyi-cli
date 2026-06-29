import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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

  it("exports the current dashboard snapshot as JSON", async () => {
    vi.useRealTimers();
    const createObjectUrl = vi.fn(() => "blob:dashboard-export");
    const revokeObjectUrl = vi.fn();
    const click = vi.fn();
    const appendChild = vi.spyOn(document.body, "appendChild");
    const removeChild = vi.spyOn(document.body, "removeChild");
    vi.stubGlobal("URL", {
      createObjectURL: createObjectUrl,
      revokeObjectURL: revokeObjectUrl,
    });
    vi.spyOn(document, "createElement").mockImplementation((tagName) => {
      const element = document.createElementNS("http://www.w3.org/1999/xhtml", tagName);
      if (tagName === "a") {
        Object.defineProperty(element, "click", { value: click });
      }
      return element as HTMLElement;
    });

    render(
      <DashboardSummaryShell
        initialSummary={{
          totalRequests: 1,
          attentionNeeded: 0,
          overdue: 0,
          authoritiesCount: 1,
        }}
        initialCharts={{
          statusDistribution: [{ status: "draft", count: 1 }],
          requestTimeline: [{ month: "2026-04", requests: 1 }],
          attentionTrend: [],
        }}
      />
    );

    await userEvent.click(screen.getByRole("button", { name: "Export JSON" }));

    expect(createObjectUrl).toHaveBeenCalledWith(expect.any(Blob));
    expect(click).toHaveBeenCalled();
    expect(appendChild).toHaveBeenCalled();
    expect(removeChild).toHaveBeenCalled();
    expect(revokeObjectUrl).toHaveBeenCalledWith("blob:dashboard-export");
  });

  it("exports dashboard chart data as CSV", async () => {
    vi.useRealTimers();
    const createObjectUrl = vi.fn(() => "blob:dashboard-export");
    const revokeObjectUrl = vi.fn();
    const click = vi.fn();
    const appendChild = vi.spyOn(document.body, "appendChild");
    const removeChild = vi.spyOn(document.body, "removeChild");
    vi.stubGlobal("URL", {
      createObjectURL: createObjectUrl,
      revokeObjectURL: revokeObjectUrl,
    });
    vi.spyOn(document, "createElement").mockImplementation((tagName) => {
      const element = document.createElementNS("http://www.w3.org/1999/xhtml", tagName);
      if (tagName === "a") {
        Object.defineProperty(element, "click", { value: click });
      }
      return element as HTMLElement;
    });

    render(
      <DashboardSummaryShell
        initialSummary={{
          totalRequests: 1,
          attentionNeeded: 0,
          overdue: 0,
          authoritiesCount: 1,
        }}
        initialCharts={{
          statusDistribution: [{ status: "draft", count: 1 }],
          requestTimeline: [{ month: "2026-04", requests: 1 }],
          attentionTrend: [{ month: "2026-04", attentionNeeded: 2 }],
        }}
      />
    );

    await userEvent.click(screen.getByRole("button", { name: "Export CSV" }));

    expect(createObjectUrl).toHaveBeenCalledWith(expect.any(Blob));
    expect(click).toHaveBeenCalled();
    expect(appendChild).toHaveBeenCalled();
    expect(removeChild).toHaveBeenCalled();
    expect(revokeObjectUrl).toHaveBeenCalledWith("blob:dashboard-export");
  });
});
