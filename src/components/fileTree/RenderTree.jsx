export function renderTree(
  nodes,
  openFile,
  selectedPath,
  onCreateFileInFolder,
  level = 0,
) {
  if (!Array.isArray(nodes)) return null;

  return nodes.map((node) => (
    <div key={node.path} style={{ marginLeft: level * 10 }}>
      <div
        style={{
          cursor: "pointer",
          padding: "2px 6px",
          borderRadius: 6,
          display: "flex",
          alignItems: "center",
          gap: 6,
          color: "#dbe7ff",
          fontSize: 12,
          lineHeight: 1.25,
          minHeight: 22,
          userSelect: "none",
          background: selectedPath === node.path ? "#1e293b" : "transparent",
        }}
        onClick={() => openFile(node)}
        onContextMenu={(e) => {
          e.preventDefault();
          openFile(node);
          if (node.is_dir && typeof onCreateFileInFolder === "function") {
            onCreateFileInFolder(node);
          }
        }}
        onMouseEnter={(e) => (e.currentTarget.style.background = "#1e293b")}
        onMouseLeave={(e) => {
          e.currentTarget.style.background =
            selectedPath === node.path ? "#1e293b" : "transparent";
        }}
        title={node.path}
      >
        <span style={{ opacity: 0.85, width: 12, textAlign: "center" }}>
          {node.is_dir ? "▸" : "•"}
        </span>
        <span
          style={{
            whiteSpace: "nowrap",
            overflow: "hidden",
            textOverflow: "ellipsis",
            flex: 1,
          }}
        >
          {node.name}
        </span>
      </div>

      {node.children &&
        renderTree(
          node.children,
          openFile,
          selectedPath,
          onCreateFileInFolder,
          level + 1,
        )}
    </div>
  ));
}
