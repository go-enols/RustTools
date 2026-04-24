import { useEffect, useState } from "react";

const STORAGE_KEY = "theme-dark";

function getInitialDark(): boolean {
  if (typeof window === "undefined") return false;
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored !== null) return stored === "true";
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function useTheme() {
  const [dark, setDark] = useState(() => {
    const initial = getInitialDark();
    if (typeof window !== "undefined") {
      document.documentElement.classList.toggle("dark", initial);
    }
    return initial;
  });

  useEffect(() => {
    document.documentElement.classList.toggle("dark", dark);
    localStorage.setItem(STORAGE_KEY, String(dark));
  }, [dark]);

  const toggle = () => setDark((d) => !d);

  return { dark, toggle };
}
