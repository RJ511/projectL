import { useEffect, useMemo, useRef, useState } from "react";
import { ChevronLeft, ChevronRight, Sparkles, Target } from "lucide-react";
import {
  addEdge,
  exportOlmJson,
  getExplain,
  getContentMetrics,
  getOlmState,
  ingestEvent,
  listConcepts,
  listEdges,
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
  activeDomain = "",
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
  const [edges, setEdges] = useState([]);
  const [recommendations, setRecommendations] = useState([]);
  const [contentMetrics, setContentMetrics] = useState([]);
  const [graphFilter, setGraphFilter] = useState("");
  const [conceptFilesById, setConceptFilesById] = useState({});
  const [selectedExplainId, setSelectedExplainId] = useState("");
  const [explainRows, setExplainRows] = useState([]);
  const [error, setError] = useState("");
  const resizeRef = useRef(null);
  const contentAreaRef = useRef(null);
  const [viewportWidth, setViewportWidth] = useState(() => {
    if (typeof window === "undefined") return 1280;
    return window.innerWidth;
  });
  const [contentAreaWidth, setContentAreaWidth] = useState(220);
  const minDrawerWidth = 220;
  const maxDrawerWidth = 460;
  const [drawerWidthPx, setDrawerWidthPx] = useState(260);
  const drawerLeft = viewportWidth <= 720 ? 0 : 48;
  const drawerMaxByViewport = Math.max(
    minDrawerWidth,
    viewportWidth - drawerLeft - 8,
  );
  const drawerWidth = Math.min(
    Math.min(maxDrawerWidth, drawerMaxByViewport),
    Math.max(minDrawerWidth, drawerWidthPx),
  );
  const domainPrefix = useMemo(() => {
    const slug = (activeDomain || "")
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");
    return slug ? `${slug}.` : null;
  }, [activeDomain]);

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

  useEffect(() => {
    function onResize() {
      setViewportWidth(window.innerWidth);
    }

    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  useEffect(() => {
    setDrawerWidthPx((prev) =>
      Math.min(
        Math.max(prev, minDrawerWidth),
        Math.min(maxDrawerWidth, drawerMaxByViewport),
      ),
    );
  }, [drawerMaxByViewport]);

  useEffect(() => {
    const element = contentAreaRef.current;
    if (!element) return;

    function updateWidth() {
      const next = element.clientWidth;
      if (next > 0) setContentAreaWidth(next);
    }

    updateWidth();

    const observer = new ResizeObserver(() => updateWidth());
    observer.observe(element);

    return () => observer.disconnect();
  }, [drawerWidth, open]);

  function startPanelResize(event) {
    event.preventDefault();
    event.stopPropagation();

    resizeRef.current = {
      startX: event.clientX,
      startWidth: drawerWidth,
    };

    function onPointerMove(moveEvent) {
      if (!resizeRef.current) return;
      const deltaX = moveEvent.clientX - resizeRef.current.startX;
      const proposed = resizeRef.current.startWidth + deltaX;
      const clamped = Math.min(
        Math.min(maxDrawerWidth, drawerMaxByViewport),
        Math.max(minDrawerWidth, proposed),
      );
      setDrawerWidthPx(clamped);
    }

    function stopPanelResize() {
      resizeRef.current = null;
      window.removeEventListener("mousemove", onPointerMove);
      window.removeEventListener("mouseup", stopPanelResize);
    }

    window.addEventListener("mousemove", onPointerMove);
    window.addEventListener("mouseup", stopPanelResize);
  }

  const graphData = useMemo(() => {
    if (concepts.length === 0) {
      return { nodes: [], edges: [] };
    }

    const byId = new Map(concepts.map((concept) => [concept.id, concept]));
    const term = graphFilter.trim().toLowerCase();
    let selectedIds = new Set(concepts.map((concept) => concept.id));

    if (term) {
      const matched = concepts
        .filter((concept) => {
          const name = String(concept.name || "").toLowerCase();
          const id = String(concept.id || "").toLowerCase();
          return name.includes(term) || id.includes(term);
        })
        .map((concept) => concept.id);

      const expanded = new Set(matched);
      for (const edge of edges) {
        const touchesMatch =
          expanded.has(edge.prereq_id) || expanded.has(edge.target_id);
        if (touchesMatch) {
          expanded.add(edge.prereq_id);
          expanded.add(edge.target_id);
        }
      }
      selectedIds = expanded;
    }

    const filteredNodes = Array.from(selectedIds)
      .map((id) => byId.get(id))
      .filter(Boolean);

    const filteredEdges = edges.filter(
      (edge) =>
        selectedIds.has(edge.prereq_id) && selectedIds.has(edge.target_id),
    );

    return {
      nodes: filteredNodes,
      edges: filteredEdges,
    };
  }, [concepts, edges, graphFilter]);

  const graphLayout = useMemo(() => {
    const width = Math.max(150, contentAreaWidth - 28);
    const height = Math.max(156, Math.min(190, Math.round(width * 0.72)));
    const cx = width / 2;
    const cy = height / 2;
    const count = graphData.nodes.length;
    const radius = Math.max(48, Math.min(82, 22 + count * 4));
    const nodePositions = new Map();
    const ordered = [...graphData.nodes].sort((a, b) =>
      String(a.id).localeCompare(String(b.id)),
    );

    ordered.forEach((node, index) => {
      const angle = ((Math.PI * 2) / Math.max(1, count)) * index - Math.PI / 2;
      nodePositions.set(node.id, {
        x: cx + Math.cos(angle) * radius,
        y: cy + Math.sin(angle) * radius,
      });
    });

    return {
      width,
      height,
      nodePositions,
      nodes: ordered,
      edges: graphData.edges,
    };
  }, [contentAreaWidth, graphData]);

  const selectedConceptFiles = useMemo(() => {
    if (!selectedExplainId) return [];
    return conceptFilesById[selectedExplainId] || [];
  }, [conceptFilesById, selectedExplainId]);

  async function refresh() {
    setError("");
    try {
      const [conceptList, edgeList, stateList, nextList, olmSnapshot] =
        await Promise.all([
          listConcepts(),
          listEdges(),
          getOlmState(),
          nextToStudy({
            top: 5,
            lambda: 0.7,
            readinessThreshold: 0.5,
            domainFilter: domainPrefix,
          }),
          exportOlmJson(),
        ]);
      const metrics = await getContentMetrics();

      const filteredConcepts = domainPrefix
        ? conceptList.filter((concept) => concept.id.startsWith(domainPrefix))
        : conceptList;
      const filteredState = domainPrefix
        ? stateList.filter((row) => row.concept_id.startsWith(domainPrefix))
        : stateList;
      const filteredEdges = domainPrefix
        ? edgeList.filter(
            (edge) =>
              String(edge.prereq_id || "").startsWith(domainPrefix) &&
              String(edge.target_id || "").startsWith(domainPrefix),
          )
        : edgeList;
      const filteredMetrics = domainPrefix
        ? (metrics || []).filter((item) =>
            String(item.content_id || "").startsWith(`${activeDomain}/`),
          )
        : metrics || [];
      const nextConceptFiles = {};
      const contentItems =
        olmSnapshot && typeof olmSnapshot.content_items === "object"
          ? olmSnapshot.content_items
          : {};
      const contentConcepts = Array.isArray(olmSnapshot?.content_concepts)
        ? olmSnapshot.content_concepts
        : [];

      for (const map of contentConcepts) {
        const conceptKey = String(map?.concept_id || "");
        if (!conceptKey) continue;
        if (domainPrefix && !conceptKey.startsWith(domainPrefix)) continue;

        const contentId = String(map?.content_id || "");
        const item = contentItems[contentId];
        const label = String(item?.title || contentId || "").trim();
        if (!label) continue;

        if (!nextConceptFiles[conceptKey]) {
          nextConceptFiles[conceptKey] = [];
        }
        if (!nextConceptFiles[conceptKey].includes(label)) {
          nextConceptFiles[conceptKey].push(label);
        }
      }

      for (const conceptKey of Object.keys(nextConceptFiles)) {
        nextConceptFiles[conceptKey].sort((a, b) => a.localeCompare(b));
      }

      setConcepts(filteredConcepts);
      setStateRows(filteredState);
      setEdges(filteredEdges);
      setRecommendations(nextList);
      setContentMetrics(filteredMetrics);
      setConceptFilesById(nextConceptFiles);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    refresh();
  }, [domainPrefix]);

  async function onAddConcept(e) {
    e.preventDefault();
    setError("");

    if (!conceptId.trim() || !conceptName.trim()) {
      setError("ID e nome são obrigatórios.");
      return;
    }

    const rawId = conceptId.trim();
    const scopedId =
      domainPrefix && !rawId.startsWith(domainPrefix)
        ? `${domainPrefix}${rawId}`
        : rawId;

    try {
      await upsertConcept({
        id: scopedId,
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
          maxWidth: `calc(100vw - ${drawerLeft}px)`,
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
          overflowX: "hidden",
          boxSizing: "border-box",
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
            <span style={{ fontSize: 10, color: "#64748b" }}>
              {activeDomain ? `Domain: ${activeDomain}` : "All domains"}
            </span>
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

        <div
          ref={contentAreaRef}
          style={{
            padding: 8,
            overflowY: "auto",
            overflowX: "hidden",
            display: "grid",
            gridTemplateColumns: "minmax(0, 1fr)",
            gap: 8,
            minWidth: 0,
            width: "100%",
            boxSizing: "border-box",
          }}
        >
          <div
            style={{
              background: "#ffffff",
              border: "1px solid #dbe5f4",
              borderRadius: 10,
              padding: 8,
              minWidth: 0,
              width: "100%",
              boxSizing: "border-box",
            }}
          >
            <div style={{ display: "flex", gap: 6, marginBottom: 6 }}>
              <div
                style={{
                  flex: 1,
                  minWidth: 0,
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
                  minWidth: 0,
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
                  minWidth: 0,
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
              minWidth: 0,
              width: "100%",
              boxSizing: "border-box",
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
                    <span
                      style={{
                        fontWeight: 600,
                        fontSize: 12,
                        overflowWrap: "anywhere",
                        wordBreak: "break-word",
                      }}
                    >
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
              minWidth: 0,
              width: "100%",
              boxSizing: "border-box",
            }}
          >
            <strong style={{ fontSize: 12 }}>Item Metrics</strong>
            {contentMetrics.length === 0 ? (
              <div style={{ fontSize: 12, color: "#6b7280" }}>
                Sem métricas de conteúdo ainda.
              </div>
            ) : (
              [...contentMetrics]
                .sort((a, b) => b.attempts - a.attempts)
                .slice(0, 5)
                .map((metric) => (
                  <div
                    key={metric.content_id}
                    style={{
                      padding: 7,
                      borderRadius: 8,
                      border: "1px solid #e2e8f0",
                      background: "#f8fbff",
                      display: "grid",
                      gap: 2,
                    }}
                  >
                    <div
                      style={{
                        fontSize: 11,
                        fontWeight: 600,
                        overflowWrap: "anywhere",
                        wordBreak: "break-word",
                      }}
                    >
                      {metric.title || metric.content_id}
                    </div>
                    <div style={{ fontSize: 10, color: "#64748b" }}>
                      tentativas: {metric.attempts} · sucesso:{" "}
                      {(metric.success_rate * 100).toFixed(0)}% · score médio:{" "}
                      {(metric.avg_score * 100).toFixed(0)}%
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
              minWidth: 0,
              width: "100%",
              boxSizing: "border-box",
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
                      gap: 6,
                      marginBottom: 3,
                      fontSize: 11,
                    }}
                  >
                    <strong
                      style={{
                        fontSize: 12,
                        minWidth: 0,
                        overflowWrap: "anywhere",
                        wordBreak: "break-word",
                      }}
                    >
                      {item.name}
                    </strong>
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
              minWidth: 0,
              width: "100%",
              boxSizing: "border-box",
            }}
          >
            <strong style={{ fontSize: 12 }}>Concept Graph</strong>
            <input
              placeholder="Filtrar conceito (id ou nome)"
              value={graphFilter}
              onChange={(e) => setGraphFilter(e.target.value)}
              style={{
                padding: "6px 8px",
                borderRadius: 8,
                border: "1px solid #cbd5e1",
                fontSize: 12,
              }}
            />
            {graphLayout.nodes.length === 0 ? (
              <div style={{ fontSize: 12, color: "#6b7280" }}>
                Sem conceitos para desenhar no grafo.
              </div>
            ) : (
              <div
                style={{
                  border: "1px solid #e2e8f0",
                  borderRadius: 10,
                  background:
                    "radial-gradient(circle at 20% 20%, #f0f9ff 0%, #f8fbff 55%, #f1f5f9 100%)",
                  padding: 6,
                }}
              >
                <svg
                  viewBox={`0 0 ${graphLayout.width} ${graphLayout.height}`}
                  width="100%"
                  height={graphLayout.height}
                  role="img"
                  aria-label="Grafo de conceitos e arestas"
                >
                  <defs>
                    <marker
                      id="olm-graph-arrow"
                      viewBox="0 0 10 10"
                      refX="9"
                      refY="5"
                      markerWidth="6"
                      markerHeight="6"
                      orient="auto-start-reverse"
                    >
                      <path d="M 0 0 L 10 5 L 0 10 z" fill="#64748b" />
                    </marker>
                  </defs>

                  {graphLayout.edges.map((edge) => {
                    const from = graphLayout.nodePositions.get(edge.prereq_id);
                    const to = graphLayout.nodePositions.get(edge.target_id);
                    if (!from || !to) return null;

                    const dx = to.x - from.x;
                    const dy = to.y - from.y;
                    const len = Math.hypot(dx, dy) || 1;
                    const ux = dx / len;
                    const uy = dy / len;
                    const nodeRadius = 11;
                    const x1 = from.x + ux * nodeRadius;
                    const y1 = from.y + uy * nodeRadius;
                    const x2 = to.x - ux * (nodeRadius + 2);
                    const y2 = to.y - uy * (nodeRadius + 2);

                    return (
                      <line
                        key={`${edge.prereq_id}-${edge.target_id}`}
                        x1={x1}
                        y1={y1}
                        x2={x2}
                        y2={y2}
                        stroke="#94a3b8"
                        strokeWidth="1.5"
                        strokeLinecap="round"
                        markerEnd="url(#olm-graph-arrow)"
                      />
                    );
                  })}

                  {graphLayout.nodes.map((node) => {
                    const pos = graphLayout.nodePositions.get(node.id);
                    const stateRow = stateRows.find(
                      (row) => row.concept_id === node.id,
                    );
                    const mastery = stateRow?.mastery ?? 0.5;
                    const nodeFill =
                      mastery >= 0.7
                        ? "#86efac"
                        : mastery >= 0.4
                          ? "#fde68a"
                          : "#fca5a5";
                    const isSelected = selectedExplainId === node.id;

                    if (!pos) return null;
                    return (
                      <g
                        key={node.id}
                        role="button"
                        tabIndex={0}
                        style={{ cursor: "pointer" }}
                        onClick={() => onSelectExplain(node.id)}
                        onKeyDown={(event) => {
                          if (event.key === "Enter" || event.key === " ") {
                            event.preventDefault();
                            onSelectExplain(node.id);
                          }
                        }}
                      >
                        <title>{`Abrir evidências de ${node.name || node.id}`}</title>
                        <circle
                          cx={pos.x}
                          cy={pos.y}
                          r="11"
                          fill={nodeFill}
                          stroke={isSelected ? "#0f172a" : "#1e293b"}
                          strokeWidth={isSelected ? "2" : "1"}
                        />
                        <text
                          x={pos.x}
                          y={pos.y + 25}
                          textAnchor="middle"
                          fontSize="9"
                          fill="#334155"
                        >
                          {String(node.name || node.id).slice(0, 20)}
                        </text>
                      </g>
                    );
                  })}
                </svg>
              </div>
            )}
            <div style={{ fontSize: 10, color: "#64748b" }}>
              nós: {graphLayout.nodes.length} · arestas:{" "}
              {graphLayout.edges.length}
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
              minWidth: 0,
              width: "100%",
              boxSizing: "border-box",
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
                <option value="review">review (low)</option>
                <option value="note_taking">note taking (low)</option>
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
              minWidth: 0,
              width: "100%",
              boxSizing: "border-box",
            }}
          >
            <strong style={{ fontSize: 12 }}>Evidence</strong>
            {selectedExplainId ? (
              <div
                style={{
                  fontSize: 10,
                  color: "#64748b",
                  background: "#f8fafc",
                  border: "1px solid #e2e8f0",
                  borderRadius: 8,
                  padding: 6,
                }}
              >
                <div style={{ fontWeight: 700, marginBottom: 4 }}>
                  Ficheiros do conceito
                </div>
                {selectedConceptFiles.length === 0 ? (
                  <div>Nenhum ficheiro mapeado para este conceito.</div>
                ) : (
                  selectedConceptFiles.slice(0, 8).map((label) => (
                    <div
                      key={label}
                      style={{
                        overflowWrap: "anywhere",
                        wordBreak: "break-word",
                      }}
                      title={label}
                    >
                      - {label}
                    </div>
                  ))
                )}
              </div>
            ) : null}
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

        <div
          role="separator"
          aria-label="Redimensionar painel OLM"
          onMouseDown={startPanelResize}
          style={{
            position: "absolute",
            top: 0,
            right: -4,
            width: 10,
            height: "100%",
            cursor: "ew-resize",
            zIndex: 1001,
          }}
        />
        <div
          role="separator"
          aria-label="Redimensionar painel OLM no canto"
          onMouseDown={startPanelResize}
          style={{
            position: "absolute",
            right: 2,
            bottom: 2,
            width: 14,
            height: 14,
            borderRight: "2px solid #94a3b8",
            borderBottom: "2px solid #94a3b8",
            borderRadius: 2,
            cursor: "nwse-resize",
            opacity: 0.7,
            zIndex: 1002,
          }}
        />
      </aside>
    </>
  );
}
