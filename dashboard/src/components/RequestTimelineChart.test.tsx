import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { RequestTimelineChart } from "./RequestTimelineChart";

describe("RequestTimelineChart", () => {
  beforeAll(() => {
    global.ResizeObserver = class ResizeObserver {
      observe() {}
      unobserve() {}
      disconnect() {}
    };
  });

  it("renders request timeline labels and counts", () => {
    render(
      <RequestTimelineChart
        data={[
          { month: "2026-04", requests: 2 },
          { month: "2026-05", requests: 5 },
        ]}
      />
    );

    expect(screen.getByText("Request timeline")).toBeDefined();
    expect(screen.getByText("Apr 2026")).toBeDefined();
    expect(screen.getByText("May 2026")).toBeDefined();
    expect(screen.getByText("2")).toBeDefined();
    expect(screen.getByText("5")).toBeDefined();
  });

  it("renders an empty state when there is no timeline data", () => {
    render(<RequestTimelineChart data={[]} />);

    expect(screen.getByText("No request timeline data")).toBeDefined();
  });
});
