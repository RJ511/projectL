import CodeEditor from "../../components/editor/CodeEditor";

export default function EditorContainer({
  tree,
  selectedFile,
  content,
  setContent,
  isDirty,
  saveFile,
  renameFile,
  openFile,
}) {
  return (
    <div style={{ flex: 1, display: "flex", flexDirection: "column" }}>
      <div style={{ padding: 10, borderBottom: "1px solid #ddd" }}>
        {selectedFile ? selectedFile.path : "Nenhum ficheiro aberto"}
        {isDirty && " *"}

        {selectedFile && <button onClick={renameFile}>Renomear</button>}

        <button
          onClick={saveFile}
          disabled={!isDirty}
          style={{ float: "right" }}
        >
          Guardar
        </button>
      </div>

      <CodeEditor
        value={content}
        onChange={setContent}
        tree={tree}
        openFile={openFile}
      />
    </div>
  );
}
