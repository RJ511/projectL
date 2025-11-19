import { useState } from "react";
import { readFile, writeFile, getTree, createFile, renameFile } from "../services/fs.service"
import { normalize } from "../services/pathTools";

export function useFileSystem(rootPath) {
  const [tree, setTree] = useState([]);
  const [selectedFile, setSelectedFile] = useState(null);
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

  async function openFile(node) {
    if (node.is_dir) return;

    const ext = getExt(node.name);

    setFileType(
      ["png", "jpg", "jpeg"].includes(ext)
        ? "image"
        : ["md", "markdown"].includes(ext)
        ? "markdown"
        : "text"
    );

    const relPath = normalize(node.path);
    const text = await readFile(rootPath, relPath);

    setSelectedFile(node);
    setContent(text);
    setIsDirty(false);
  }

  async function saveFile() {
    if (!selectedFile) return;
    await writeFile(rootPath, normalize(selectedFile.path), content);
    setIsDirty(false);
  }

  async function createMarkdown() {
    const name = prompt("Nome do ficheiro:");
    if (!name) return;

    await createFile(rootPath, name);
    await loadTree();
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
    renameSelected,
  };
}
