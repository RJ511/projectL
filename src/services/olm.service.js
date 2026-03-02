import { call } from "./tauriBridge";

export function upsertConcept(concept) {
  return call("olm_upsert_concept", { concept });
}

export function listConcepts() {
  return call("olm_list_concepts");
}

export function addEdge(edge) {
  return call("olm_add_edge", { edge });
}

export function listEdges() {
  return call("olm_list_edges");
}

export function upsertContentItem(item) {
  return call("olm_upsert_content_item", { item });
}

export function mapContentConcept(map) {
  return call("olm_map_content_concept", { map });
}

export function ingestEvent(event) {
  return call("olm_ingest_event", { event });
}

export function getOlmState() {
  return call("olm_get_state");
}

export function getExplain(conceptId, limit = 5) {
  return call("olm_get_explain", { conceptId, limit });
}

export function nextToStudy(options = {}) {
  const { top = 10, lambda = 0.7, readinessThreshold = 0.6 } = options;

  return call("olm_next_to_study", {
    top,
    lambda,
    readinessThreshold,
  });
}
