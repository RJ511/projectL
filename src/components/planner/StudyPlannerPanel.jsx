import { useEffect, useMemo, useState } from "react";
import {
  CalendarDays,
  ChevronLeft,
  ChevronRight,
  BrainCircuit,
} from "lucide-react";
import { nextToStudy } from "../../services/olm.service";
import { getAnkiDeckStats, listAnkiDecks } from "../../services/anki.service";

const PLANS_KEY = "appStudyPlans";

function toIsoDay(date) {
  return new Date(date.getTime() - date.getTimezoneOffset() * 60000)
    .toISOString()
    .slice(0, 10);
}

function getMonthDays(viewDate) {
  const year = viewDate.getFullYear();
  const month = viewDate.getMonth();
  const first = new Date(year, month, 1);
  const last = new Date(year, month + 1, 0);
  const firstWeekday = (first.getDay() + 6) % 7;
  const daysInMonth = last.getDate();

  const cells = [];
  for (let i = 0; i < firstWeekday; i += 1) cells.push(null);
  for (let day = 1; day <= daysInMonth; day += 1) {
    cells.push(new Date(year, month, day));
  }

  return cells;
}

function loadPlans() {
  try {
    const raw = localStorage.getItem(PLANS_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    if (!Array.isArray(parsed)) return [];
    return parsed;
  } catch {
    return [];
  }
}

export default function StudyPlannerPanel({ open, onClose }) {
  const [viewDate, setViewDate] = useState(new Date());
  const [selectedDate, setSelectedDate] = useState(toIsoDay(new Date()));
  const [plans, setPlans] = useState(loadPlans);
  const [newPlanTitle, setNewPlanTitle] = useState("");
  const [recommendations, setRecommendations] = useState([]);
  const [ankiDecks, setAnkiDecks] = useState([]);
  const [selectedDeck, setSelectedDeck] = useState("");
  const [ankiStats, setAnkiStats] = useState(null);
  const [ankiError, setAnkiError] = useState("");
  const [loadingAnki, setLoadingAnki] = useState(false);

  useEffect(() => {
    localStorage.setItem(PLANS_KEY, JSON.stringify(plans));
  }, [plans]);

  useEffect(() => {
    if (!open) return;

    nextToStudy({ top: 5, lambda: 0.7, readinessThreshold: 0.5 })
      .then((rows) => setRecommendations(Array.isArray(rows) ? rows : []))
      .catch(() => setRecommendations([]));
  }, [open]);

  const monthCells = useMemo(() => getMonthDays(viewDate), [viewDate]);

  const plansByDate = useMemo(() => {
    const byDate = new Map();
    for (const plan of plans) {
      if (!byDate.has(plan.date)) byDate.set(plan.date, []);
      byDate.get(plan.date).push(plan);
    }
    return byDate;
  }, [plans]);

  const selectedPlans = plansByDate.get(selectedDate) || [];

  function addPlan(title, date = selectedDate) {
    if (!title.trim()) return;

    const plan = {
      id: `plan-${Date.now()}-${Math.round(Math.random() * 1000)}`,
      date,
      title: title.trim(),
      done: false,
    };

    setPlans((prev) => [plan, ...prev]);
    setNewPlanTitle("");
  }

  function togglePlan(id) {
    setPlans((prev) =>
      prev.map((plan) =>
        plan.id === id ? { ...plan, done: !plan.done } : plan,
      ),
    );
  }

  function removePlan(id) {
    setPlans((prev) => prev.filter((plan) => plan.id !== id));
  }

  async function loadAnkiDecks() {
    setLoadingAnki(true);
    setAnkiError("");
    try {
      const decks = await listAnkiDecks();
      setAnkiDecks(decks);
      if (decks.length > 0) {
        setSelectedDeck(decks[0]);
      }
    } catch (error) {
      setAnkiError(String(error));
    } finally {
      setLoadingAnki(false);
    }
  }

  async function refreshDeckStats(deckName) {
    if (!deckName) {
      setAnkiStats(null);
      return;
    }

    setLoadingAnki(true);
    setAnkiError("");
    try {
      const stats = await getAnkiDeckStats(deckName);
      setAnkiStats(stats);
    } catch (error) {
      setAnkiError(String(error));
      setAnkiStats(null);
    } finally {
      setLoadingAnki(false);
    }
  }

  useEffect(() => {
    if (!open) return;
    if (!selectedDeck) return;
    refreshDeckStats(selectedDeck);
  }, [selectedDeck, open]);

  const monthTitle = viewDate.toLocaleDateString("pt-PT", {
    month: "long",
    year: "numeric",
  });

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
        zIndex: 990,
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
          <CalendarDays size={16} />
          <strong style={{ fontSize: 14 }}>Calendário & Estudo</strong>
        </div>
        <button type="button" onClick={onClose} style={{ padding: "3px 8px" }}>
          ×
        </button>
      </div>

      <div style={{ overflow: "auto", padding: 10, display: "grid", gap: 10 }}>
        <section
          style={{
            background: "var(--card-bg)",
            border: "1px solid var(--panel-border)",
            borderRadius: 10,
            padding: 10,
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              marginBottom: 8,
            }}
          >
            <button
              type="button"
              onClick={() =>
                setViewDate(
                  (prev) =>
                    new Date(prev.getFullYear(), prev.getMonth() - 1, 1),
                )
              }
              style={{ padding: "3px 8px" }}
            >
              <ChevronLeft size={14} />
            </button>
            <strong style={{ textTransform: "capitalize", fontSize: 13 }}>
              {monthTitle}
            </strong>
            <button
              type="button"
              onClick={() =>
                setViewDate(
                  (prev) =>
                    new Date(prev.getFullYear(), prev.getMonth() + 1, 1),
                )
              }
              style={{ padding: "3px 8px" }}
            >
              <ChevronRight size={14} />
            </button>
          </div>

          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(7, minmax(0, 1fr))",
              gap: 4,
              fontSize: 10,
              color: "var(--muted-text)",
              marginBottom: 6,
            }}
          >
            {["S", "T", "Q", "Q", "S", "S", "D"].map((d, i) => (
              <div key={`${d}-${i}`} style={{ textAlign: "center" }}>
                {d}
              </div>
            ))}
          </div>

          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(7, minmax(0, 1fr))",
              gap: 4,
            }}
          >
            {monthCells.map((date, idx) => {
              if (!date) {
                return <div key={`e-${idx}`} style={{ height: 38 }} />;
              }

              const iso = toIsoDay(date);
              const dayPlans = plansByDate.get(iso) || [];
              const selected = iso === selectedDate;
              const hasDone = dayPlans.some((plan) => plan.done);

              return (
                <button
                  key={iso}
                  type="button"
                  onClick={() => setSelectedDate(iso)}
                  style={{
                    height: 38,
                    borderRadius: 8,
                    border: selected
                      ? "1px solid #3b82f6"
                      : "1px solid var(--panel-border)",
                    background: selected ? "#dbeafe" : "var(--card-bg)",
                    color: "var(--app-text-color)",
                    position: "relative",
                    padding: 0,
                    fontSize: 12,
                    fontWeight: 700,
                  }}
                >
                  {date.getDate()}
                  {dayPlans.length > 0 && (
                    <span
                      style={{
                        position: "absolute",
                        right: 4,
                        bottom: 4,
                        width: 7,
                        height: 7,
                        borderRadius: 999,
                        background: hasDone ? "#10b981" : "#f59e0b",
                      }}
                    />
                  )}
                </button>
              );
            })}
          </div>
        </section>

        <section
          style={{
            background: "var(--card-bg)",
            border: "1px solid var(--panel-border)",
            borderRadius: 10,
            padding: 10,
            display: "grid",
            gap: 6,
          }}
        >
          <strong style={{ fontSize: 12 }}>
            Plano diário (
            {new Date(`${selectedDate}T00:00:00`).toLocaleDateString("pt-PT")})
          </strong>
          <div style={{ display: "flex", gap: 6 }}>
            <input
              value={newPlanTitle}
              onChange={(e) => setNewPlanTitle(e.target.value)}
              placeholder="Nova tarefa de estudo"
              style={{ flex: 1 }}
            />
            <button type="button" onClick={() => addPlan(newPlanTitle)}>
              +
            </button>
          </div>

          <div style={{ display: "grid", gap: 4 }}>
            {selectedPlans.length === 0 && (
              <div style={{ fontSize: 11, color: "var(--muted-text)" }}>
                Sem tarefas neste dia.
              </div>
            )}
            {selectedPlans.map((plan) => (
              <div
                key={plan.id}
                style={{
                  border: "1px solid var(--panel-border)",
                  borderRadius: 8,
                  padding: "6px 8px",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  gap: 8,
                }}
              >
                <label
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 8,
                    flex: 1,
                    cursor: "pointer",
                    fontSize: 12,
                  }}
                >
                  <input
                    type="checkbox"
                    checked={plan.done}
                    onChange={() => togglePlan(plan.id)}
                  />
                  <span
                    style={{
                      textDecoration: plan.done ? "line-through" : "none",
                      opacity: plan.done ? 0.6 : 1,
                    }}
                  >
                    {plan.title}
                  </span>
                </label>

                <button
                  type="button"
                  onClick={() => removePlan(plan.id)}
                  style={{ padding: "2px 6px" }}
                >
                  ×
                </button>
              </div>
            ))}
          </div>

          {recommendations.length > 0 && (
            <div style={{ marginTop: 2 }}>
              <div
                style={{
                  fontSize: 11,
                  fontWeight: 700,
                  marginBottom: 4,
                  color: "var(--muted-text)",
                }}
              >
                Sugestões OLM
              </div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                {recommendations.slice(0, 4).map((item) => (
                  <button
                    key={item.concept_id}
                    type="button"
                    onClick={() =>
                      addPlan(`Revisar ${item.name || item.concept_id}`)
                    }
                    style={{ fontSize: 11, padding: "4px 8px" }}
                  >
                    {item.name || item.concept_id}
                  </button>
                ))}
              </div>
            </div>
          )}
        </section>

        <section
          style={{
            background: "var(--card-bg)",
            border: "1px solid var(--panel-border)",
            borderRadius: 10,
            padding: 10,
            display: "grid",
            gap: 7,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <BrainCircuit size={15} />
            <strong style={{ fontSize: 12 }}>Anki decks</strong>
          </div>

          <div style={{ display: "flex", gap: 6 }}>
            <button
              type="button"
              onClick={loadAnkiDecks}
              disabled={loadingAnki}
            >
              {loadingAnki ? "A carregar…" : "Ligar Anki"}
            </button>

            <select
              value={selectedDeck}
              onChange={(e) => setSelectedDeck(e.target.value)}
              style={{ flex: 1 }}
            >
              <option value="">Selecionar deck</option>
              {ankiDecks.map((deck) => (
                <option key={deck} value={deck}>
                  {deck}
                </option>
              ))}
            </select>
          </div>

          {selectedDeck && (
            <button
              type="button"
              onClick={() => refreshDeckStats(selectedDeck)}
              disabled={loadingAnki}
              style={{ width: "fit-content" }}
            >
              Atualizar deck
            </button>
          )}

          {ankiError && (
            <div style={{ color: "#dc2626", fontSize: 11 }}>
              Falha AnkiConnect: {ankiError}
            </div>
          )}

          {ankiStats && (
            <div
              style={{
                border: "1px solid var(--panel-border)",
                borderRadius: 8,
                padding: 8,
                fontSize: 12,
                display: "grid",
                gap: 4,
              }}
            >
              <div>
                <strong>Due:</strong> {ankiStats.due}
              </div>
              <div>
                <strong>New:</strong> {ankiStats.newCards}
              </div>
              <div>
                <strong>Learning:</strong> {ankiStats.learning}
              </div>
            </div>
          )}
        </section>
      </div>
    </aside>
  );
}
