import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Badge } from "./Badge";

describe("Badge", () => {
  it("renders children", () => {
    render(<Badge>Draft</Badge>);
    expect(screen.getByText("Draft")).toBeDefined();
  });

  it("applies variant classes", () => {
    render(<Badge variant="success">Completed</Badge>);
    const badge = screen.getByText("Completed");
    expect(badge.className).toContain("bg-green");
  });

  it('uses default variant when not specified', () => {
    render(<Badge>Default</Badge>);
    const badge = screen.getByText("Default");
    expect(badge.className).toContain("rounded-full");
  });
});