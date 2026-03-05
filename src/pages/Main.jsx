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
import { loadOlmState, saveOlmState } from "../services/olm.service";
import { normalize } from "../services/pathTools";
import { open } from "@tauri-apps/plugin-dialog";

const ONBOARDING_KEY = "appOnboardingDone";
const OPEN_DAYS_KEY = "appOpenDays";
const LAST_OPENED_BY_DOMAIN_KEY = "lastOpenedByDomain.v1";

function joinPath(base, segment) {
  if (!base) return segment;
  return `${base.replace(/[\\/]+$/, "")}\\${segment}`;
}

function getTopSegment(path) {
  const rel = normalize(path || "");
  const [first] = rel.split("/").filter(Boolean);
  return first || "";
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
  const [treeViewMode, setTreeViewMode] = useState("root");
  const [activeDomain, setActiveDomain] = useState("");
  const [tutorialDomainName, setTutorialDomainName] = useState("");
  const [busy, setBusy] = useState(false);
  const [onboardingError, setOnboardingError] = useState("");
  const [openDaysCount, setOpenDaysCount] = useState(0);
  const [lastOpenedByDomain, setLastOpenedByDomain] = useState(() => {
    try {
      const raw = localStorage.getItem(LAST_OPENED_BY_DOMAIN_KEY);
      const parsed = raw ? JSON.parse(raw) : {};
      return parsed && typeof parsed === "object" ? parsed : {};
    } catch {
      return {};
    }
  });
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

  useEffect(() => {
    loadOlmState().catch(() => {});
  }, []);

  useEffect(() => {
    const autoSave = () => {
      saveOlmState().catch(() => {});
    };

    const intervalId = window.setInterval(autoSave, 60000);
    window.addEventListener("beforeunload", autoSave);

    return () => {
      window.clearInterval(intervalId);
      window.removeEventListener("beforeunload", autoSave);
      autoSave();
    };
  }, []);

  const {
    tree,
    selectedFile,
    selectedNode,
    nodeProfiles,
    learningAnalytics,
    content,
    isDirty,
    setContent,
    setIsDirty,
    loadTree,
    openFile,
    saveFile,
    saveNodeProfile,
    handleNodeContextAction,
    createFolderAtSelection,
    createMarkdown,
    createQuizTemplate,
    renameSelected,
    clearSelection,
  } = useFileSystem(rootPath);

  const selectedNodeProfile = useMemo(
    () => (selectedNode ? nodeProfiles[normalize(selectedNode.path)] : null),
    [nodeProfiles, selectedNode],
  );

  useEffect(() => {
    if (rootPath) {
      loadTree();
    }
  }, [rootPath]);

  const domains = useMemo(() => tree.filter((node) => node.is_dir), [tree]);

  useEffect(() => {
    if (!domains.length) {
      setActiveDomain("");
      return;
    }

    if (treeViewMode === "root") {
      return;
    }

    const exists = domains.some(
      (domainNode) => domainNode.name === activeDomain,
    );
    if (!exists) {
      setActiveDomain(domains[0].name);
    }
  }, [domains, activeDomain, treeViewMode]);

  const visibleTree = useMemo(() => {
    if (treeViewMode === "root") return domains;
    if (!activeDomain) return domains;
    const match = domains.find(
      (domainNode) => domainNode.name === activeDomain,
    );
    return match ? [match] : domains;
  }, [activeDomain, domains, treeViewMode]);

  const rootLevelView = treeViewMode === "root";
  const isItemLevel = Boolean(selectedFile);
  const currentLevel = isItemLevel ? "item" : rootLevelView ? "root" : "domain";
  const isDomainLevel = currentLevel === "domain";

  useEffect(() => {
    if (!isDomainLevel && isOlmOpen) {
      setIsOlmOpen(false);
    }
  }, [isDomainLevel, isOlmOpen]);

  useEffect(() => {
    if (!selectedFile?.path) return;
    const domain = getTopSegment(selectedFile.path);
    if (!domain) return;

    const next = {
      ...lastOpenedByDomain,
      [domain]: {
        path: normalize(selectedFile.path),
        name: selectedFile.name,
        openedAt: new Date().toISOString(),
      },
    };

    setLastOpenedByDomain(next);
    localStorage.setItem(LAST_OPENED_BY_DOMAIN_KEY, JSON.stringify(next));
  }, [selectedFile]);

  async function handleOpenNode(node) {
    const top = getTopSegment(node?.path);
    if (top) {
      setActiveDomain(top);
      setTreeViewMode("domain");
    }
    await openFile(node);
  }

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
          if (!isDomainLevel) return;
          setIsOlmOpen(true);
          setIsPlannerOpen(false);
        }}
        isOlmOpen={isOlmOpen && isDomainLevel}
        canOpenOlm={isDomainLevel}
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
        open={isOlmOpen && isDomainLevel}
        onToggle={() => {
          if (!isDomainLevel) return;
          setIsOlmOpen((prev) => !prev);
        }}
        onClose={() => setIsOlmOpen(false)}
        activeDomain={activeDomain}
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
        selectedNode={selectedNode}
        selectedNodeProfile={selectedNodeProfile}
        onSaveNodeProfile={saveNodeProfile}
        activeDomain={activeDomain}
      />

      <Sidebar
        tree={visibleTree}
        domains={domains}
        activeDomain={activeDomain}
        onSelectDomain={(domainNode) => {
          if (!domainNode) return;
          setActiveDomain(domainNode.name);
          setTreeViewMode("domain");
          handleOpenNode(domainNode);
        }}
        onBackToRoot={() => {
          setTreeViewMode("root");
          setActiveDomain("");
          clearSelection();
        }}
        rootLevelView={rootLevelView}
        chooseDirectory={chooseDirectory}
        createFolder={() =>
          createFolderAtSelection(selectedNode || selectedFile)
        }
        createMarkdown={createMarkdown}
        openFile={handleOpenNode}
        selectedNode={selectedNode}
        onNodeContextAction={handleNodeContextAction}
      />

      <div
        style={{
          flex: 1,
          minWidth: 0,
          display: "flex",
          flexDirection: "column",
        }}
      >
        {isItemLevel ? (
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
            openFile={handleOpenNode}
          />
        ) : (
          <KnowledgeLevels
            tree={tree}
            selectedFile={selectedFile}
            activeDomain={activeDomain}
            openDaysCount={openDaysCount}
            level={currentLevel}
            lastOpenedItem={
              activeDomain ? lastOpenedByDomain[activeDomain] || null : null
            }
            learningAnalytics={learningAnalytics}
          />
        )}
      </div>
    </div>
  );
}
