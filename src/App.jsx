import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import MarkdownEditor from "./MarkdownEditor";

function App() {
  const [rootPath, setRootPath] = useState("");
  const [tree, setTree] = useState([]);
  const [selectedFile, setSelectedFile] = useState(null);
  const [content, setContent] = useState("");
  const [isDirty, setIsDirty] = useState(false);
  const [fileType, setFileType] = useState("text");

  // -------------------------------
  // Load file tree when folder changes
  // -------------------------------
  useEffect(() => {
    if (rootPath) loadTree();
  }, [rootPath]);

  async function loadTree() {
    try {
      const result = await invoke("get_tree", { root: rootPath });
      setTree(result);
    } catch (err) {
      console.error("Erro ao carregar árvore:", err);
    }
  }

  function getExt(name) {
    const parts = name.split(".");
    return parts.length > 1 ? parts.pop().toLowerCase() : "";
  }

  // -------------------------------
  // Escolher diretório — TAURI 2
  // -------------------------------
  async function chooseDirectory() {
    try {
      const folder = await open({
        directory: true,
        multiple: false,
      });

      if (folder) {
        setRootPath(folder);
        loadTree();
      }
    } catch (e) {
      console.error("Erro ao escolher diretório:", e);
    }
  }


  // -------------------------------
  // Abrir ficheiro .md ou imagem
  // -------------------------------
  async function openFile(node) {
    if (node.is_dir) return;

    const ext = getExt(node.name);

    if (["png", "jpg", "jpeg"].includes(ext)) {
      setFileType("image");
    } else if (["md", "markdown"].includes(ext)) {
      setFileType("markdown");
    } else if (ext === "txt") {
      setFileType("text");
    } else {
      setFileType("text");
    }

    try {
      const relPath = node.path.replace(/\\/g, "/");

      const text = await invoke("read_file", {
        root: rootPath,
        relPath: relPath,
      });

      setSelectedFile(node);
      setContent(text);
      setIsDirty(false);
    } catch (err) {
      console.error("Erro ao abrir ficheiro:", err);
    }
  }

  // -------------------------------
  // Criar ficheiro .md
  // -------------------------------
  async function createMarkdownFile() {
    if (!rootPath) return alert("Escolhe um diretório primeiro!");

    const name = prompt("Nome do ficheiro (ex: nota.md):");
    if (!name) return;

    try {
      await invoke("create_file", {
        root: rootPath,
        name,
      });
      loadTree();
    } catch (err) {
      alert("Erro ao criar ficheiro: " + err);
    }
  }

  // -------------------------------
  // Renomear ficheiro
  // -------------------------------
  async function renameFile() {
    if (!selectedFile) return;

    const newName = prompt("Novo nome:", selectedFile.name);
    if (!newName) return;

    try {
      await invoke("rename_file", {
        root: rootPath,
        old_path: selectedFile.path.replace(/\\/g, "/"),
        new_name: newName,
      });

      setSelectedFile(null);
      setContent("");
      loadTree();
    } catch (err) {
      alert("Erro ao renomear: " + err);
    }
  }

  // -------------------------------
  // Gravar ficheiro editado
  // -------------------------------
  async function saveFile() {
    if (!selectedFile) return;

    try {
      await invoke("write_file", {
        root: rootPath,
        relPath: selectedFile.path.replace(/\\/g, "/"),
        content,
      });

      setIsDirty(false);
    } catch (err) {
      console.error("Erro ao gravar:", err);
    }
  }

  // -------------------------------
  // Renderizar árvore de ficheiros
  // -------------------------------
  function renderTree(nodes, level = 0) {
    return nodes.map((node) => (
      <div key={node.path} style={{ marginLeft: level * 10 }}>
        <div
          style={{
            cursor: node.is_dir ? "default" : "pointer",
            fontWeight: node.is_dir ? "bold" : "normal",
          }}
          onClick={() => openFile(node)}
        >
          {node.is_dir ? "📁 " : "📄 "}
          {node.name}
        </div>

        {node.children && renderTree(node.children, level + 1)}
      </div>
    ));
  }

  // ---------------------------------------------------
  // UI PRINCIPAL
  // ---------------------------------------------------
  return (
    <div style={{ display: "flex", height: "100vh" }}>
      {/* Sidebar esquerda */}
      <div
        style={{
          width: "250px",
          padding: "10px",
          borderRight: "1px solid #ddd",
          display: "flex",
          flexDirection: "column",
          gap: "10px",
        }}
      >
        <button onClick={chooseDirectory}>Escolher Pasta</button>
        <button onClick={createMarkdownFile}>Novo .md</button>

        <h3>Ficheiros</h3>
        {rootPath ? renderTree(tree) : <p>Escolhe uma pasta</p>}
      </div>

      {/* Editor */}
      <div style={{ flex: 1, display: "flex", flexDirection: "column" }}>
        {/* Header */}
        <div style={{ padding: "10px", borderBottom: "1px solid #ddd" }}>
          {selectedFile ? selectedFile.path : "Nenhum ficheiro aberto"}
          {isDirty && " *"}

          {selectedFile && (
            <button onClick={renameFile} style={{ marginLeft: "20px" }}>
              Renomear
            </button>
          )}

          <button
            style={{ float: "right" }}
            onClick={saveFile}
            disabled={!isDirty}
          >
            Guardar
          </button>
        </div>

        <div style={{ flex: 1 }}>
          <MarkdownEditor
            value={content}
            onChange={(txt) => {
              setContent(txt);
              setIsDirty(true);
            }}
          />
        </div>

      </div>
    </div>
  );
}

export default App;
