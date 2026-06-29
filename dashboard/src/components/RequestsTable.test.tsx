import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RequestsTable } from "./RequestsTable";

function readBlob(blob: Blob) {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(reader.error);
    reader.onload = () => resolve(String(reader.result));
    reader.readAsText(blob);
  });
}

function expectRequestVisible(title: string) {
  expect(screen.getAllByText(title).length).toBeGreaterThan(0);
}

function expectRequestMissing(title: string) {
  expect(screen.queryAllByText(title)).toHaveLength(0);
}

describe("RequestsTable", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

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

    expectRequestVisible("Procurement records");
    expectRequestMissing("Meeting minutes");
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

    expectRequestVisible("Procurement records");
    expectRequestMissing("Meeting minutes");
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

    expectRequestVisible("April request");
    expectRequestMissing("March request");
  });

  it("bulk updates selected request statuses", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ id: 1, status: "completed" }),
    });
    vi.stubGlobal("fetch", fetchMock);

    render(<RequestsTable requests={requests} />);

    fireEvent.click(screen.getAllByLabelText("Select Procurement records")[0]);
    fireEvent.click(screen.getAllByLabelText("Select Meeting minutes")[0]);
    fireEvent.change(screen.getByLabelText("Bulk status"), {
      target: { value: "completed" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Apply status" }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/requests/1",
      expect.objectContaining({
        method: "PATCH",
        body: JSON.stringify({
          title: "Procurement records",
          body: "Contracts and invoices",
          status: "completed",
          user_name: "Alex",
          tags: ["finance"],
        }),
      })
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/requests/2",
      expect.objectContaining({
        method: "PATCH",
        body: JSON.stringify({
          title: "Meeting minutes",
          body: "Board papers",
          status: "completed",
          user_name: "Sam",
          tags: ["governance"],
        }),
      })
    );
  });

  it("toggles request selection from the keyboard", () => {
    render(<RequestsTable requests={requests} />);

    fireEvent.keyDown(screen.getAllByLabelText("Request row: Procurement records")[0], {
      key: "Enter",
    });

    expect(
      (screen.getAllByLabelText("Select Procurement records")[0] as HTMLInputElement)
        .checked
    ).toBe(true);
    expect(screen.getByText("1 selected")).toBeDefined();
  });

  it("exports selected requests as CSV", async () => {
    const createObjectUrl = vi.fn<(blob: Blob) => string>(() => "blob:requests-export");
    const revokeObjectUrl = vi.fn();
    const click = vi.fn();
    vi.stubGlobal("URL", {
      createObjectURL: createObjectUrl,
      revokeObjectURL: revokeObjectUrl,
    });
    vi.spyOn(document, "createElement").mockImplementation((tagName) => {
      const element = document.createElementNS("http://www.w3.org/1999/xhtml", tagName);
      if (tagName === "a") {
        Object.defineProperty(element, "click", { value: click });
      }
      return element as HTMLElement;
    });

    render(<RequestsTable requests={requests} />);

    fireEvent.click(screen.getAllByLabelText("Select Procurement records")[0]);
    fireEvent.click(
      screen.getByRole("button", { name: "Export 1 selected requests as CSV" })
    );

    expect(createObjectUrl).toHaveBeenCalledWith(expect.any(Blob));
    const [[blob]] = createObjectUrl.mock.calls;
    const csv = await readBlob(blob);
    expect(csv).toContain(
      "id,title,status,requester,authority,created_at,updated_at,url,tags"
    );
    expect(csv).toContain("1,Procurement records,submitted,Alex,,,,,finance");
    expect(click).toHaveBeenCalled();
    expect(revokeObjectUrl).toHaveBeenCalledWith("blob:requests-export");
  });
});
