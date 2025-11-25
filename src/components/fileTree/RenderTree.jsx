export function renderTree(nodes, openFile, level = 0) {
  if (!Array.isArray(nodes)) return null;

  return nodes.map((node) => (
    <div key={node.path} style={{ marginLeft: level * 12 }}>
      <div
        style={{
          cursor: "pointer",
          padding: "4px 6px",
          borderRadius: 4,
          display: "flex",
          alignItems: "center",
          gap: 4,
          color: "#E0E0E0",
        }}
        onClick={() => openFile(node)}
        onMouseEnter={(e) => (e.currentTarget.style.background = "#2A2A2A")}
        onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
      >
        {/*{node.is_dir ? "📁" : "📄"}*/}
        {node.name}
      </div>

      {node.children && renderTree(node.children, openFile, level + 1)}
    </div>
  ));
}
