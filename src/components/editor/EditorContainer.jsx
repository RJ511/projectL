import CodeEditor from "./CodeEditor";

export default function EditorContainer({ selectedFile, content, setContent, isDirty, saveFile, renameFile }) {
  return (
    <div style={{ flex: 1, display: "flex", flexDirection: "column" }}>
      <div style={{ padding: 10, borderBottom: "1px solid #ddd" }}>
        {selectedFile ? selectedFile.path : "Nenhum ficheiro aberto"}
        {isDirty && " *"}

        {selectedFile && <button onClick={renameFile}>Renomear</button>}

        <button onClick={saveFile} disabled={!isDirty} style={{ float: "right" }}>
          Guardar
        </button>
      </div>

      <CodeEditor value={content} onChange={setContent} />
    </div>
  );
}
