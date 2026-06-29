import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { Sidebar } from "./Sidebar";

vi.mock("next/navigation", () => ({
  usePathname: () => "/",
}));

describe("Sidebar", () => {
  it("renders all navigation links", () => {
    render(<Sidebar />);
    expect(screen.getAllByText("Dashboard")).toHaveLength(2);
    expect(screen.getAllByText("Requests")).toHaveLength(2);
    expect(screen.getAllByText("Authorities")).toHaveLength(2);
    expect(screen.getAllByText("Search")).toHaveLength(2);
  });

  it("renders brand name", () => {
    render(<Sidebar />);
    expect(screen.getByText("FYI Dashboard")).toBeDefined();
  });

  it("highlights active link", () => {
    render(<Sidebar />);
    const dashLink = screen.getAllByText("Dashboard")[0].closest("a");
    expect(dashLink?.className).toContain("bg-brand-50");
    expect(dashLink?.getAttribute("aria-current")).toBe("page");
  });

  it("renders desktop and mobile navigation landmarks", () => {
    render(<Sidebar />);
    expect(screen.getByLabelText("Primary navigation")).toBeDefined();
    expect(screen.getByLabelText("Mobile navigation")).toBeDefined();
  });
});
