import { useState, useEffect } from "react";
import Sidebar from "../components/layout/Sidebar";
import EditorContainer from "../components/editor/EditorContainer";
import { useFileSystem } from "../hooks/useFileSystem";
import { open } from "@tauri-apps/plugin-dialog";

export default function Main() {
  const [rootPath, setRootPath] = useState("");
  useEffect(() => {
    const saved = localStorage.getItem("lastRootPath");
    if (saved) {
      setRootPath(saved);
    }
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
    renameSelected,
  } = useFileSystem(rootPath);

  useEffect(() => {
    if (rootPath) {loadTree();}
  }, [rootPath]);

  async function chooseDirectory() {
    const folder = await open({
      directory: true,
      multiple: false,
    });

    if (folder) {
      setRootPath(folder);
      localStorage.setItem("lastRootPath", folder);
    }
  }

  return (
    <div style={{ display: "flex", height: "100vh" }}>
      <Sidebar
        tree={tree}
        chooseDirectory={chooseDirectory}
        createMarkdown={createMarkdown}
        openFile={openFile}
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
  );
}
