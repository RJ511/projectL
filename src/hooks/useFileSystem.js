import { useState } from "react";
import {
  readFile,
  writeFile,
  getTree,
  createFile,
  renameFile,
} from "../services/fs.service";
import { ingestEvent, upsertConcept } from "../services/olm.service";
import { normalize } from "../services/pathTools";

const DEFAULT_OLM_CONCEPT = {
  id: "note_library_auto",
  name: "Note Library Auto",
  description: "Conceito automático para eventos do editor de ficheiros.",
};

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

export function useFileSystem(rootPath) {
  const [tree, setTree] = useState([]);
  const [selectedFile, setSelectedFile] = useState(null);
  const [selectedNode, setSelectedNode] = useState(null);
  const [content, setContent] = useState("");
  const [isDirty, setIsDirty] = useState(false);
  const [fileType, setFileType] = useState("text");

  async function loadTree() {
    if (!rootPath) return;
    const t = await getTree(rootPath);
    setTree(t);
  }

  function getExt(name) {
    const parts = name.split(".");
    return parts.length > 1 ? parts.pop().toLowerCase() : "";
  }

  async function trackOlmEvent(eventType, node, payload = {}) {
    if (!rootPath || !node || node.is_dir) return;

    const eventDate = new Date();

    try {
      await upsertConcept(DEFAULT_OLM_CONCEPT);

      await ingestEvent({
        event_id: `evt-${eventDate.getTime()}-${Math.floor(Math.random() * 100000)}`,
        timestamp: eventDate.toISOString(),
        source: "editor",
        event_type: eventType,
        content_id: null,
        concept_ids: [DEFAULT_OLM_CONCEPT.id],
        payload,
      });
    } catch (err) {
      console.warn("OLM auto-track skipped:", err);
    }
  }

  async function openFile(node) {
    setSelectedNode(node || null);
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
    createMarkdown,
    createMarkdownInFolder,
    createQuizTemplate,
    renameSelected,
  };
}
