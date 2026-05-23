"use client";

import { Label, Switch } from "@heroui/react";
import { useTheme } from "next-themes";
import { useEffect, useState } from "react";

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();
  const [mounted, setMounted] = useState(false);

  useEffect(() => setMounted(true), []);
  if (!mounted) return null;

  return (
    <Switch
      isSelected={theme === "dark"}
      onChange={(on) => setTheme(on ? "dark" : "light")}
      size="sm"
    >
      <Switch.Control>
        <Switch.Thumb />
      </Switch.Control>
      <Switch.Content>
        <Label>{theme === "dark" ? "Dark" : "Light"}</Label>
      </Switch.Content>
    </Switch>
  );
}
