import { useEffect, useRef, useState } from "react";
import {
  createFolder,
  readFile,
  writeFile,
  getTree,
  createFile,
  deletePath,
  renameFile,
} from "../services/fs.service";
import {
  ingestEvent,
  mapContentConcept,
  upsertConcept,
  upsertContentItem,
} from "../services/olm.service";
import { normalize } from "../services/pathTools";

const NODE_LEARNING_PROFILES_KEY = "nodeLearningProfiles.v1";
const LEARNING_ANALYTICS_KEY = "learningAnalytics.v1";
const INLINE_CONCEPT_REGEX = /(;{2,3})\s*([^;\n][^;\n]{0,80}?)\s*\1/g;

function slugify(value) {
  return (value || "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function makeTemplateContent(kind, title) {
  const heading = title || (kind === "teste" ? "Teste" : "Quiz");

  if (kind === "teste") {
    return `# ${heading}\n\n## Instruções\n- Duração:\n- Pontuação:\n- Tema:\n\n## Parte A — Escolha múltipla\n1. Pergunta 1\n- [ ] A\n- [ ] B\n- [ ] C\n- [ ] D\n\n2. Pergunta 2\n- [ ] A\n- [ ] B\n- [ ] C\n- [ ] D\n\n## Parte B — Resposta curta\n1.\n\n2.\n\n## Gabarito\n- 1:\n- 2:\n`;
  }

  return `# ${heading}\n\n## Metadados\n- Tema:\n- Nível:\n- Objetivo:\n\n## Questões\n1. Pergunta\n- [ ] Opção A\n- [ ] Opção B\n- [ ] Opção C\n- [ ] Opção D\n\n2. Pergunta\n- [ ] Opção A\n- [ ] Opção B\n- [ ] Opção C\n- [ ] Opção D\n\n## Respostas corretas\n- 1:\n- 2:\n\n## Explicações\n- 1:\n- 2:\n`;
}

function safeParseJson(raw, fallback) {
  try {
    return JSON.parse(raw);
  } catch {
    return fallback;
  }
}

function stripExtension(fileName) {
  const idx = fileName.lastIndexOf(".");
  if (idx <= 0) return fileName;
  return fileName.slice(0, idx);
}

function classifyNode(node) {
  if (!node?.is_dir) return "file";
  const depth = normalize(node.path).split("/").filter(Boolean).length;
  return depth <= 1 ? "domain" : "folder";
}

function domainFromPath(path) {
  const rel = normalize(path || "");
  const [first] = rel.split("/").filter(Boolean);
  return first || "";
}

function normalizeConceptId(rawConceptId, domainId) {
  const raw = String(rawConceptId || "").trim();
  const rawParts = raw
    .split(".")
    .map((part) => slugify(part))
    .filter(Boolean);

  const safeDomain = slugify(domainId || "") || "root";

  if (!rawParts.length) {
    return `${safeDomain}.concept-auto`;
  }

  if (rawParts[0] !== safeDomain) {
    return [safeDomain, ...rawParts].join(".");
  }

  return rawParts.join(".");
}

function defaultDifficulty(kind) {
  if (kind === "domain") return 0.6;
  if (kind === "folder") return 0.55;
  return 0.5;
}

function extractInlineConcepts(content, domainId) {
  const text = String(content || "");
  if (!text) return [];

  const byId = new Map();

  for (const match of text.matchAll(INLINE_CONCEPT_REGEX)) {
    const rawLabel = String(match?.[2] || "").trim();
    if (!rawLabel) continue;

    const conceptId = normalizeConceptId(`inline.${rawLabel}`, domainId);
    if (!conceptId) continue;

    if (!byId.has(conceptId)) {
      byId.set(conceptId, {
        id: conceptId,
        name: rawLabel,
        description: `Conceito inline extraído de marcação ;;;${rawLabel};;;`,
      });
    }
  }

  return Array.from(byId.values());
}

function buildDefaultProfile(node) {
  const relPath = normalize(node?.path || "");
  const segments = relPath.split("/").filter(Boolean);
  const kind = classifyNode(node);
  const domainName = domainFromPath(relPath);
  const domainId = slugify(domainName || "") || "root";

  let conceptBase = node?.name || "Concept";
  let conceptId = `${domainId}.concept-auto`;

  if (kind === "domain") {
    conceptBase = `${node?.name || "Domain"} Core`;
    conceptId = `${domainId}.core`;
  } else if (kind === "folder") {
    const scope = segments
      .slice(1)
      .map((part) => slugify(part))
      .filter(Boolean);
    conceptId = [domainId, ...scope, "core"].filter(Boolean).join(".");
  } else if (kind === "file") {
    const parentScope = segments
      .slice(1, -1)
      .map((part) => slugify(part))
      .filter(Boolean);
    const fileStem = slugify(stripExtension(node?.name || "")) || "note";
    conceptBase = stripExtension(node?.name || "") || "Concept";
    conceptId = [domainId, ...parentScope, fileStem].filter(Boolean).join(".");
  }
  const conceptName = conceptBase;

  return {
    path: relPath,
    kind,
    domainId,
    domainName,
    conceptId,
    conceptName,
    difficulty: defaultDifficulty(kind),
  };
}

function normalizeProfile(profile, node) {
  const base = profile || buildDefaultProfile(node);
  const domainName =
    base.domainName || domainFromPath(base.path || node?.path || "");
  const domainId = slugify(base.domainId || domainName || "") || "root";
  const conceptId = normalizeConceptId(
    base.conceptId || base.conceptName || "concept-auto",
    domainId,
  );

  return {
    ...base,
    domainId,
    domainName,
    conceptId,
    conceptName: (base.conceptName || "Concept").trim(),
    difficulty: Math.max(
      0,
      Math.min(1, Number(base.difficulty ?? defaultDifficulty(base.kind))),
    ),
  };
}

function defaultAnalytics() {
  return {
    sessions: [],
    fileOpenCount: {},
    fileTimeSec: {},
    domainTimeSec: {},
    domainResourceOpenCount: {},
    completionByFile: {},
    metaCognitiveProgress: [],
  };
}

function safeAnalytics(raw) {
  const fallback = defaultAnalytics();
  const parsed = safeParseJson(raw, fallback);
  return {
    ...fallback,
    ...(parsed && typeof parsed === "object" ? parsed : {}),
    sessions: Array.isArray(parsed?.sessions) ? parsed.sessions : [],
    fileOpenCount:
      parsed?.fileOpenCount && typeof parsed.fileOpenCount === "object"
        ? parsed.fileOpenCount
        : {},
    fileTimeSec:
      parsed?.fileTimeSec && typeof parsed.fileTimeSec === "object"
        ? parsed.fileTimeSec
        : {},
    domainTimeSec:
      parsed?.domainTimeSec && typeof parsed.domainTimeSec === "object"
        ? parsed.domainTimeSec
        : {},
    domainResourceOpenCount:
      parsed?.domainResourceOpenCount &&
      typeof parsed.domainResourceOpenCount === "object"
        ? parsed.domainResourceOpenCount
        : {},
    completionByFile:
      parsed?.completionByFile && typeof parsed.completionByFile === "object"
        ? parsed.completionByFile
        : {},
    metaCognitiveProgress: Array.isArray(parsed?.metaCognitiveProgress)
      ? parsed.metaCognitiveProgress
      : [],
  };
}

export function useFileSystem(rootPath) {
  const [tree, setTree] = useState([]);
  const [selectedFile, setSelectedFile] = useState(null);
  const [selectedNode, setSelectedNode] = useState(null);
  const [nodeProfiles, setNodeProfiles] = useState(() => {
    const saved = localStorage.getItem(NODE_LEARNING_PROFILES_KEY);
    const parsed = safeParseJson(saved, {});
    return parsed && typeof parsed === "object" ? parsed : {};
  });
  const [content, setContent] = useState("");
  const [isDirty, setIsDirty] = useState(false);
  const [fileType, setFileType] = useState("text");
  const [learningAnalytics, setLearningAnalytics] = useState(() =>
    safeAnalytics(localStorage.getItem(LEARNING_ANALYTICS_KEY)),
  );
  const sessionStartRef = useRef(Date.now());
  const activeFileRef = useRef({ path: null, domain: null, startedAt: 0 });

  useEffect(() => {
    localStorage.setItem(
      NODE_LEARNING_PROFILES_KEY,
      JSON.stringify(nodeProfiles || {}),
    );
  }, [nodeProfiles]);

  useEffect(() => {
    localStorage.setItem(
      LEARNING_ANALYTICS_KEY,
      JSON.stringify(learningAnalytics || defaultAnalytics()),
    );
  }, [learningAnalytics]);

  function updateAnalytics(mutator) {
    setLearningAnalytics((prev) => {
      const base = prev || defaultAnalytics();
      const next = mutator({ ...base }) || base;
      return next;
    });
  }

  function closeActiveFileTimer() {
    const active = activeFileRef.current;
    if (!active?.path || !active.startedAt) return;

    const elapsedSec = Math.max(
      0,
      Math.round((Date.now() - active.startedAt) / 1000),
    );
    if (elapsedSec <= 0) {
      activeFileRef.current = { path: null, domain: null, startedAt: 0 };
      return;
    }

    updateAnalytics((current) => {
      const fileTimeSec = { ...(current.fileTimeSec || {}) };
      const domainTimeSec = { ...(current.domainTimeSec || {}) };
      fileTimeSec[active.path] = (fileTimeSec[active.path] || 0) + elapsedSec;
      if (active.domain) {
        domainTimeSec[active.domain] =
          (domainTimeSec[active.domain] || 0) + elapsedSec;
      }
      return {
        ...current,
        fileTimeSec,
        domainTimeSec,
      };
    });

    activeFileRef.current = { path: null, domain: null, startedAt: 0 };
  }

  function startActiveFileTimer(node) {
    closeActiveFileTimer();
    const path = normalize(node?.path || "");
    const domain = domainFromPath(path);
    activeFileRef.current = {
      path,
      domain,
      startedAt: Date.now(),
    };
  }

  function closeSessionWindow() {
    closeActiveFileTimer();
    const startedAt = sessionStartRef.current;
    const endedAt = Date.now();
    const durationSec = Math.max(0, Math.round((endedAt - startedAt) / 1000));

    updateAnalytics((current) => {
      const sessions = [...(current.sessions || [])];
      sessions.push({
        startedAt: new Date(startedAt).toISOString(),
        endedAt: new Date(endedAt).toISOString(),
        durationSec,
      });

      if (sessions.length > 300) {
        sessions.splice(0, sessions.length - 300);
      }

      return {
        ...current,
        sessions,
      };
    });

    sessionStartRef.current = Date.now();
  }

  useEffect(() => {
    const onBeforeUnload = () => {
      closeSessionWindow();
    };

    window.addEventListener("beforeunload", onBeforeUnload);
    return () => {
      window.removeEventListener("beforeunload", onBeforeUnload);
      closeSessionWindow();
    };
  }, []);

  function getProfile(node) {
    if (!node?.path) return null;
    const key = normalize(node.path);
    return nodeProfiles[key] || null;
  }

  function ensureProfile(node) {
    if (!node?.path) return null;
    const key = normalize(node.path);
    const existing = nodeProfiles[key];
    if (existing) return normalizeProfile(existing, node);

    const fallback = buildDefaultProfile(node);
    setNodeProfiles((prev) => ({
      ...prev,
      [key]: fallback,
    }));
    return fallback;
  }

  function saveNodeProfile(nodePath, updates = {}) {
    const key = normalize(nodePath || "");
    if (!key) return;

    setNodeProfiles((prev) => {
      const base = prev[key] || {
        path: key,
        kind: "file",
        conceptId: "concept-auto",
        conceptName: "Concept",
        difficulty: 0.5,
      };

      const next = {
        ...base,
        ...updates,
      };

      next.path = key;
      const normalizedProfile = normalizeProfile(next, {
        path: key,
        is_dir: next.kind !== "file",
        name: key.split("/").filter(Boolean).pop() || "Concept",
      });

      return {
        ...prev,
        [key]: normalizedProfile,
      };
    });
  }

  function ensureDefaultsForTree(nodes) {
    setNodeProfiles((prev) => {
      let changed = false;
      const next = { ...prev };

      function walk(list) {
        if (!Array.isArray(list)) return;

        for (const node of list) {
          if (!node?.path) continue;
          const key = normalize(node.path);
          if (!next[key]) {
            next[key] = buildDefaultProfile(node);
            changed = true;
          } else {
            const normalizedExisting = normalizeProfile(next[key], node);
            if (
              normalizedExisting.conceptId !== next[key].conceptId ||
              normalizedExisting.domainId !== next[key].domainId ||
              normalizedExisting.domainName !== next[key].domainName
            ) {
              next[key] = normalizedExisting;
              changed = true;
            }
          }

          if (node.is_dir && Array.isArray(node.children)) {
            walk(node.children);
          }
        }
      }

      walk(nodes);
      return changed ? next : prev;
    });
  }

  async function loadTree() {
    if (!rootPath) return;
    const t = await getTree(rootPath);
    setTree(t);
    ensureDefaultsForTree(t);
  }

  function getExt(name) {
    const parts = name.split(".");
    return parts.length > 1 ? parts.pop().toLowerCase() : "";
  }

  async function ensureInlineConceptCatalog(node, editorContent = "") {
    if (!rootPath || !node || node.is_dir) return;

    const profile =
      getProfile(node) || ensureProfile(node) || buildDefaultProfile(node);
    const inlineConcepts = extractInlineConcepts(
      editorContent,
      profile.domainId,
    );

    for (const concept of inlineConcepts) {
      await upsertConcept(concept);
    }
  }

  async function trackOlmEvent(
    eventType,
    node,
    payload = {},
    editorContent = "",
  ) {
    if (!rootPath || !node || node.is_dir) return;

    const eventDate = new Date();
    const profile =
      getProfile(node) || ensureProfile(node) || buildDefaultProfile(node);

    const concept = {
      id: profile.conceptId,
      name: profile.conceptName,
      description: `Conceito automático para ${profile.kind}: ${profile.path}`,
    };
    const inlineConcepts = extractInlineConcepts(
      editorContent,
      profile.domainId,
    );
    const conceptIds = [
      concept.id,
      ...inlineConcepts.map((inlineConcept) => inlineConcept.id),
    ];
    const contentId = normalize(node.path);

    try {
      await upsertConcept(concept);
      for (const inlineConcept of inlineConcepts) {
        await upsertConcept(inlineConcept);
      }
      await upsertContentItem({
        id: contentId,
        item_type: "note",
        title: node.name,
      });
      await mapContentConcept({
        content_id: contentId,
        concept_id: concept.id,
        coverage_weight: 1.0,
      });

      await ingestEvent({
        event_id: `evt-${eventDate.getTime()}-${Math.floor(Math.random() * 100000)}`,
        timestamp: eventDate.toISOString(),
        source: "editor",
        event_type: eventType,
        content_id: contentId,
        concept_ids: conceptIds,
        payload: {
          ...payload,
          difficulty: profile.difficulty,
          file_path: contentId,
        },
      });

      updateAnalytics((current) => {
        const nextMeta = [...(current.metaCognitiveProgress || [])];
        const conf = Number(payload?.confidence);
        if (Number.isFinite(conf)) {
          nextMeta.push({
            at: eventDate.toISOString(),
            file: contentId,
            domain: domainFromPath(contentId),
            confidence: Math.max(0, Math.min(1, conf)),
            eventType,
          });
          if (nextMeta.length > 500) {
            nextMeta.splice(0, nextMeta.length - 500);
          }
        }

        const completionByFile = { ...(current.completionByFile || {}) };
        if (Number.isFinite(Number(payload?.completion_pct))) {
          completionByFile[contentId] = Math.max(
            0,
            Math.min(1, Number(payload.completion_pct)),
          );
        }

        return {
          ...current,
          completionByFile,
          metaCognitiveProgress: nextMeta,
        };
      });
    } catch (err) {
      console.warn("OLM auto-track skipped:", err);
    }
  }

  async function openFile(node) {
    setSelectedNode(node || null);
    ensureProfile(node);
    if (node.is_dir) {
      closeActiveFileTimer();
      setSelectedFile(null);
      setContent("");
      setIsDirty(false);
      return;
    }

    const ext = getExt(node.name);

    setFileType(
      ["png", "jpg", "jpeg"].includes(ext)
        ? "image"
        : ["md", "markdown"].includes(ext)
          ? "markdown"
          : "text",
    );

    const relPath = normalize(node.path);
    const text = await readFile(rootPath, relPath);

    setSelectedFile(node);
    setContent(text);
    setIsDirty(false);

    await ensureInlineConceptCatalog(node, text);

    const normalizedPath = normalize(node.path);
    const domainName = domainFromPath(normalizedPath);
    updateAnalytics((current) => {
      const fileOpenCount = { ...(current.fileOpenCount || {}) };
      const domainResourceOpenCount = {
        ...(current.domainResourceOpenCount || {}),
      };
      fileOpenCount[normalizedPath] = (fileOpenCount[normalizedPath] || 0) + 1;
      if (domainName) {
        if (!domainResourceOpenCount[domainName]) {
          domainResourceOpenCount[domainName] = {};
        }
        domainResourceOpenCount[domainName][normalizedPath] =
          (domainResourceOpenCount[domainName][normalizedPath] || 0) + 1;
      }

      return {
        ...current,
        fileOpenCount,
        domainResourceOpenCount,
      };
    });
    startActiveFileTimer(node);

    await trackOlmEvent(
      "review",
      node,
      {
        duration_sec: 60,
        target_duration_sec: 120,
        confidence: 0.6,
      },
      text,
    );
  }

  async function saveFile() {
    if (!selectedFile) return;
    await writeFile(rootPath, normalize(selectedFile.path), content);
    await ensureInlineConceptCatalog(selectedFile, content);
    setIsDirty(false);
  }

  async function createMarkdown() {
    const rawName = prompt("Nome do ficheiro (.md):");
    if (!rawName) return;

    const trimmedName = rawName.trim();
    if (!trimmedName) return;

    const name = trimmedName.toLowerCase().endsWith(".md")
      ? trimmedName
      : `${trimmedName}.md`;

    const targetNode = selectedNode || selectedFile;
    const basePath = targetNode?.is_dir
      ? normalize(targetNode.path)
      : normalize(targetNode?.path || "")
          .split("/")
          .slice(0, -1)
          .join("/");
    const relPath = basePath ? `${basePath}/${name}` : name;

    await createFile(rootPath, relPath);
    await loadTree();
  }

  async function createFolderAtSelection(node = null) {
    if (!rootPath) return;

    const rawName = prompt("Nome da pasta:");
    if (!rawName) return;

    const folderName = rawName.trim();
    if (!folderName) return;

    const targetNode = node || selectedNode || selectedFile;
    const basePath = targetNode?.is_dir
      ? normalize(targetNode.path)
      : normalize(targetNode?.path || "")
          .split("/")
          .slice(0, -1)
          .join("/");

    const relPath = basePath ? `${basePath}/${folderName}` : folderName;
    await createFolder(rootPath, relPath);
    await loadTree();
  }

  async function createMarkdownInFolder(node) {
    if (!node || !node.is_dir) return;

    setSelectedNode(node);

    const rawName = prompt(`Nome do ficheiro em ${node.name} (.md):`);
    if (!rawName) return;

    const trimmedName = rawName.trim();
    if (!trimmedName) return;

    const fileName = trimmedName.toLowerCase().endsWith(".md")
      ? trimmedName
      : `${trimmedName}.md`;

    const relPath = `${normalize(node.path)}/${fileName}`;

    await createFile(rootPath, relPath);
    await loadTree();
  }

  async function createQuizTemplate() {
    if (!rootPath) {
      alert("Seleciona primeiro a pasta root.");
      return;
    }

    const rawType = prompt("Tipo de template (quiz ou teste):", "quiz");

    if (!rawType) return;

    const normalized = rawType.trim().toLowerCase();
    const kind = normalized === "teste" ? "teste" : "quiz";

    const title = prompt(
      "Título do ficheiro:",
      kind === "teste" ? "Teste rápido" : "Quiz rápido",
    );

    if (!title || !title.trim()) return;

    const datePart = new Date().toISOString().slice(0, 10);
    const cleanTitle = slugify(title) || kind;

    const targetNode = selectedNode || selectedFile;
    const selectedRelPath = targetNode?.path || "";
    const segments = selectedRelPath.split(/[\\/]/).filter(Boolean);
    const folderPath = targetNode?.is_dir
      ? segments.join("/")
      : segments.slice(0, -1).join("/");

    const fileName = `${kind}-${datePart}-${cleanTitle}.md`;
    const relPath = folderPath ? `${folderPath}/${fileName}` : fileName;
    const templateContent = makeTemplateContent(kind, title.trim());

    await createFile(rootPath, relPath);
    await writeFile(rootPath, relPath, templateContent);
    await loadTree();
    await openFile({
      name: fileName,
      path: relPath,
      is_dir: false,
    });
  }

  async function renameSelected() {
    if (!selectedFile) return;

    const newName = prompt("Novo nome:", selectedFile.name);
    if (!newName) return;

    await renameFile(rootPath, normalize(selectedFile.path), newName);
    setSelectedFile(null);
    setContent("");
    await loadTree();
  }

  async function renameNode(node) {
    if (!node?.path) return;
    const newName = prompt("Novo nome:", node.name);
    if (!newName || !newName.trim()) return;

    await renameFile(rootPath, normalize(node.path), newName.trim());
    if (selectedFile?.path === node.path || selectedNode?.path === node.path) {
      setSelectedFile(null);
      setSelectedNode(null);
      setContent("");
      setIsDirty(false);
    }
    await loadTree();
  }

  async function deleteNode(node) {
    if (!node?.path) return;
    const ok = window.confirm(
      `Eliminar ${node.name}? Esta ação é irreversível.`,
    );
    if (!ok) return;

    await deletePath(rootPath, normalize(node.path));
    if (selectedFile?.path === node.path || selectedNode?.path === node.path) {
      clearSelection();
    }
    await loadTree();
  }

  async function handleNodeContextAction(node, action) {
    if (!node || !action) return;

    if (action === "new-file") {
      if (node.is_dir) {
        await createMarkdownInFolder(node);
      } else {
        const parent = normalize(node.path).split("/").slice(0, -1).join("/");
        await createMarkdownInFolder({
          ...node,
          is_dir: true,
          path: parent,
          name: parent || "/",
        });
      }
      return;
    }

    if (action === "new-folder") {
      await createFolderAtSelection(node);
      return;
    }

    if (action === "rename") {
      await renameNode(node);
      return;
    }

    if (action === "delete") {
      await deleteNode(node);
    }
  }

  function clearSelection() {
    closeActiveFileTimer();
    setSelectedNode(null);
    setSelectedFile(null);
    setContent("");
    setIsDirty(false);
  }

  return {
    // estado
    tree,
    selectedFile,
    selectedNode,
    nodeProfiles,
    learningAnalytics,
    content,
    isDirty,
    fileType,

    // setters
    setContent,
    setIsDirty,

    // ações
    loadTree,
    openFile,
    saveFile,
    saveNodeProfile,
    getNodeProfile: getProfile,
    createMarkdown,
    createMarkdownInFolder,
    createFolderAtSelection,
    createQuizTemplate,
    renameSelected,
    renameNode,
    deleteNode,
    handleNodeContextAction,
    clearSelection,
  };
}
