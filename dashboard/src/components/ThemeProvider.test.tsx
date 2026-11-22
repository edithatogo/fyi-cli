import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderToString } from "react-dom/server";
import { ThemeProvider, useTheme } from "./ThemeProvider";

function TestConsumer() {
  const { theme, toggleTheme } = useTheme();
  return (
    <div>
      <span data-testid="theme">{theme}</span>
      <button data-testid="toggle" onClick={toggleTheme}>
        Toggle
      </button>
    </div>
  );
}

describe("ThemeProvider", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("provides default light theme", () => {
    render(
      <ThemeProvider>
        <TestConsumer />
      </ThemeProvider>
    );
    expect(screen.getByTestId("theme").textContent).toBe("light");
  });

  it("stores theme preference in localStorage", async () => {
    const user = userEvent.setup();
    render(
      <ThemeProvider>
        <TestConsumer />
      </ThemeProvider>
    );
    await waitFor(() => {
      expect(localStorage.getItem("fyi-theme")).toBe("light");
    });
    await user.click(screen.getByTestId("toggle"));
    await waitFor(() => {
      expect(localStorage.getItem("fyi-theme")).toBe("dark");
    });
  });

  it("uses the saved theme after hydrating the default server theme", async () => {
    localStorage.setItem("fyi-theme", "dark");
    const app = (
      <ThemeProvider>
        <TestConsumer />
      </ThemeProvider>
    );
    const container = document.createElement("div");
    container.innerHTML = renderToString(app);
    expect(container.querySelector('[data-testid="theme"]')?.textContent).toBe("light");

    const onRecoverableError = vi.fn();
    const { getByTestId } = render(app, {
      container,
      hydrate: true,
      onRecoverableError,
    });
    await waitFor(() => {
      expect(getByTestId("theme").textContent).toBe("dark");
    });
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(onRecoverableError).not.toHaveBeenCalled();
  });

  it("throws error when useTheme is used outside provider", () => {
    // Suppress console.error for this test
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() => render(<TestConsumer />)).toThrow(
      "useTheme must be used within a ThemeProvider"
    );
    spy.mockRestore();
  });
});
