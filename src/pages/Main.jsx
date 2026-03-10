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
import {
  getRootStateValue,
  setRootStateValue,
} from "../services/rootDataStore";
import { toRootOlmAbsolutePath } from "../services/dataPolicy";
import { open } from "@tauri-apps/plugin-dialog";

const ONBOARDING_KEY = "appOnboardingDone";
const OPEN_DAYS_KEY = "appOpenDays";
const LAST_OPENED_BY_DOMAIN_KEY = "lastOpenedByDomain.v1";
const LAST_USED_ROOT_KEY = "projectL.lastUsedRootPath";

function joinPath(base, segment) {
  if (!base) return segment;
  return `${base.replace(/[\\/]+$/, "")}\\${segment}`;
}

function getTopSegment(path) {
  const rel = normalize(path || "");
  const [first] = rel.split("/").filter(Boolean);
  return first || "";
}

export default function Main() {
  const [rootPath, setRootPath] = useState("");
  const [isOlmOpen, setIsOlmOpen] = useState(false);
  const [isPlannerOpen, setIsPlannerOpen] = useState(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [onboardingDone, setOnboardingDone] = useState(false);
  const [treeViewMode, setTreeViewMode] = useState("root");
  const [activeDomain, setActiveDomain] = useState("");
  const [tutorialDomainName, setTutorialDomainName] = useState("");
  const [busy, setBusy] = useState(false);
  const [onboardingError, setOnboardingError] = useState("");
  const [rootResolved, setRootResolved] = useState(false);
  const [openDaysCount, setOpenDaysCount] = useState(0);
  const [lastOpenedByDomain, setLastOpenedByDomain] = useState({});
  const {
    settings,
    setTheme,
    setFontSize,
    setLineHeight,
    reset: resetSettings,
  } = useAppSettings(rootPath);

  useEffect(() => {
    // Bootstrap with last used root so onboarding only appears on first run.
    const savedRoot = localStorage.getItem(LAST_USED_ROOT_KEY) || "";
    if (savedRoot) {
      setRootPath(savedRoot);
    }
    setRootResolved(true);
  }, []);

  useEffect(() => {
    let mounted = true;

    async function loadRootUiState() {
      if (!rootPath) {
        setOnboardingDone(false);
        setOpenDaysCount(0);
        setLastOpenedByDomain({});
        return;
      }

      const [onboardingRaw, openDaysRaw, lastOpenedRaw] = await Promise.all([
        getRootStateValue(rootPath, ONBOARDING_KEY, false),
        getRootStateValue(rootPath, OPEN_DAYS_KEY, []),
        getRootStateValue(rootPath, LAST_OPENED_BY_DOMAIN_KEY, {}),
      ]);

      if (!mounted) return;

      const day = new Date().toISOString().slice(0, 10);
      const existingDays = Array.isArray(openDaysRaw) ? openDaysRaw : [];
      const nextDays = existingDays.includes(day)
        ? existingDays
        : [...existingDays, day];

      setOnboardingDone(Boolean(onboardingRaw));
      setOpenDaysCount(nextDays.length);
      setLastOpenedByDomain(
        lastOpenedRaw && typeof lastOpenedRaw === "object" ? lastOpenedRaw : {},
      );

      if (!existingDays.includes(day)) {
        setRootStateValue(rootPath, OPEN_DAYS_KEY, nextDays).catch(() => {});
      }
    }

    loadRootUiState();
    return () => {
      mounted = false;
    };
  }, [rootPath]);

  useEffect(() => {
    if (!rootPath) return;
    loadOlmState(toRootOlmAbsolutePath(rootPath)).catch(() => {});
  }, [rootPath]);

  useEffect(() => {
    const autoSave = () => {
      if (!rootPath) return;
      saveOlmState(toRootOlmAbsolutePath(rootPath)).catch(() => {});
    };

    const intervalId = window.setInterval(autoSave, 60000);
    window.addEventListener("beforeunload", autoSave);

    return () => {
      window.clearInterval(intervalId);
      window.removeEventListener("beforeunload", autoSave);
      autoSave();
    };
  }, [rootPath]);

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
    requestDomainTransitionFeedback,
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

    setLastOpenedByDomain((prev) => {
      const next = {
        ...prev,
        [domain]: {
          path: normalize(selectedFile.path),
          name: selectedFile.name,
          openedAt: new Date().toISOString(),
        },
      };

      if (rootPath) {
        setRootStateValue(rootPath, LAST_OPENED_BY_DOMAIN_KEY, next).catch(
          () => {},
        );
      }

      return next;
    });
  }, [selectedFile, rootPath]);

  async function handleOpenNode(node) {
    const top = getTopSegment(node?.path);
    if (top && activeDomain && activeDomain !== top) {
      await requestDomainTransitionFeedback(activeDomain, top);
    }
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
      localStorage.setItem(LAST_USED_ROOT_KEY, folder);
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
      localStorage.setItem(LAST_USED_ROOT_KEY, newRoot);
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
    if (rootPath) {
      setRootStateValue(rootPath, ONBOARDING_KEY, true).catch(() => {});
    }
    setOnboardingDone(true);
  }

  if (!rootResolved) {
    return null;
  }

  if (!rootPath || !onboardingDone) {
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
        rootPath={rootPath}
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
        rootPath={rootPath}
      />

      <Sidebar
        tree={visibleTree}
        domains={domains}
        activeDomain={activeDomain}
        onSelectDomain={async (domainNode) => {
          if (!domainNode) return;
          if (activeDomain && activeDomain !== domainNode.name) {
            await requestDomainTransitionFeedback(
              activeDomain,
              domainNode.name,
            );
          }
          setActiveDomain(domainNode.name);
          setTreeViewMode("domain");
          await handleOpenNode(domainNode);
        }}
        onBackToRoot={async () => {
          if (activeDomain) {
            await requestDomainTransitionFeedback(activeDomain, "");
          }
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
            rootPath={rootPath}
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
