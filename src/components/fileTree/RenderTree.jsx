export function renderTree(nodes, openFile, level = 0) {
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

      {node.children && renderTree(node.children, openFile, level + 1)}
    </div>
  ));
}
