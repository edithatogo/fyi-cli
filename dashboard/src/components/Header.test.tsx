import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { Header } from "./Header";
import { ThemeProvider } from "./ThemeProvider";

// Mock lucide-react icons
vi.mock("lucide-react", () => ({
  Sun: () => <span data-testid="icon-sun">Sun</span>,
  Moon: () => <span data-testid="icon-moon">Moon</span>,
}));

describe("Header", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("renders the header title", () => {
    render(
      <ThemeProvider>
        <Header />
      </ThemeProvider>
    );
    expect(screen.getByText("Official Information Act Requests")).toBeDefined();
  });

  it("renders theme toggle button", () => {
    render(
      <ThemeProvider>
        <Header />
      </ThemeProvider>
    );
    expect(screen.getByLabelText(/switch to dark mode/i)).toBeDefined();
  });
});