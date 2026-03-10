import { renderTree } from "../../components/fileTree/RenderTree";

export default function Sidebar({
  tree = [],
  domains = [],
  activeDomain = "",
  onSelectDomain,
  onBackToRoot,
  rootLevelView = true,
  chooseDirectory,
  createFolder,
  createMarkdown,
  openFile,
  selectedNode,
  onNodeContextAction,
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
          onClick={createFolder}
          style={{ padding: "7px 8px", boxShadow: "none" }}
        >
          Criar Pasta
        </button>
        <button
          onClick={createMarkdown}
          style={{ padding: "7px 8px", boxShadow: "none" }}
        >
          Novo ficheiro
        </button>
      </div>
      <button
        onClick={() => {
          chooseDirectory();
        }}
        style={{ padding: "7px 8px", boxShadow: "none" }}
      >
        Mudar Pasta
      </button>

      <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
        <button
          onClick={() => onBackToRoot?.()}
          style={{ padding: "6px 8px", boxShadow: "none" }}
        >
          Root View
        </button>
        {domains.map((domainNode) => (
          <button
            key={domainNode.path}
            onClick={() => onSelectDomain?.(domainNode)}
            style={{
              padding: "6px 8px",
              boxShadow: "none",
              background:
                activeDomain === domainNode.name ? "#e2e8f0" : undefined,
            }}
          >
            {domainNode.name}
          </button>
        ))}
      </div>

      <h3 style={{ margin: "8px 0 2px", fontSize: 13, color: "#334155" }}>
        {rootLevelView ? "Root Level View" : `Domain View — ${activeDomain}`}
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
            {rootLevelView
              ? "Sem domínios no root"
              : "Sem conteúdo neste domínio"}
          </p>
        ) : (
          renderTree(tree, openFile, selectedNode?.path, onNodeContextAction)
        )}
      </div>
    </div>
  );
}
