const INLINE_CONCEPT_REGEX = /(;{2,3})\s*([^;\n][^;\n]{0,160}?)\s*\1/g;

function slugify(value) {
  return (value || "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
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

export function parseInlineConceptToken(rawToken = "") {
  let token = String(rawToken || "").trim();
  if (!token) return null;

  let coverageWeight = 1.0;
  const weightMatch = token.match(/\[(\d+(?:\.\d+)?)\]\s*$/);
  if (weightMatch) {
    const parsed = parseFloat(weightMatch[1]);
    if (Number.isFinite(parsed) && parsed > 0 && parsed <= 1) {
      coverageWeight = parsed;
    }
    token = token.slice(0, weightMatch.index).trim();
  }

  const [rawConceptLabel, ...rawRequiresParts] = token.split(":");
  const conceptLabel = String(rawConceptLabel || "").trim();
  if (!conceptLabel) return null;

  const prereqLabels = rawRequiresParts
    .join(":")
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);

  return {
    conceptLabel,
    prereqLabels,
    coverageWeight,
  };
}

export function extractInlineConceptGraph(content, domainId) {
  const text = String(content || "");
  if (!text) {
    return {
      concepts: [],
      edges: [],
      conceptIds: [],
    };
  }

  const byId = new Map();
  const declaredConceptIds = new Set();
  const edgeKeySet = new Set();
  const edges = [];

  for (const match of text.matchAll(INLINE_CONCEPT_REGEX)) {
    const token = parseInlineConceptToken(match?.[2] || "");
    if (!token) continue;

    const conceptId = normalizeConceptId(
      `inline.${token.conceptLabel}`,
      domainId,
    );
    if (!conceptId) continue;
    declaredConceptIds.add(conceptId);

    const existing = byId.get(conceptId);
    const requires = existing?.requires || new Set();
    const resolvedWeight = existing
      ? Math.max(existing.coverageWeight ?? 1.0, token.coverageWeight)
      : token.coverageWeight;

    for (const prereqLabel of token.prereqLabels) {
      const prereqId = normalizeConceptId(`inline.${prereqLabel}`, domainId);
      if (!prereqId || prereqId === conceptId) continue;

      const prereqSlug = prereqId.split(".").pop() || "";
      let resolvedPrereqId = prereqId;
      for (const [existingId] of byId) {
        const existingSlug = existingId.split(".").pop() || "";
        if (
          existingId !== prereqId &&
          existingSlug &&
          prereqSlug &&
          (existingSlug === prereqSlug ||
            existingSlug.includes(prereqSlug) ||
            prereqSlug.includes(existingSlug))
        ) {
          resolvedPrereqId = existingId;
          break;
        }
      }

      requires.add(resolvedPrereqId);

      if (!byId.has(resolvedPrereqId)) {
        byId.set(resolvedPrereqId, {
          id: resolvedPrereqId,
          name: prereqLabel,
          description: `Conceito pré-requisito inferido de marcação ;;;${token.conceptLabel}:${prereqLabel};;;`,
          requires: new Set(),
          coverageWeight: 1.0,
        });
      }

      const edgeKey = `${resolvedPrereqId}=>${conceptId}`;
      if (!edgeKeySet.has(edgeKey)) {
        edgeKeySet.add(edgeKey);
        edges.push({ prereq_id: resolvedPrereqId, target_id: conceptId });
      }
    }

    byId.set(conceptId, {
      id: conceptId,
      name: token.conceptLabel,
      description: `Conceito inline extraído de marcação ;;;${token.conceptLabel};;;`,
      requires,
      coverageWeight: resolvedWeight,
    });
  }

  const concepts = Array.from(byId.values()).map((concept) => ({
    id: concept.id,
    name: concept.name,
    description: concept.description,
    requires: Array.from(concept.requires).sort((a, b) => a.localeCompare(b)),
    coverageWeight: concept.coverageWeight ?? 1.0,
  }));

  return {
    concepts,
    edges,
    conceptIds: Array.from(declaredConceptIds),
  };
}

export function buildFallbackInlineConcept(fileName, domainId) {
  const safeName = String(fileName || "").trim() || "unnamed";
  const safeDomain = slugify(domainId || "") || "root";
  return {
    id: `${safeDomain}.file.${slugify(safeName) || "unnamed"}`,
    name: safeName,
    description: `Conceito de ficheiro para ${safeName}`,
    coverageWeight: 1.0,
  };
}
