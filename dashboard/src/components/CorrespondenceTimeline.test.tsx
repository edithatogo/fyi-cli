import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CorrespondenceTimeline } from "./CorrespondenceTimeline";

describe("CorrespondenceTimeline", () => {
  it("renders correspondence events in chronological order", () => {
    render(
      <CorrespondenceTimeline
        correspondence={[
          {
            direction: "response",
            body: "The agency provided a partial release.",
            sent_at: "2026-04-12T09:30:00Z",
            state: "partial",
            attachments: ["release.pdf", "schedule.csv"],
          },
          {
            direction: "request",
            body: "Please provide the procurement records.",
            sent_at: "2026-04-01T10:00:00Z",
            state: "sent",
          },
        ]}
      />
    );

    expect(screen.getByText("Correspondence timeline")).toBeDefined();
    expect(screen.getByText("Request sent")).toBeDefined();
    expect(screen.getByText("Response received")).toBeDefined();
    expect(screen.getByText("sent")).toBeDefined();
    expect(screen.getByText("partial")).toBeDefined();
    expect(screen.getByText("2 attachments")).toBeDefined();

    const bodies = screen.getAllByText(/records|partial release/);
    expect(bodies[0].textContent).toContain("procurement records");
    expect(bodies[1].textContent).toContain("partial release");
  });

  it("renders request lifecycle events with correspondence", () => {
    render(
      <CorrespondenceTimeline
        request={{
          id: 7,
          title: "Procurement records",
          body: "Please provide the procurement records.",
          status: "awaiting_response",
          created_at: "2026-03-30T08:00:00Z",
          updated_at: "2026-04-12T11:00:00Z",
        }}
        correspondence={[
          {
            direction: "request",
            body: "Please provide the procurement records.",
            sent_at: "2026-04-01T10:00:00Z",
            state: "sent",
          },
        ]}
      />
    );

    expect(screen.getByText("Request created")).toBeDefined();
    expect(screen.getByText("Request updated")).toBeDefined();
    expect(screen.getAllByText("awaiting_response")).toHaveLength(2);

    const eventTitles = screen.getAllByText(
      /Request created|Request sent|Request updated/
    );
    expect(eventTitles.map((title) => title.textContent)).toEqual([
      "Request created",
      "Request sent",
      "Request updated",
    ]);
  });

  it("renders status-aware timeline indicators", () => {
    render(
      <CorrespondenceTimeline
        request={{
          id: 7,
          title: "Procurement records",
          body: "Please provide the procurement records.",
          status: "overdue",
          created_at: "2026-03-30T08:00:00Z",
        }}
        correspondence={[
          {
            direction: "response",
            body: "The agency provided a partial release.",
            sent_at: "2026-04-12T09:30:00Z",
            state: "partial",
          },
        ]}
      />
    );

    expect(screen.getByLabelText("Timeline indicator: overdue")).toBeDefined();
    expect(screen.getByLabelText("Timeline indicator: partial")).toBeDefined();
  });

  it("renders an empty state when no correspondence exists", () => {
    render(<CorrespondenceTimeline correspondence={[]} />);

    expect(
      screen.getByText("No correspondence has been captured for this request yet.")
    ).toBeDefined();
  });
});
