import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { AttentionTrendChart } from "./AttentionTrendChart";

describe("AttentionTrendChart", () => {
  beforeAll(() => {
    global.ResizeObserver = class ResizeObserver {
      observe() {}
      unobserve() {}
      disconnect() {}
    };
  });

  it("renders attention trend labels and counts", () => {
    render(
      <AttentionTrendChart
        data={[
          { month: "2026-04", attentionNeeded: 1 },
          { month: "2026-05", attentionNeeded: 4 },
        ]}
      />
    );

    expect(screen.getByText("Attention trend")).toBeDefined();
    expect(screen.getByText("Apr 2026")).toBeDefined();
    expect(screen.getByText("May 2026")).toBeDefined();
    expect(screen.getByText("1")).toBeDefined();
    expect(screen.getByText("4")).toBeDefined();
  });

  it("renders an empty state when there is no attention data", () => {
    render(<AttentionTrendChart data={[]} />);

    expect(screen.getByText("No attention trend data")).toBeDefined();
  });
});
