import { useEffect, useMemo, useState } from "react";

const SETTINGS_KEY = "appSettings";

const DEFAULT_SETTINGS = {
  theme: "light",
  fontSize: 15,
  lineHeight: 1.4,
};

function sanitizeSettings(input = {}) {
  return {
    theme: input.theme === "dark" ? "dark" : "light",
    fontSize: Number.isFinite(Number(input.fontSize))
      ? Math.min(20, Math.max(13, Number(input.fontSize)))
      : DEFAULT_SETTINGS.fontSize,
    lineHeight: Number.isFinite(Number(input.lineHeight))
      ? Math.min(1.8, Math.max(1.2, Number(input.lineHeight)))
      : DEFAULT_SETTINGS.lineHeight,
  };
}

export function useAppSettings() {
  const [settings, setSettings] = useState(() => {
    try {
      const raw = localStorage.getItem(SETTINGS_KEY);
      if (!raw) return DEFAULT_SETTINGS;
      return sanitizeSettings(JSON.parse(raw));
    } catch {
      return DEFAULT_SETTINGS;
    }
  });

  useEffect(() => {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
    document.documentElement.style.setProperty(
      "--app-font-size",
      `${settings.fontSize}px`,
    );
    document.documentElement.style.setProperty(
      "--app-line-height",
      String(settings.lineHeight),
    );
    document.documentElement.setAttribute("data-theme", settings.theme);
  }, [settings]);

  const actions = useMemo(
    () => ({
      setTheme: (theme) =>
        setSettings((prev) => sanitizeSettings({ ...prev, theme })),
      setFontSize: (fontSize) =>
        setSettings((prev) => sanitizeSettings({ ...prev, fontSize })),
      setLineHeight: (lineHeight) =>
        setSettings((prev) => sanitizeSettings({ ...prev, lineHeight })),
      reset: () => setSettings(DEFAULT_SETTINGS),
    }),
    [],
  );

  return {
    settings,
    ...actions,
  };
}
