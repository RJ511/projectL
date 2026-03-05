import { useEffect, useState } from "react";

const CONTEXT_ACTIONS = [
  { id: "new-file", label: "Novo ficheiro" },
  { id: "new-folder", label: "Nova pasta" },
  { id: "rename", label: "Renomear" },
  { id: "delete", label: "Eliminar" },
];

function RenderTree({
  nodes,
  openFile,
  selectedPath,
  onNodeContextAction,
  level = 0,
}) {
  const [menuState, setMenuState] = useState(null);

  useEffect(() => {
    function closeMenu() {
      setMenuState(null);
    }

    function onKeyDown(event) {
      if (event.key === "Escape") {
        closeMenu();
      }
    }

    window.addEventListener("click", closeMenu);
    window.addEventListener("scroll", closeMenu, true);
    window.addEventListener("resize", closeMenu);
    window.addEventListener("keydown", onKeyDown);

    return () => {
      window.removeEventListener("click", closeMenu);
      window.removeEventListener("scroll", closeMenu, true);
      window.removeEventListener("resize", closeMenu);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  if (!Array.isArray(nodes)) return null;

  const renderNodes = (list, depth) =>
    list.map((node) => (
      <div key={node.path} style={{ marginLeft: depth * 10 }}>
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
            const maxX = Math.max(8, window.innerWidth - 180);
            const maxY = Math.max(8, window.innerHeight - 170);
            setMenuState({
              node,
              x: Math.min(e.clientX, maxX),
              y: Math.min(e.clientY, maxY),
            });
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

        {node.children && renderNodes(node.children, depth + 1)}
      </div>
    ));

  return (
    <>
      {renderNodes(nodes, level)}

      {menuState && (
        <div
          style={{
            position: "fixed",
            top: menuState.y,
            left: menuState.x,
            background: "#0f172a",
            border: "1px solid #334155",
            borderRadius: 8,
            minWidth: 160,
            padding: 4,
            zIndex: 1200,
            boxShadow: "0 10px 25px rgba(2, 6, 23, 0.45)",
          }}
          onClick={(e) => e.stopPropagation()}
          onContextMenu={(e) => e.preventDefault()}
        >
          {CONTEXT_ACTIONS.map((action) => (
            <button
              key={action.id}
              type="button"
              onClick={() => {
                setMenuState(null);
                if (typeof onNodeContextAction === "function") {
                  onNodeContextAction(menuState.node, action.id);
                }
              }}
              style={{
                width: "100%",
                textAlign: "left",
                border: "none",
                borderRadius: 6,
                background: "transparent",
                color: action.id === "delete" ? "#fda4af" : "#e2e8f0",
                fontSize: 12,
                padding: "7px 8px",
                cursor: "pointer",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = "#1e293b";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = "transparent";
              }}
            >
              {action.label}
            </button>
          ))}
        </div>
      )}
    </>
  );
}

export function renderTree(
  nodes,
  openFile,
  selectedPath,
  onNodeContextAction,
  level = 0,
) {
  return (
    <RenderTree
      nodes={nodes}
      openFile={openFile}
      selectedPath={selectedPath}
      onNodeContextAction={onNodeContextAction}
      level={level}
    />
  );
}
