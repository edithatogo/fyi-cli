import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { DashboardSummary } from "./DashboardSummary";

describe("DashboardSummary", () => {
  const summary = {
    totalRequests: 42,
    attentionNeeded: 7,
    overdue: 3,
    authoritiesCount: 12,
  };

  it("renders welcome heading", () => {
    render(<DashboardSummary summary={summary} />);
    expect(screen.getByText("Welcome to FYI")).toBeDefined();
  });

  it("renders all four KPI cards", () => {
    render(<DashboardSummary summary={summary} />);
    expect(screen.getByText("Total Requests")).toBeDefined();
    expect(screen.getByText("Attention Needed")).toBeDefined();
    expect(screen.getByText("Overdue")).toBeDefined();
    expect(screen.getByText("Authorities")).toBeDefined();
  });

  it("renders provided KPI values", () => {
    render(<DashboardSummary summary={summary} />);
    expect(screen.getByText("42")).toBeDefined();
    expect(screen.getByText("7")).toBeDefined();
    expect(screen.getByText("3")).toBeDefined();
    expect(screen.getByText("12")).toBeDefined();
  });

  it("renders empty values when summary data is unavailable", () => {
    render(<DashboardSummary />);
    expect(screen.getAllByText("0")).toHaveLength(4);
  });

  it("renders recent activity section", () => {
    render(<DashboardSummary summary={summary} />);
    expect(screen.getByText("Recent Activity")).toBeDefined();
  });
});
