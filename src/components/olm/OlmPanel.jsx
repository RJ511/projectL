import { useEffect, useMemo, useState } from "react";
import { ChevronLeft, ChevronRight, Sparkles, Target } from "lucide-react";
import {
  addEdge,
  getExplain,
  getOlmState,
  ingestEvent,
  listConcepts,
  nextToStudy,
  upsertConcept,
} from "../../services/olm.service";

function barStyle(value, tone = "#3b82f6") {
  const width = `${Math.max(0, Math.min(100, value * 100))}%`;

  return {
    background: tone,
    width,
    height: 6,
    borderRadius: 999,
    transition: "width 200ms ease",
  };
}

export default function OlmPanel({
  open = false,
  onToggle,
  onClose,
  hideHandleWhenClosed = false,
}) {
  const [conceptId, setConceptId] = useState("");
  const [conceptName, setConceptName] = useState("");
  const [prereqId, setPrereqId] = useState("");
  const [targetId, setTargetId] = useState("");
  const [eventType, setEventType] = useState("practice_attempt");
  const [eventConceptId, setEventConceptId] = useState("");
  const [correct, setCorrect] = useState("1");
  const [total, setTotal] = useState("1");
  const [confidence, setConfidence] = useState("0.7");

  const [concepts, setConcepts] = useState([]);
  const [stateRows, setStateRows] = useState([]);
  const [recommendations, setRecommendations] = useState([]);
  const [selectedExplainId, setSelectedExplainId] = useState("");
  const [explainRows, setExplainRows] = useState([]);
  const [error, setError] = useState("");
  const drawerLeft = 48;
  const drawerWidth = 296;

  const summary = useMemo(() => {
    if (stateRows.length === 0) {
      return {
        conceptsCount: 0,
        avgMastery: 0,
        avgUncertainty: 0,
      };
    }

    const masterySum = stateRows.reduce((acc, row) => acc + row.mastery, 0);
    const uncertaintySum = stateRows.reduce(
      (acc, row) => acc + row.uncertainty,
      0,
    );

    return {
      conceptsCount: stateRows.length,
      avgMastery: masterySum / stateRows.length,
      avgUncertainty: uncertaintySum / stateRows.length,
    };
  }, [stateRows]);

  async function refresh() {
    setError("");
    try {
      const [conceptList, stateList, nextList] = await Promise.all([
        listConcepts(),
        getOlmState(),
        nextToStudy({ top: 5, lambda: 0.7, readinessThreshold: 0.5 }),
      ]);

      setConcepts(conceptList);
      setStateRows(stateList);
      setRecommendations(nextList);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  async function onAddConcept(e) {
    e.preventDefault();
    setError("");

    if (!conceptId.trim() || !conceptName.trim()) {
      setError("ID e nome são obrigatórios.");
      return;
    }

    try {
      await upsertConcept({
        id: conceptId.trim(),
        name: conceptName.trim(),
        description: null,
      });
      setConceptId("");
      setConceptName("");
      await refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  async function onAddEdge(e) {
    e.preventDefault();
    setError("");

    if (!prereqId || !targetId) {
      setError("Escolhe pré e alvo.");
      return;
    }

    try {
      await addEdge({ prereq_id: prereqId, target_id: targetId });
      setPrereqId("");
      setTargetId("");
      await refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  async function onIngestEvent(e) {
    e.preventDefault();
    setError("");

    if (!eventConceptId) {
      setError("Escolhe conceito.");
      return;
    }

    const now = new Date();
    const safeCorrect = Number(correct);
    const safeTotal = Number(total);
    const safeConfidence = Number(confidence);

    try {
      await ingestEvent({
        event_id: `evt-${now.getTime()}`,
        timestamp: now.toISOString(),
        source: "manual",
        event_type: eventType,
        content_id: null,
        concept_ids: [eventConceptId],
        payload: {
          correct: Number.isFinite(safeCorrect) ? safeCorrect : 0,
          total: Number.isFinite(safeTotal) && safeTotal > 0 ? safeTotal : 1,
          confidence: Number.isFinite(safeConfidence) ? safeConfidence : 0.7,
        },
      });

      await refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  async function onSelectExplain(conceptRowId) {
    setSelectedExplainId(conceptRowId);
    setError("");

    try {
      const rows = await getExplain(conceptRowId, 5);
      setExplainRows(rows);
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <>
      {(open || !hideHandleWhenClosed) && (
        <button
          type="button"
          title={open ? "Fechar OLM" : "Abrir OLM"}
          onClick={onToggle}
          style={{
            position: "fixed",
            left: open ? drawerLeft + drawerWidth : 56,
            top: "50%",
            transform: "translateY(-50%)",
            zIndex: 1000,
            width: 30,
            height: 78,
            borderRadius: "0 10px 10px 0",
            border: "1px solid #d0dbef",
            borderLeft: "none",
            background: "#ffffff",
            boxShadow: "0 10px 24px rgba(15,23,42,0.14)",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            gap: 4,
            color: "#334155",
            fontSize: 10,
            fontWeight: 700,
            letterSpacing: 0.6,
            transition: "left 220ms ease",
          }}
        >
          {open ? <ChevronLeft size={14} /> : <ChevronRight size={14} />}
          <span
            style={{ writingMode: "vertical-rl", transform: "rotate(180deg)" }}
          >
            OLM
          </span>
        </button>
      )}

      <aside
        style={{
          position: "fixed",
          top: 0,
          left: drawerLeft,
          width: drawerWidth,
          height: "100vh",
          background: "#f8fbff",
          borderRight: "1px solid #d0dbef",
          boxShadow: "10px 0 28px rgba(15,23,42,0.12)",
          zIndex: 999,
          transform: open ? "translateX(0)" : "translateX(calc(-100% - 64px))",
          opacity: open ? 1 : 0,
          visibility: open ? "visible" : "hidden",
          pointerEvents: open ? "auto" : "none",
          transition:
            "transform 220ms ease, opacity 180ms ease, visibility 0s linear 220ms",
          display: "flex",
          flexDirection: "column",
        }}
      >
        <div
          style={{
            padding: "8px 10px",
            borderBottom: "1px solid #dbe5f4",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            background: "#ffffff",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <Sparkles size={14} color="#2563eb" />
            <strong style={{ fontSize: 13 }}>OLM</strong>
          </div>
          <div style={{ display: "flex", gap: 6 }}>
            <button
              type="button"
              onClick={refresh}
              style={{ padding: "4px 7px", boxShadow: "none" }}
            >
              ⟳
            </button>
            <button
              type="button"
              onClick={onClose}
              style={{ padding: "4px 7px", boxShadow: "none" }}
            >
              ×
            </button>
          </div>
        </div>

        <div style={{ padding: 8, overflow: "auto", display: "grid", gap: 8 }}>
          <div
            style={{
              background: "#ffffff",
              border: "1px solid #dbe5f4",
              borderRadius: 10,
              padding: 8,
            }}
          >
            <div style={{ display: "flex", gap: 6, marginBottom: 6 }}>
              <div
                style={{
                  flex: 1,
                  background: "#eef4ff",
                  borderRadius: 8,
                  padding: 6,
                }}
              >
                <div style={{ fontSize: 11, color: "#64748b" }}>Conceitos</div>
                <div style={{ fontWeight: 700, fontSize: 13 }}>
                  {summary.conceptsCount}
                </div>
              </div>
              <div
                style={{
                  flex: 1,
                  background: "#ecfeff",
                  borderRadius: 8,
                  padding: 6,
                }}
              >
                <div style={{ fontSize: 11, color: "#64748b" }}>Mastery</div>
                <div style={{ fontWeight: 700, fontSize: 13 }}>
                  {(summary.avgMastery * 100).toFixed(0)}%
                </div>
              </div>
              <div
                style={{
                  flex: 1,
                  background: "#fff7ed",
                  borderRadius: 8,
                  padding: 6,
                }}
              >
                <div style={{ fontSize: 11, color: "#64748b" }}>Unc.</div>
                <div style={{ fontWeight: 700, fontSize: 13 }}>
                  {(summary.avgUncertainty * 100).toFixed(0)}%
                </div>
              </div>
            </div>
            <div style={{ fontSize: 10, color: "#6b7280" }}>
              Visão rápida do estado.
            </div>
          </div>

          <div
            style={{
              background: "#ffffff",
              border: "1px solid #dbe5f4",
              borderRadius: 10,
              padding: 8,
              display: "grid",
              gap: 6,
            }}
          >
            <strong style={{ fontSize: 12 }}>Estado</strong>
            {stateRows.length === 0 ? (
              <div style={{ fontSize: 12, color: "#6b7280" }}>Sem dados.</div>
            ) : (
              stateRows.map((row) => (
                <button
                  type="button"
                  key={row.concept_id}
                  onClick={() => onSelectExplain(row.concept_id)}
                  style={{
                    textAlign: "left",
                    padding: 7,
                    borderRadius: 8,
                    border: "1px solid #e2e8f0",
                    background: "#f8fbff",
                    boxShadow: "none",
                  }}
                >
                  <div
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      marginBottom: 3,
                    }}
                  >
                    <span style={{ fontWeight: 600, fontSize: 12 }}>
                      {row.name}
                    </span>
                    <span
                      style={{
                        fontSize: 10,
                        padding: "2px 6px",
                        borderRadius: 999,
                        background:
                          row.uncertainty > 0.35 ? "#fee2e2" : "#dcfce7",
                        color: row.uncertainty > 0.35 ? "#b91c1c" : "#166534",
                      }}
                    >
                      u {(row.uncertainty * 100).toFixed(0)}%
                    </span>
                  </div>
                  <div
                    style={{
                      background: "#dbe7fb",
                      borderRadius: 999,
                      height: 6,
                    }}
                  >
                    <div style={barStyle(row.mastery, "#2563eb")} />
                  </div>
                </button>
              ))
            )}
          </div>

          <div
            style={{
              background: "#ffffff",
              border: "1px solid #dbe5f4",
              borderRadius: 10,
              padding: 8,
              display: "grid",
              gap: 6,
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
              <Target size={13} color="#0ea5e9" />
              <strong style={{ fontSize: 12 }}>Next</strong>
            </div>
            {recommendations.length === 0 ? (
              <div style={{ fontSize: 12, color: "#6b7280" }}>
                Sem recomendações.
              </div>
            ) : (
              recommendations.map((item) => (
                <div
                  key={item.concept_id}
                  style={{
                    padding: 7,
                    borderRadius: 8,
                    border: "1px solid #e2e8f0",
                    background: "#f8fbff",
                  }}
                >
                  <div
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      marginBottom: 3,
                      fontSize: 11,
                    }}
                  >
                    <strong style={{ fontSize: 12 }}>{item.name}</strong>
                    <span>{(item.score * 100).toFixed(0)}%</span>
                  </div>
                  <div
                    style={{
                      background: "#dbe7fb",
                      borderRadius: 999,
                      height: 6,
                    }}
                  >
                    <div style={barStyle(item.score, "#0ea5e9")} />
                  </div>
                </div>
              ))
            )}
          </div>

          <div
            style={{
              background: "#ffffff",
              border: "1px solid #dbe5f4",
              borderRadius: 10,
              padding: 8,
              display: "grid",
              gap: 6,
            }}
          >
            <strong style={{ fontSize: 12 }}>Quick Add</strong>

            <form onSubmit={onAddConcept} style={{ display: "grid", gap: 6 }}>
              <div style={{ fontSize: 11, color: "#6b7280" }}>Conceito</div>
              <input
                placeholder="id"
                value={conceptId}
                onChange={(e) => setConceptId(e.target.value)}
                style={{ padding: "5px 7px", boxShadow: "none", fontSize: 12 }}
              />
              <input
                placeholder="nome"
                value={conceptName}
                onChange={(e) => setConceptName(e.target.value)}
                style={{ padding: "5px 7px", boxShadow: "none", fontSize: 12 }}
              />
              <button
                type="submit"
                style={{ padding: "6px 9px", boxShadow: "none", fontSize: 12 }}
              >
                + conceito
              </button>
            </form>

            <form onSubmit={onAddEdge} style={{ display: "grid", gap: 6 }}>
              <div style={{ fontSize: 11, color: "#6b7280" }}>Ligação</div>
              <select
                value={prereqId}
                onChange={(e) => setPrereqId(e.target.value)}
                style={{
                  padding: "6px 8px",
                  borderRadius: 8,
                  border: "1px solid #cbd5e1",
                  fontSize: 12,
                }}
              >
                <option value="">pré</option>
                {concepts.map((concept) => (
                  <option key={`pre-${concept.id}`} value={concept.id}>
                    {concept.name}
                  </option>
                ))}
              </select>
              <select
                value={targetId}
                onChange={(e) => setTargetId(e.target.value)}
                style={{
                  padding: "6px 8px",
                  borderRadius: 8,
                  border: "1px solid #cbd5e1",
                  fontSize: 12,
                }}
              >
                <option value="">alvo</option>
                {concepts.map((concept) => (
                  <option key={`target-${concept.id}`} value={concept.id}>
                    {concept.name}
                  </option>
                ))}
              </select>
              <button
                type="submit"
                style={{ padding: "6px 9px", boxShadow: "none", fontSize: 12 }}
              >
                + aresta
              </button>
            </form>

            <form onSubmit={onIngestEvent} style={{ display: "grid", gap: 6 }}>
              <div style={{ fontSize: 11, color: "#6b7280" }}>Evento</div>
              <select
                value={eventType}
                onChange={(e) => setEventType(e.target.value)}
                style={{
                  padding: "6px 8px",
                  borderRadius: 8,
                  border: "1px solid #cbd5e1",
                  fontSize: 12,
                }}
              >
                <option value="practice_attempt">practice</option>
                <option value="quiz_attempt">quiz</option>
                <option value="flashcard_review">flashcard</option>
                <option value="study_read">read</option>
                <option value="self_assessment">self</option>
              </select>
              <select
                value={eventConceptId}
                onChange={(e) => setEventConceptId(e.target.value)}
                style={{
                  padding: "6px 8px",
                  borderRadius: 8,
                  border: "1px solid #cbd5e1",
                  fontSize: 12,
                }}
              >
                <option value="">conceito</option>
                {concepts.map((concept) => (
                  <option key={`evt-${concept.id}`} value={concept.id}>
                    {concept.name}
                  </option>
                ))}
              </select>
              <div style={{ display: "flex", gap: 6 }}>
                <input
                  placeholder="ok"
                  value={correct}
                  onChange={(e) => setCorrect(e.target.value)}
                  style={{
                    padding: "5px 7px",
                    boxShadow: "none",
                    minWidth: 0,
                    fontSize: 12,
                  }}
                />
                <input
                  placeholder="tot"
                  value={total}
                  onChange={(e) => setTotal(e.target.value)}
                  style={{
                    padding: "5px 7px",
                    boxShadow: "none",
                    minWidth: 0,
                    fontSize: 12,
                  }}
                />
                <input
                  placeholder="conf"
                  value={confidence}
                  onChange={(e) => setConfidence(e.target.value)}
                  style={{
                    padding: "5px 7px",
                    boxShadow: "none",
                    minWidth: 0,
                    fontSize: 12,
                  }}
                />
              </div>
              <button
                type="submit"
                style={{ padding: "6px 9px", boxShadow: "none", fontSize: 12 }}
              >
                + evento
              </button>
            </form>
          </div>

          <div
            style={{
              background: "#ffffff",
              border: "1px solid #dbe5f4",
              borderRadius: 10,
              padding: 8,
              display: "grid",
              gap: 6,
            }}
          >
            <strong style={{ fontSize: 12 }}>Evidence</strong>
            {!selectedExplainId ? (
              <div style={{ fontSize: 12, color: "#6b7280" }}>
                Clica num conceito para detalhes.
              </div>
            ) : explainRows.length === 0 ? (
              <div style={{ fontSize: 12, color: "#6b7280" }}>
                Sem evidências.
              </div>
            ) : (
              explainRows.map((evidence) => (
                <div
                  key={evidence.event_id + evidence.created_at}
                  style={{
                    fontSize: 11,
                    background: "#f8fbff",
                    border: "1px solid #e2e8f0",
                    borderRadius: 8,
                    padding: 5,
                  }}
                >
                  <div
                    style={{ display: "flex", justifyContent: "space-between" }}
                  >
                    <strong>{evidence.event_type.replace("_", " ")}</strong>
                    <span>{evidence.created_at.slice(11, 16)}</span>
                  </div>
                  <div>
                    +α {evidence.delta_alpha.toFixed(2)} | +β{" "}
                    {evidence.delta_beta.toFixed(2)}
                  </div>
                </div>
              ))
            )}
          </div>

          {error ? (
            <div
              style={{
                background: "#fef2f2",
                border: "1px solid #fecaca",
                color: "#b91c1c",
                borderRadius: 10,
                padding: 8,
                fontSize: 12,
              }}
            >
              {error}
            </div>
          ) : null}
        </div>
      </aside>
    </>
  );
}
