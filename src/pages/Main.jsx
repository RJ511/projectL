import { useEffect, useMemo, useState } from "react";
import Sidebar from "../components/layout/Sidebar";
import EditorContainer from "../components/editor/EditorContainer";
import ToolBar from "../components/editor/ToolBar";
import OlmPanel from "../components/olm/OlmPanel";
import StudyPlannerPanel from "../components/planner/StudyPlannerPanel";
import AppSettingsPanel from "../components/settings/AppSettingsPanel";
import HelloPage from "../components/onboarding/HelloPage";
import KnowledgeLevels from "../components/insights/KnowledgeLevels";
import { useFileSystem } from "../hooks/useFileSystem";
import { useAppSettings } from "../hooks/useAppSettings";
import { createFolder } from "../services/fs.service";
import { open } from "@tauri-apps/plugin-dialog";

const ONBOARDING_KEY = "appOnboardingDone";
const OPEN_DAYS_KEY = "appOpenDays";

function joinPath(base, segment) {
  if (!base) return segment;
  return `${base.replace(/[\\/]+$/, "")}\\${segment}`;
}

function recordOpenDay() {
  const day = new Date().toISOString().slice(0, 10);
  const existing = JSON.parse(localStorage.getItem(OPEN_DAYS_KEY) || "[]");
  if (!existing.includes(day)) {
    const next = [...existing, day];
    localStorage.setItem(OPEN_DAYS_KEY, JSON.stringify(next));
    return next.length;
  }
  return existing.length;
}

export default function Main() {
  const [rootPath, setRootPath] = useState("");
  const [isOlmOpen, setIsOlmOpen] = useState(false);
  const [isPlannerOpen, setIsPlannerOpen] = useState(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [onboardingDone, setOnboardingDone] = useState(
    localStorage.getItem(ONBOARDING_KEY) === "true",
  );
  const [tutorialDomainName, setTutorialDomainName] = useState("");
  const [busy, setBusy] = useState(false);
  const [onboardingError, setOnboardingError] = useState("");
  const [openDaysCount, setOpenDaysCount] = useState(0);
  const {
    settings,
    setTheme,
    setFontSize,
    setLineHeight,
    reset: resetSettings,
  } = useAppSettings();

  useEffect(() => {
    const saved = localStorage.getItem("lastRootPath");
    if (saved) {
      setRootPath(saved);
    }
    setOpenDaysCount(recordOpenDay());
  }, []);

  const {
    tree,
    selectedFile,
    content,
    isDirty,
    setContent,
    setIsDirty,
    loadTree,
    openFile,
    saveFile,
    createMarkdown,
    createQuizTemplate,
    renameSelected,
  } = useFileSystem(rootPath);

  useEffect(() => {
    if (rootPath) {
      loadTree();
    }
  }, [rootPath]);

  const domains = useMemo(() => tree.filter((node) => node.is_dir), [tree]);

  async function chooseDirectory() {
    const folder = await open({
      directory: true,
      multiple: false,
    });

    if (folder) {
      setRootPath(folder);
      localStorage.setItem("lastRootPath", folder);
      setOnboardingError("");
    }
  }

  async function createRootFolder() {
    setOnboardingError("");
    setBusy(true);

    try {
      const parent = await open({ directory: true, multiple: false });
      if (!parent) return;

      const rootName = prompt("Nome da pasta root:", "my-learning-root");
      if (!rootName || !rootName.trim()) return;

      await createFolder(parent, rootName.trim());
      const newRoot = joinPath(parent, rootName.trim());
      setRootPath(newRoot);
      localStorage.setItem("lastRootPath", newRoot);
    } catch (err) {
      setOnboardingError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function createFirstDomain() {
    if (!rootPath) {
      setOnboardingError("Seleciona primeiro a pasta root.");
      return;
    }

    const safeDomainName = tutorialDomainName.trim();
    if (!safeDomainName) {
      setOnboardingError("Escolhe um nome para o domínio.");
      return;
    }

    setOnboardingError("");
    setBusy(true);

    try {
      await createFolder(rootPath, safeDomainName);
      setTutorialDomainName("");
      await loadTree();
    } catch (err) {
      setOnboardingError(String(err));
    } finally {
      setBusy(false);
    }
  }

  function finishOnboarding() {
    localStorage.setItem(ONBOARDING_KEY, "true");
    setOnboardingDone(true);
  }

  if (!onboardingDone) {
    return (
      <div
        style={{
          display: "flex",
          height: "100vh",
          background: "var(--app-bg-gradient)",
          color: "var(--app-text-color)",
        }}
      >
        <HelloPage
          rootPath={rootPath}
          domains={domains}
          tutorialDomainName={tutorialDomainName}
          setTutorialDomainName={setTutorialDomainName}
          onOpenExistingRoot={chooseDirectory}
          onCreateRoot={createRootFolder}
          onCreateFirstDomain={createFirstDomain}
          onContinue={finishOnboarding}
          busy={busy}
          error={onboardingError}
        />
      </div>
    );
  }

  return (
    <div
      style={{
        display: "flex",
        height: "100vh",
        background: "var(--app-bg-gradient)",
        color: "var(--app-text-color)",
      }}
    >
      <ToolBar
        onOpenOlm={() => {
          setIsOlmOpen(true);
          setIsPlannerOpen(false);
        }}
        isOlmOpen={isOlmOpen}
        onOpenPlanner={() => {
          setIsPlannerOpen(true);
          setIsOlmOpen(false);
        }}
        isPlannerOpen={isPlannerOpen}
        onOpenSettings={() => setIsSettingsOpen(true)}
        isSettingsOpen={isSettingsOpen}
        onCreateQuizTemplate={createQuizTemplate}
      />
      <OlmPanel
        open={isOlmOpen}
        onToggle={() => setIsOlmOpen((prev) => !prev)}
        onClose={() => setIsOlmOpen(false)}
        hideHandleWhenClosed
      />

      <StudyPlannerPanel
        open={isPlannerOpen}
        onClose={() => setIsPlannerOpen(false)}
      />

      <AppSettingsPanel
        open={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
        settings={settings}
        onThemeChange={setTheme}
        onFontSizeChange={setFontSize}
        onLineHeightChange={setLineHeight}
        onReset={resetSettings}
      />

      <Sidebar
        tree={tree}
        chooseDirectory={chooseDirectory}
        createMarkdown={createMarkdown}
        openFile={openFile}
      />

      <div
        style={{
          flex: 1,
          minWidth: 0,
          display: "flex",
          flexDirection: "column",
        }}
      >
        <KnowledgeLevels
          tree={tree}
          selectedFile={selectedFile}
          content={content}
          openDaysCount={openDaysCount}
        />

        <EditorContainer
          selectedFile={selectedFile}
          content={content}
          setContent={(txt) => {
            setContent(txt);
            setIsDirty(true);
          }}
          isDirty={isDirty}
          saveFile={saveFile}
          renameFile={renameSelected}
          tree={tree}
          openFile={openFile}
        />
      </div>
    </div>
  );
}
