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

  it("filters requests by status and authority", () => {
    render(
      <RequestsTable
        requests={[
          {
            id: 1,
            title: "Procurement records",
            body: "Contracts and invoices",
            status: "submitted",
            authority_slug: "dia",
            authority_name: "Department of Internal Affairs",
          },
          {
            id: 2,
            title: "Meeting minutes",
            body: "Board papers",
            status: "draft",
            authority_slug: "ombudsman",
            authority_name: "Ombudsman",
          },
        ]}
      />
    );

    fireEvent.change(screen.getByLabelText("Status filter"), {
      target: { value: "submitted" },
    });
    fireEvent.change(screen.getByLabelText("Authority filter"), {
      target: { value: "dia" },
    });

    expect(screen.getByText("Procurement records")).toBeDefined();
    expect(screen.queryByText("Meeting minutes")).toBeNull();
  });

  it("filters requests by updated date range", () => {
    render(
      <RequestsTable
        requests={[
          {
            id: 1,
            title: "April request",
            body: "Recent request",
            status: "submitted",
            updated_at: "2026-04-12T09:30:00Z",
          },
          {
            id: 2,
            title: "March request",
            body: "Older request",
            status: "draft",
            updated_at: "2026-03-20T09:30:00Z",
          },
        ]}
      />
    );

    fireEvent.change(screen.getByLabelText("Updated from"), {
      target: { value: "2026-04-01" },
    });
    fireEvent.change(screen.getByLabelText("Updated to"), {
      target: { value: "2026-04-30" },
    });

    expect(screen.getByText("April request")).toBeDefined();
    expect(screen.queryByText("March request")).toBeNull();
  });
});
