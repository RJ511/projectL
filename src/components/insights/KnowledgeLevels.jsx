import { useEffect, useMemo, useState } from "react";
import { nextToStudy } from "../../services/olm.service";

const EMO_QUOTES = [
  "Small steps still move you forward.",
  "Consistency beats intensity.",
  "Progress today, clarity tomorrow.",
  "You don’t need perfect — just present.",
];

function dailyQuote() {
  const day = new Date().toISOString().slice(0, 10);
  const hash = day
    .split("-")
    .join("")
    .split("")
    .reduce((acc, n) => acc + Number(n), 0);
  return EMO_QUOTES[hash % EMO_QUOTES.length];
}

function collectDomains(tree) {
  return (tree || []).filter((node) => node.is_dir).map((node) => node.name);
}

export default function KnowledgeLevels({
  tree,
  selectedFile,
  content,
  openDaysCount,
}) {
  const [topSuggestion, setTopSuggestion] = useState("");

  const domains = useMemo(() => collectDomains(tree), [tree]);

  const activeDomain = useMemo(() => {
    if (!selectedFile?.path) return domains[0] || "No domain selected";
    const [firstSegment] = selectedFile.path.split(/[\\/]/);
    return firstSegment || domains[0] || "No domain selected";
  }, [domains, selectedFile]);

  const itemInfo = useMemo(() => {
    if (!selectedFile) {
      return {
        title: "No item selected",
        difficulty: "--",
      };
    }

    const textSize = (content || "").trim().length;
    const difficulty =
      textSize < 200 ? "Low" : textSize < 1200 ? "Medium" : "High";

    return {
      title: selectedFile.name,
      difficulty,
    };
  }, [content, selectedFile]);

  useEffect(() => {
    let mounted = true;

    async function loadSuggestion() {
      try {
        const rows = await nextToStudy({
          top: 1,
          lambda: 0.7,
          readinessThreshold: 0.5,
        });
        if (!mounted) return;
        setTopSuggestion(rows?.[0]?.name || "No recommendation yet");
      } catch {
        if (!mounted) return;
        setTopSuggestion("No recommendation yet");
      }
    }

    loadSuggestion();

    return () => {
      mounted = false;
    };
  }, [tree]);

  return (
    <div
      style={{
        margin: "10px 10px 0",
        display: "grid",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
        gap: 8,
      }}
    >
      <div
        style={{
          border: "1px solid #dbe5f4",
          borderRadius: 10,
          background: "#fff",
          padding: 10,
        }}
      >
        <div style={{ fontSize: 11, color: "#64748b" }}>ROOT LEVEL</div>
        <div style={{ fontWeight: 700, fontSize: 13 }}>
          Streak (open days): {openDaysCount}
        </div>
        <div style={{ fontSize: 12, color: "#334155", marginTop: 3 }}>
          {dailyQuote()}
        </div>
      </div>

      <div
        style={{
          border: "1px solid #dbe5f4",
          borderRadius: 10,
          background: "#fff",
          padding: 10,
        }}
      >
        <div style={{ fontSize: 11, color: "#64748b" }}>DOMAIN LEVEL</div>
        <div
          style={{
            fontWeight: 700,
            fontSize: 13,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          Active domain: {activeDomain}
        </div>
        <div style={{ fontSize: 12, color: "#334155", marginTop: 3 }}>
          Next suggestion: {topSuggestion}
        </div>
      </div>

      <div
        style={{
          border: "1px solid #dbe5f4",
          borderRadius: 10,
          background: "#fff",
          padding: 10,
        }}
      >
        <div style={{ fontSize: 11, color: "#64748b" }}>ITEM LEVEL</div>
        <div
          style={{
            fontWeight: 700,
            fontSize: 13,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {itemInfo.title}
        </div>
        <div style={{ fontSize: 12, color: "#334155", marginTop: 3 }}>
          Difficulty proxy: {itemInfo.difficulty}
        </div>
      </div>
    </div>
  );
}
