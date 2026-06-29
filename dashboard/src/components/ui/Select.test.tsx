import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Select } from "./Select";

describe("Select", () => {
  const options = [
    { value: "draft", label: "Draft" },
    { value: "submitted", label: "Submitted" },
  ];

  it("renders select with options", () => {
    render(<Select options={options} />);
    expect(screen.getByText("Draft")).toBeDefined();
    expect(screen.getByText("Submitted")).toBeDefined();
  });

  it("applies error state", () => {
    render(<Select options={options} error="Required" />);
    expect(screen.getByText("Required")).toBeDefined();
    const select = screen.getByRole("combobox");
    expect(select.className).toContain("border-red");
  });

  it("renders placeholder option", () => {
    render(<Select options={options} placeholder="Select status" />);
    expect(screen.getByText("Select status")).toBeDefined();
  });
});