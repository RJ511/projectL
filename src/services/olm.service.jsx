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
  const {
    top = 10,
    lambda = 0.7,
    readinessThreshold = 0.6,
    domainFilter = null,
    exclude = null,
  } = options;

  return call("olm_next_to_study", {
    top,
    lambda,
    readinessThreshold,
    domainFilter,
    exclude,
  });
}

export function getOlmConfig() {
  return call("olm_get_config");
}

export function setOlmConfig(config) {
  return call("olm_set_config", { config });
}

export function resetOlmState() {
  return call("olm_reset_state");
}

export function exportOlmJson() {
  return call("olm_export_json");
}

export function importOlmJson(json) {
  return call("olm_import_json", { json });
}

export function saveOlmState(path = null) {
  return call("olm_save_state", { path });
}

export function loadOlmState(path = null) {
  return call("olm_load_state", { path });
}

export function getDebugRanking(options = {}) {
  const { top = null, domainFilter = null, exclude = null } = options;
  return call("olm_get_debug_ranking", { top, domainFilter, exclude });
}
