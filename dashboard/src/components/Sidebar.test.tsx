import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { Sidebar } from "./Sidebar";

vi.mock("next/navigation", () => ({
  usePathname: () => "/",
}));

describe("Sidebar", () => {
  it("renders all navigation links", () => {
    render(<Sidebar />);
    expect(screen.getByText("Dashboard")).toBeDefined();
    expect(screen.getByText("Requests")).toBeDefined();
    expect(screen.getByText("Authorities")).toBeDefined();
    expect(screen.getByText("Search")).toBeDefined();
  });

  it("renders brand name", () => {
    render(<Sidebar />);
    expect(screen.getByText("FYI Dashboard")).toBeDefined();
  });

  it("highlights active link", () => {
    render(<Sidebar />);
    const dashLink = screen.getByText("Dashboard").closest("a");
    expect(dashLink?.className).toContain("bg-brand-50");
  });
});