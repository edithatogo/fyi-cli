import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { DashboardSummary } from "./DashboardSummary";

describe("DashboardSummary", () => {
  it("renders welcome heading", () => {
    render(<DashboardSummary />);
    expect(screen.getByText("Welcome to FYI")).toBeDefined();
  });

  it("renders all four KPI cards", () => {
    render(<DashboardSummary />);
    expect(screen.getByText("Total Requests")).toBeDefined();
    expect(screen.getByText("Pending")).toBeDefined();
    expect(screen.getByText("Completed")).toBeDefined();
    expect(screen.getByText("Overdue")).toBeDefined();
  });

  it("renders KPI values", () => {
    render(<DashboardSummary />);
    expect(screen.getByText("142")).toBeDefined();
    expect(screen.getByText("23")).toBeDefined();
    expect(screen.getByText("108")).toBeDefined();
    expect(screen.getByText("4")).toBeDefined();
  });

  it("renders recent activity section", () => {
    render(<DashboardSummary />);
    expect(screen.getByText("Recent Activity")).toBeDefined();
  });
});