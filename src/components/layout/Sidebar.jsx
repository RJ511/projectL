import { renderTree } from "../../components/fileTree/RenderTree";

export default function Sidebar({
  tree = [],
  chooseDirectory,
  createMarkdown,
  openFile,
}) {
  return (
    <div
      style={{
        width: 280,
        padding: 10,
        borderRight: "1px solid #dbe5f4",
        background: "#ffffff",
        display: "flex",
        flexDirection: "column",
        gap: 6,
      }}
    >
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6 }}>
        <button
          onClick={chooseDirectory}
          style={{ padding: "7px 8px", boxShadow: "none" }}
        >
          Pasta
        </button>
        <button
          onClick={createMarkdown}
          style={{ padding: "7px 8px", boxShadow: "none" }}
        >
          Novo .md
        </button>
      </div>
      <button
        onClick={() => {
          localStorage.removeItem("lastRootPath");
          chooseDirectory();
        }}
        style={{ padding: "7px 8px", boxShadow: "none" }}
      >
        Mudar Pasta
      </button>

      <h3 style={{ margin: "8px 0 2px", fontSize: 13, color: "#334155" }}>
        Ficheiros
      </h3>
      <div
        style={{
          flex: 1,
          overflow: "auto",
          border: "1px solid #dbe5f4",
          borderRadius: 10,
          background: "#0f172a",
          padding: 6,
        }}
      >
        {tree.length === 0 ? (
          <p style={{ margin: 0, color: "#94a3b8", fontSize: 12 }}>
            Nenhuma pasta
          </p>
        ) : (
          renderTree(tree, openFile)
        )}
      </div>
    </div>
  );
}
