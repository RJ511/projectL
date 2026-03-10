import { useEffect, useState } from "react";
import { SlidersHorizontal } from "lucide-react";
import {
  getOlmConfig,
  setOlmConfig,
  saveOlmState,
  loadOlmState,
  resetOlmState,
} from "../../services/olm.service";
import {
  getRootStateValue,
  setRootStateValue,
} from "../../services/rootDataStore";
import { toRootOlmAbsolutePath } from "../../services/dataPolicy";

const OLM_CONFIG_BY_DOMAIN_KEY = "olmConfigByDomain.v1";

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

function isValidConceptId(value) {
  const id = String(value || "").trim();
  if (!id) return false;
  return /^[a-z0-9]+(?:[.-][a-z0-9]+)*$/.test(id);
}

export default function AppSettingsPanel({
  open,
  onClose,
  settings,
  onThemeChange,
  onFontSizeChange,
  onLineHeightChange,
  onReset,
  selectedNode,
  selectedNodeProfile,
  onSaveNodeProfile,
  activeDomain,
  rootPath,
}) {
  const [olmConfig, setOlmConfigLocal] = useState(DEFAULT_OLM_CONFIG);
  const [olmMsg, setOlmMsg] = useState("");
  const [excludeInput, setExcludeInput] = useState("");
  const [nodeConceptId, setNodeConceptId] = useState("");
  const [nodeConceptName, setNodeConceptName] = useState("");
  const [nodeDifficulty, setNodeDifficulty] = useState(0.5);
  const [nodeMsg, setNodeMsg] = useState("");

  const [domainConfigMap, setDomainConfigMap] = useState({});

  function domainConfigKey() {
    const slug = (activeDomain || "")
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");
    return slug || "__root__";
  }

  function persistDomainConfigMap(nextMap) {
    setDomainConfigMap(nextMap);
    if (rootPath) {
      setRootStateValue(rootPath, OLM_CONFIG_BY_DOMAIN_KEY, nextMap).catch(
        () => {},
      );
    }
  }

  useEffect(() => {
    if (!open) return;

    let mounted = true;

    async function loadConfig() {
      const map =
        (await getRootStateValue(rootPath, OLM_CONFIG_BY_DOMAIN_KEY, {})) || {};
      if (!mounted) return;
      setDomainConfigMap(map && typeof map === "object" ? map : {});

      const key = domainConfigKey();
      const existing = map?.[key];

      if (existing) {
        setOlmConfigLocal(existing);
        setExcludeInput((existing.exclude_concepts || []).join(", "));
        setOlmConfig(existing).catch(() => {});
        return;
      }

      getOlmConfig()
        .then((cfg) => {
          if (!mounted) return;
          setOlmConfigLocal(cfg);
          setExcludeInput((cfg.exclude_concepts || []).join(", "));
          const nextMap = { ...map, [key]: cfg };
          persistDomainConfigMap(nextMap);
        })
        .catch(() => {});
    }

    loadConfig();
    return () => {
      mounted = false;
    };
  }, [open, activeDomain, rootPath]);

  useEffect(() => {
    if (!selectedNodeProfile) {
      setNodeConceptId("");
      setNodeConceptName("");
      setNodeDifficulty(0.5);
      return;
    }

    setNodeConceptId(selectedNodeProfile.conceptId || "");
    setNodeConceptName(selectedNodeProfile.conceptName || "");
    setNodeDifficulty(Number(selectedNodeProfile.difficulty ?? 0.5));
  }, [selectedNodeProfile]);

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
      const key = domainConfigKey();
      persistDomainConfigMap({
        ...domainConfigMap,
        [key]: parsed,
      });
      setOlmMsg("Config guardada ✓");
    } catch (e) {
      setOlmMsg(`Erro: ${e}`);
    }
    setTimeout(() => setOlmMsg(""), 3000);
  }

  async function handleSaveState() {
    try {
      const path = await saveOlmState(toRootOlmAbsolutePath(rootPath));
      setOlmMsg(`Estado guardado em ${path}`);
    } catch (e) {
      setOlmMsg(`Erro ao guardar: ${e}`);
    }
    setTimeout(() => setOlmMsg(""), 4000);
  }

  async function handleLoadState() {
    try {
      const path = await loadOlmState(toRootOlmAbsolutePath(rootPath));
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

  async function handleSaveNodeProfile() {
    if (!selectedNode || typeof onSaveNodeProfile !== "function") return;

    if (!isValidConceptId(nodeConceptId)) {
      setNodeMsg(
        "Concept ID inválido. Usa apenas minúsculas, números, '.' ou '-'.",
      );
      setTimeout(() => setNodeMsg(""), 3500);
      return;
    }

    onSaveNodeProfile(selectedNode.path, {
      kind: selectedNodeProfile?.kind,
      conceptId: nodeConceptId,
      conceptName: nodeConceptName,
      difficulty: nodeDifficulty,
    });

    setNodeMsg("Configuração do nó guardada ✓");
    setTimeout(() => setNodeMsg(""), 3000);
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
          <strong style={{ fontSize: 12 }}>
            Nó selecionado — Conceito & Dificuldade
          </strong>

          {!selectedNode ? (
            <span style={{ fontSize: 11, color: "var(--text-muted, #888)" }}>
              Seleciona um ficheiro/pasta na árvore para configurar.
            </span>
          ) : (
            <>
              <span style={{ fontSize: 11, color: "var(--text-muted, #888)" }}>
                {selectedNode.path}
              </span>
              <span style={{ fontSize: 11, color: "var(--text-muted, #888)" }}>
                Tipo:{" "}
                {selectedNodeProfile?.kind ||
                  (selectedNode.is_dir ? "folder" : "file")}
              </span>

              <label style={fieldStyle}>
                Concept ID
                <input
                  type="text"
                  value={nodeConceptId}
                  onChange={(e) => setNodeConceptId(e.target.value)}
                  placeholder="hiragana"
                />
              </label>

              <label style={fieldStyle}>
                Concept Name
                <input
                  type="text"
                  value={nodeConceptName}
                  onChange={(e) => setNodeConceptName(e.target.value)}
                  placeholder="Hiragana"
                />
              </label>

              <label style={fieldStyle}>
                Difficulty ({Number(nodeDifficulty).toFixed(2)})
                <input
                  type="range"
                  min={0}
                  max={1}
                  step={0.05}
                  value={nodeDifficulty}
                  onChange={(e) => setNodeDifficulty(Number(e.target.value))}
                />
              </label>

              <button type="button" onClick={handleSaveNodeProfile}>
                Guardar nó
              </button>

              {nodeMsg && (
                <span
                  style={{ fontSize: 11, color: "var(--text-muted, #888)" }}
                >
                  {nodeMsg}
                </span>
              )}
            </>
          )}
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
          <strong style={{ fontSize: 12 }}>
            OLM — Como recomendar o próximo estudo
          </strong>
          <span style={{ fontSize: 11, color: "var(--text-muted, #888)" }}>
            Escopo atual: {activeDomain || "Root (global)"}
          </span>
          <span style={{ fontSize: 11, color: "var(--text-muted, #888)" }}>
            Ajusta estes controlos para tornar as recomendações mais
            conservadoras ou mais agressivas.
          </span>

          <div style={rowStyle}>
            <label style={fieldStyle}>
              Foco em lacunas de domínio ({olmConfig.lambda.toFixed(2)})
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
              Penalizar incerteza dos pré-requisitos (
              {olmConfig.gamma.toFixed(2)})
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
              Prontidão mínima para estar “pronto” ({olmConfig.theta.toFixed(2)}
              )
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
              Peso de sinais metacognitivos (
              {olmConfig.meta_strength.toFixed(2)})
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
              Dureza do gate para tópicos menos prontos (
              {olmConfig.soft_gate_k.toFixed(1)})
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
              Reduzir prioridade de conceitos base (
              {olmConfig.root_penalty.toFixed(2)})
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
            Stop Mastery ({olmConfig.stop_mastery.toFixed(2)}) — oculta raiz
            acima deste valor
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
              style={{ fontSize: 12 }}
            />
          </label>

          <label
            style={{
              display: "flex",
              gap: 8,
              alignItems: "center",
              fontSize: 12,
            }}
          >
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
                  handleOlmChange(
                    "decay_half_life_days",
                    Number(e.target.value),
                  )
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
