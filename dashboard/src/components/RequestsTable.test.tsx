import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { RequestsTable } from "./RequestsTable";

describe("RequestsTable", () => {
  const requests = [
    {
      id: 1,
      title: "Procurement records",
      body: "Contracts and invoices",
      status: "submitted",
      user_name: "Alex",
      tags: ["finance"],
    },
    {
      id: 2,
      title: "Meeting minutes",
      body: "Board papers",
      status: "draft",
      user_name: "Sam",
      tags: ["governance"],
    },
  ];

  it("filters requests with full-text search", () => {
    render(<RequestsTable requests={requests} />);

    fireEvent.change(screen.getByLabelText("Search requests"), {
      target: { value: "invoice" },
    });

    expect(screen.getByText("Procurement records")).toBeDefined();
    expect(screen.queryByText("Meeting minutes")).toBeNull();
  });
});
