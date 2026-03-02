import { SlidersHorizontal } from "lucide-react";

export default function AppSettingsPanel({
  open,
  onClose,
  settings,
  onThemeChange,
  onFontSizeChange,
  onLineHeightChange,
  onReset,
}) {
  return (
    <aside
      style={{
        position: "fixed",
        top: 0,
        right: 0,
        width: 320,
        height: "100vh",
        background: "var(--panel-bg)",
        borderLeft: "1px solid var(--panel-border)",
        boxShadow: "-12px 0 28px rgba(15,23,42,0.12)",
        transform: open ? "translateX(0)" : "translateX(100%)",
        transition: "transform 220ms ease",
        zIndex: 995,
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div
        style={{
          padding: "10px 12px",
          borderBottom: "1px solid var(--panel-border)",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <SlidersHorizontal size={16} />
          <strong style={{ fontSize: 14 }}>Configurações</strong>
        </div>
        <button type="button" onClick={onClose} style={{ padding: "3px 8px" }}>
          ×
        </button>
      </div>

      <div style={{ padding: 12, display: "grid", gap: 12 }}>
        <section
          style={{
            background: "var(--card-bg)",
            border: "1px solid var(--panel-border)",
            borderRadius: 10,
            padding: 10,
            display: "grid",
            gap: 8,
          }}
        >
          <label style={{ display: "grid", gap: 6, fontSize: 12 }}>
            Tema
            <select
              value={settings.theme}
              onChange={(e) => onThemeChange(e.target.value)}
            >
              <option value="light">Claro</option>
              <option value="dark">Escuro</option>
            </select>
          </label>

          <label style={{ display: "grid", gap: 6, fontSize: 12 }}>
            Tamanho da letra ({settings.fontSize}px)
            <input
              type="range"
              min={13}
              max={20}
              value={settings.fontSize}
              onChange={(e) => onFontSizeChange(Number(e.target.value))}
            />
          </label>

          <label style={{ display: "grid", gap: 6, fontSize: 12 }}>
            Altura da linha ({settings.lineHeight.toFixed(2)})
            <input
              type="range"
              min={1.2}
              max={1.8}
              step={0.05}
              value={settings.lineHeight}
              onChange={(e) => onLineHeightChange(Number(e.target.value))}
            />
          </label>

          <button
            type="button"
            onClick={onReset}
            style={{ width: "fit-content" }}
          >
            Repor padrão
          </button>
        </section>
      </div>
    </aside>
  );
}
