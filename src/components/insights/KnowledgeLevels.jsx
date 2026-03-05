import { useEffect, useMemo, useState } from "react";
import { getContentMetrics } from "../../services/olm.service";

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

function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

export default function KnowledgeLevels({
  tree,
  selectedFile,
  activeDomain,
  openDaysCount,
  level = "root",
  lastOpenedItem = null,
  learningAnalytics,
}) {
  const [topSuggestion, setTopSuggestion] = useState("");

  const domains = useMemo(() => collectDomains(tree), [tree]);

  const activeDomainLabel = useMemo(() => {
    if (activeDomain) return activeDomain;
    if (!selectedFile?.path) return domains[0] || "No domain selected";
    const [firstSegment] = selectedFile.path.split(/[\\/]/);
    return firstSegment || domains[0] || "No domain selected";
  }, [activeDomain, domains, selectedFile]);

  const domainCount = domains.length;

  const analytics = learningAnalytics || {
    sessions: [],
    fileTimeSec: {},
    domainTimeSec: {},
    domainResourceOpenCount: {},
    completionByFile: {},
  };

  const domainUsage = useMemo(() => {
    const resources =
      analytics.domainResourceOpenCount?.[activeDomainLabel] || {};
    const entries = Object.entries(resources);
    if (!entries.length) {
      return { most: null, least: null };
    }
    entries.sort((a, b) => b[1] - a[1]);
    return {
      most: entries[0],
      least: entries[entries.length - 1],
    };
  }, [analytics.domainResourceOpenCount, activeDomainLabel]);

  const domainTimeSec = Number(
    analytics.domainTimeSec?.[activeDomainLabel] || 0,
  );
  const fileTimeSec = lastOpenedItem
    ? Number(analytics.fileTimeSec?.[lastOpenedItem.path] || 0)
    : 0;

  const recentSession = useMemo(() => {
    const sessions = Array.isArray(analytics.sessions)
      ? analytics.sessions
      : [];
    return sessions.length ? sessions[sessions.length - 1] : null;
  }, [analytics.sessions]);

  const domainCompletionStats = useMemo(() => {
    const completion = analytics.completionByFile || {};
    const domainPrefix = activeDomainLabel ? `${activeDomainLabel}/` : "";
    const domainValues = Object.entries(completion)
      .filter(([path]) =>
        domainPrefix ? String(path).startsWith(domainPrefix) : true,
      )
      .map(([, value]) => Number(value))
      .filter((value) => Number.isFinite(value));
    if (!domainValues.length) {
      return null;
    }
    const avg =
      domainValues.reduce((acc, value) => acc + value, 0) / domainValues.length;
    return {
      count: domainValues.length,
      avg,
    };
  }, [analytics.completionByFile, activeDomainLabel]);

  function formatDuration(sec) {
    const total = Math.max(0, Math.round(Number(sec || 0)));
    const hours = Math.floor(total / 3600);
    const minutes = Math.floor((total % 3600) / 60);
    if (hours > 0) return `${hours}h ${minutes}m`;
    return `${minutes}m`;
  }

  const domainConfigSaved = useMemo(() => {
    if (!activeDomain) return false;
    try {
      const raw = localStorage.getItem("olmConfigByDomain.v1");
      const parsed = raw ? JSON.parse(raw) : {};
      const slug = activeDomain
        .normalize("NFD")
        .replace(/[\u0300-\u036f]/g, "")
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/^-+|-+$/g, "");
      return Boolean(parsed?.[slug]);
    } catch {
      return false;
    }
  }, [activeDomain]);

  useEffect(() => {
    let mounted = true;

    async function loadSuggestion() {
      try {
        const allMetrics = await getContentMetrics();
        const domainPathPrefix = activeDomainLabel
          ? `${activeDomainLabel}/`
          : "";

        const itemRows = (allMetrics || []).filter((row) => {
          if (!row?.content_id) return false;
          if (!domainPathPrefix) return true;
          return normalizePath(row.content_id).startsWith(domainPathPrefix);
        });

        const ranked = [...itemRows].sort((a, b) => {
          const scoreDiff = Number(a.avg_score || 0) - Number(b.avg_score || 0);
          if (scoreDiff !== 0) return scoreDiff;
          return Number(b.attempts || 0) - Number(a.attempts || 0);
        });

        const topItem = ranked[0];
        if (!mounted) return;
        setTopSuggestion(
          topItem?.title || topItem?.content_id || "No item recommendation yet",
        );
      } catch {
        if (!mounted) return;
        setTopSuggestion("No item recommendation yet");
      }
    }

    loadSuggestion();

    return () => {
      mounted = false;
    };
  }, [tree, activeDomainLabel]);

  const panelTitle = level === "domain" ? "DOMAIN LEVEL" : "ROOT LEVEL VIEW";

  return (
    <div
      style={{
        margin: 10,
        display: "grid",
        gap: 10,
      }}
    >
      <div
        style={{
          border: "1px solid #cdddf6",
          borderRadius: 10,
          background: "#fff",
          padding: 14,
        }}
      >
        <div style={{ fontSize: 11, color: "#64748b" }}>{panelTitle}</div>
        <div style={{ fontWeight: 700, fontSize: 16, marginTop: 4 }}>
          {level === "domain"
            ? activeDomainLabel || "Sem domínio ativo"
            : "Visão geral do Root"}
        </div>
        <div style={{ fontSize: 13, color: "#334155", marginTop: 6 }}>
          {dailyQuote()}
        </div>
      </div>

      {level === "root" ? (
        <div
          style={{
            border: "1px solid #dbe5f4",
            borderRadius: 10,
            background: "#fff",
            padding: 14,
            display: "grid",
            gap: 6,
            fontSize: 13,
            color: "#334155",
          }}
        >
          <div>Streak de abertura: {openDaysCount} dia(s)</div>
          <div>Domínios no Root: {domainCount}</div>
          <div>
            Última sessão:{" "}
            {recentSession
              ? formatDuration(recentSession.durationSec)
              : "Sem sessões"}
          </div>
          <div>
            Seleciona um domínio na barra lateral para entrar no Domain Level.
          </div>
        </div>
      ) : (
        <div
          style={{
            border: "1px solid #dbe5f4",
            borderRadius: 10,
            background: "#fff",
            padding: 14,
            display: "grid",
            gap: 8,
            fontSize: 13,
            color: "#334155",
          }}
        >
          <div>
            Próximo recomendado: <strong>{topSuggestion}</strong>
          </div>
          <div>Tempo total no domínio: {formatDuration(domainTimeSec)}</div>
          <div>
            Último ficheiro aberto: {lastOpenedItem?.name || "Sem histórico"}
          </div>
          <div>Tempo no último ficheiro: {formatDuration(fileTimeSec)}</div>
          <div>
            Config OLM do domínio: {domainConfigSaved ? "Guardada" : "Default"}
          </div>
          <div>
            Recurso mais usado:{" "}
            {domainUsage.most
              ? `${domainUsage.most[0]} (${domainUsage.most[1]})`
              : "Sem dados"}
          </div>
          <div>
            Recurso menos usado:{" "}
            {domainUsage.least
              ? `${domainUsage.least[0]} (${domainUsage.least[1]})`
              : "Sem dados"}
          </div>
          <div>
            Conclusão média (testes/decks):{" "}
            {domainCompletionStats
              ? `${Math.round(domainCompletionStats.avg * 100)}% em ${domainCompletionStats.count} recurso(s)`
              : "Sem dados"}
          </div>
          <div style={{ color: "#64748b", fontSize: 12 }}>
            O editor só aparece quando entrares num item (ficheiro).
          </div>
          {lastOpenedItem ? (
            <div style={{ color: "#64748b", fontSize: 12 }}>
              Caminho: {lastOpenedItem.path}
            </div>
          ) : null}
        </div>
      )}
    </div>
  );
}
