import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { StatusDistributionChart } from "./StatusDistributionChart";

describe("StatusDistributionChart", () => {
  beforeAll(() => {
    global.ResizeObserver = class ResizeObserver {
      observe() {}
      unobserve() {}
      disconnect() {}
    };
  });

  it("renders status distribution labels and counts", () => {
    render(
      <StatusDistributionChart
        data={[
          { status: "draft", count: 2 },
          { status: "submitted", count: 3 },
        ]}
      />
    );

    expect(screen.getByText("Status distribution")).toBeDefined();
    expect(screen.getByText("Draft")).toBeDefined();
    expect(screen.getByText("Submitted")).toBeDefined();
    expect(screen.getByText("2")).toBeDefined();
    expect(screen.getByText("3")).toBeDefined();
  });

  it("renders an empty state when there is no chart data", () => {
    render(<StatusDistributionChart data={[]} />);

    expect(screen.getByText("No request status data")).toBeDefined();
  });
});
