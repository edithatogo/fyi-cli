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
});
