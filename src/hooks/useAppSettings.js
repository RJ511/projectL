import { useEffect, useMemo, useState } from "react";
import {
  getRootStateValue,
  setRootStateValue,
} from "../services/rootDataStore";

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

export function useAppSettings(rootPath) {
  const [settings, setSettings] = useState(DEFAULT_SETTINGS);

  useEffect(() => {
    let mounted = true;

    async function loadSettings() {
      if (!rootPath) {
        setSettings(DEFAULT_SETTINGS);
        return;
      }
      const stored = await getRootStateValue(rootPath, SETTINGS_KEY, null);
      if (!mounted) return;
      if (!stored) {
        setSettings(DEFAULT_SETTINGS);
        return;
      }
      setSettings(sanitizeSettings(stored));
    }

    loadSettings();
    return () => {
      mounted = false;
    };
  }, [rootPath]);

  useEffect(() => {
    if (rootPath) {
      setRootStateValue(rootPath, SETTINGS_KEY, settings).catch(() => {});
    }
    document.documentElement.style.setProperty(
      "--app-font-size",
      `${settings.fontSize}px`,
    );
    document.documentElement.style.setProperty(
      "--app-line-height",
      String(settings.lineHeight),
    );
    document.documentElement.setAttribute("data-theme", settings.theme);
  }, [settings, rootPath]);

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
