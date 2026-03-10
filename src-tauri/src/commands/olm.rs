#![allow(dead_code)]

use chrono::{DateTime, Utc};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use tauri::State;

const DEFAULT_META_STRENGTH: f64 = 0.6;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OlmConfig {
    pub lambda: f64,
    pub gamma: f64,
    pub theta: f64,
    pub min_readiness: f64,
    pub soft_gate_k: f64,
    pub meta_strength: f64,
    pub root_penalty: f64,
    pub stop_mastery: f64,
    pub exclude_concepts: Vec<String>,
    pub uncertainty_formula: String,
    pub decay_enabled: bool,
    pub decay_half_life_days: f64,
}

impl Default for OlmConfig {
    fn default() -> Self {
        OlmConfig {
            lambda: 0.7,
            gamma: 0.35,
            theta: 0.5,
            min_readiness: 0.1,
            soft_gate_k: 1.6,
            meta_strength: DEFAULT_META_STRENGTH,
            root_penalty: 0.12,
            stop_mastery: 0.85,
            exclude_concepts: vec![],
            uncertainty_formula: "standard".to_string(),
            decay_enabled: false,
            decay_half_life_days: 30.0,
        }
    }
}

#[derive(Default)]
pub struct OlmState {
    inner: Mutex<OlmStore>,
}

#[derive(Default)]
struct OlmStore {
    concepts: HashMap<String, Concept>,
    edges: Vec<ConceptEdge>,
    content_items: HashMap<String, ContentItem>,
    content_concepts: Vec<ContentConceptMap>,
    study_events: Vec<StudyEvent>,
    concept_state: HashMap<String, ConceptState>,
    evidence: HashMap<String, Vec<EvidenceChunk>>,
    config: OlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptEdge {
    pub prereq_id: String,
    pub target_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItem {
    pub id: String,
    pub item_type: String,
    pub title: String,
    pub domain_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentConceptMap {
    pub content_id: String,
    pub concept_id: String,
    pub coverage_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudyEvent {
    pub event_id: String,
    pub timestamp: String,
    pub source: String,
    pub event_type: String,
    pub content_id: Option<String>,
    pub concept_ids: Vec<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptState {
    pub alpha: f64,
    pub beta: f64,
    pub last_update: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceChunk {
    pub event_id: String,
    pub delta_alpha: f64,
    pub delta_beta: f64,
    pub event_type: String,
    pub score: f64,
    pub applied_weight: f64,
    pub metacognitive_weight: f64,
    pub metacognitive_alignment: f64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConceptStateView {
    pub concept_id: String,
    pub name: String,
    pub mastery: f64,
    pub uncertainty: f64,
    pub alpha: f64,
    pub beta: f64,
    pub last_update: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NextToStudyItem {
    pub concept_id: String,
    pub name: String,
    pub score: f64,
    pub readiness: f64,
    pub mastery: f64,
    pub uncertainty: f64,
    pub gate_factor: f64,
    pub event_count: usize,
    pub why: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IngestResult {
    pub updated_concepts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentMetric {
    pub content_id: String,
    pub title: Option<String>,
    pub attempts: usize,
    pub correct_sum: f64,
    pub total_sum: f64,
    pub success_rate: f64,
    pub avg_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct NextContentToStudyItem {
    pub content_id: String,
    pub title: Option<String>,
    pub score: f64,
    pub attempts: usize,
    pub avg_score: f64,
    pub mapping_quality: f64,
    pub mapping_penalty: f64,
    pub supporting_concepts: Vec<String>,
    pub why: Vec<String>,
}

#[derive(Debug, Clone)]
struct ContentMappingQuality {
    quality: f64,
    penalty: f64,
    raw_weight_sum: f64,
    mapped_concepts: usize,
}

#[derive(Debug, Clone)]
struct ApproachConfig {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    lambda: f64,
    readiness_threshold: f64,
    min_readiness: f64,
    readiness_gamma: f64,
    root_penalty: f64,
    meta_strength: f64,
    soft_gate_k: Option<f64>,
    stop_mastery: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationOptions {
    pub generated_scenarios: usize,
    pub graph_size: usize,
    pub event_count: usize,
    pub prereq_density: f64,
    pub depth: usize,
    pub mapping_quality: f64,
}

impl Default for SimulationOptions {
    fn default() -> Self {
        Self {
            generated_scenarios: 0,
            graph_size: 6,
            event_count: 24,
            prereq_density: 0.35,
            depth: 3,
            mapping_quality: 0.8,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioMetadata {
    pub mode: String,
    pub seed: u64,
    pub graph_size: usize,
    pub prereq_density: f64,
    pub depth: usize,
    pub event_count: usize,
    pub event_mix: HashMap<String, f64>,
    pub error_rate: f64,
    pub confidence_level: f64,
    pub effort_level: f64,
    pub metacognitive_alignment: f64,
    pub mapping_quality: f64,
}

#[derive(Debug, Clone)]
struct ScenarioBlueprint {
    id: String,
    name: String,
    expected_any: Vec<String>,
    concepts: Vec<String>,
    edges: Vec<(String, String)>,
    content_items: Vec<ContentItem>,
    content_concepts: Vec<ContentConceptMap>,
    events: Vec<StudyEvent>,
    metadata: ScenarioMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulationScenarioResult {
    pub scenario_id: String,
    pub scenario_name: String,
    pub expected_any: Vec<String>,
    pub scenario_metadata: ScenarioMetadata,
    pub top_recommendation: Option<String>,
    pub top_recommendations: Vec<String>,
    pub rank_of_first_expected: Option<usize>,
    pub pass: bool,
    pub hit_at_3: bool,
    pub mrr: f64,
    pub ndcg_at_3: f64,
    pub avg_mastery: f64,
    pub avg_uncertainty: f64,
    pub diagnostics: ScenarioDiagnostics,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulationApproachResult {
    pub approach_id: String,
    pub approach_name: String,
    pub description: String,
    pub pass_rate: f64,
    pub hit_at_3_rate: f64,
    pub avg_mrr: f64,
    pub avg_ndcg_at_3: f64,
    pub train_pass_rate: f64,
    pub test_pass_rate: f64,
    pub train_avg_mrr: f64,
    pub test_avg_mrr: f64,
    pub scenarios: Vec<SimulationScenarioResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioDiagnostics {
    pub candidates_before_gate: usize,
    pub candidates_after_gate: usize,
    pub candidates_ranked: usize,
    pub candidates_excluded_min_readiness: usize,
    pub readiness_distribution: Vec<f64>,
    pub concept_event_counts: HashMap<String, usize>,
    pub top_candidates: Vec<NextToStudyItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulationCalibrationSummary {
    pub train_scenarios: Vec<String>,
    pub test_scenarios: Vec<String>,
    pub selection_metric: String,
    pub selected_approach_id: String,
    pub selected_test_pass_rate: f64,
    pub selected_test_avg_mrr: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulationReport {
    pub generated_at: String,
    pub scenarios_count: usize,
    pub approaches: Vec<SimulationApproachResult>,
    pub best_approach_id: String,
    pub calibration: SimulationCalibrationSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OlmSnapshot {
    concepts: HashMap<String, Concept>,
    edges: Vec<ConceptEdge>,
    content_items: HashMap<String, ContentItem>,
    content_concepts: Vec<ContentConceptMap>,
    #[serde(default)]
    study_events: Vec<StudyEvent>,
    concept_state: HashMap<String, ConceptState>,
    evidence: HashMap<String, Vec<EvidenceChunk>>,
    config: OlmConfig,
}

impl From<&OlmStore> for OlmSnapshot {
    fn from(store: &OlmStore) -> Self {
        OlmSnapshot {
            concepts: store.concepts.clone(),
            edges: store.edges.clone(),
            content_items: store.content_items.clone(),
            content_concepts: store.content_concepts.clone(),
            study_events: store.study_events.clone(),
            concept_state: store.concept_state.clone(),
            evidence: store.evidence.clone(),
            config: store.config.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RankingDebugResponse {
    pub candidates: Vec<NextToStudyItem>,
    pub diagnostics: ScenarioDiagnostics,
}

fn clamp_01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

fn finite_weight(weight: f64) -> Option<f64> {
    if !weight.is_finite() || weight <= 0.0 {
        return None;
    }
    Some(weight)
}

fn normalized_weights(raw: &[(String, f64)]) -> Vec<(String, f64)> {
    let mut acc: HashMap<String, f64> = HashMap::new();
    for (id, weight) in raw {
        if let Some(valid) = finite_weight(*weight) {
            *acc.entry(id.clone()).or_insert(0.0) += valid;
        }
    }

    let total: f64 = acc.values().sum();
    if total <= 0.0 {
        return Vec::new();
    }

    acc.into_iter()
        .map(|(id, weight)| (id, weight / total))
        .collect()
}

fn content_mapping_quality(raw_weight_sum: f64, mapped_concepts: usize) -> ContentMappingQuality {
    let coverage_quality = clamp_01(raw_weight_sum);
    let breadth_quality = match mapped_concepts {
        0 => 0.0,
        1 => 0.6,
        2 => 0.8,
        _ => 1.0,
    };

    let quality = clamp_01((0.7 * coverage_quality) + (0.3 * breadth_quality));
    let penalty = 0.55 + (0.45 * quality);

    ContentMappingQuality {
        quality,
        penalty,
        raw_weight_sum,
        mapped_concepts,
    }
}

fn ensure_state<'a>(store: &'a mut OlmStore, concept_id: &str) -> &'a mut ConceptState {
    store
        .concept_state
        .entry(concept_id.to_string())
        .or_insert(ConceptState {
            alpha: 1.0,
            beta: 1.0,
            last_update: None,
        })
}

fn mastery(state: &ConceptState) -> f64 {
    let total = state.alpha + state.beta;
    if total <= 0.0 {
        0.5
    } else {
        state.alpha / total
    }
}

fn uncertainty(state: &ConceptState) -> f64 {
    let total = state.alpha + state.beta;
    if total <= 0.0 {
        1.0
    } else {
        1.0 / total
    }
}

fn uncertainty_with_formula(state: &ConceptState, formula: &str) -> f64 {
    let total = state.alpha + state.beta;
    if total <= 0.0 {
        return 1.0;
    }
    match formula {
        "sqrt" => 1.0 / total.sqrt(),
        _ => 1.0 / total,
    }
}

fn decay_state_if_needed(state: &ConceptState, config: &OlmConfig) -> ConceptState {
    if !config.decay_enabled {
        return state.clone();
    }

    let half_life_days = config.decay_half_life_days.max(1.0);
    let Some(last_update) = state.last_update.as_ref() else {
        return state.clone();
    };

    let Ok(parsed_last) = DateTime::parse_from_rfc3339(last_update) else {
        return state.clone();
    };

    let now = Utc::now();
    let last_utc = parsed_last.with_timezone(&Utc);
    let seconds_elapsed = now.signed_duration_since(last_utc).num_seconds();

    if seconds_elapsed <= 0 {
        return state.clone();
    }

    let days_elapsed = (seconds_elapsed as f64) / 86_400.0;
    let decay_factor = 2f64.powf(-days_elapsed / half_life_days);

    ConceptState {
        alpha: 1.0 + ((state.alpha - 1.0).max(0.0) * decay_factor),
        beta: 1.0 + ((state.beta - 1.0).max(0.0) * decay_factor),
        last_update: state.last_update.clone(),
    }
}

fn build_exclude_set(config_excludes: &[String], extra: Option<&[String]>) -> HashSet<String> {
    let mut set: HashSet<String> = config_excludes.iter().cloned().collect();
    if let Some(extra_list) = extra {
        set.extend(extra_list.iter().cloned());
    }
    set
}

fn parse_payload_float(payload: &Value, key: &str) -> Option<f64> {
    payload.get(key).and_then(|v| v.as_f64())
}

fn event_type_weight(event_type: &str) -> f64 {
    match event_type {
        "quiz_attempt" => 1.0,
        "practice_attempt" => 0.7,
        "flashcard_review" => 0.6,
        "study_read" => 0.2,
        "review" => 0.12,
        "note_taking" => 0.08,
        "self_assessment" => 0.3,
        _ => 0.2,
    }
}
fn event_score(event: &StudyEvent) -> f64 {
    match event.event_type.as_str() {
        "quiz_attempt" | "practice_attempt" => {
            let correct = parse_payload_float(&event.payload, "correct").unwrap_or(0.0);
            let total = parse_payload_float(&event.payload, "total").unwrap_or(1.0);
            if total > 0.0 {
                clamp_01(correct / total)
            } else {
                0.5
            }
        }
        "study_read" => {
            let duration = parse_payload_float(&event.payload, "duration_sec").unwrap_or(0.0);
            let target =
                parse_payload_float(&event.payload, "target_duration_sec").unwrap_or(600.0);
            if target <= 0.0 {
                0.0
            } else {
                clamp_01(duration / target) * 0.5
            }
        }
        "review" => {
            let duration = parse_payload_float(&event.payload, "duration_sec").unwrap_or(0.0);
            let target =
                parse_payload_float(&event.payload, "target_duration_sec").unwrap_or(600.0);
            if target <= 0.0 {
                0.0
            } else {
                clamp_01(duration / target) * 0.35
            }
        }
        "note_taking" => {
            let chars_written = parse_payload_float(&event.payload, "chars_written").unwrap_or(0.0);
            let target_chars = parse_payload_float(&event.payload, "target_chars").unwrap_or(300.0);
            if target_chars <= 0.0 {
                0.0
            } else {
                clamp_01(chars_written / target_chars) * 0.35
            }
        }
        "flashcard_review" => {
            if let Some(rating) = event.payload.get("rating").and_then(|v| v.as_str()) {
                if rating.eq_ignore_ascii_case("again") {
                    0.0
                } else {
                    1.0
                }
            } else {
                0.5
            }
        }
        "self_assessment" => parse_payload_float(&event.payload, "score")
            .or_else(|| parse_payload_float(&event.payload, "confidence"))
            .map(clamp_01)
            .unwrap_or(0.5),
        _ => 0.5,
    }
}

fn limiter_for_event(event_type: &str, prior_evidence_count: usize) -> (f64, f64) {
    let warmup_steps = match event_type {
        "quiz_attempt" => 8.0,
        "practice_attempt" => 10.0,
        "flashcard_review" => 8.0,
        "self_assessment" => 10.0,
        "review" | "study_read" | "note_taking" => 14.0,
        _ => 12.0,
    };

    let n = prior_evidence_count as f64;
    let warmup_factor = ((n + 1.0) / warmup_steps).clamp(0.15, 1.0);

    let hard_cap = match event_type {
        "quiz_attempt" => 0.35,
        "practice_attempt" => 0.25,
        "flashcard_review" => 0.20,
        "self_assessment" => 0.18,
        "review" | "study_read" | "note_taking" => 0.10,
        _ => 0.15,
    };

    (warmup_factor, hard_cap)
}

fn objective_score(event: &StudyEvent) -> Option<f64> {
    match event.event_type.as_str() {
        "quiz_attempt" | "practice_attempt" => {
            let correct = parse_payload_float(&event.payload, "correct")?;
            let total = parse_payload_float(&event.payload, "total").unwrap_or(1.0);
            if total > 0.0 {
                Some(clamp_01(correct / total))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn confidence_weight(event: &StudyEvent) -> f64 {
    parse_payload_float(&event.payload, "confidence")
        .map(clamp_01)
        .map(|c| 0.5 + (0.5 * c))
        .unwrap_or(1.0)
}

#[derive(Debug, Clone, Copy)]
struct MetaAdjustment {
    effective_weight: f64,
    alignment: f64,
    reliability: f64,
    effective_score: f64,
}

fn count_self_assessment_same_day(entries: &[EvidenceChunk], timestamp: &str) -> usize {
    let Some(day_key) = timestamp.get(0..10) else {
        return 0;
    };

    entries
        .iter()
        .filter(|row| {
            row.event_type == "self_assessment"
                && row
                    .created_at
                    .get(0..10)
                    .map(|d| d == day_key)
                    .unwrap_or(false)
        })
        .count()
}

fn meta_session_cap(event_type: &str, prior_self_assessment_same_day: usize) -> f64 {
    if event_type != "self_assessment" {
        return 1.0;
    }

    if prior_self_assessment_same_day <= 2 {
        return 1.0;
    }

    let overflow = (prior_self_assessment_same_day - 2) as f64;
    (1.0 - (0.20 * overflow)).clamp(0.4, 1.0)
}

fn blend_with_objective(score: f64, objective: Option<f64>, reliability: f64) -> f64 {
    let Some(objective_score) = objective else {
        return score;
    };

    let objective_weight = (0.65 + (0.25 * clamp_01(reliability))).clamp(0.65, 0.9);
    clamp_01((objective_weight * objective_score) + ((1.0 - objective_weight) * score))
}

fn meta_reliability(
    event: &StudyEvent,
    objective: Option<f64>,
    alignment: f64,
    prior_evidence_count: usize,
    session_cap: f64,
) -> f64 {
    let confidence = parse_payload_float(&event.payload, "confidence")
        .map(clamp_01)
        .unwrap_or(0.5);

    let objective_term = if objective.is_some() { 1.0 } else { 0.65 };
    let confidence_term = 0.6 + (0.4 * confidence);
    let alignment_term = 0.55 + (0.45 * clamp_01(alignment));
    let evidence_term = ((prior_evidence_count as f64 + 1.0) / 12.0).clamp(0.3, 1.0);
    let historical_term = 0.5 + (0.5 * evidence_term);

    clamp_01(objective_term * confidence_term * alignment_term * historical_term * session_cap)
}

fn metacognitive_signal(
    event: &StudyEvent,
    objective: Option<f64>,
    raw_score: f64,
    meta_strength: f64,
    prior_evidence_count: usize,
    prior_self_assessment_same_day: usize,
) -> MetaAdjustment {
    let strength = clamp_01(meta_strength);

    let confidence = parse_payload_float(&event.payload, "confidence")
        .map(clamp_01)
        .unwrap_or(0.5);
    let perceived_score = parse_payload_float(&event.payload, "perceived_score").map(clamp_01);
    let difficulty = parse_payload_float(&event.payload, "difficulty")
        .or_else(|| parse_payload_float(&event.payload, "perceived_difficulty"))
        .map(clamp_01)
        .unwrap_or(0.5);
    let effort = parse_payload_float(&event.payload, "effort")
        .or_else(|| parse_payload_float(&event.payload, "effort_level"))
        .map(clamp_01)
        .unwrap_or(0.6);

    let alignment = if let (Some(obj), Some(perceived)) = (objective, perceived_score) {
        clamp_01(1.0 - (obj - perceived).abs())
    } else if let Some(obj) = objective {
        clamp_01(1.0 - (obj - confidence).abs())
    } else if let Some(perceived) = perceived_score {
        clamp_01(1.0 - (perceived - confidence).abs())
    } else {
        0.5
    };

    let difficulty_term = 1.0 - (0.3 * ((difficulty - 0.5).abs() * 2.0));
    let effort_term = 0.7 + (0.3 * effort);
    let confidence_term = 0.75 + (0.25 * confidence);
    let alignment_term = 0.6 + (0.4 * alignment);

    let meta_raw = clamp_01(difficulty_term * effort_term * confidence_term * alignment_term);

    let session_cap = meta_session_cap(&event.event_type, prior_self_assessment_same_day);
    let reliability = if strength <= 0.0 {
        1.0
    } else {
        meta_reliability(
            event,
            objective,
            alignment,
            prior_evidence_count,
            session_cap,
        )
    };

    let raw_meta_weight = if strength <= 0.0 {
        1.0
    } else {
        clamp_01((1.0 - strength) + (strength * meta_raw))
    };

    // Low reliability shrinks metacognitive influence towards neutral weight=1.0.
    let effective_weight = clamp_01(1.0 + ((raw_meta_weight - 1.0) * reliability));

    let shrunk_score = 0.5 + ((raw_score - 0.5) * (0.4 + (0.6 * reliability)));
    let effective_score = blend_with_objective(clamp_01(shrunk_score), objective, reliability);

    MetaAdjustment {
        effective_weight,
        alignment,
        reliability,
        effective_score,
    }
}

fn normalize_maps(store: &OlmStore, event: &StudyEvent) -> Vec<(String, f64)> {
    if !event.concept_ids.is_empty() {
        let raw: Vec<(String, f64)> = event
            .concept_ids
            .iter()
            .map(|concept_id| (concept_id.clone(), 1.0))
            .collect();
        return normalized_weights(&raw);
    }

    if let Some(content_id) = &event.content_id {
        let raw: Vec<(String, f64)> = store
            .content_concepts
            .iter()
            .filter(|map| &map.content_id == content_id)
            .map(|map| (map.concept_id.clone(), map.coverage_weight))
            .collect();
        return normalized_weights(&raw);
    }

    Vec::new()
}

fn ingest_into_store(
    store: &mut OlmStore,
    event: &StudyEvent,
    meta_strength: f64,
) -> Result<Vec<String>, String> {
    if event.event_id.trim().is_empty() {
        return Err("event_id is required".to_string());
    }

    let score = event_score(event);
    let objective = objective_score(event);
    let base_weight = event_type_weight(&event.event_type);
    let confidence = confidence_weight(event);

    let mapped_concepts = normalize_maps(store, event);
    if mapped_concepts.is_empty() {
        return Err(
            "Event must include concept_ids or a content_id mapped to concepts".to_string(),
        );
    }

    store.study_events.push(event.clone());
    if store.study_events.len() > 5000 {
        let overflow = store.study_events.len() - 5000;
        store.study_events.drain(0..overflow);
    }

    let mut updated = HashSet::new();

    for (concept_id, mapping_weight) in mapped_concepts {
        if !store.concepts.contains_key(&concept_id) {
            continue;
        }

        let prior_entries = store.evidence.get(&concept_id).cloned().unwrap_or_default();
        let prior_evidence_count = prior_entries.len();
        let prior_self_assessment_same_day =
            count_self_assessment_same_day(&prior_entries, &event.timestamp);

        let meta = metacognitive_signal(
            event,
            objective,
            score,
            meta_strength,
            prior_evidence_count,
            prior_self_assessment_same_day,
        );

        let (warmup_factor, hard_cap) = limiter_for_event(&event.event_type, prior_evidence_count);

        let mut applied_weight = base_weight * mapping_weight * confidence * meta.effective_weight;
        applied_weight *= warmup_factor;
        applied_weight = applied_weight.min(hard_cap);

        let conservative_score = 0.5 + ((meta.effective_score - 0.5) * 0.6);
        let bounded_score = clamp_01(conservative_score);

        let delta_alpha = applied_weight * bounded_score;
        let delta_beta = applied_weight * (1.0 - bounded_score);

        let concept_state = ensure_state(store, &concept_id);
        concept_state.alpha += delta_alpha;
        concept_state.beta += delta_beta;
        concept_state.last_update = Some(event.timestamp.clone());

        let entry = store.evidence.entry(concept_id.clone()).or_default();
        entry.push(EvidenceChunk {
            event_id: event.event_id.clone(),
            delta_alpha,
            delta_beta,
            event_type: event.event_type.clone(),
            score: bounded_score,
            applied_weight,
            metacognitive_weight: meta.effective_weight,
            metacognitive_alignment: meta.alignment,
            created_at: event.timestamp.clone(),
        });

        if entry.len() > 200 {
            let overflow = entry.len() - 200;
            entry.drain(0..overflow);
        }

        updated.insert(concept_id);
    }

    Ok(updated.into_iter().collect())
}

fn next_to_study_from_store(
    store: &OlmStore,
    top: Option<usize>,
    lambda: f64,
    readiness_threshold: f64,
    exclude_set: &HashSet<String>,
    domain_filter: Option<&str>,
) -> Vec<NextToStudyItem> {
    let config = &store.config;
    let (ranked, _) = rank_next_to_study(
        store,
        top,
        lambda,
        readiness_threshold,
        config.min_readiness,
        config.gamma,
        Some(config.soft_gate_k),
        config.root_penalty,
        config.stop_mastery,
        exclude_set,
        domain_filter,
        &config.uncertainty_formula,
    );
    ranked
}

fn next_content_to_study_from_store(
    store: &OlmStore,
    top: Option<usize>,
    lambda: f64,
    readiness_threshold: f64,
    exclude_set: &HashSet<String>,
    concept_domain_filter: Option<&str>,
    domain_id: Option<&str>,
) -> Vec<NextContentToStudyItem> {
    let concept_rank = next_to_study_from_store(
        store,
        Some(100),
        lambda,
        readiness_threshold,
        exclude_set,
        concept_domain_filter,
    );

    if concept_rank.is_empty() {
        return Vec::new();
    }

    let concept_score_by_id: HashMap<String, f64> = concept_rank
        .iter()
        .map(|row| (row.concept_id.clone(), row.score.max(0.0)))
        .collect();

    let concept_name_by_id: HashMap<String, String> = concept_rank
        .iter()
        .map(|row| (row.concept_id.clone(), row.name.clone()))
        .collect();

    let mut metric_acc: HashMap<String, (usize, f64)> = HashMap::new();
    for event in &store.study_events {
        let Some(cid) = event.content_id.as_ref() else {
            continue;
        };
        let score = event_score(event);
        let entry = metric_acc.entry(cid.clone()).or_insert((0, 0.0));
        entry.0 += 1;
        entry.1 += score;
    }

    let mut raw_by_content: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    for map in &store.content_concepts {
        if let Some(target_domain_id) = domain_id {
            let Some(item) = store.content_items.get(&map.content_id) else {
                continue;
            };

            let item_domain = item.domain_id.as_deref().unwrap_or("");
            if item_domain != target_domain_id {
                continue;
            }
        }

        if !concept_score_by_id.contains_key(&map.concept_id) {
            continue;
        }

        let Some(valid_weight) = finite_weight(map.coverage_weight) else {
            continue;
        };

        raw_by_content
            .entry(map.content_id.clone())
            .or_default()
            .push((map.concept_id.clone(), valid_weight));
    }

    let mut ranked: Vec<NextContentToStudyItem> = raw_by_content
        .into_iter()
        .filter_map(|(content_id, raw_maps)| {
            let raw_weight_sum: f64 = raw_maps
                .iter()
                .filter_map(|(_, weight)| finite_weight(*weight))
                .sum();
            let mapped_concepts = raw_maps
                .iter()
                .map(|(concept_id, _)| concept_id)
                .collect::<HashSet<_>>()
                .len();
            let mapping_quality = content_mapping_quality(raw_weight_sum, mapped_concepts);

            let normalized = normalized_weights(&raw_maps);
            if normalized.is_empty() {
                return None;
            }

            let mut concept_signal = 0.0;
            let mut contributors: Vec<(String, f64)> = Vec::new();
            for (concept_id, weight) in normalized {
                let Some(concept_score) = concept_score_by_id.get(&concept_id).copied() else {
                    continue;
                };
                let contribution = concept_score * weight;
                concept_signal += contribution;
                contributors.push((concept_id, contribution));
            }

            if contributors.is_empty() {
                return None;
            }

            contributors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

            let (attempts, score_sum) = metric_acc.get(&content_id).copied().unwrap_or((0, 0.0));
            let avg_score = if attempts > 0 {
                score_sum / attempts as f64
            } else {
                0.0
            };
            let need_factor = 0.7 + (0.3 * (1.0 - clamp_01(avg_score)));
            let final_score = concept_signal * need_factor * mapping_quality.penalty;

            let supporting_concepts: Vec<String> = contributors
                .iter()
                .take(3)
                .map(|(cid, _)| cid.clone())
                .collect();

            let readable_concepts: Vec<String> = supporting_concepts
                .iter()
                .map(|cid| {
                    concept_name_by_id
                        .get(cid)
                        .cloned()
                        .unwrap_or_else(|| cid.clone())
                })
                .collect();

            let why = vec![
                format!(
                    "Conceitos prioritários relacionados: {}",
                    readable_concepts.join(", ")
                ),
                format!("Prioridade agregada por conceito: {:.3}", concept_signal),
                format!(
                    "Qualidade do mapeamento: {:.2} (peso_total={:.2}, conceitos={})",
                    mapping_quality.quality,
                    mapping_quality.raw_weight_sum,
                    mapping_quality.mapped_concepts
                ),
                format!(
                    "Penalização por mapeamento fraco: x{:.2}",
                    mapping_quality.penalty
                ),
                format!(
                    "Histórico do item: avg_score={:.2}, attempts={}",
                    avg_score, attempts
                ),
            ];

            Some(NextContentToStudyItem {
                content_id: content_id.clone(),
                title: store
                    .content_items
                    .get(&content_id)
                    .map(|item| item.title.clone()),
                score: final_score,
                attempts,
                avg_score,
                mapping_quality: mapping_quality.quality,
                mapping_penalty: mapping_quality.penalty,
                supporting_concepts,
                why,
            })
        })
        .collect();

    ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));

    if let Some(limit) = top {
        ranked.truncate(limit);
    }

    ranked
}

fn prereq_readiness(prereq_mastery: f64, prereq_uncertainty: f64, gamma: f64) -> f64 {
    clamp_01(prereq_mastery - (clamp_01(gamma) * prereq_uncertainty))
}

fn rank_next_to_study(
    store: &OlmStore,
    top: Option<usize>,
    lambda: f64,
    readiness_threshold: f64,
    min_readiness: f64,
    readiness_gamma: f64,
    soft_gate_k: Option<f64>,
    root_penalty: f64,
    stop_mastery: f64,
    exclude_set: &HashSet<String>,
    domain_filter: Option<&str>,
    uncertainty_formula: &str,
) -> (Vec<NextToStudyItem>, ScenarioDiagnostics) {
    let lam = clamp_01(lambda);
    let threshold = clamp_01(readiness_threshold);
    let min_ready = clamp_01(min_readiness);
    let gamma = clamp_01(readiness_gamma);
    let soft_k = soft_gate_k.unwrap_or(1.6).max(0.1);
    let root_score_penalty = clamp_01(root_penalty);

    let mut state_by_id: HashMap<String, (f64, f64)> = HashMap::new();
    for concept in store.concepts.values() {
        let raw_state = store
            .concept_state
            .get(&concept.id)
            .cloned()
            .unwrap_or(ConceptState {
                alpha: 1.0,
                beta: 1.0,
                last_update: None,
            });
        let concept_state = decay_state_if_needed(&raw_state, &store.config);
        state_by_id.insert(
            concept.id.clone(),
            (
                mastery(&concept_state),
                uncertainty_with_formula(&concept_state, uncertainty_formula),
            ),
        );
    }

    let mut ranked = Vec::new();
    let mut readiness_distribution = Vec::new();
    let mut candidates_before_gate = 0usize;
    let mut candidates_after_gate = 0usize;
    let mut candidates_ranked = 0usize;
    let mut candidates_excluded_min_readiness = 0usize;

    for concept in store.concepts.values() {
        // Domain filter: concept_id must start with domain_filter prefix
        if let Some(prefix) = domain_filter {
            if !concept.id.starts_with(prefix) {
                continue;
            }
        }

        // Exclude list
        if exclude_set.contains(&concept.id) {
            continue;
        }

        candidates_before_gate += 1;

        let (concept_mastery, concept_uncertainty) =
            state_by_id.get(&concept.id).copied().unwrap_or((0.5, 1.0));

        let prereqs: Vec<&ConceptEdge> = store
            .edges
            .iter()
            .filter(|edge| edge.target_id == concept.id)
            .collect();

        // Stop rule: root concepts with mastery above stop_mastery are removed
        if prereqs.is_empty() && concept_mastery > stop_mastery {
            continue;
        }

        let readiness = if prereqs.is_empty() {
            1.0
        } else {
            prereqs
                .iter()
                .map(|edge| {
                    let (m, u) = state_by_id
                        .get(&edge.prereq_id)
                        .copied()
                        .unwrap_or((0.5, 1.0));
                    prereq_readiness(m, u, gamma)
                })
                .fold(1.0, f64::min)
        };

        readiness_distribution.push(readiness);

        if readiness < min_ready {
            candidates_excluded_min_readiness += 1;
            continue;
        }

        let is_ready = readiness >= threshold;
        if is_ready {
            candidates_after_gate += 1;
        }

        let gate_factor = if readiness >= threshold {
            1.0
        } else {
            (readiness / threshold.max(0.01)).powf(soft_k)
        };

        if gate_factor <= 0.0 {
            continue;
        }

        let base_priority = (lam * (1.0 - concept_mastery)) + ((1.0 - lam) * concept_uncertainty);
        let root_multiplier = if prereqs.is_empty() {
            1.0 - root_score_penalty
        } else {
            1.0
        };
        let slight_readiness_factor = 0.9 + (0.1 * readiness);
        let score = base_priority * gate_factor * slight_readiness_factor * root_multiplier;

        let event_count = store
            .evidence
            .get(&concept.id)
            .map(|e| e.len())
            .unwrap_or(0);

        let mut why = vec![
            format!("Mastery baixo/moderado ({:.2})", concept_mastery),
            format!("Readiness pelos pré-requisitos ({:.2})", readiness),
        ];

        if !is_ready {
            why.push(format!(
                "Abaixo do limiar ({:.2}), mantido por soft-gating (k={:.2})",
                threshold, soft_k
            ));
        }

        if prereqs.is_empty() && root_score_penalty > 0.0 {
            why.push(format!(
                "Conceito raiz com penalização suave ({:.2})",
                root_score_penalty
            ));
        }

        if concept_uncertainty >= 0.3 {
            why.push(format!(
                "Incerteza elevada ({:.2}) por evidência limitada",
                concept_uncertainty
            ));
        } else {
            why.push(format!("Incerteza controlada ({:.2})", concept_uncertainty));
        }

        ranked.push(NextToStudyItem {
            concept_id: concept.id.clone(),
            name: concept.name.clone(),
            score,
            readiness,
            mastery: concept_mastery,
            uncertainty: concept_uncertainty,
            gate_factor,
            event_count,
            why,
        });
        candidates_ranked += 1;
    }

    ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));

    if let Some(limit) = top {
        ranked.truncate(limit);
    }

    let mut concept_event_counts = HashMap::new();
    for concept_id in store.concepts.keys() {
        let count = store
            .evidence
            .get(concept_id)
            .map(|events| events.len())
            .unwrap_or(0);
        concept_event_counts.insert(concept_id.clone(), count);
    }

    (
        ranked.clone(),
        ScenarioDiagnostics {
            candidates_before_gate,
            candidates_after_gate,
            candidates_ranked,
            candidates_excluded_min_readiness,
            readiness_distribution,
            concept_event_counts,
            top_candidates: ranked.into_iter().take(5).collect(),
        },
    )
}

fn scenario_ranking_metrics(
    ranked: &[NextToStudyItem],
    expected_any: &[&str],
) -> (Option<usize>, bool, bool, f64, f64) {
    let rank_of_first_expected = ranked.iter().position(|item| {
        expected_any
            .iter()
            .any(|expected| *expected == item.concept_id.as_str())
    });

    let hit_at_1 = rank_of_first_expected == Some(0);
    let hit_at_3 = rank_of_first_expected.map(|idx| idx < 3).unwrap_or(false);
    let mrr = rank_of_first_expected
        .map(|idx| 1.0 / ((idx + 1) as f64))
        .unwrap_or(0.0);

    let ndcg_at_3 = match rank_of_first_expected {
        Some(idx) if idx < 3 => {
            let rank = (idx + 1) as f64;
            1.0 / (rank + 1.0).log2()
        }
        _ => 0.0,
    };

    (
        rank_of_first_expected.map(|idx| idx + 1),
        hit_at_1,
        hit_at_3,
        mrr,
        ndcg_at_3,
    )
}

fn aggregate_subset_metrics(
    scenarios: &[SimulationScenarioResult],
    subset_ids: &[&str],
) -> (f64, f64, f64, f64) {
    let subset: Vec<&SimulationScenarioResult> = scenarios
        .iter()
        .filter(|scenario| subset_ids.contains(&scenario.scenario_id.as_str()))
        .collect();

    if subset.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }

    let size = subset.len() as f64;
    let pass_rate = subset.iter().filter(|scenario| scenario.pass).count() as f64 / size;
    let hit_at_3_rate = subset.iter().filter(|scenario| scenario.hit_at_3).count() as f64 / size;
    let avg_mrr = subset.iter().map(|scenario| scenario.mrr).sum::<f64>() / size;
    let avg_ndcg = subset
        .iter()
        .map(|scenario| scenario.ndcg_at_3)
        .sum::<f64>()
        / size;

    (pass_rate, hit_at_3_rate, avg_mrr, avg_ndcg)
}

fn upsert_local_concept(store: &mut OlmStore, id: &str, name: &str) {
    store.concepts.insert(
        id.to_string(),
        Concept {
            id: id.to_string(),
            name: name.to_string(),
            description: None,
        },
    );
    ensure_state(store, id);
}

fn add_local_edge(store: &mut OlmStore, prereq_id: &str, target_id: &str) {
    store.edges.push(ConceptEdge {
        prereq_id: prereq_id.to_string(),
        target_id: target_id.to_string(),
    });
}

fn sim_event(
    id: &str,
    step: u32,
    source: &str,
    event_type: &str,
    concept_ids: Vec<&str>,
    content_id: Option<&str>,
    payload: Value,
) -> StudyEvent {
    StudyEvent {
        event_id: format!("{}-{}", id, step),
        timestamp: format!("2026-01-01T00:00:{:02}Z", step % 60),
        source: source.to_string(),
        event_type: event_type.to_string(),
        content_id: content_id.map(|s| s.to_string()),
        concept_ids: concept_ids.into_iter().map(|s| s.to_string()).collect(),
        payload,
    }
}

fn scenario_event_mix(rng: &mut StdRng) -> HashMap<String, f64> {
    let event_types = [
        "quiz_attempt",
        "practice_attempt",
        "review",
        "study_read",
        "self_assessment",
    ];
    let mut weights = Vec::new();
    for _ in event_types {
        weights.push(rng.gen_range(0.1_f64..1.0_f64));
    }
    let total: f64 = weights.iter().sum();
    event_types
        .iter()
        .enumerate()
        .map(|(idx, t)| (t.to_string(), weights[idx] / total))
        .collect()
}

fn sample_event_type(rng: &mut StdRng, mix: &HashMap<String, f64>) -> String {
    let mut roll = rng.gen_range(0.0_f64..1.0_f64);
    for event_type in [
        "quiz_attempt",
        "practice_attempt",
        "review",
        "study_read",
        "self_assessment",
    ] {
        let weight = *mix.get(event_type).unwrap_or(&0.0);
        if roll <= weight {
            return event_type.to_string();
        }
        roll -= weight;
    }
    "practice_attempt".to_string()
}

fn baseline_scenario_blueprints(seed: u64) -> Vec<ScenarioBlueprint> {
    let concepts = vec![
        "foundation".to_string(),
        "domain_core".to_string(),
        "advanced_problem".to_string(),
    ];
    let mut list = Vec::new();
    let base_mix = HashMap::from([
        ("quiz_attempt".to_string(), 0.25),
        ("practice_attempt".to_string(), 0.35),
        ("review".to_string(), 0.15),
        ("study_read".to_string(), 0.15),
        ("self_assessment".to_string(), 0.10),
    ]);

    let make_meta = |id: &str,
                     name: &str,
                     error_rate: f64,
                     meta: f64,
                     mapping_quality: f64,
                     event_count: usize| ScenarioMetadata {
        mode: "baseline".to_string(),
        seed,
        graph_size: 3,
        prereq_density: 0.66,
        depth: 3,
        event_count,
        event_mix: base_mix.clone(),
        error_rate,
        confidence_level: 0.7,
        effort_level: 0.7,
        metacognitive_alignment: meta,
        mapping_quality,
    };

    let mut s1_events = Vec::new();
    s1_events.push(sim_event(
        "S1",
        1,
        "manual",
        "quiz_attempt",
        vec!["foundation"],
        None,
        json!({"correct":1.0,"total":1.0,"confidence":0.8,"effort":0.8,"perceived_score":0.8}),
    ));
    s1_events.push(sim_event(
        "S1",
        2,
        "manual",
        "practice_attempt",
        vec!["foundation"],
        None,
        json!({"correct":1.0,"total":1.0,"confidence":0.75,"effort":0.7}),
    ));
    s1_events.push(sim_event(
        "S1",
        3,
        "manual",
        "practice_attempt",
        vec!["domain_core"],
        None,
        json!({"correct":1.0,"total":1.0,"confidence":0.7,"effort":0.7}),
    ));
    s1_events.push(sim_event(
        "S1",
        4,
        "manual",
        "practice_attempt",
        vec!["domain_core"],
        None,
        json!({"correct":1.0,"total":1.0,"confidence":0.7,"effort":0.65}),
    ));
    s1_events.push(sim_event(
        "S1",
        5,
        "manual",
        "practice_attempt",
        vec!["advanced_problem"],
        None,
        json!({"correct":0.0,"total":1.0,"confidence":0.6,"effort":0.7,"perceived_score":0.7}),
    ));
    list.push(ScenarioBlueprint {
        id: "S1".to_string(),
        name: "Progressão linear".to_string(),
        expected_any: vec!["foundation".to_string(), "domain_core".to_string()],
        concepts: concepts.clone(),
        edges: vec![
            ("foundation".to_string(), "domain_core".to_string()),
            ("domain_core".to_string(), "advanced_problem".to_string()),
        ],
        content_items: vec![],
        content_concepts: vec![],
        events: s1_events,
        metadata: make_meta("S1", "Progressão linear", 0.2, 0.75, 1.0, 5),
    });

    let s2_events = vec![
        sim_event(
            "S2",
            1,
            "manual",
            "quiz_attempt",
            vec!["foundation"],
            None,
            json!({"correct":0.0,"total":1.0,"confidence":0.8,"perceived_score":0.9,"effort":0.8}),
        ),
        sim_event(
            "S2",
            2,
            "manual",
            "practice_attempt",
            vec!["foundation"],
            None,
            json!({"correct":0.0,"total":1.0,"confidence":0.75,"perceived_score":0.8,"effort":0.7}),
        ),
        sim_event(
            "S2",
            3,
            "manual",
            "practice_attempt",
            vec!["advanced_problem"],
            None,
            json!({"correct":1.0,"total":1.0,"confidence":0.55,"effort":0.6}),
        ),
    ];
    list.push(ScenarioBlueprint {
        id: "S2".to_string(),
        name: "Pré-requisito bloqueado".to_string(),
        expected_any: vec!["foundation".to_string()],
        concepts: concepts.clone(),
        edges: vec![
            ("foundation".to_string(), "domain_core".to_string()),
            ("domain_core".to_string(), "advanced_problem".to_string()),
        ],
        content_items: vec![],
        content_concepts: vec![],
        events: s2_events,
        metadata: make_meta("S2", "Pré-requisito bloqueado", 0.67, 0.2, 1.0, 3),
    });

    let mut s3_events = vec![sim_event(
        "S3",
        1,
        "manual",
        "practice_attempt",
        vec!["foundation"],
        None,
        json!({"correct":1.0,"total":1.0,"confidence":0.8}),
    )];
    for step in 2..=5 {
        s3_events.push(sim_event(
            "S3",
            step,
            "manual",
            "practice_attempt",
            vec!["domain_core"],
            None,
            json!({"correct":0.0,"total":1.0,"confidence":0.7,"effort":0.65}),
        ));
    }
    list.push(ScenarioBlueprint {
        id: "S3".to_string(),
        name: "Erros repetidos no core".to_string(),
        expected_any: vec!["domain_core".to_string(), "foundation".to_string()],
        concepts: concepts.clone(),
        edges: vec![
            ("foundation".to_string(), "domain_core".to_string()),
            ("domain_core".to_string(), "advanced_problem".to_string()),
        ],
        content_items: vec![],
        content_concepts: vec![],
        events: s3_events,
        metadata: make_meta("S3", "Erros repetidos no core", 0.8, 0.5, 1.0, 5),
    });

    let s4_events = vec![
        sim_event(
            "S4",
            1,
            "manual",
            "review",
            vec!["foundation"],
            None,
            json!({"confidence":0.9,"perceived_score":0.95,"effort":0.4}),
        ),
        sim_event(
            "S4",
            2,
            "manual",
            "review",
            vec!["domain_core"],
            None,
            json!({"confidence":0.88,"perceived_score":0.9,"effort":0.4}),
        ),
        sim_event(
            "S4",
            3,
            "manual",
            "study_read",
            vec!["advanced_problem"],
            None,
            json!({"confidence":0.85,"perceived_score":0.9,"effort":0.3}),
        ),
    ];
    list.push(ScenarioBlueprint {
        id: "S4".to_string(),
        name: "Incerteza alta".to_string(),
        expected_any: vec!["foundation".to_string()],
        concepts: concepts.clone(),
        edges: vec![
            ("foundation".to_string(), "domain_core".to_string()),
            ("domain_core".to_string(), "advanced_problem".to_string()),
        ],
        content_items: vec![],
        content_concepts: vec![],
        events: s4_events,
        metadata: make_meta("S4", "Incerteza alta", 0.35, 0.95, 1.0, 3),
    });

    let s5_events = vec![
        sim_event(
            "S5",
            1,
            "manual",
            "quiz_attempt",
            vec!["foundation"],
            None,
            json!({"correct":1.0,"total":1.0,"confidence":0.78,"effort":0.7}),
        ),
        sim_event(
            "S5",
            2,
            "manual",
            "practice_attempt",
            vec!["domain_core"],
            None,
            json!({"correct":1.0,"total":1.0,"confidence":0.72,"effort":0.7}),
        ),
        sim_event(
            "S5",
            3,
            "manual",
            "review",
            vec!["advanced_problem"],
            None,
            json!({"confidence":0.68,"perceived_score":0.7,"effort":0.5}),
        ),
        sim_event(
            "S5",
            4,
            "manual",
            "self_assessment",
            vec!["advanced_problem"],
            None,
            json!({"confidence":0.65,"perceived_score":0.6,"effort":0.6}),
        ),
    ];
    list.push(ScenarioBlueprint {
        id: "S5".to_string(),
        name: "Multi-fonte".to_string(),
        expected_any: vec![
            "foundation".to_string(),
            "domain_core".to_string(),
            "advanced_problem".to_string(),
        ],
        concepts: concepts.clone(),
        edges: vec![
            ("foundation".to_string(), "domain_core".to_string()),
            ("domain_core".to_string(), "advanced_problem".to_string()),
        ],
        content_items: vec![],
        content_concepts: vec![],
        events: s5_events,
        metadata: make_meta("S5", "Multi-fonte", 0.25, 0.7, 1.0, 4),
    });

    let content_items = vec![ContentItem {
        id: "lesson_1".to_string(),
        item_type: "lesson".to_string(),
        title: "Lesson 1".to_string(),
        domain_id: None,
    }];
    let content_concepts = vec![
        ContentConceptMap {
            content_id: "lesson_1".to_string(),
            concept_id: "foundation".to_string(),
            coverage_weight: 1.0,
        },
        ContentConceptMap {
            content_id: "lesson_1".to_string(),
            concept_id: "domain_core".to_string(),
            coverage_weight: 0.4,
        },
    ];
    let s6_events = vec![
        sim_event(
            "S6",
            1,
            "manual",
            "practice_attempt",
            vec![],
            Some("lesson_1"),
            json!({"correct":1.0,"total":1.0,"confidence":0.75,"effort":0.7}),
        ),
        sim_event(
            "S6",
            2,
            "manual",
            "practice_attempt",
            vec![],
            Some("lesson_1"),
            json!({"correct":1.0,"total":1.0,"confidence":0.7,"effort":0.7}),
        ),
        sim_event(
            "S6",
            3,
            "manual",
            "practice_attempt",
            vec!["domain_core"],
            None,
            json!({"correct":0.0,"total":1.0,"confidence":0.6,"effort":0.7}),
        ),
    ];
    list.push(ScenarioBlueprint {
        id: "S6".to_string(),
        name: "Cobertura parcial".to_string(),
        expected_any: vec!["foundation".to_string(), "domain_core".to_string()],
        concepts: concepts,
        edges: vec![
            ("foundation".to_string(), "domain_core".to_string()),
            ("domain_core".to_string(), "advanced_problem".to_string()),
        ],
        content_items,
        content_concepts,
        events: s6_events,
        metadata: make_meta("S6", "Cobertura parcial", 0.34, 0.7, 0.7, 3),
    });

    list
}

fn generated_scenario_blueprints(seed: u64, options: &SimulationOptions) -> Vec<ScenarioBlueprint> {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x9E3779B97F4A7C15);
    let mut blueprints = Vec::new();

    for idx in 0..options.generated_scenarios {
        let scenario_seed = seed.wrapping_add((idx as u64) * 7919);
        let mut local_rng = StdRng::seed_from_u64(scenario_seed);
        let graph_size = options.graph_size.max(3);
        let depth = options.depth.max(2).min(graph_size);
        let prereq_density = options.prereq_density.clamp(0.0, 1.0);
        let event_count = options.event_count.max(3);
        let mapping_quality = options.mapping_quality.clamp(0.1, 1.0);

        let concepts: Vec<String> = (0..graph_size)
            .map(|i| format!("c{}_{}", idx + 1, i + 1))
            .collect();
        let mut concept_depth = vec![0usize; graph_size];
        for d in concept_depth.iter_mut().skip(1) {
            *d = local_rng.gen_range(1..=depth - 1);
        }

        let mut edges = Vec::new();
        for target in 1..graph_size {
            let target_depth = concept_depth[target];
            for prereq in 0..target {
                if concept_depth[prereq] < target_depth && local_rng.gen_bool(prereq_density) {
                    edges.push((concepts[prereq].clone(), concepts[target].clone()));
                }
            }
            if !edges.iter().any(|(_, t)| t == &concepts[target]) {
                let fallback = local_rng.gen_range(0..target);
                edges.push((concepts[fallback].clone(), concepts[target].clone()));
            }
        }

        let event_mix = scenario_event_mix(&mut rng);
        let error_rate = local_rng.gen_range(0.05_f64..0.65_f64);
        let confidence_level = local_rng.gen_range(0.4_f64..0.95_f64);
        let effort_level = local_rng.gen_range(0.35_f64..0.95_f64);
        let metacognitive_alignment = local_rng.gen_range(0.2_f64..0.95_f64);

        let mut content_items = Vec::new();
        let mut content_concepts = Vec::new();
        for content_idx in 0..(graph_size / 2).max(1) {
            let content_id = format!("g{}_lesson_{}", idx + 1, content_idx + 1);
            content_items.push(ContentItem {
                id: content_id.clone(),
                item_type: "lesson".to_string(),
                title: format!("Generated Lesson {}", content_idx + 1),
                domain_id: None,
            });
            for concept_id in concepts.iter().filter(|_| local_rng.gen_bool(0.5)) {
                let weight =
                    (local_rng.gen_range(0.2_f64..1.0_f64) * mapping_quality).clamp(0.05, 1.0);
                content_concepts.push(ContentConceptMap {
                    content_id: content_id.clone(),
                    concept_id: concept_id.clone(),
                    coverage_weight: weight,
                });
            }
        }

        let mut events = Vec::new();
        for step in 0..event_count {
            let event_type = sample_event_type(&mut local_rng, &event_mix);
            let concept_index = local_rng.gen_range(0..graph_size);
            let concept_id = concepts[concept_index].clone();
            let had_error = local_rng.gen_bool(error_rate);
            let base_score = if had_error {
                local_rng.gen_range(0.0_f64..0.45_f64)
            } else {
                local_rng.gen_range(0.55_f64..1.0_f64)
            };
            let perceived = (base_score * metacognitive_alignment)
                + ((1.0 - metacognitive_alignment) * local_rng.gen_range(0.0_f64..1.0_f64));
            let confidence =
                (confidence_level + local_rng.gen_range(-0.15_f64..0.15_f64)).clamp(0.0, 1.0);
            let effort = (effort_level + local_rng.gen_range(-0.15_f64..0.15_f64)).clamp(0.0, 1.0);
            let payload = match event_type.as_str() {
                "quiz_attempt" | "practice_attempt" => {
                    let total = 1.0;
                    json!({"correct": if base_score >= 0.5 { 1.0 } else { 0.0 }, "total": total, "confidence": confidence, "effort": effort, "perceived_score": perceived})
                }
                _ => {
                    json!({"confidence": confidence, "effort": effort, "perceived_score": perceived})
                }
            };

            let use_content = local_rng.gen_bool(0.35) && !content_items.is_empty();
            let content_id = if use_content {
                Some(
                    content_items[local_rng.gen_range(0..content_items.len())]
                        .id
                        .as_str(),
                )
            } else {
                None
            };
            let concept_ids = if use_content && local_rng.gen_bool(0.4) {
                vec![]
            } else {
                vec![concept_id.as_str()]
            };
            events.push(sim_event(
                &format!("G{}", idx + 1),
                (step + 1) as u32,
                "generated",
                &event_type,
                concept_ids,
                content_id,
                payload,
            ));
        }

        let mut avg_scores: HashMap<String, (f64, usize)> = HashMap::new();
        for event in &events {
            let score = event_score(event);
            for c in event.concept_ids.iter() {
                let entry = avg_scores.entry(c.clone()).or_insert((0.0, 0));
                entry.0 += score;
                entry.1 += 1;
            }
        }
        let expected_any = avg_scores
            .iter()
            .filter_map(|(k, (sum, n))| {
                if *n > 0 && (*sum / *n as f64) < 0.55 {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .take(3)
            .collect::<Vec<_>>();
        let expected_any = if expected_any.is_empty() {
            vec![concepts[0].clone()]
        } else {
            expected_any
        };

        blueprints.push(ScenarioBlueprint {
            id: format!("G{}", idx + 1),
            name: format!("Generated scenario {}", idx + 1),
            expected_any,
            concepts,
            edges,
            content_items,
            content_concepts,
            events,
            metadata: ScenarioMetadata {
                mode: "generated".to_string(),
                seed: scenario_seed,
                graph_size,
                prereq_density,
                depth,
                event_count,
                event_mix,
                error_rate,
                confidence_level,
                effort_level,
                metacognitive_alignment,
                mapping_quality,
            },
        });
    }

    blueprints
}

fn build_simulation_scenarios(seed: u64, options: &SimulationOptions) -> Vec<ScenarioBlueprint> {
    let mut scenarios = baseline_scenario_blueprints(seed);
    scenarios.extend(generated_scenario_blueprints(seed, options));
    scenarios
}

fn materialize_scenario(
    store: &mut OlmStore,
    scenario: &ScenarioBlueprint,
    approach: &ApproachConfig,
) -> Result<(), String> {
    for concept_id in &scenario.concepts {
        upsert_local_concept(store, concept_id, concept_id);
    }
    for (prereq, target) in &scenario.edges {
        add_local_edge(store, prereq, target);
    }

    for item in &scenario.content_items {
        store.content_items.insert(item.id.clone(), item.clone());
    }
    for map in &scenario.content_concepts {
        store.content_concepts.push(map.clone());
    }

    for event in &scenario.events {
        ingest_into_store(store, event, approach.meta_strength)?;
    }

    Ok(())
}

fn approach_definitions() -> Vec<ApproachConfig> {
    vec![
        ApproachConfig {
            id: "baseline",
            name: "Baseline",
            description: "Sem uso metacognitivo (meta_strength=0)",
            lambda: 0.7,
            readiness_threshold: 0.5,
            min_readiness: 0.1,
            readiness_gamma: 0.35,
            root_penalty: 0.12,
            meta_strength: 0.0,
            soft_gate_k: Some(1.6),
            stop_mastery: 0.99,
        },
        ApproachConfig {
            id: "metacog_balanced",
            name: "Metacognitive Balanced",
            description: "Ponderação metacognitiva moderada",
            lambda: 0.7,
            readiness_threshold: 0.5,
            min_readiness: 0.1,
            readiness_gamma: 0.35,
            root_penalty: 0.12,
            meta_strength: 0.6,
            soft_gate_k: Some(1.6),
            stop_mastery: 0.99,
        },
        ApproachConfig {
            id: "metacog_strict",
            name: "Metacognitive Strict",
            description: "Ponderação metacognitiva forte + gating mais exigente",
            lambda: 0.65,
            readiness_threshold: 0.55,
            min_readiness: 0.1,
            readiness_gamma: 0.35,
            root_penalty: 0.12,
            meta_strength: 1.0,
            soft_gate_k: Some(1.8),
            stop_mastery: 0.99,
        },
    ]
}

fn average_state(store: &OlmStore) -> (f64, f64) {
    if store.concepts.is_empty() {
        return (0.0, 1.0);
    }

    let mut mastery_sum = 0.0;
    let mut uncertainty_sum = 0.0;

    for concept in store.concepts.values() {
        let concept_state = store
            .concept_state
            .get(&concept.id)
            .cloned()
            .unwrap_or(ConceptState {
                alpha: 1.0,
                beta: 1.0,
                last_update: None,
            });
        mastery_sum += mastery(&concept_state);
        uncertainty_sum += uncertainty(&concept_state);
    }

    let n = store.concepts.len() as f64;
    (mastery_sum / n, uncertainty_sum / n)
}

#[tauri::command]
pub fn olm_upsert_concept(state: State<OlmState>, concept: Concept) -> Result<Concept, String> {
    if concept.id.trim().is_empty() || concept.name.trim().is_empty() {
        return Err("Concept id and name are required".to_string());
    }

    let mut store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    store.concepts.insert(concept.id.clone(), concept.clone());
    ensure_state(&mut store, &concept.id);
    Ok(concept)
}

#[tauri::command]
pub fn olm_list_concepts(state: State<OlmState>) -> Result<Vec<Concept>, String> {
    let store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    let mut concepts: Vec<Concept> = store.concepts.values().cloned().collect();
    concepts.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(concepts)
}

#[tauri::command]
pub fn olm_add_edge(state: State<OlmState>, edge: ConceptEdge) -> Result<ConceptEdge, String> {
    if edge.prereq_id == edge.target_id {
        return Err("Prerequisite and target cannot be the same concept".to_string());
    }

    let mut store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    if !store.concepts.contains_key(&edge.prereq_id)
        || !store.concepts.contains_key(&edge.target_id)
    {
        return Err("Both concepts must exist before creating an edge".to_string());
    }

    let exists = store
        .edges
        .iter()
        .any(|current| current.prereq_id == edge.prereq_id && current.target_id == edge.target_id);

    if !exists {
        store.edges.push(edge.clone());
    }

    Ok(edge)
}

#[tauri::command]
pub fn olm_list_edges(state: State<OlmState>) -> Result<Vec<ConceptEdge>, String> {
    let store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    Ok(store.edges.clone())
}

#[tauri::command]
pub fn olm_upsert_content_item(
    state: State<OlmState>,
    item: ContentItem,
) -> Result<ContentItem, String> {
    if item.id.trim().is_empty() || item.title.trim().is_empty() || item.item_type.trim().is_empty()
    {
        return Err("Content id, type and title are required".to_string());
    }

    if let Some(domain_id) = item.domain_id.as_deref() {
        let clean = domain_id.trim();
        if clean.is_empty() {
            return Err("domain_id cannot be empty when provided".to_string());
        }

        let valid = clean
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '.');
        if !valid {
            return Err("domain_id must use lowercase letters, numbers, '-' or '.'".to_string());
        }
    }

    let mut store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    store.content_items.insert(item.id.clone(), item.clone());
    Ok(item)
}

#[tauri::command]
pub fn olm_map_content_concept(
    state: State<OlmState>,
    map: ContentConceptMap,
) -> Result<ContentConceptMap, String> {
    let mut store = state.inner.lock().map_err(|_| "State lock poisoned")?;

    if map.content_id.trim().is_empty() || map.concept_id.trim().is_empty() {
        return Err("content_id and concept_id are required".to_string());
    }

    let Some(coverage_weight) = finite_weight(map.coverage_weight) else {
        return Err("coverage_weight must be a finite number > 0".to_string());
    };

    if !store.content_items.contains_key(&map.content_id) {
        return Err("Content item must exist before mapping".to_string());
    }

    if !store.concepts.contains_key(&map.concept_id) {
        return Err("Concept must exist before mapping".to_string());
    }

    let normalized = ContentConceptMap {
        coverage_weight: clamp_01(coverage_weight),
        ..map
    };

    if let Some(existing) = store.content_concepts.iter_mut().find(|current| {
        current.content_id == normalized.content_id && current.concept_id == normalized.concept_id
    }) {
        existing.coverage_weight = normalized.coverage_weight;
        return Ok(existing.clone());
    }

    store.content_concepts.push(normalized.clone());
    Ok(normalized)
}

#[tauri::command]
pub fn olm_ingest_event(state: State<OlmState>, event: StudyEvent) -> Result<IngestResult, String> {
    let mut store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    let updated_concepts = ingest_into_store(&mut store, &event, DEFAULT_META_STRENGTH)?;

    Ok(IngestResult { updated_concepts })
}

#[tauri::command]
pub fn olm_get_state(state: State<OlmState>) -> Result<Vec<ConceptStateView>, String> {
    let store = state.inner.lock().map_err(|_| "State lock poisoned")?;

    let mut rows: Vec<ConceptStateView> = store
        .concepts
        .values()
        .map(|concept| {
            let concept_state =
                store
                    .concept_state
                    .get(&concept.id)
                    .cloned()
                    .unwrap_or(ConceptState {
                        alpha: 1.0,
                        beta: 1.0,
                        last_update: None,
                    });
            let effective_state = decay_state_if_needed(&concept_state, &store.config);

            ConceptStateView {
                concept_id: concept.id.clone(),
                name: concept.name.clone(),
                mastery: mastery(&effective_state),
                uncertainty: uncertainty_with_formula(
                    &effective_state,
                    &store.config.uncertainty_formula,
                ),
                alpha: effective_state.alpha,
                beta: effective_state.beta,
                last_update: concept_state.last_update,
            }
        })
        .collect();

    rows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(rows)
}

#[tauri::command]
pub fn olm_get_explain(
    state: State<OlmState>,
    concept_id: String,
    limit: Option<usize>,
) -> Result<Vec<EvidenceChunk>, String> {
    let store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    let mut rows = store.evidence.get(&concept_id).cloned().unwrap_or_default();

    rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    if let Some(max_rows) = limit {
        rows.truncate(max_rows);
    }

    Ok(rows)
}

#[tauri::command]
pub fn olm_next_to_study(
    state: State<OlmState>,
    top: Option<usize>,
    lambda: Option<f64>,
    readiness_threshold: Option<f64>,
    domain_filter: Option<String>,
    exclude: Option<Vec<String>>,
) -> Result<Vec<NextToStudyItem>, String> {
    let store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    let config = &store.config;
    let exclude_set = build_exclude_set(&config.exclude_concepts, exclude.as_deref());
    Ok(next_to_study_from_store(
        &store,
        top,
        lambda.unwrap_or(config.lambda),
        readiness_threshold.unwrap_or(config.theta),
        &exclude_set,
        domain_filter.as_deref(),
    ))
}

#[tauri::command]
pub fn olm_run_synthetic_scenarios() -> Result<SimulationReport, String> {
    olm_run_synthetic_scenarios_seeded_with_options(0, SimulationOptions::default())
}

pub fn olm_run_synthetic_scenarios_seeded(seed: u64) -> Result<SimulationReport, String> {
    olm_run_synthetic_scenarios_seeded_with_options(seed, SimulationOptions::default())
}

pub fn olm_run_synthetic_scenarios_seeded_with_options(
    seed: u64,
    options: SimulationOptions,
) -> Result<SimulationReport, String> {
    let scenarios = build_simulation_scenarios(seed, &options);
    let approaches = approach_definitions();
    let train_split = ["S1", "S2", "S3"];
    let test_split = ["S4", "S5", "S6"];

    // Seed now perturbs scenario states (deterministic per seed/scenario), enabling
    // reproducible variability across multi-seed simulation runs.
    let generated_at = format!("simulated-seed-{}", seed);

    let mut approach_reports = Vec::new();

    for approach in approaches.iter() {
        let mut scenario_results = Vec::new();
        let mut passed = 0usize;
        let mut hit_at_3_count = 0usize;
        let mut mrr_sum = 0.0;
        let mut ndcg_sum = 0.0;

        for scenario in scenarios.iter() {
            let mut store = OlmStore::default();
            materialize_scenario(&mut store, scenario, approach)?;
            apply_seeded_scenario_noise(&mut store, seed, &scenario.id);

            let sim_exclude: HashSet<String> = HashSet::new();
            let (ranked, diagnostics) = rank_next_to_study(
                &store,
                Some(3),
                approach.lambda,
                approach.readiness_threshold,
                approach.min_readiness,
                approach.readiness_gamma,
                approach.soft_gate_k,
                approach.root_penalty,
                approach.stop_mastery,
                &sim_exclude,
                None,
                "standard",
            );
            let top_recommendation = ranked.first().map(|item| item.concept_id.clone());
            let top_recommendations: Vec<String> =
                ranked.iter().map(|item| item.concept_id.clone()).collect();
            let (rank_of_first_expected, pass, hit_at_3, mrr, ndcg_at_3) =
                scenario_ranking_metrics(&ranked, &scenario.expected_any);

            if pass {
                passed += 1;
            }
            if hit_at_3 {
                hit_at_3_count += 1;
            }
            mrr_sum += mrr;
            ndcg_sum += ndcg_at_3;

            let (avg_mastery, avg_uncertainty) = average_state(&store);

            scenario_results.push(SimulationScenarioResult {
                scenario_id: scenario.id.to_string(),
                scenario_name: scenario.name.to_string(),
                expected_any: scenario.expected_any.iter().map(|s| s.to_string()).collect(),
                scenario_metadata: scenario.metadata.clone(),
                top_recommendation,
                top_recommendations,
                rank_of_first_expected,
                pass,
                hit_at_3,
                mrr,
                ndcg_at_3,
                avg_mastery,
                avg_uncertainty,
                diagnostics,
                notes: format!(
                    "approach={}, lambda={:.2}, threshold={:.2}, min_readiness={:.2}, readiness_gamma={:.2}, root_penalty={:.2}, meta_strength={:.2}, soft_gate_k={}",
                    approach.id,
                    approach.lambda,
                    approach.readiness_threshold,
                    approach.min_readiness,
                    approach.readiness_gamma,
                    approach.root_penalty,
                    approach.meta_strength,
                    approach
                        .soft_gate_k
                        .map(|k| format!("{:.2}", k))
                        .unwrap_or_else(|| "none".to_string())
                ),
            });
        }

        let pass_rate = if scenarios.is_empty() {
            0.0
        } else {
            passed as f64 / scenarios.len() as f64
        };
        let hit_at_3_rate = if scenarios.is_empty() {
            0.0
        } else {
            hit_at_3_count as f64 / scenarios.len() as f64
        };
        let avg_mrr = if scenarios.is_empty() {
            0.0
        } else {
            mrr_sum / scenarios.len() as f64
        };
        let avg_ndcg_at_3 = if scenarios.is_empty() {
            0.0
        } else {
            ndcg_sum / scenarios.len() as f64
        };

        let (train_pass_rate, _, train_avg_mrr, _) =
            aggregate_subset_metrics(&scenario_results, &train_split);
        let (test_pass_rate, _, test_avg_mrr, _) =
            aggregate_subset_metrics(&scenario_results, &test_split);

        approach_reports.push(SimulationApproachResult {
            approach_id: approach.id.to_string(),
            approach_name: approach.name.to_string(),
            description: approach.description.to_string(),
            pass_rate,
            hit_at_3_rate,
            avg_mrr,
            avg_ndcg_at_3,
            train_pass_rate,
            test_pass_rate,
            train_avg_mrr,
            test_avg_mrr,
            scenarios: scenario_results,
        });
    }

    let best_approach_id = approach_reports
        .iter()
        .max_by(|a, b| {
            a.pass_rate
                .partial_cmp(&b.pass_rate)
                .unwrap_or(Ordering::Equal)
        })
        .map(|report| report.approach_id.clone())
        .unwrap_or_else(|| "none".to_string());

    let selected_for_calibration = approach_reports
        .iter()
        .max_by(|a, b| {
            a.train_avg_mrr
                .partial_cmp(&b.train_avg_mrr)
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    a.hit_at_3_rate
                        .partial_cmp(&b.hit_at_3_rate)
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| {
                    a.train_pass_rate
                        .partial_cmp(&b.train_pass_rate)
                        .unwrap_or(Ordering::Equal)
                })
        })
        .cloned();

    let calibration = if let Some(selected) = selected_for_calibration {
        SimulationCalibrationSummary {
            train_scenarios: train_split.iter().map(|id| id.to_string()).collect(),
            test_scenarios: test_split.iter().map(|id| id.to_string()).collect(),
            selection_metric: "train_avg_mrr_then_hit_at_3_rate_then_train_pass_rate".to_string(),
            selected_approach_id: selected.approach_id,
            selected_test_pass_rate: selected.test_pass_rate,
            selected_test_avg_mrr: selected.test_avg_mrr,
        }
    } else {
        SimulationCalibrationSummary {
            train_scenarios: vec![],
            test_scenarios: vec![],
            selection_metric: "none".to_string(),
            selected_approach_id: "none".to_string(),
            selected_test_pass_rate: 0.0,
            selected_test_avg_mrr: 0.0,
        }
    };

    Ok(SimulationReport {
        generated_at,
        scenarios_count: scenarios.len(),
        approaches: approach_reports,
        best_approach_id,
        calibration,
    })
}

// ── New commands: config management ──────────────────────────────────────────

#[tauri::command]
pub fn olm_get_config(state: State<OlmState>) -> Result<OlmConfig, String> {
    let store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    Ok(store.config.clone())
}

#[tauri::command]
pub fn olm_set_config(state: State<OlmState>, config: OlmConfig) -> Result<OlmConfig, String> {
    let mut store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    store.config = config.clone();
    Ok(config)
}

// ── New commands: state persistence ──────────────────────────────────────────

#[tauri::command]
pub fn olm_reset_state(state: State<OlmState>) -> Result<(), String> {
    let mut store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    *store = OlmStore::default();
    Ok(())
}

#[tauri::command]
pub fn olm_export_json(state: State<OlmState>) -> Result<String, String> {
    let store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    let snapshot = OlmSnapshot::from(&*store);
    serde_json::to_string_pretty(&snapshot).map_err(|e| format!("Serialization error: {}", e))
}

#[tauri::command]
pub fn olm_import_json(state: State<OlmState>, json: String) -> Result<(), String> {
    let snapshot: OlmSnapshot =
        serde_json::from_str(&json).map_err(|e| format!("Deserialization error: {}", e))?;
    let mut store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    store.concepts = snapshot.concepts;
    store.edges = snapshot.edges;
    store.content_items = snapshot.content_items;
    store.content_concepts = snapshot.content_concepts;
    store.study_events = snapshot.study_events;
    store.concept_state = snapshot.concept_state;
    store.evidence = snapshot.evidence;
    store.config = snapshot.config;
    Ok(())
}

#[tauri::command]
pub fn olm_save_state(
    app: tauri::AppHandle,
    state: State<OlmState>,
    path: Option<String>,
) -> Result<String, String> {
    use tauri::Manager;
    let store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    let snapshot = OlmSnapshot::from(&*store);
    let json = serde_json::to_string_pretty(&snapshot)
        .map_err(|e| format!("Serialization error: {}", e))?;

    let file_path = if let Some(p) = path {
        std::path::PathBuf::from(p)
    } else {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("App data dir error: {}", e))?;
        std::fs::create_dir_all(&data_dir).map_err(|e| format!("Create dir error: {}", e))?;
        data_dir.join("olm_state.json")
    };

    let temp_path = file_path.with_extension("tmp");
    std::fs::write(&temp_path, &json).map_err(|e| format!("Write error: {}", e))?;
    std::fs::rename(&temp_path, &file_path).map_err(|e| format!("Rename error: {}", e))?;
    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn olm_load_state(
    app: tauri::AppHandle,
    state: State<OlmState>,
    path: Option<String>,
) -> Result<String, String> {
    use tauri::Manager;
    let file_path = if let Some(p) = path {
        std::path::PathBuf::from(p)
    } else {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("App data dir error: {}", e))?;
        data_dir.join("olm_state.json")
    };

    if !file_path.exists() {
        return Err(format!("State file not found: {}", file_path.display()));
    }

    let json = std::fs::read_to_string(&file_path).map_err(|e| format!("Read error: {}", e))?;
    let snapshot: OlmSnapshot =
        serde_json::from_str(&json).map_err(|e| format!("Deserialization error: {}", e))?;

    let mut store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    store.concepts = snapshot.concepts;
    store.edges = snapshot.edges;
    store.content_items = snapshot.content_items;
    store.content_concepts = snapshot.content_concepts;
    store.study_events = snapshot.study_events;
    store.concept_state = snapshot.concept_state;
    store.evidence = snapshot.evidence;
    store.config = snapshot.config;
    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn olm_get_content_metrics(
    state: State<OlmState>,
    content_id: Option<String>,
) -> Result<Vec<ContentMetric>, String> {
    let store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    let mut acc: HashMap<String, (usize, f64, f64, f64)> = HashMap::new();

    for event in &store.study_events {
        let Some(cid) = event.content_id.as_ref() else {
            continue;
        };

        if let Some(target) = content_id.as_ref() {
            if cid != target {
                continue;
            }
        }

        let score = event_score(event);
        let correct = parse_payload_float(&event.payload, "correct").unwrap_or(score);
        let total = parse_payload_float(&event.payload, "total")
            .unwrap_or(1.0)
            .max(1.0);

        let entry = acc.entry(cid.clone()).or_insert((0, 0.0, 0.0, 0.0));
        entry.0 += 1;
        entry.1 += correct;
        entry.2 += total;
        entry.3 += score;
    }

    let mut rows: Vec<ContentMetric> = acc
        .into_iter()
        .map(|(cid, (attempts, correct_sum, total_sum, score_sum))| {
            let title = store.content_items.get(&cid).map(|item| item.title.clone());
            let success_rate = if total_sum > 0.0 {
                clamp_01(correct_sum / total_sum)
            } else {
                0.0
            };
            let avg_score = if attempts > 0 {
                score_sum / attempts as f64
            } else {
                0.0
            };

            ContentMetric {
                content_id: cid,
                title,
                attempts,
                correct_sum,
                total_sum,
                success_rate,
                avg_score,
            }
        })
        .collect();

    rows.sort_by(|a, b| a.content_id.cmp(&b.content_id));
    Ok(rows)
}

#[tauri::command]
pub fn olm_next_content_to_study(
    state: State<OlmState>,
    top: Option<usize>,
    lambda: Option<f64>,
    readiness_threshold: Option<f64>,
    domain_filter: Option<String>,
    domain_id: Option<String>,
    exclude: Option<Vec<String>>,
) -> Result<Vec<NextContentToStudyItem>, String> {
    let store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    let config = &store.config;
    let exclude_set = build_exclude_set(&config.exclude_concepts, exclude.as_deref());

    Ok(next_content_to_study_from_store(
        &store,
        top,
        lambda.unwrap_or(config.lambda),
        readiness_threshold.unwrap_or(config.theta),
        &exclude_set,
        domain_filter.as_deref(),
        domain_id.as_deref(),
    ))
}

// ── New command: debug ranking ────────────────────────────────────────────────

#[tauri::command]
pub fn olm_get_debug_ranking(
    state: State<OlmState>,
    top: Option<usize>,
    domain_filter: Option<String>,
    exclude: Option<Vec<String>>,
) -> Result<RankingDebugResponse, String> {
    let store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    let config = store.config.clone();
    let exclude_set = build_exclude_set(&config.exclude_concepts, exclude.as_deref());
    let (candidates, diagnostics) = rank_next_to_study(
        &store,
        top,
        config.lambda,
        config.theta,
        config.min_readiness,
        config.gamma,
        Some(config.soft_gate_k),
        config.root_penalty,
        config.stop_mastery,
        &exclude_set,
        domain_filter.as_deref(),
        &config.uncertainty_formula,
    );
    Ok(RankingDebugResponse {
        candidates,
        diagnostics,
    })
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state_cs(alpha: f64, beta: f64) -> ConceptState {
        ConceptState {
            alpha,
            beta,
            last_update: None,
        }
    }

    fn make_event(event_type: &str, payload: Value) -> StudyEvent {
        StudyEvent {
            event_id: "test-1".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            source: "test".to_string(),
            event_type: event_type.to_string(),
            content_id: None,
            concept_ids: vec!["c1".to_string()],
            payload,
        }
    }

    #[test]
    fn test_event_type_weight_all() {
        assert_eq!(event_type_weight("quiz_attempt"), 1.0);
        assert_eq!(event_type_weight("practice_attempt"), 0.7);
        assert_eq!(event_type_weight("flashcard_review"), 0.6);
        assert_eq!(event_type_weight("study_read"), 0.2);
        assert_eq!(event_type_weight("self_assessment"), 0.3);
        assert_eq!(event_type_weight("unknown"), 0.2);
    }

    #[test]
    fn test_event_score_quiz() {
        let e = make_event("quiz_attempt", json!({"correct": 3.0, "total": 4.0}));
        assert!((event_score(&e) - 0.75).abs() < 1e-9);
    }

    #[test]
    fn test_event_score_study_read() {
        let e = make_event(
            "study_read",
            json!({"duration_sec": 150.0, "target_duration_sec": 300.0}),
        );
        // 0.5 * (150/300) = 0.25
        assert!((event_score(&e) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn test_event_score_flashcard_again() {
        let e = make_event("flashcard_review", json!({"rating": "again"}));
        assert_eq!(event_score(&e), 0.0);
    }

    #[test]
    fn test_event_score_flashcard_good() {
        let e = make_event("flashcard_review", json!({"rating": "good"}));
        assert_eq!(event_score(&e), 1.0);
    }

    #[test]
    fn test_confidence_weight_present() {
        let e = make_event("quiz_attempt", json!({"confidence": 0.8}));
        // 0.5 + 0.5 * 0.8 = 0.9
        assert!((confidence_weight(&e) - 0.9).abs() < 1e-9);
    }

    #[test]
    fn test_confidence_weight_absent() {
        let e = make_event("quiz_attempt", json!({}));
        assert_eq!(confidence_weight(&e), 1.0);
    }

    #[test]
    fn test_mastery_uncertainty_basic() {
        let s = make_state_cs(3.0, 1.0);
        assert!((mastery(&s) - 0.75).abs() < 1e-9);
        assert!((uncertainty(&s) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn test_mastery_initial() {
        let s = make_state_cs(1.0, 1.0);
        assert!((mastery(&s) - 0.5).abs() < 1e-9);
        assert!((uncertainty(&s) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_uncertainty_sqrt_formula() {
        let s = make_state_cs(3.0, 1.0); // total = 4
        let u_std = uncertainty_with_formula(&s, "standard");
        let u_sqrt = uncertainty_with_formula(&s, "sqrt");
        assert!((u_std - 0.25).abs() < 1e-9); // 1/4
        assert!((u_sqrt - 0.5).abs() < 1e-9); // 1/sqrt(4)
    }

    #[test]
    fn test_alpha_beta_update_bounds() {
        let mut store = OlmStore::default();
        store.concepts.insert(
            "c1".to_string(),
            Concept {
                id: "c1".to_string(),
                name: "C1".to_string(),
                description: None,
            },
        );
        ensure_state(&mut store, "c1");
        let event = make_event("quiz_attempt", json!({"correct": 1.0, "total": 1.0}));
        ingest_into_store(&mut store, &event, 0.0).unwrap();

        let st = store.concept_state.get("c1").unwrap();
        assert!(st.alpha > 1.0, "alpha should increase after correct answer");
        assert!(st.beta >= 1.0, "beta should not drop below initial");
        assert!(
            st.alpha <= 3.0,
            "alpha should be reasonable after one event"
        );
    }

    #[test]
    fn test_metacognitive_weight_zero_strength() {
        let e = make_event(
            "quiz_attempt",
            json!({"correct": 1.0, "total": 1.0, "confidence": 0.8}),
        );
        let meta = metacognitive_signal(&e, Some(1.0), 1.0, 0.0, 0, 0);
        assert_eq!(
            meta.effective_weight, 1.0,
            "meta_strength=0 should give weight=1.0"
        );
        assert!((meta.effective_score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_metacognitive_session_cap_reduces_reliability() {
        let e = make_event(
            "self_assessment",
            json!({"score": 1.0, "confidence": 0.9, "effort": 0.8}),
        );
        let no_cap = metacognitive_signal(&e, None, 1.0, 0.6, 5, 0);
        let with_cap = metacognitive_signal(&e, None, 1.0, 0.6, 5, 6);

        assert!(
            with_cap.reliability < no_cap.reliability,
            "session cap should reduce reliability for repeated self_assessment"
        );
    }

    #[test]
    fn test_metacognitive_low_reliability_shrinks_score_towards_neutral() {
        let e = make_event(
            "self_assessment",
            json!({"score": 1.0, "confidence": 0.05, "effort": 0.2}),
        );
        let meta = metacognitive_signal(&e, None, 1.0, 0.8, 0, 8);

        assert!(
            meta.effective_score < 0.9,
            "low reliability should shrink effective score away from extreme values"
        );
        assert!(
            meta.effective_score > 0.5,
            "effective score should stay above neutral for raw score=1"
        );
    }

    #[test]
    fn test_root_penalty_applied() {
        let mut store = OlmStore::default();
        store.concepts.insert(
            "root".to_string(),
            Concept {
                id: "root".to_string(),
                name: "Root".to_string(),
                description: None,
            },
        );

        let exclude = HashSet::new();
        // Without penalty
        let (ranked_no_penalty, _) = rank_next_to_study(
            &store,
            None,
            0.7,
            0.5,
            0.1,
            0.35,
            Some(1.6),
            0.0,
            0.99,
            &exclude,
            None,
            "standard",
        );
        // With penalty=0.5
        let (ranked_penalty, _) = rank_next_to_study(
            &store,
            None,
            0.7,
            0.5,
            0.1,
            0.35,
            Some(1.6),
            0.5,
            0.99,
            &exclude,
            None,
            "standard",
        );

        let root_no_penalty = ranked_no_penalty
            .iter()
            .find(|i| i.concept_id == "root")
            .unwrap();
        let root_penalty = ranked_penalty
            .iter()
            .find(|i| i.concept_id == "root")
            .unwrap();

        assert!(
            root_penalty.score < root_no_penalty.score,
            "root score ({:.3}) should be reduced by penalty (was {:.3})",
            root_penalty.score,
            root_no_penalty.score
        );
    }

    #[test]
    fn test_stop_mastery_excludes_root() {
        let mut store = OlmStore::default();
        store.concepts.insert(
            "root".to_string(),
            Concept {
                id: "root".to_string(),
                name: "Root".to_string(),
                description: None,
            },
        );
        // Give root high mastery: alpha=10, beta=1 → mastery = 10/11 ≈ 0.91
        store.concept_state.insert(
            "root".to_string(),
            ConceptState {
                alpha: 10.0,
                beta: 1.0,
                last_update: None,
            },
        );

        let exclude = HashSet::new();
        // stop_mastery=0.7 → root mastery (0.91) > 0.7 → excluded
        let (ranked, _) = rank_next_to_study(
            &store,
            None,
            0.7,
            0.5,
            0.1,
            0.35,
            Some(1.6),
            0.12,
            0.7,
            &exclude,
            None,
            "standard",
        );

        assert!(
            ranked.iter().all(|i| i.concept_id != "root"),
            "root should be excluded by stop_mastery"
        );
    }

    #[test]
    fn test_stop_mastery_keeps_root_below_threshold() {
        let mut store = OlmStore::default();
        store.concepts.insert(
            "root".to_string(),
            Concept {
                id: "root".to_string(),
                name: "Root".to_string(),
                description: None,
            },
        );
        // mastery = 1/2 = 0.5 < 0.7
        store.concept_state.insert(
            "root".to_string(),
            ConceptState {
                alpha: 1.0,
                beta: 1.0,
                last_update: None,
            },
        );

        let exclude = HashSet::new();
        let (ranked, _) = rank_next_to_study(
            &store,
            None,
            0.7,
            0.5,
            0.1,
            0.35,
            Some(1.6),
            0.12,
            0.7,
            &exclude,
            None,
            "standard",
        );

        assert!(
            ranked.iter().any(|i| i.concept_id == "root"),
            "root should remain when mastery < stop_mastery"
        );
    }

    #[test]
    fn test_exclude_list() {
        let mut store = OlmStore::default();
        store.concepts.insert(
            "c1".to_string(),
            Concept {
                id: "c1".to_string(),
                name: "C1".to_string(),
                description: None,
            },
        );
        store.concepts.insert(
            "c2".to_string(),
            Concept {
                id: "c2".to_string(),
                name: "C2".to_string(),
                description: None,
            },
        );

        let mut exclude = HashSet::new();
        exclude.insert("c1".to_string());

        let (ranked, _) = rank_next_to_study(
            &store,
            None,
            0.7,
            0.5,
            0.1,
            0.35,
            Some(1.6),
            0.12,
            0.99,
            &exclude,
            None,
            "standard",
        );

        assert!(
            ranked.iter().all(|i| i.concept_id != "c1"),
            "c1 should be excluded"
        );
        assert!(
            ranked.iter().any(|i| i.concept_id == "c2"),
            "c2 should be present"
        );
    }

    #[test]
    fn test_build_exclude_set() {
        let config_excludes = vec!["a".to_string(), "b".to_string()];
        let extra = vec!["c".to_string()];
        let set = build_exclude_set(&config_excludes, Some(&extra));
        assert!(set.contains("a"));
        assert!(set.contains("b"));
        assert!(set.contains("c"));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_domain_filter() {
        let mut store = OlmStore::default();
        store.concepts.insert(
            "math.algebra".to_string(),
            Concept {
                id: "math.algebra".to_string(),
                name: "Algebra".to_string(),
                description: None,
            },
        );
        store.concepts.insert(
            "physics.mechanics".to_string(),
            Concept {
                id: "physics.mechanics".to_string(),
                name: "Mechanics".to_string(),
                description: None,
            },
        );

        let exclude = HashSet::new();
        let (ranked, _) = rank_next_to_study(
            &store,
            None,
            0.7,
            0.5,
            0.1,
            0.35,
            Some(1.6),
            0.12,
            0.99,
            &exclude,
            Some("math"),
            "standard",
        );

        assert!(
            ranked.iter().all(|i| i.concept_id.starts_with("math")),
            "only math concepts should be included"
        );
        assert_eq!(ranked.len(), 1, "exactly one math concept");
    }

    #[test]
    fn test_next_content_filters_by_domain_id() {
        let mut store = OlmStore::default();

        store.concepts.insert(
            "math.core".to_string(),
            Concept {
                id: "math.core".to_string(),
                name: "Math Core".to_string(),
                description: None,
            },
        );

        store.content_items.insert(
            "math/item-1".to_string(),
            ContentItem {
                id: "math/item-1".to_string(),
                item_type: "note".to_string(),
                title: "Math Item".to_string(),
                domain_id: Some("math".to_string()),
            },
        );
        store.content_items.insert(
            "physics/item-1".to_string(),
            ContentItem {
                id: "physics/item-1".to_string(),
                item_type: "note".to_string(),
                title: "Physics Item".to_string(),
                domain_id: Some("physics".to_string()),
            },
        );

        store.content_concepts.push(ContentConceptMap {
            content_id: "math/item-1".to_string(),
            concept_id: "math.core".to_string(),
            coverage_weight: 1.0,
        });
        store.content_concepts.push(ContentConceptMap {
            content_id: "physics/item-1".to_string(),
            concept_id: "math.core".to_string(),
            coverage_weight: 1.0,
        });

        let exclude = HashSet::new();
        let rows = next_content_to_study_from_store(
            &store,
            Some(10),
            0.7,
            0.5,
            &exclude,
            Some("math"),
            Some("math"),
        );

        assert!(rows.iter().all(|row| row.content_id.starts_with("math/")));
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_readiness_soft_vs_strict_gating() {
        let mut store = OlmStore::default();
        store.concepts.insert(
            "prereq".to_string(),
            Concept {
                id: "prereq".to_string(),
                name: "Prereq".to_string(),
                description: None,
            },
        );
        store.concepts.insert(
            "target".to_string(),
            Concept {
                id: "target".to_string(),
                name: "Target".to_string(),
                description: None,
            },
        );
        store.edges.push(ConceptEdge {
            prereq_id: "prereq".to_string(),
            target_id: "target".to_string(),
        });

        let exclude = HashSet::new();
        // k=2 soft gate
        let (ranked_soft, _) = rank_next_to_study(
            &store,
            None,
            0.7,
            0.5,
            0.1,
            0.35,
            Some(2.0),
            0.0,
            0.99,
            &exclude,
            None,
            "standard",
        );
        // k=3 strict gate
        let (ranked_strict, _) = rank_next_to_study(
            &store,
            None,
            0.7,
            0.5,
            0.1,
            0.35,
            Some(3.0),
            0.0,
            0.99,
            &exclude,
            None,
            "standard",
        );

        let target_soft = ranked_soft.iter().find(|i| i.concept_id == "target");
        let target_strict = ranked_strict.iter().find(|i| i.concept_id == "target");

        assert!(target_soft.is_some(), "target should appear in soft gating");
        assert!(
            target_strict.is_some(),
            "target should appear in strict gating"
        );
        assert!(
            target_soft.unwrap().score >= target_strict.unwrap().score,
            "soft gating (k=2) should give higher score than strict (k=3) for unready concept"
        );
    }

    #[test]
    fn test_persistence_snapshot_roundtrip() {
        let mut store = OlmStore::default();
        store.concepts.insert(
            "c1".to_string(),
            Concept {
                id: "c1".to_string(),
                name: "C1".to_string(),
                description: None,
            },
        );
        store.concept_state.insert(
            "c1".to_string(),
            ConceptState {
                alpha: 3.5,
                beta: 1.2,
                last_update: Some("2026-01-01T12:00:00Z".to_string()),
            },
        );
        store.config.lambda = 0.65;
        store.config.stop_mastery = 0.80;

        let snapshot = OlmSnapshot::from(&store);
        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: OlmSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.concepts.len(), 1);
        let restored_state = restored.concept_state.get("c1").unwrap();
        assert!((restored_state.alpha - 3.5).abs() < 1e-9);
        assert!((restored_state.beta - 1.2).abs() < 1e-9);
        assert!((restored.config.lambda - 0.65).abs() < 1e-9);
        assert!((restored.config.stop_mastery - 0.80).abs() < 1e-9);
    }

    #[test]
    fn test_olm_config_default() {
        let cfg = OlmConfig::default();
        assert!((cfg.lambda - 0.7).abs() < 1e-9);
        assert!((cfg.gamma - 0.35).abs() < 1e-9);
        assert!((cfg.stop_mastery - 0.85).abs() < 1e-9);
        assert!(!cfg.decay_enabled);
        assert_eq!(cfg.uncertainty_formula, "standard");
        assert!(cfg.exclude_concepts.is_empty());
    }

    #[test]
    fn test_flow_open_file_emits_review_signal() {
        let mut store = OlmStore::default();
        store.concepts.insert(
            "math.algebra.core".to_string(),
            Concept {
                id: "math.algebra.core".to_string(),
                name: "Math Algebra Core".to_string(),
                description: None,
            },
        );

        let event = StudyEvent {
            event_id: "evt-open-1".to_string(),
            timestamp: "2026-02-01T10:00:00Z".to_string(),
            source: "editor".to_string(),
            event_type: "review".to_string(),
            content_id: Some("Matematica/algebra.md".to_string()),
            concept_ids: vec!["math.algebra.core".to_string()],
            payload: json!({
                "duration_sec": 60.0,
                "target_duration_sec": 120.0,
                "confidence": 0.6,
                "file_path": "Matematica/algebra.md"
            }),
        };

        ingest_into_store(&mut store, &event, 0.6).expect("review event should ingest");

        assert_eq!(store.study_events.len(), 1);
        let evidence = store
            .evidence
            .get("math.algebra.core")
            .expect("evidence should be present for mapped concept");
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].event_type, "review");
    }

    #[test]
    fn test_flow_quiz_save_triggers_test_end_self_assessment() {
        let mut store = OlmStore::default();
        store.concepts.insert(
            "math.meta.reflection".to_string(),
            Concept {
                id: "math.meta.reflection".to_string(),
                name: "Math Meta Reflection".to_string(),
                description: None,
            },
        );

        let event = StudyEvent {
            event_id: "evt-test-end-1".to_string(),
            timestamp: "2026-02-01T10:05:00Z".to_string(),
            source: "meta_prompt".to_string(),
            event_type: "self_assessment".to_string(),
            content_id: Some("Matematica/quiz-2026-02-01-algebra.md".to_string()),
            concept_ids: vec!["math.meta.reflection".to_string()],
            payload: json!({
                "score": 0.66,
                "confidence": 0.75,
                "meta_reflection_rating": 3,
                "meta_trigger": "test_end"
            }),
        };

        ingest_into_store(&mut store, &event, 0.6).expect("test_end self_assessment should ingest");

        assert_eq!(store.study_events.len(), 1);
        assert_eq!(
            store.study_events[0]
                .payload
                .get("meta_trigger")
                .and_then(Value::as_str),
            Some("test_end")
        );

        let evidence = store
            .evidence
            .get("math.meta.reflection")
            .expect("meta reflection concept should receive evidence");
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].event_type, "self_assessment");
    }

    #[test]
    fn test_flow_domain_switch_triggers_self_assessment() {
        let mut store = OlmStore::default();
        store.concepts.insert(
            "physics.meta.reflection".to_string(),
            Concept {
                id: "physics.meta.reflection".to_string(),
                name: "Physics Meta Reflection".to_string(),
                description: None,
            },
        );

        let event = StudyEvent {
            event_id: "evt-domain-switch-1".to_string(),
            timestamp: "2026-02-01T11:00:00Z".to_string(),
            source: "meta_prompt".to_string(),
            event_type: "self_assessment".to_string(),
            content_id: Some("_meta/physics/reflection".to_string()),
            concept_ids: vec!["physics.meta.reflection".to_string()],
            payload: json!({
                "score": 0.33,
                "confidence": 0.5,
                "meta_reflection_rating": 2,
                "meta_trigger": "domain_switch"
            }),
        };

        ingest_into_store(&mut store, &event, 0.6)
            .expect("domain_switch self_assessment should ingest");

        assert_eq!(store.study_events.len(), 1);
        assert_eq!(
            store.study_events[0]
                .payload
                .get("meta_trigger")
                .and_then(Value::as_str),
            Some("domain_switch")
        );
        assert_eq!(
            store.study_events[0].content_id.as_deref(),
            Some("_meta/physics/reflection")
        );
    }

    #[test]
    fn test_next_content_domain_id_filter_without_concept_prefix_dependency() {
        let mut store = OlmStore::default();

        store.concepts.insert(
            "cross.shared".to_string(),
            Concept {
                id: "cross.shared".to_string(),
                name: "Shared Concept".to_string(),
                description: None,
            },
        );

        store.content_items.insert(
            "math/item-a".to_string(),
            ContentItem {
                id: "math/item-a".to_string(),
                item_type: "note".to_string(),
                title: "Math Item A".to_string(),
                domain_id: Some("math".to_string()),
            },
        );
        store.content_items.insert(
            "physics/item-b".to_string(),
            ContentItem {
                id: "physics/item-b".to_string(),
                item_type: "note".to_string(),
                title: "Physics Item B".to_string(),
                domain_id: Some("physics".to_string()),
            },
        );

        store.content_concepts.push(ContentConceptMap {
            content_id: "math/item-a".to_string(),
            concept_id: "cross.shared".to_string(),
            coverage_weight: 1.0,
        });
        store.content_concepts.push(ContentConceptMap {
            content_id: "physics/item-b".to_string(),
            concept_id: "cross.shared".to_string(),
            coverage_weight: 1.0,
        });

        let exclude = HashSet::new();
        let rows = next_content_to_study_from_store(
            &store,
            Some(10),
            0.7,
            0.5,
            &exclude,
            None,
            Some("math"),
        );

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].content_id, "math/item-a");
    }
}
