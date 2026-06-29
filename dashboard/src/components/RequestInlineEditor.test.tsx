import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RequestInlineEditor } from "./RequestInlineEditor";

describe("RequestInlineEditor", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("auto-saves edited request fields after a debounce", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ id: 7, title: "Updated title" }),
    });
    vi.stubGlobal("fetch", fetchMock);

    render(
      <RequestInlineEditor
        request={{
          id: 7,
          title: "Original title",
          body: "Original body",
          status: "draft",
          user_name: "Alex",
          url: "https://example.test/request",
          tags: ["oia", "draft"],
        }}
      />
    );

    fireEvent.change(screen.getByLabelText("Title"), {
      target: { value: "Updated title" },
    });

    expect(fetchMock).not.toHaveBeenCalled();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(750);
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/requests/7",
      expect.objectContaining({
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          title: "Updated title",
          body: "Original body",
          status: "draft",
          user_name: "Alex",
          url: "https://example.test/request",
          tags: ["oia", "draft"],
        }),
      })
    );
    await act(async () => {
      await Promise.resolve();
    });
    expect(screen.getByText("Saved")).toBeDefined();
  });
});
