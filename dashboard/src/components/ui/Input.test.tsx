import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Input } from "./Input";

describe("Input", () => {
  it("renders input element", () => {
    render(<Input placeholder="Enter text" />);
    expect(screen.getByPlaceholderText("Enter text")).toBeDefined();
  });

  it("applies error state styling", () => {
    render(<Input error="Required field" />);
    const input = screen.getByRole("textbox");
    expect(input.className).toContain("border-red");
  });

  it("displays error message", () => {
    render(<Input error="Required field" />);
    expect(screen.getByText("Required field")).toBeDefined();
  });

  it("forwards ref", () => {
    const ref = { current: null };
    render(<Input ref={ref} />);
    expect(ref.current).toBeInstanceOf(HTMLInputElement);
  });
});