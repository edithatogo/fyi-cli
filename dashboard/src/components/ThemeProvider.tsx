"use client";

import {
  createContext,
  useContext,
  useEffect,
  useSyncExternalStore,
  type ReactNode,
} from "react";

type Theme = "light" | "dark";

interface ThemeContextValue {
  theme: Theme;
  toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextValue | undefined>(undefined);
const THEME_STORAGE_KEY = "fyi-theme";
const THEME_CHANGE_EVENT = "fyi-theme-change";

function getBrowserTheme(): Theme {
  const stored = localStorage.getItem(THEME_STORAGE_KEY);
  if (stored === "light" || stored === "dark") {
    return stored;
  }
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function subscribeToTheme(onChange: () => void) {
  window.addEventListener(THEME_CHANGE_EVENT, onChange);
  return () => window.removeEventListener(THEME_CHANGE_EVENT, onChange);
}

export function useTheme(): ThemeContextValue {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error("useTheme must be used within a ThemeProvider");
  }
  return context;
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const browserTheme = useSyncExternalStore<Theme | null>(
    subscribeToTheme,
    getBrowserTheme,
    () => null,
  );

  useEffect(() => {
    if (browserTheme === null) return;
    document.documentElement.classList.toggle("dark", browserTheme === "dark");
    if (localStorage.getItem(THEME_STORAGE_KEY) !== browserTheme) {
      localStorage.setItem(THEME_STORAGE_KEY, browserTheme);
    }
  }, [browserTheme]);

  const toggleTheme = () => {
    const nextTheme = getBrowserTheme() === "light" ? "dark" : "light";
    localStorage.setItem(THEME_STORAGE_KEY, nextTheme);
    window.dispatchEvent(new window.Event(THEME_CHANGE_EVENT));
  };

  return (
    <ThemeContext.Provider value={{ theme: browserTheme ?? "light", toggleTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}
