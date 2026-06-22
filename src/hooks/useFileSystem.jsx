import { useEffect, useRef, useState } from "react";
import {
  createFolder,
  readFile,
  writeFile,
  getTree,
  createFile,
  deletePath,
  renameFile,
} from "../services/fs.service";
import {
  addEdge,
  ingestEvent,
  mapContentConcept,
  removeContentConceptMaps,
  upsertConcept,
  upsertContentItem,
} from "../services/olm.service";
import { normalize } from "../services/pathTools";
import {
  getRootStateValue,
  setRootStateValue,
} from "../services/rootDataStore";
import {
  buildFallbackInlineConcept,
  extractInlineConceptGraph,
} from "./inlineConcepts";
import {
  MAX_META_PROMPTS_PER_SESSION,
  nextQuestionCursor,
  resolveMetaTrigger,
  shouldThrottleMetaPrompt,
} from "./metaPromptPolicy";

const NODE_LEARNING_PROFILES_KEY = "nodeLearningProfiles.v1";
const LEARNING_ANALYTICS_KEY = "learningAnalytics.v1";
const INLINE_CONCEPT_CATALOG_KEY = "inlineConceptCatalog.v1";
const META_PROMPT_STATE_KEY = "metaPromptState.v1";
const META_MIN_SESSION_SEC = 12 * 60;
const META_QUESTIONS = [
  "Quão confiante estás de que consegues explicar ou aplicar o que estudaste sem ajuda?",
  "Quão bem achas que dominaste o conteúdo trabalhado nesta sessão?",
  "Quão difícil foi o conteúdo ou a tarefa para ti?",
  "Quanto esforço mental investiste para compreender ou resolver isto?",
  "Quão bem conseguiste manter o foco, sem te perderes ou distraíres?",
  "A estratégia que usaste para estudar ou resolver isto foi adequada?",
  "Quão frustrante foi esta sessão?",
];

function slugify(value) {
  return (value || "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function makeTemplateContent(kind, title) {
  const heading = title || (kind === "teste" ? "Teste" : "Quiz");

  if (kind === "teste") {
    return `# ${heading}\n\n## Instruções\n- Duração:\n- Pontuação:\n- Tema:\n\n## Parte A — Escolha múltipla\n1. Pergunta 1\n- [ ] A\n- [ ] B\n- [ ] C\n- [ ] D\n\n2. Pergunta 2\n- [ ] A\n- [ ] B\n- [ ] C\n- [ ] D\n\n## Parte B — Resposta curta\n1.\n\n2.\n\n## Gabarito\n- 1:\n- 2:\n`;
  }

  return `# ${heading}\n\n## Metadados\n- Tema:\n- Nível:\n- Objetivo:\n\n## Questões\n1. Pergunta\n- [ ] Opção A\n- [ ] Opção B\n- [ ] Opção C\n- [ ] Opção D\n\n2. Pergunta\n- [ ] Opção A\n- [ ] Opção B\n- [ ] Opção C\n- [ ] Opção D\n\n## Respostas corretas\n- 1:\n- 2:\n\n## Explicações\n- 1:\n- 2:\n`;
}

function safeParseJson(raw, fallback) {
  try {
    return JSON.parse(raw);
  } catch {
    return fallback;
  }
}

function stripExtension(fileName) {
  const idx = fileName.lastIndexOf(".");
  if (idx <= 0) return fileName;
  return fileName.slice(0, idx);
}

function classifyNode(node) {
  if (!node?.is_dir) return "file";
  const depth = normalize(node.path).split("/").filter(Boolean).length;
  return depth <= 1 ? "domain" : "folder";
}

function domainFromPath(path) {
  const rel = normalize(path || "");
  const [first] = rel.split("/").filter(Boolean);
  return first || "";
}

function isQuizLikePath(path) {
  const normalized = normalize(path || "").toLowerCase();
  const fileName = normalized.split("/").pop() || "";
  return (
    fileName.startsWith("quiz-") ||
    fileName.startsWith("teste-") ||
    fileName.includes("quiz") ||
    fileName.includes("teste")
  );
}

function normalizeConceptId(rawConceptId, domainId) {
  const raw = String(rawConceptId || "").trim();
  const rawParts = raw
    .split(".")
    .map((part) => slugify(part))
    .filter(Boolean);

  const safeDomain = slugify(domainId || "") || "root";

  if (!rawParts.length) {
    return `${safeDomain}.concept-auto`;
  }

  if (rawParts[0] !== safeDomain) {
    return [safeDomain, ...rawParts].join(".");
  }

  return rawParts.join(".");
}

function defaultDifficulty(kind) {
  if (kind === "domain") return 0.6;
  if (kind === "folder") return 0.55;
  return 0.5;
}

function buildDefaultProfile(node) {
  const relPath = normalize(node?.path || "");
  const segments = relPath.split("/").filter(Boolean);
  const kind = classifyNode(node);
  const domainName = domainFromPath(relPath);
  const domainId = slugify(domainName || "") || "root";

  let conceptBase = node?.name || "Concept";
  let conceptId = `${domainId}.concept-auto`;

  if (kind === "domain") {
    conceptBase = `${node?.name || "Domain"} Core`;
    conceptId = `${domainId}.core`;
  } else if (kind === "folder") {
    const scope = segments
      .slice(1)
      .map((part) => slugify(part))
      .filter(Boolean);
    conceptId = [domainId, ...scope, "core"].filter(Boolean).join(".");
  } else if (kind === "file") {
    const parentScope = segments
      .slice(1, -1)
      .map((part) => slugify(part))
      .filter(Boolean);
    const fileStem = slugify(stripExtension(node?.name || "")) || "note";
    conceptBase = stripExtension(node?.name || "") || "Concept";
    conceptId = [domainId, ...parentScope, fileStem].filter(Boolean).join(".");
  }
  const conceptName = conceptBase;

  return {
    path: relPath,
    kind,
    domainId,
    domainName,
    conceptId,
    conceptName,
    difficulty: defaultDifficulty(kind),
  };
}

function normalizeProfile(profile, node) {
  const base = profile || buildDefaultProfile(node);
  const domainName =
    base.domainName || domainFromPath(base.path || node?.path || "");
  const domainId = slugify(base.domainId || domainName || "") || "root";
  const conceptId = normalizeConceptId(
    base.conceptId || base.conceptName || "concept-auto",
    domainId,
  );

  return {
    ...base,
    domainId,
    domainName,
    conceptId,
    conceptName: (base.conceptName || "Concept").trim(),
    difficulty: Math.max(
      0,
      Math.min(1, Number(base.difficulty ?? defaultDifficulty(base.kind))),
    ),
  };
}

function defaultAnalytics() {
  return {
    sessions: [],
    fileOpenCount: {},
    fileTimeSec: {},
    domainTimeSec: {},
    domainResourceOpenCount: {},
    completionByFile: {},
    metaCognitiveProgress: [],
  };
}

function safeAnalytics(raw) {
  const fallback = defaultAnalytics();
  const parsed = safeParseJson(raw, fallback);
  return {
    ...fallback,
    ...(parsed && typeof parsed === "object" ? parsed : {}),
    sessions: Array.isArray(parsed?.sessions) ? parsed.sessions : [],
    fileOpenCount:
      parsed?.fileOpenCount && typeof parsed.fileOpenCount === "object"
        ? parsed.fileOpenCount
        : {},
    fileTimeSec:
      parsed?.fileTimeSec && typeof parsed.fileTimeSec === "object"
        ? parsed.fileTimeSec
        : {},
    domainTimeSec:
      parsed?.domainTimeSec && typeof parsed.domainTimeSec === "object"
        ? parsed.domainTimeSec
        : {},
    domainResourceOpenCount:
      parsed?.domainResourceOpenCount &&
      typeof parsed.domainResourceOpenCount === "object"
        ? parsed.domainResourceOpenCount
        : {},
    completionByFile:
      parsed?.completionByFile && typeof parsed.completionByFile === "object"
        ? parsed.completionByFile
        : {},
    metaCognitiveProgress: Array.isArray(parsed?.metaCognitiveProgress)
      ? parsed.metaCognitiveProgress
      : [],
  };
}

function openMetaReflectionDialog({ triggerLabel, scope, question }) {
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    const card = document.createElement("div");
    const title = document.createElement("div");
    const subtitle = document.createElement("div");
    const questionEl = document.createElement("div");
    const actions = document.createElement("div");
    const dismissBtn = document.createElement("button");

    overlay.style.position = "fixed";
    overlay.style.inset = "0";
    overlay.style.background = "rgba(15, 23, 42, 0.35)";
    overlay.style.display = "flex";
    overlay.style.alignItems = "center";
    overlay.style.justifyContent = "center";
    overlay.style.zIndex = "99999";

    card.style.width = "min(540px, calc(100vw - 32px))";
    card.style.background = "#ffffff";
    card.style.border = "1px solid #cbd5e1";
    card.style.borderRadius = "12px";
    card.style.boxShadow = "0 18px 40px rgba(15, 23, 42, 0.28)";
    card.style.padding = "14px";
    card.style.display = "grid";
    card.style.gap = "10px";

    title.textContent = `[Meta] ${triggerLabel}`;
    title.style.fontWeight = "700";
    title.style.fontSize = "14px";
    title.style.color = "#0f172a";

    subtitle.textContent = scope;
    subtitle.style.fontSize = "12px";
    subtitle.style.color = "#475569";

    questionEl.textContent = question;
    questionEl.style.fontSize = "13px";
    questionEl.style.lineHeight = "1.4";
    questionEl.style.color = "#1e293b";

    actions.style.display = "grid";
    actions.style.gridTemplateColumns = "repeat(4, minmax(0, 1fr))";
    actions.style.gap = "8px";

    const cleanup = () => {
      overlay.remove();
      window.removeEventListener("keydown", onKeydown);
    };

    const closeWith = (value) => {
      cleanup();
      resolve(value);
    };

    const onKeydown = (event) => {
      if (event.key === "Escape") {
        closeWith(null);
      }
    };

    for (const rating of [1, 2, 3, 4]) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.textContent = String(rating);
      btn.style.padding = "8px 0";
      btn.style.border = "1px solid #cbd5e1";
      btn.style.borderRadius = "8px";
      btn.style.background = "#f8fbff";
      btn.style.fontWeight = "700";
      btn.style.cursor = "pointer";
      btn.onclick = () => closeWith(rating);
      actions.appendChild(btn);
    }

    dismissBtn.type = "button";
    dismissBtn.textContent = "Agora não";
    dismissBtn.style.marginTop = "2px";
    dismissBtn.style.padding = "8px 10px";
    dismissBtn.style.border = "1px solid #cbd5e1";
    dismissBtn.style.borderRadius = "8px";
    dismissBtn.style.background = "#ffffff";
    dismissBtn.style.color = "#334155";
    dismissBtn.style.cursor = "pointer";
    dismissBtn.onclick = () => closeWith(null);

    overlay.onclick = (event) => {
      if (event.target === overlay) {
        closeWith(null);
      }
    };

    card.appendChild(title);
    card.appendChild(subtitle);
    card.appendChild(questionEl);
    card.appendChild(actions);
    card.appendChild(dismissBtn);
    overlay.appendChild(card);

    window.addEventListener("keydown", onKeydown);
    document.body.appendChild(overlay);
  });
}

export function useFileSystem(rootPath) {
  const [tree, setTree] = useState([]);
  const [selectedFile, setSelectedFile] = useState(null);
  const [selectedNode, setSelectedNode] = useState(null);
  const [nodeProfiles, setNodeProfiles] = useState({});
  const [content, setContent] = useState("");
  const [isDirty, setIsDirty] = useState(false);
  const [fileType, setFileType] = useState("text");
  const [learningAnalytics, setLearningAnalytics] =
    useState(defaultAnalytics());
  const sessionStartRef = useRef(Date.now());
  const activeFileRef = useRef({ path: null, domain: null, startedAt: 0 });
  const sessionMetaPromptedRef = useRef(false);
  const promptsThisSessionRef = useRef(0);
  const lastMetaPromptAtRef = useRef(0);
  const lastMetaOutcomeRef = useRef("answered");
  const questionCursorRef = useRef(0);
  const metaPromptInFlightRef = useRef(false);

  useEffect(() => {
    let mounted = true;

    async function loadRootState() {
      if (!rootPath) {
        setNodeProfiles({});
        setLearningAnalytics(defaultAnalytics());
        return;
      }

      const [profilesRaw, analyticsRaw, metaPromptStateRaw] = await Promise.all(
        [
          getRootStateValue(rootPath, NODE_LEARNING_PROFILES_KEY, {}),
          getRootStateValue(
            rootPath,
            LEARNING_ANALYTICS_KEY,
            defaultAnalytics(),
          ),
          getRootStateValue(rootPath, META_PROMPT_STATE_KEY, {}),
        ],
      );

      if (!mounted) return;
      setNodeProfiles(
        profilesRaw && typeof profilesRaw === "object" ? profilesRaw : {},
      );
      setLearningAnalytics(
        analyticsRaw && typeof analyticsRaw === "object"
          ? safeAnalytics(JSON.stringify(analyticsRaw))
          : defaultAnalytics(),
      );

      const hydratedState =
        metaPromptStateRaw && typeof metaPromptStateRaw === "object"
          ? metaPromptStateRaw
          : {};

      const hydratedLastPromptAt = Number(hydratedState.lastPromptAt ?? 0);
      const hydratedQuestionCursor = Number(hydratedState.questionCursor ?? 0);
      const hydratedLastOutcome = String(
        hydratedState.lastOutcome || "answered",
      );

      lastMetaPromptAtRef.current = Number.isFinite(hydratedLastPromptAt)
        ? hydratedLastPromptAt
        : 0;
      questionCursorRef.current = Number.isFinite(hydratedQuestionCursor)
        ? Math.max(0, Math.floor(hydratedQuestionCursor))
        : 0;
      lastMetaOutcomeRef.current = hydratedLastOutcome;
    }

    loadRootState();
    return () => {
      mounted = false;
    };
  }, [rootPath]);

  useEffect(() => {
    if (!rootPath) return;
    setRootStateValue(
      rootPath,
      NODE_LEARNING_PROFILES_KEY,
      nodeProfiles || {},
    ).catch(() => {});
  }, [nodeProfiles, rootPath]);

  useEffect(() => {
    if (!rootPath) return;
    setRootStateValue(
      rootPath,
      LEARNING_ANALYTICS_KEY,
      learningAnalytics || defaultAnalytics(),
    ).catch(() => {});
  }, [learningAnalytics, rootPath]);

  function updateAnalytics(mutator) {
    setLearningAnalytics((prev) => {
      const base = prev || defaultAnalytics();
      const next = mutator({ ...base }) || base;
      return next;
    });
  }

  function buildMetaPromptLabel(trigger) {
    if (trigger === "domain_switch") return "Mudanca de domínio";
    if (trigger === "test_end") return "Fim de teste/quiz";
    return "Check-in periodico";
  }

  async function persistMetaPromptState(patch = {}) {
    if (!rootPath) return;
    const current = await getRootStateValue(
      rootPath,
      META_PROMPT_STATE_KEY,
      {},
    );
    const next = {
      ...(current && typeof current === "object" ? current : {}),
      ...patch,
    };
    await setRootStateValue(rootPath, META_PROMPT_STATE_KEY, next);
  }

  async function askMetaReflection({
    trigger,
    domain = "",
    contentId = "",
    force = false,
  } = {}) {
    if (!rootPath || metaPromptInFlightRef.current) return null;

    const safeTrigger = resolveMetaTrigger(trigger);
    const now = Date.now();
    if (promptsThisSessionRef.current >= MAX_META_PROMPTS_PER_SESSION) {
      return null;
    }

    const throttled = shouldThrottleMetaPrompt({
      now,
      lastPromptAt: lastMetaPromptAtRef.current,
      lastOutcome: lastMetaOutcomeRef.current,
      trigger: safeTrigger,
      force,
    });
    if (throttled) {
      return null;
    }

    const question =
      META_QUESTIONS[questionCursorRef.current % META_QUESTIONS.length];

    const scope = domain ? `Domínio: ${domain}` : "Domínio: geral";
    const triggerLabel = buildMetaPromptLabel(safeTrigger);

    metaPromptInFlightRef.current = true;
    const answer = await openMetaReflectionDialog({
      triggerLabel,
      scope,
      question,
    });
    metaPromptInFlightRef.current = false;

    if (!Number.isInteger(answer) || answer < 1 || answer > 4) {
      lastMetaPromptAtRef.current = Date.now();
      lastMetaOutcomeRef.current = "dismissed";
      await persistMetaPromptState({
        lastPromptAt: lastMetaPromptAtRef.current,
        lastOutcome: lastMetaOutcomeRef.current,
        lastTrigger: safeTrigger,
        questionCursor: questionCursorRef.current,
      });
      return null;
    }

    const rating = answer;
    questionCursorRef.current = nextQuestionCursor({
      currentCursor: questionCursorRef.current,
      hasValidAnswer: true,
      questionCount: META_QUESTIONS.length,
    });

    const eventDate = new Date();
    const safeDomain = slugify(domain || "root") || "root";
    const conceptId = `${safeDomain}.meta.reflection`;
    const effectiveContentId = contentId || `_meta/${safeDomain}/reflection`;
    const score01 = (rating - 1) / 3;

    try {
      await upsertConcept({
        id: conceptId,
        name: `${domain || "Root"} Meta Reflection`,
        description:
          "Conceito sintetico para registar autoavaliacoes metacognitivas 1-4.",
      });
      await upsertContentItem({
        id: effectiveContentId,
        item_type: "reflection",
        title: `Meta reflection: ${domain || "root"}`,
        domain_id: safeDomain,
      });
      await mapContentConcept({
        content_id: effectiveContentId,
        concept_id: conceptId,
        coverage_weight: 1.0,
      });

      await ingestEvent({
        event_id: `meta-${eventDate.getTime()}-${Math.floor(Math.random() * 100000)}`,
        timestamp: eventDate.toISOString(),
        source: "meta_prompt",
        event_type: "self_assessment",
        content_id: effectiveContentId,
        concept_ids: [conceptId],
        payload: {
          score: score01,
          confidence: Math.max(0.25, Math.min(1, rating / 4)),
          meta_reflection_rating: rating,
          meta_trigger: safeTrigger,
          file_path: effectiveContentId,
        },
      });

      updateAnalytics((current) => {
        const nextMeta = [...(current.metaCognitiveProgress || [])];
        nextMeta.push({
          at: eventDate.toISOString(),
          domain: domain || "",
          file: effectiveContentId,
          eventType: "self_assessment",
          trigger: safeTrigger,
          rating,
          confidence: Math.max(0.25, Math.min(1, rating / 4)),
        });

        if (nextMeta.length > 500) {
          nextMeta.splice(0, nextMeta.length - 500);
        }

        return {
          ...current,
          metaCognitiveProgress: nextMeta,
        };
      });

      lastMetaPromptAtRef.current = Date.now();
      lastMetaOutcomeRef.current = "answered";
      promptsThisSessionRef.current += 1;
      sessionMetaPromptedRef.current =
        promptsThisSessionRef.current >= MAX_META_PROMPTS_PER_SESSION;

      await persistMetaPromptState({
        lastPromptAt: lastMetaPromptAtRef.current,
        lastOutcome: lastMetaOutcomeRef.current,
        lastTrigger: safeTrigger,
        questionCursor: questionCursorRef.current,
      });

      return rating;
    } catch (err) {
      console.warn("Meta reflection ingest skipped:", err);
      return null;
    }
  }

  async function requestDomainTransitionFeedback(fromDomain, toDomain) {
    const from = String(fromDomain || "").trim();
    const to = String(toDomain || "").trim();
    if (!from || from === to) return;
    await askMetaReflection({
      trigger: "domain_switch",
      domain: from,
      force: false,
    });
  }

  function closeActiveFileTimer() {
    const active = activeFileRef.current;
    if (!active?.path || !active.startedAt) return;

    const elapsedSec = Math.max(
      0,
      Math.round((Date.now() - active.startedAt) / 1000),
    );
    if (elapsedSec <= 0) {
      activeFileRef.current = { path: null, domain: null, startedAt: 0 };
      return;
    }

    updateAnalytics((current) => {
      const fileTimeSec = { ...(current.fileTimeSec || {}) };
      const domainTimeSec = { ...(current.domainTimeSec || {}) };
      fileTimeSec[active.path] = (fileTimeSec[active.path] || 0) + elapsedSec;
      if (active.domain) {
        domainTimeSec[active.domain] =
          (domainTimeSec[active.domain] || 0) + elapsedSec;
      }
      return {
        ...current,
        fileTimeSec,
        domainTimeSec,
      };
    });

    activeFileRef.current = { path: null, domain: null, startedAt: 0 };
  }

  function startActiveFileTimer(node) {
    closeActiveFileTimer();
    const path = normalize(node?.path || "");
    const domain = domainFromPath(path);
    activeFileRef.current = {
      path,
      domain,
      startedAt: Date.now(),
    };
  }

  function closeSessionWindow({ allowPrompt = true } = {}) {
    closeActiveFileTimer();
    const startedAt = sessionStartRef.current;
    const endedAt = Date.now();
    const durationSec = Math.max(0, Math.round((endedAt - startedAt) / 1000));

    updateAnalytics((current) => {
      const sessions = [...(current.sessions || [])];
      sessions.push({
        startedAt: new Date(startedAt).toISOString(),
        endedAt: new Date(endedAt).toISOString(),
        durationSec,
      });

      if (sessions.length > 300) {
        sessions.splice(0, sessions.length - 300);
      }

      return {
        ...current,
        sessions,
      };
    });

    if (
      allowPrompt &&
      durationSec >= META_MIN_SESSION_SEC &&
      !sessionMetaPromptedRef.current
    ) {
      const domainHint =
        activeFileRef.current?.domain ||
        selectedFile?.path?.split(/[\\/]/)?.[0] ||
        "";
      askMetaReflection({
        trigger: "periodic",
        domain: domainHint,
      });
    }

    sessionStartRef.current = Date.now();
    sessionMetaPromptedRef.current = false;
    promptsThisSessionRef.current = 0;
  }

  useEffect(() => {
    const onBeforeUnload = () => {
      closeSessionWindow({ allowPrompt: false });
    };

    window.addEventListener("beforeunload", onBeforeUnload);
    return () => {
      window.removeEventListener("beforeunload", onBeforeUnload);
      closeSessionWindow({ allowPrompt: false });
    };
  }, []);

  useEffect(() => {
    const intervalId = window.setInterval(() => {
      if (document.hidden) return;
      if (sessionMetaPromptedRef.current) return;
      if (promptsThisSessionRef.current >= MAX_META_PROMPTS_PER_SESSION) return;

      const elapsedSec = Math.max(
        0,
        Math.round((Date.now() - sessionStartRef.current) / 1000),
      );
      if (elapsedSec < META_MIN_SESSION_SEC) return;

      const domainHint =
        activeFileRef.current?.domain ||
        selectedFile?.path?.split(/[\\/]/)?.[0] ||
        "";

      askMetaReflection({
        trigger: "periodic",
        domain: domainHint,
      });
    }, 60000);

    return () => {
      window.clearInterval(intervalId);
    };
  }, [selectedFile, rootPath]);

  function getProfile(node) {
    if (!node?.path) return null;
    const key = normalize(node.path);
    return nodeProfiles[key] || null;
  }

  function ensureProfile(node) {
    if (!node?.path) return null;
    const key = normalize(node.path);
    const existing = nodeProfiles[key];
    if (existing) return normalizeProfile(existing, node);

    const fallback = buildDefaultProfile(node);
    setNodeProfiles((prev) => ({
      ...prev,
      [key]: fallback,
    }));
    return fallback;
  }

  function saveNodeProfile(nodePath, updates = {}) {
    const key = normalize(nodePath || "");
    if (!key) return;

    setNodeProfiles((prev) => {
      const base = prev[key] || {
        path: key,
        kind: "file",
        conceptId: "concept-auto",
        conceptName: "Concept",
        difficulty: 0.5,
      };

      const next = {
        ...base,
        ...updates,
      };

      next.path = key;
      const normalizedProfile = normalizeProfile(next, {
        path: key,
        is_dir: next.kind !== "file",
        name: key.split("/").filter(Boolean).pop() || "Concept",
      });

      return {
        ...prev,
        [key]: normalizedProfile,
      };
    });
  }

  function ensureDefaultsForTree(nodes) {
    setNodeProfiles((prev) => {
      let changed = false;
      const next = { ...prev };

      function walk(list) {
        if (!Array.isArray(list)) return;

        for (const node of list) {
          if (!node?.path) continue;
          const key = normalize(node.path);
          if (!next[key]) {
            next[key] = buildDefaultProfile(node);
            changed = true;
          } else {
            const normalizedExisting = normalizeProfile(next[key], node);
            if (
              normalizedExisting.conceptId !== next[key].conceptId ||
              normalizedExisting.domainId !== next[key].domainId ||
              normalizedExisting.domainName !== next[key].domainName
            ) {
              next[key] = normalizedExisting;
              changed = true;
            }
          }

          if (node.is_dir && Array.isArray(node.children)) {
            walk(node.children);
          }
        }
      }

      walk(nodes);
      return changed ? next : prev;
    });
  }

  async function loadTree() {
    if (!rootPath) return;
    const t = await getTree(rootPath);
    setTree(t);
    ensureDefaultsForTree(t);
  }

  function getExt(name) {
    const parts = name.split(".");
    return parts.length > 1 ? parts.pop().toLowerCase() : "";
  }

  async function updateInlineConceptCatalog(contentId, domainId, graph) {
    if (!rootPath || !contentId || !graph) return;

    const nowIso = new Date().toISOString();
    const currentCatalog = await getRootStateValue(
      rootPath,
      INLINE_CONCEPT_CATALOG_KEY,
      {},
    );
    const nextCatalog = {
      ...(currentCatalog && typeof currentCatalog === "object"
        ? currentCatalog
        : {}),
    };

    for (const concept of graph.concepts || []) {
      const existing = nextCatalog[concept.id] || {};
      nextCatalog[concept.id] = {
        ...existing,
        id: concept.id,
        name: concept.name,
        requires: Array.isArray(concept.requires) ? concept.requires : [],
        domain_id: domainId,
        last_seen_content_id: contentId,
        updated_at: nowIso,
      };
    }

    await setRootStateValue(rootPath, INLINE_CONCEPT_CATALOG_KEY, nextCatalog);
  }

  async function syncInlineConceptGraph(node, editorContent = "") {
    if (!rootPath || !node || node.is_dir) {
      return { profile: null, contentId: "", conceptIds: [] };
    }

    const profile =
      getProfile(node) || ensureProfile(node) || buildDefaultProfile(node);
    const contentId = normalize(node.path);
    const graph = extractInlineConceptGraph(editorContent, profile.domainId);

    // Fix 2: File-level fallback concept — every opened file contributes evidence
    if (!graph.concepts.length) {
      const fallbackConcept = buildFallbackInlineConcept(
        node.name,
        profile.domainId,
      );
      await upsertConcept({
        id: fallbackConcept.id,
        name: fallbackConcept.name,
        description: fallbackConcept.description,
      });
      await upsertContentItem({
        id: contentId,
        item_type: "note",
        title: node.name,
        domain_id: profile.domainId,
      });
      // Fix 3: Clear stale mappings before writing the fallback
      await removeContentConceptMaps(contentId);
      await mapContentConcept({
        content_id: contentId,
        concept_id: fallbackConcept.id,
        coverage_weight: fallbackConcept.coverageWeight,
      });
      return { profile, contentId, conceptIds: [fallbackConcept.id] };
    }

    for (const concept of graph.concepts) {
      await upsertConcept({
        id: concept.id,
        name: concept.name,
        description: concept.description,
      });
    }

    for (const edge of graph.edges) {
      await addEdge(edge);
    }

    await upsertContentItem({
      id: contentId,
      item_type: "note",
      title: node.name,
      domain_id: profile.domainId,
    });

    // Fix 3: Remove stale mappings before re-mapping current concepts
    await removeContentConceptMaps(contentId);

    // Fix 1: Use per-concept coverageWeight from declaration syntax
    const declaredConceptSet = new Set(graph.conceptIds || []);
    for (const concept of graph.concepts) {
      if (!declaredConceptSet.has(concept.id)) {
        continue;
      }
      await mapContentConcept({
        content_id: contentId,
        concept_id: concept.id,
        coverage_weight: concept.coverageWeight ?? 1.0,
      });
    }

    await updateInlineConceptCatalog(contentId, profile.domainId, graph);

    return {
      profile,
      contentId,
      conceptIds: graph.conceptIds,
    };
  }

  async function ensureInlineConceptCatalog(node, editorContent = "") {
    if (!rootPath || !node || node.is_dir) return;

    await syncInlineConceptGraph(node, editorContent);
  }

  async function trackOlmEvent(
    eventType,
    node,
    payload = {},
    editorContent = "",
  ) {
    if (!rootPath || !node || node.is_dir) return;

    const eventDate = new Date();

    try {
      const { profile, contentId, conceptIds } = await syncInlineConceptGraph(
        node,
        editorContent,
      );

      if (!profile || !conceptIds.length) {
        return;
      }

      await ingestEvent({
        event_id: `evt-${eventDate.getTime()}-${Math.floor(Math.random() * 100000)}`,
        timestamp: eventDate.toISOString(),
        source: "editor",
        event_type: eventType,
        content_id: contentId,
        concept_ids: conceptIds,
        payload: {
          ...payload,
          difficulty: profile.difficulty,
          file_path: contentId,
        },
      });

      updateAnalytics((current) => {
        const nextMeta = [...(current.metaCognitiveProgress || [])];
        const conf = Number(payload?.confidence);
        if (Number.isFinite(conf)) {
          nextMeta.push({
            at: eventDate.toISOString(),
            file: contentId,
            domain: domainFromPath(contentId),
            confidence: Math.max(0, Math.min(1, conf)),
            eventType,
          });
          if (nextMeta.length > 500) {
            nextMeta.splice(0, nextMeta.length - 500);
          }
        }

        const completionByFile = { ...(current.completionByFile || {}) };
        if (Number.isFinite(Number(payload?.completion_pct))) {
          completionByFile[contentId] = Math.max(
            0,
            Math.min(1, Number(payload.completion_pct)),
          );
        }

        return {
          ...current,
          completionByFile,
          metaCognitiveProgress: nextMeta,
        };
      });
    } catch (err) {
      console.warn("OLM auto-track skipped:", err);
    }
  }

  async function openFile(node) {
    setSelectedNode(node || null);
    ensureProfile(node);
    if (node.is_dir) {
      closeActiveFileTimer();
      setSelectedFile(null);
      setContent("");
      setIsDirty(false);
      return;
    }

    const ext = getExt(node.name);

    setFileType(
      ["png", "jpg", "jpeg"].includes(ext)
        ? "image"
        : ["md", "markdown"].includes(ext)
          ? "markdown"
          : "text",
    );

    const relPath = normalize(node.path);
    const text = await readFile(rootPath, relPath);

    setSelectedFile(node);
    setContent(text);
    setIsDirty(false);

    await ensureInlineConceptCatalog(node, text);

    const normalizedPath = normalize(node.path);
    const domainName = domainFromPath(normalizedPath);
    updateAnalytics((current) => {
      const fileOpenCount = { ...(current.fileOpenCount || {}) };
      const domainResourceOpenCount = {
        ...(current.domainResourceOpenCount || {}),
      };
      fileOpenCount[normalizedPath] = (fileOpenCount[normalizedPath] || 0) + 1;
      if (domainName) {
        if (!domainResourceOpenCount[domainName]) {
          domainResourceOpenCount[domainName] = {};
        }
        domainResourceOpenCount[domainName][normalizedPath] =
          (domainResourceOpenCount[domainName][normalizedPath] || 0) + 1;
      }

      return {
        ...current,
        fileOpenCount,
        domainResourceOpenCount,
      };
    });
    startActiveFileTimer(node);

    await trackOlmEvent(
      "review",
      node,
      {
        duration_sec: 60,
        target_duration_sec: 120,
        confidence: 0.6,
      },
      text,
    );
  }

  async function saveFile() {
    if (!selectedFile) return;
    await writeFile(rootPath, normalize(selectedFile.path), content);
    await ensureInlineConceptCatalog(selectedFile, content);
    setIsDirty(false);

    if (isQuizLikePath(selectedFile.path)) {
      await askMetaReflection({
        trigger: "test_end",
        domain: domainFromPath(selectedFile.path),
        contentId: normalize(selectedFile.path),
      });
    }
  }

  async function createMarkdown() {
    const rawName = prompt("Nome do ficheiro (.md):");
    if (!rawName) return;

    const trimmedName = rawName.trim();
    if (!trimmedName) return;

    const name = trimmedName.toLowerCase().endsWith(".md")
      ? trimmedName
      : `${trimmedName}.md`;

    const targetNode = selectedNode || selectedFile;
    const basePath = targetNode?.is_dir
      ? normalize(targetNode.path)
      : normalize(targetNode?.path || "")
          .split("/")
          .slice(0, -1)
          .join("/");
    const relPath = basePath ? `${basePath}/${name}` : name;

    await createFile(rootPath, relPath);
    await loadTree();
  }

  async function createFolderAtSelection(node = null) {
    if (!rootPath) return;

    const rawName = prompt("Nome da pasta:");
    if (!rawName) return;

    const folderName = rawName.trim();
    if (!folderName) return;

    const targetNode = node || selectedNode || selectedFile;
    const basePath = targetNode?.is_dir
      ? normalize(targetNode.path)
      : normalize(targetNode?.path || "")
          .split("/")
          .slice(0, -1)
          .join("/");

    const relPath = basePath ? `${basePath}/${folderName}` : folderName;
    await createFolder(rootPath, relPath);
    await loadTree();
  }

  async function createMarkdownInFolder(node) {
    if (!node || !node.is_dir) return;

    setSelectedNode(node);

    const rawName = prompt(`Nome do ficheiro em ${node.name} (.md):`);
    if (!rawName) return;

    const trimmedName = rawName.trim();
    if (!trimmedName) return;

    const fileName = trimmedName.toLowerCase().endsWith(".md")
      ? trimmedName
      : `${trimmedName}.md`;

    const relPath = `${normalize(node.path)}/${fileName}`;

    await createFile(rootPath, relPath);
    await loadTree();
  }

  async function createQuizTemplate() {
    if (!rootPath) {
      alert("Seleciona primeiro a pasta root.");
      return;
    }

    const rawType = prompt("Tipo de template (quiz ou teste):", "quiz");

    if (!rawType) return;

    const normalized = rawType.trim().toLowerCase();
    const kind = normalized === "teste" ? "teste" : "quiz";

    const title = prompt(
      "Título do ficheiro:",
      kind === "teste" ? "Teste rápido" : "Quiz rápido",
    );

    if (!title || !title.trim()) return;

    const datePart = new Date().toISOString().slice(0, 10);
    const cleanTitle = slugify(title) || kind;

    const targetNode = selectedNode || selectedFile;
    const selectedRelPath = targetNode?.path || "";
    const segments = selectedRelPath.split(/[\\/]/).filter(Boolean);
    const folderPath = targetNode?.is_dir
      ? segments.join("/")
      : segments.slice(0, -1).join("/");

    const fileName = `${kind}-${datePart}-${cleanTitle}.md`;
    const relPath = folderPath ? `${folderPath}/${fileName}` : fileName;
    const templateContent = makeTemplateContent(kind, title.trim());

    await createFile(rootPath, relPath);
    await writeFile(rootPath, relPath, templateContent);
    await loadTree();
    await openFile({
      name: fileName,
      path: relPath,
      is_dir: false,
    });
  }

  async function renameSelected() {
    if (!selectedFile) return;

    const newName = prompt("Novo nome:", selectedFile.name);
    if (!newName) return;

    await renameFile(rootPath, normalize(selectedFile.path), newName);
    setSelectedFile(null);
    setContent("");
    await loadTree();
  }

  async function renameNode(node) {
    if (!node?.path) return;
    const newName = prompt("Novo nome:", node.name);
    if (!newName || !newName.trim()) return;

    await renameFile(rootPath, normalize(node.path), newName.trim());
    if (selectedFile?.path === node.path || selectedNode?.path === node.path) {
      setSelectedFile(null);
      setSelectedNode(null);
      setContent("");
      setIsDirty(false);
    }
    await loadTree();
  }

  async function deleteNode(node) {
    if (!node?.path) return;
    const ok = window.confirm(
      `Eliminar ${node.name}? Esta ação é irreversível.`,
    );
    if (!ok) return;

    await deletePath(rootPath, normalize(node.path));
    if (selectedFile?.path === node.path || selectedNode?.path === node.path) {
      clearSelection();
    }
    await loadTree();
  }

  async function handleNodeContextAction(node, action) {
    if (!node || !action) return;

    if (action === "new-file") {
      if (node.is_dir) {
        await createMarkdownInFolder(node);
      } else {
        const parent = normalize(node.path).split("/").slice(0, -1).join("/");
        await createMarkdownInFolder({
          ...node,
          is_dir: true,
          path: parent,
          name: parent || "/",
        });
      }
      return;
    }

    if (action === "new-folder") {
      await createFolderAtSelection(node);
      return;
    }

    if (action === "rename") {
      await renameNode(node);
      return;
    }

    if (action === "delete") {
      await deleteNode(node);
    }
  }

  function clearSelection() {
    closeActiveFileTimer();
    setSelectedNode(null);
    setSelectedFile(null);
    setContent("");
    setIsDirty(false);
  }

  return {
    // estado
    tree,
    selectedFile,
    selectedNode,
    nodeProfiles,
    learningAnalytics,
    content,
    isDirty,
    fileType,

    // setters
    setContent,
    setIsDirty,

    // ações
    loadTree,
    openFile,
    saveFile,
    saveNodeProfile,
    getNodeProfile: getProfile,
    createMarkdown,
    createMarkdownInFolder,
    createFolderAtSelection,
    createQuizTemplate,
    renameSelected,
    renameNode,
    deleteNode,
    handleNodeContextAction,
    clearSelection,
    askMetaReflection,
    requestDomainTransitionFeedback,
  };
}
