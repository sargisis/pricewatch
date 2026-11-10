import { useState } from "react";
import { MoonIcon, SunIcon } from "./Icons";

export function ThemeToggle() {
  const [dark, setDark] = useState(() => document.documentElement.dataset.theme === "dark");

  const toggle = () => {
    const next = !dark;
    setDark(next);
    document.documentElement.dataset.theme = next ? "dark" : "light";
    try {
      localStorage.setItem("pw-theme", next ? "dark" : "light");
    } catch {
      /* not saved; still applies for this visit */
    }
  };

  return (
    <button className="icon-btn" onClick={toggle} title={dark ? "Switch to light theme" : "Switch to dark theme"} aria-label="Toggle theme">
      {dark ? <SunIcon /> : <MoonIcon />}
    </button>
  );
}
