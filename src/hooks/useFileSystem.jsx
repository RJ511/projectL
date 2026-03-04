import { useEffect, useState } from "react";
import {
  readFile,
  writeFile,
  getTree,
  createFile,
  renameFile,
} from "../services/fs.service";
import { ingestEvent, upsertConcept } from "../services/olm.service";
import { normalize } from "../services/pathTools";

const NODE_LEARNING_PROFILES_KEY = "nodeLearningProfiles.v1";

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

function defaultDifficulty(kind) {
  if (kind === "domain") return 0.6;
  if (kind === "folder") return 0.55;
  return 0.5;
}

function buildDefaultProfile(node) {
  const relPath = normalize(node?.path || "");
  const segments = relPath.split("/").filter(Boolean);
  const kind = classifyNode(node);

  let conceptBase = node?.name || "Concept";

  if (kind === "file") {
    const parentFolder =
      segments.length > 1 ? segments[segments.length - 2] : "";
    conceptBase = parentFolder || stripExtension(node?.name || "") || "Concept";
  }

  const conceptId = slugify(conceptBase) || "concept-auto";
  const conceptName = conceptBase;

  return {
    path: relPath,
    kind,
    conceptId,
    conceptName,
    difficulty: defaultDifficulty(kind),
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

  useEffect(() => {
    localStorage.setItem(
      NODE_LEARNING_PROFILES_KEY,
      JSON.stringify(nodeProfiles || {}),
    );
  }, [nodeProfiles]);

  function getProfile(node) {
    if (!node?.path) return null;
    const key = normalize(node.path);
    return nodeProfiles[key] || null;
  }

  function ensureProfile(node) {
    if (!node?.path) return null;
    const key = normalize(node.path);
    const existing = nodeProfiles[key];
    if (existing) return existing;

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
      next.conceptId =
        slugify(next.conceptId || base.conceptId) || base.conceptId;
      next.conceptName = (
        next.conceptName ||
        base.conceptName ||
        "Concept"
      ).trim();
      next.difficulty = Math.max(
        0,
        Math.min(1, Number(next.difficulty ?? base.difficulty)),
      );

      return {
        ...prev,
        [key]: next,
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

  async function trackOlmEvent(eventType, node, payload = {}) {
    if (!rootPath || !node || node.is_dir) return;

    const eventDate = new Date();
    const profile =
      getProfile(node) || ensureProfile(node) || buildDefaultProfile(node);

    const concept = {
      id: profile.conceptId,
      name: profile.conceptName,
      description: `Conceito automático para ${profile.kind}: ${profile.path}`,
    };

    try {
      await upsertConcept(concept);

      await ingestEvent({
        event_id: `evt-${eventDate.getTime()}-${Math.floor(Math.random() * 100000)}`,
        timestamp: eventDate.toISOString(),
        source: "editor",
        event_type: eventType,
        content_id: null,
        concept_ids: [concept.id],
        payload: {
          ...payload,
          difficulty: profile.difficulty,
        },
      });
    } catch (err) {
      console.warn("OLM auto-track skipped:", err);
    }
  }

  async function openFile(node) {
    setSelectedNode(node || null);
    ensureProfile(node);
    if (node.is_dir) return;

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

    await trackOlmEvent("study_read", node, {
      duration_sec: 60,
      target_duration_sec: 120,
      confidence: 0.6,
    });
  }

  async function saveFile() {
    if (!selectedFile) return;
    await writeFile(rootPath, normalize(selectedFile.path), content);
    setIsDirty(false);

    await trackOlmEvent("practice_attempt", selectedFile, {
      correct: 1,
      total: 1,
      confidence: 0.7,
    });
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

  return {
    // estado
    tree,
    selectedFile,
    selectedNode,
    nodeProfiles,
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
    createQuizTemplate,
    renameSelected,
  };
}
