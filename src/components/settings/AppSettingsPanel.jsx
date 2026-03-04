import { useEffect, useState } from "react";
import { SlidersHorizontal } from "lucide-react";
import {
  getOlmConfig,
  setOlmConfig,
  saveOlmState,
  loadOlmState,
  resetOlmState,
} from "../../services/olm.service";

const DEFAULT_OLM_CONFIG = {
  lambda: 0.7,
  gamma: 0.35,
  theta: 0.5,
  min_readiness: 0.1,
  soft_gate_k: 1.6,
  meta_strength: 0.6,
  root_penalty: 0.12,
  stop_mastery: 0.85,
  exclude_concepts: [],
  uncertainty_formula: "standard",
  decay_enabled: false,
  decay_half_life_days: 30.0,
};

export default function AppSettingsPanel({
  open,
  onClose,
  settings,
  onThemeChange,
  onFontSizeChange,
  onLineHeightChange,
  onReset,
}) {
  const [olmConfig, setOlmConfigLocal] = useState(DEFAULT_OLM_CONFIG);
  const [olmMsg, setOlmMsg] = useState("");
  const [excludeInput, setExcludeInput] = useState("");

  useEffect(() => {
    if (!open) return;
    getOlmConfig()
      .then((cfg) => {
        setOlmConfigLocal(cfg);
        setExcludeInput((cfg.exclude_concepts || []).join(", "));
      })
      .catch(() => {});
  }, [open]);

  function handleOlmChange(key, value) {
    setOlmConfigLocal((prev) => ({ ...prev, [key]: value }));
  }

  async function handleSaveConfig() {
    const parsed = {
      ...olmConfig,
      exclude_concepts: excludeInput
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean),
    };
    try {
      await setOlmConfig(parsed);
      setOlmMsg("Config guardada ✓");
    } catch (e) {
      setOlmMsg(`Erro: ${e}`);
    }
    setTimeout(() => setOlmMsg(""), 3000);
  }

  async function handleSaveState() {
    try {
      const path = await saveOlmState();
      setOlmMsg(`Estado guardado em ${path}`);
    } catch (e) {
      setOlmMsg(`Erro ao guardar: ${e}`);
    }
    setTimeout(() => setOlmMsg(""), 4000);
  }

  async function handleLoadState() {
    try {
      const path = await loadOlmState();
      setOlmMsg(`Estado carregado de ${path}`);
    } catch (e) {
      setOlmMsg(`Erro ao carregar: ${e}`);
    }
    setTimeout(() => setOlmMsg(""), 4000);
  }

  async function handleResetState() {
    try {
      await resetOlmState();
      setOlmMsg("Estado OLM reiniciado ✓");
    } catch (e) {
      setOlmMsg(`Erro: ${e}`);
    }
    setTimeout(() => setOlmMsg(""), 3000);
  }

  const fieldStyle = { display: "grid", gap: 4, fontSize: 12 };
  const rowStyle = {
    display: "grid",
    gridTemplateColumns: "1fr 1fr",
    gap: 8,
  };

  return (
    <aside
      style={{
        position: "fixed",
        top: 0,
        right: 0,
        width: 360,
        height: "100vh",
        background: "var(--panel-bg)",
        borderLeft: "1px solid var(--panel-border)",
        boxShadow: "-12px 0 28px rgba(15,23,42,0.12)",
        transform: open ? "translateX(0)" : "translateX(100%)",
        transition: "transform 220ms ease",
        zIndex: 995,
        display: "flex",
        flexDirection: "column",
        overflowY: "auto",
      }}
    >
      <div
        style={{
          padding: "10px 12px",
          borderBottom: "1px solid var(--panel-border)",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          position: "sticky",
          top: 0,
          background: "var(--panel-bg)",
          zIndex: 1,
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
        {/* Appearance */}
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
          <strong style={{ fontSize: 12 }}>Aparência</strong>
          <label style={fieldStyle}>
            Tema
            <select
              value={settings.theme}
              onChange={(e) => onThemeChange(e.target.value)}
            >
              <option value="light">Claro</option>
              <option value="dark">Escuro</option>
            </select>
          </label>

          <label style={fieldStyle}>
            Tamanho da letra ({settings.fontSize}px)
            <input
              type="range"
              min={13}
              max={20}
              value={settings.fontSize}
              onChange={(e) => onFontSizeChange(Number(e.target.value))}
            />
          </label>

          <label style={fieldStyle}>
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

        {/* OLM Config */}
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
          <strong style={{ fontSize: 12 }}>OLM — Parâmetros</strong>

          <div style={rowStyle}>
            <label style={fieldStyle}>
              Lambda ({olmConfig.lambda.toFixed(2)})
              <input
                type="range"
                min={0}
                max={1}
                step={0.05}
                value={olmConfig.lambda}
                onChange={(e) =>
                  handleOlmChange("lambda", Number(e.target.value))
                }
              />
            </label>
            <label style={fieldStyle}>
              Gamma ({olmConfig.gamma.toFixed(2)})
              <input
                type="range"
                min={0}
                max={1}
                step={0.05}
                value={olmConfig.gamma}
                onChange={(e) =>
                  handleOlmChange("gamma", Number(e.target.value))
                }
              />
            </label>
          </div>

          <div style={rowStyle}>
            <label style={fieldStyle}>
              Theta ({olmConfig.theta.toFixed(2)})
              <input
                type="range"
                min={0}
                max={1}
                step={0.05}
                value={olmConfig.theta}
                onChange={(e) =>
                  handleOlmChange("theta", Number(e.target.value))
                }
              />
            </label>
            <label style={fieldStyle}>
              Meta Strength ({olmConfig.meta_strength.toFixed(2)})
              <input
                type="range"
                min={0}
                max={1}
                step={0.05}
                value={olmConfig.meta_strength}
                onChange={(e) =>
                  handleOlmChange("meta_strength", Number(e.target.value))
                }
              />
            </label>
          </div>

          <div style={rowStyle}>
            <label style={fieldStyle}>
              Soft Gate k ({olmConfig.soft_gate_k.toFixed(1)})
              <input
                type="range"
                min={0.5}
                max={4}
                step={0.1}
                value={olmConfig.soft_gate_k}
                onChange={(e) =>
                  handleOlmChange("soft_gate_k", Number(e.target.value))
                }
              />
            </label>
            <label style={fieldStyle}>
              Root Penalty ({olmConfig.root_penalty.toFixed(2)})
              <input
                type="range"
                min={0}
                max={1}
                step={0.05}
                value={olmConfig.root_penalty}
                onChange={(e) =>
                  handleOlmChange("root_penalty", Number(e.target.value))
                }
              />
            </label>
          </div>

          <label style={fieldStyle}>
            Stop Mastery ({olmConfig.stop_mastery.toFixed(2)}) — oculta raiz acima deste valor
            <input
              type="range"
              min={0.5}
              max={1}
              step={0.05}
              value={olmConfig.stop_mastery}
              onChange={(e) =>
                handleOlmChange("stop_mastery", Number(e.target.value))
              }
            />
          </label>

          <label style={fieldStyle}>
            Fórmula de incerteza
            <select
              value={olmConfig.uncertainty_formula}
              onChange={(e) =>
                handleOlmChange("uncertainty_formula", e.target.value)
              }
            >
              <option value="standard">Standard (1/N)</option>
              <option value="sqrt">Sqrt (1/√N)</option>
            </select>
          </label>

          <label style={fieldStyle}>
            Excluir conceitos (separados por vírgula)
            <input
              type="text"
              value={excludeInput}
              onChange={(e) => setExcludeInput(e.target.value)}
              placeholder="Note Library Auto, ..."
              style={{ fontSize: 12 }}
            />
          </label>

          <label style={{ display: "flex", gap: 8, alignItems: "center", fontSize: 12 }}>
            <input
              type="checkbox"
              checked={olmConfig.decay_enabled}
              onChange={(e) =>
                handleOlmChange("decay_enabled", e.target.checked)
              }
            />
            Ativar decay temporal
          </label>

          {olmConfig.decay_enabled && (
            <label style={fieldStyle}>
              Half-life (dias): {olmConfig.decay_half_life_days}
              <input
                type="range"
                min={7}
                max={180}
                step={7}
                value={olmConfig.decay_half_life_days}
                onChange={(e) =>
                  handleOlmChange("decay_half_life_days", Number(e.target.value))
                }
              />
            </label>
          )}

          <button type="button" onClick={handleSaveConfig}>
            Guardar config OLM
          </button>

          {olmMsg && (
            <span style={{ fontSize: 11, color: "var(--text-muted, #888)" }}>
              {olmMsg}
            </span>
          )}
        </section>

        {/* OLM State persistence */}
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
          <strong style={{ fontSize: 12 }}>OLM — Estado</strong>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <button type="button" onClick={handleSaveState}>
              💾 Guardar estado
            </button>
            <button type="button" onClick={handleLoadState}>
              📂 Carregar estado
            </button>
            <button
              type="button"
              onClick={handleResetState}
              style={{ color: "var(--danger, #e44)" }}
            >
              🗑 Reiniciar OLM
            </button>
          </div>
          {olmMsg && (
            <span style={{ fontSize: 11, color: "var(--text-muted, #888)" }}>
              {olmMsg}
            </span>
          )}
        </section>
      </div>
    </aside>
  );
}
