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
  const fullPath = selectedFile ? selectedFile.path : "Nenhum ficheiro aberto";
  const fileName = selectedFile ? selectedFile.name : "Sem ficheiro";

  return (
    <div
      style={{
        flex: 1,
        display: "flex",
        flexDirection: "column",
        minWidth: 0,
        margin: 10,
        background: "#ffffff",
        border: "1px solid #dbe5f4",
        borderRadius: 12,
        overflow: "hidden",
      }}
    >
      <div
        style={{
          padding: "8px 10px",
          borderBottom: "1px solid #dbe5f4",
          display: "flex",
          alignItems: "center",
          gap: 6,
          background: "#f8fbff",
        }}
      >
        <div
          style={{
            flex: 1,
            minWidth: 0,
            overflow: "hidden",
            display: "grid",
            gap: 1,
          }}
          title={fullPath}
        >
          <div
            style={{
              fontSize: 12,
              fontWeight: 700,
              color: "#1e293b",
              whiteSpace: "nowrap",
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}
          >
            {fileName}
            {isDirty && " *"}
          </div>
          <div
            style={{
              fontSize: 11,
              color: "#64748b",
              whiteSpace: "nowrap",
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}
          >
            {fullPath}
          </div>
        </div>

        {selectedFile && (
          <button
            type="button"
            onClick={renameFile}
            style={{ boxShadow: "none", padding: "5px 9px", fontSize: 12 }}
          >
            Renomear
          </button>
        )}

        <button
          type="button"
          onClick={saveFile}
          disabled={!isDirty}
          style={{ boxShadow: "none", padding: "5px 9px", fontSize: 12 }}
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
