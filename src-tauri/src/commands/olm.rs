#![allow(dead_code)]

use chrono::{DateTime, Utc};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Mutex;
use tauri::State;

const DEFAULT_META_STRENGTH: f64 = 0.6;

fn default_readiness_blend_eta() -> f64 {
    0.7
}

fn default_readiness_distance_delta() -> f64 {
    0.85
}

fn default_ranking_policy() -> String {
    "adaptive".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OlmConfig {
    // Legacy shared lambda kept for backward compatibility with persisted snapshots.
    pub lambda: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ranking_lambda: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence_mismatch_lambda: Option<f64>,
    pub gamma: f64,
    #[serde(default = "default_readiness_blend_eta")]
    pub readiness_blend_eta: f64,
    #[serde(default = "default_readiness_distance_delta")]
    pub readiness_distance_delta: f64,
    #[serde(default = "default_ranking_policy")]
    pub ranking_policy: String,
    pub theta: f64,
    pub min_readiness: f64,
    pub soft_gate_k: f64,
    pub meta_strength: f64,
    pub root_penalty: f64,
    pub stop_mastery: f64,
    pub exclude_concepts: Vec<String>,
    pub decay_enabled: bool,
    pub decay_half_life_days: f64,
    /// Simulated reference time for decay during synthetic scenario evaluation.
    /// When `Some`, overrides `Utc::now()` in `decay_state_if_needed`.
    /// Never serialised — runtime-only field used by the simulation pipeline.
    #[serde(skip)]
    pub simulated_now: Option<DateTime<Utc>>,
    /// Seed used when ranking_policy == "random" to produce a reproducible shuffle
    /// per (scenario, seed) pair instead of a hash-based deterministic order.
    #[serde(skip)]
    pub ranking_seed: Option<u64>,
}

impl Default for OlmConfig {
    fn default() -> Self {
        OlmConfig {
            lambda: 0.7,
            ranking_lambda: None,
            confidence_mismatch_lambda: None,
            gamma: 0.35,
            readiness_blend_eta: default_readiness_blend_eta(),
            readiness_distance_delta: default_readiness_distance_delta(),
            ranking_policy: default_ranking_policy(),
            theta: 0.5,
            min_readiness: 0.1,
            soft_gate_k: 1.6,
            meta_strength: DEFAULT_META_STRENGTH,
            root_penalty: 0.12,
            stop_mastery: 0.85,
            exclude_concepts: vec![],
            decay_enabled: false,
            decay_half_life_days: 30.0,
            simulated_now: None,
            ranking_seed: None,
        }
    }
}

impl OlmConfig {
    fn ranking_lambda_resolved(&self) -> f64 {
        clamp_01(self.ranking_lambda.unwrap_or(self.lambda))
    }

    fn confidence_mismatch_lambda_resolved(&self) -> f64 {
        clamp_01(self.confidence_mismatch_lambda.unwrap_or(self.lambda))
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
pub struct ScoreDecomposition {
    pub readiness: f64,
    pub gap: f64,
    pub uncertainty: f64,
    pub decay_signal: f64,
    pub inconsistency: f64,
    pub gate_factor: f64,
    pub root_multiplier: f64,
    pub policy_used: String,
    /// Top-3 weakest prereqs: (concept_name, effective_readiness)
    pub weakest_prereqs: Vec<(String, f64)>,
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
    pub score_decomposition: ScoreDecomposition,
}

#[derive(Debug, Clone, Serialize)]
pub struct IngestResult {
    pub updated_concepts: Vec<String>,
    pub duplicate: bool,
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
    readiness_distance_delta: f64,
    ranking_policy: &'static str,
    root_penalty: f64,
    meta_strength: f64,
    soft_gate_k: Option<f64>,
    stop_mastery: f64,
    decay_enabled: bool,
    decay_half_life_days: f64,
    apply_mapping_penalty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationOptions {
    pub generated_scenarios: usize,
    pub graph_size: usize,
    pub event_count: usize,
    pub prereq_density: f64,
    pub depth: usize,
    pub mapping_quality: f64,
    pub scenario_mode: String,
    /// Custom approach configs run in the same scenarios as the built-in three.
    #[serde(default)]
    pub custom_approaches: Vec<CustomApproachConfig>,
    /// When true, skip built-in approaches and run only custom_approaches.
    #[serde(default)]
    pub custom_approaches_only: bool,
    /// Max items per scenario full_ranking.  0 = include all ranked concepts.
    #[serde(default)]
    pub top_n: usize,
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
            scenario_mode: "mixed".to_string(),
            custom_approaches: vec![],
            custom_approaches_only: false,
            top_n: 0,
        }
    }
}

/// Approach config that can be supplied at runtime (CLI / JSON).
/// All numeric fields are optional; unset fields fall back to the built-in
/// `base` config (default: "baseline").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomApproachConfig {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// Built-in base to inherit defaults from: "baseline" | "metacog_balanced" | "metacog_strict".
    pub base: Option<String>,
    pub lambda: Option<f64>,
    pub readiness_threshold: Option<f64>,
    pub min_readiness: Option<f64>,
    pub readiness_gamma: Option<f64>,
    pub readiness_distance_delta: Option<f64>,
    pub ranking_policy: Option<String>,
    pub root_penalty: Option<f64>,
    pub meta_strength: Option<f64>,
    pub soft_gate_k: Option<f64>,
    pub stop_mastery: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioMetadata {
    pub mode: String,
    pub difficulty: String,
    pub case_tags: Vec<String>,
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
    true_mastery: HashMap<String, f64>,
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
    pub true_weak_concepts: Vec<String>,
    pub scenario_metadata: ScenarioMetadata,
    pub top_recommendation: Option<String>,
    pub top_recommendations: Vec<String>,
    pub rank_of_first_expected: Option<usize>,
    pub pass: bool,
    pub hit_at_3: bool,
    pub candidate_count: usize,
    pub top_tie: bool,
    pub mrr: f64,
    pub ndcg_at_3: f64,
    pub avg_mastery: f64,
    pub avg_uncertainty: f64,
    pub diagnostics: ScenarioDiagnostics,
    pub average_rank_of_expected: f64,
    pub learning_gain: f64,
    pub post_test_score: f64,
    pub mastery_gain: f64,
    pub time_to_mastery: Option<usize>,
    pub number_of_bad_recommendations: usize,
    pub prerequisite_violation_rate: f64,
    pub notes: String,
    /// Complete ranked list with full score decompositions for every concept.
    pub full_ranking: Vec<NextToStudyItem>,
    /// State of every concept after all events were ingested.
    pub concept_states: Vec<ConceptStateView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulationApproachResult {
    pub approach_id: String,
    pub approach_name: String,
    pub description: String,
    pub hit_at_1_rate: f64,
    pub pass_rate: f64,
    pub hit_at_3_rate: f64,
    pub avg_mrr: f64,
    pub avg_ndcg_at_3: f64,
    pub average_rank_of_expected: f64,
    pub train_pass_rate: f64,
    pub test_pass_rate: f64,
    pub train_avg_mrr: f64,
    pub test_avg_mrr: f64,
    pub random_baseline_delta: f64,
    pub mastery_baseline_delta: f64,
    pub uncertainty_baseline_delta: f64,
    pub metacognition_rank_shift: f64,
    pub avg_learning_gain: f64,
    pub avg_post_test_score: f64,
    pub avg_mastery_gain: f64,
    pub avg_time_to_mastery: Option<f64>,
    pub avg_bad_recommendations: f64,
    pub prerequisite_violation_rate: f64,
    pub candidate_count_avg: f64,
    pub tie_rate: f64,
    pub easy_case_pass_rate: f64,
    pub hard_case_pass_rate: f64,
    pub decision_divergence_rate: f64,
    pub ranking_delta_vs_baseline: f64,
    pub meta_influence_rate: f64,
    pub scenarios: Vec<SimulationScenarioResult>,
}

#[derive(Debug, Clone)]
struct LongitudinalMetrics {
    learning_gain: f64,
    post_test_score: f64,
    mastery_gain: f64,
    time_to_mastery: Option<usize>,
    number_of_bad_recommendations: usize,
    prerequisite_violation_rate: f64,
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

/// Metrics for a single concept under one approach in a scenario.
#[derive(Debug, Clone, Serialize)]
pub struct ConceptApproachSlice {
    pub approach_id: String,
    /// 1-based rank in the full ranking; None if excluded before ranking.
    pub rank: Option<usize>,
    pub score: f64,
    pub mastery: f64,
    pub uncertainty: f64,
    pub readiness: f64,
    pub gate_factor: f64,
    pub gap: f64,
    pub decay_signal: f64,
    pub inconsistency: f64,
    pub policy_used: String,
}

/// Cross-approach view for one concept inside a scenario.
#[derive(Debug, Clone, Serialize)]
pub struct ConceptCrossApproachRow {
    pub concept_id: String,
    pub name: String,
    pub slices: Vec<ConceptApproachSlice>,
}

/// Full cross-approach comparison for a single scenario.
#[derive(Debug, Clone, Serialize)]
pub struct ScenarioApproachComparison {
    pub scenario_id: String,
    pub scenario_name: String,
    /// One row per concept, sorted by concept_id.
    pub concepts: Vec<ConceptCrossApproachRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulationCalibrationSummary {
    pub train_scenarios: Vec<String>,
    pub test_scenarios: Vec<String>,
    pub selection_metric: String,
    pub selected_approach_id: String,
    pub selected_test_pass_rate: f64,
    pub selected_test_avg_mrr: f64,
    pub status: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionDivergenceSummary {
    pub reference_approach_id: String,
    pub compared_approach_id: String,
    pub changed_decisions: usize,
    pub scenarios_compared: usize,
    pub rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulationReport {
    pub generated_at: String,
    pub scenarios_count: usize,
    pub approaches: Vec<SimulationApproachResult>,
    pub best_approach_id: String,
    pub calibration: SimulationCalibrationSummary,
    /// Per-scenario cross-approach comparison table.
    pub scenario_comparisons: Vec<ScenarioApproachComparison>,
    pub decision_divergence: Vec<DecisionDivergenceSummary>,
    pub evaluation_warnings: Vec<String>,
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
        return 1.0;
    }

    let variance = (state.alpha * state.beta) / ((total * total) * (total + 1.0));
    clamp_01(variance / (1.0 / 12.0))
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

    // Use simulated_now when running synthetic scenarios to avoid applying months of
    // real-wall-clock decay to events with fixed historical timestamps.
    let now = config.simulated_now.unwrap_or_else(Utc::now);
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
            let progress = if target <= 0.0 {
                0.0
            } else {
                clamp_01(duration / target)
            };
            0.5 + (0.10 * progress)
        }
        "review" => {
            let duration = parse_payload_float(&event.payload, "duration_sec").unwrap_or(0.0);
            let target =
                parse_payload_float(&event.payload, "target_duration_sec").unwrap_or(600.0);
            let progress = if target <= 0.0 {
                0.0
            } else {
                clamp_01(duration / target)
            };
            0.5 + (0.15 * progress)
        }
        "note_taking" => {
            let chars_written = parse_payload_float(&event.payload, "chars_written").unwrap_or(0.0);
            let target_chars = parse_payload_float(&event.payload, "target_chars").unwrap_or(300.0);
            let progress = if target_chars <= 0.0 {
                0.0
            } else {
                clamp_01(chars_written / target_chars)
            };
            0.5 + (0.15 * progress)
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

fn confidence_weight(event: &StudyEvent, objective: Option<f64>, lambda: f64) -> f64 {
    let Some(confidence) = parse_payload_float(&event.payload, "confidence").map(clamp_01) else {
        return 1.0;
    };

    let Some(objective_score) = objective else {
        return 1.0;
    };

    clamp_01(1.0 - (clamp_01(lambda) * (confidence - objective_score).abs()))
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

fn safety_gate_weight(raw_weight: f64, warmup_factor: f64, hard_cap: f64) -> f64 {
    if raw_weight <= 0.0 {
        return 0.0;
    }

    // Equivalent to: min(raw_weight * warmup_factor, hard_cap)
    // expressed as a multiplicative safety gate.
    let cap_factor = hard_cap / raw_weight;
    warmup_factor.min(cap_factor).clamp(0.0, 1.0)
}

fn safety_shrink_score(effective_score: f64) -> f64 {
    // Keep conservative pull towards neutral to reduce overreaction to noisy events.
    0.5 + ((effective_score - 0.5) * 0.6)
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
) -> Result<IngestResult, String> {
    if event.event_id.trim().is_empty() {
        return Err("event_id is required".to_string());
    }

    if store
        .study_events
        .iter()
        .any(|e| e.event_id == event.event_id)
    {
        return Ok(IngestResult {
            updated_concepts: Vec::new(),
            duplicate: true,
        });
    }

    let score = event_score(event);
    let objective = objective_score(event);
    let base_weight = event_type_weight(&event.event_type);
    let confidence = confidence_weight(
        event,
        objective,
        store.config.confidence_mismatch_lambda_resolved(),
    );
    let mapped_concepts = normalize_maps(store, event);

    if mapped_concepts.is_empty() {
        return Err(
            "Event must include concept_ids or a content_id mapped to concepts".to_string(),
        );
    }

    let valid_mapped_concepts: Vec<(String, f64)> = mapped_concepts
        .into_iter()
        .filter(|(concept_id, _)| store.concepts.contains_key(concept_id))
        .collect();

    if valid_mapped_concepts.is_empty() {
        return Err("Event does not map to any existing concept".to_string());
    }

    let valid_mapped_concepts = normalized_weights(&valid_mapped_concepts);

    if valid_mapped_concepts.is_empty() {
        return Err("Event does not map to any existing concept".to_string());
    }

    // Fix 4: When mapping resolved via content_id (Tier 2, no explicit concept_ids),
    // apply the mapping quality penalty to w_evidence to reflect weak coverage.
    let mapping_quality_penalty: f64 = if event.concept_ids.is_empty() && event.content_id.is_some()
    {
        let raw_weight_sum: f64 = valid_mapped_concepts.iter().map(|(_, w)| w).sum();
        content_mapping_quality(raw_weight_sum, valid_mapped_concepts.len()).penalty
    } else {
        1.0
    };

    store.study_events.push(event.clone());
    if store.study_events.len() > 5000 {
        let overflow = store.study_events.len() - 5000;
        store.study_events.drain(0..overflow);
    }

    let mut updated = HashSet::new();
    let decay_config = store.config.clone();
    let update_now = if DateTime::parse_from_rfc3339(&event.timestamp).is_ok() {
        event.timestamp.clone()
    } else {
        Utc::now().to_rfc3339()
    };

    for (concept_id, mapping_weight) in valid_mapped_concepts {
        let prior_entries = store.evidence.get(&concept_id).cloned().unwrap_or_default();
        let prior_evidence_count = prior_entries.len();
        let prior_self_assessment_same_day =
            count_self_assessment_same_day(&prior_entries, &update_now);

        let meta = metacognitive_signal(
            event,
            objective,
            score,
            meta_strength,
            prior_evidence_count,
            prior_self_assessment_same_day,
        );

        let (warmup_factor, hard_cap) = limiter_for_event(&event.event_type, prior_evidence_count);

        // Grouped decomposition (same final result as before):
        // w_evidence: intrinsic event strength + concept mapping strength
        // r_reliability: trust/reliability layer (confidence + metacognitive trust)
        // g_safety: warmup and hard-cap safety controls
        // mapping_quality_penalty reduces w_evidence when mapping comes from content_id (Tier 2)
        let w_evidence = base_weight * mapping_weight * mapping_quality_penalty;
        let r_reliability = confidence * meta.effective_weight;
        let raw_weight = w_evidence * r_reliability;
        let g_safety = safety_gate_weight(raw_weight, warmup_factor, hard_cap);
        let applied_weight = raw_weight * g_safety;

        let conservative_score = safety_shrink_score(meta.effective_score);
        let bounded_score = clamp_01(conservative_score);

        let delta_alpha = applied_weight * bounded_score;
        let delta_beta = applied_weight * (1.0 - bounded_score);

        let concept_state = ensure_state(store, &concept_id);
        // State-based decay: normalize stored state to "now" before applying deltas.
        let decayed_state = decay_state_if_needed(concept_state, &decay_config);
        concept_state.alpha = decayed_state.alpha;
        concept_state.beta = decayed_state.beta;
        concept_state.alpha += delta_alpha;
        concept_state.beta += delta_beta;
        concept_state.last_update = Some(update_now.clone());

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
            created_at: update_now.clone(),
        });

        if entry.len() > 200 {
            let overflow = entry.len() - 200;
            entry.drain(0..overflow);
        }

        updated.insert(concept_id);
    }

    Ok(IngestResult {
        updated_concepts: updated.into_iter().collect(),
        duplicate: false,
    })
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
        config.readiness_distance_delta,
        &config.ranking_policy,
        Some(config.soft_gate_k),
        config.root_penalty,
        config.stop_mastery,
        exclude_set,
        domain_filter,
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

    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.content_id.cmp(&b.content_id))
    });

    if let Some(limit) = top {
        ranked.truncate(limit);
    }

    ranked
}

fn prereq_readiness(prereq_mastery: f64, prereq_uncertainty: f64, gamma: f64) -> f64 {
    clamp_01(prereq_mastery - (clamp_01(gamma) * prereq_uncertainty))
}

fn ancestor_distances(
    concept_id: &str,
    prereq_graph: &HashMap<String, Vec<String>>,
) -> HashMap<String, usize> {
    let mut distances: HashMap<String, usize> = HashMap::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();

    if let Some(parents) = prereq_graph.get(concept_id) {
        for prereq_id in parents {
            queue.push_back((prereq_id.clone(), 1));
        }
    }

    while let Some((current, distance)) = queue.pop_front() {
        if let Some(previous_best) = distances.get(&current) {
            if distance >= *previous_best {
                continue;
            }
        }

        distances.insert(current.clone(), distance);

        if let Some(next_parents) = prereq_graph.get(&current) {
            for parent in next_parents {
                queue.push_back((parent.clone(), distance + 1));
            }
        }
    }

    distances
}

fn concept_decay_signal(raw_mastery: f64, effective_mastery: f64) -> f64 {
    clamp_01((raw_mastery - effective_mastery).max(0.0))
}

fn concept_inconsistency_signal(evidence: Option<&Vec<EvidenceChunk>>) -> f64 {
    let Some(chunks) = evidence else {
        return 0.0;
    };

    if chunks.len() < 2 {
        return 0.0;
    }

    let recent: Vec<f64> = chunks
        .iter()
        .rev()
        .take(8)
        .map(|chunk| clamp_01(chunk.score))
        .collect();

    if recent.len() < 2 {
        return 0.0;
    }

    let mean = recent.iter().sum::<f64>() / (recent.len() as f64);
    let variance = recent
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / (recent.len() as f64);

    clamp_01(variance.sqrt() * 2.0)
}

fn ranking_policy_normalized(policy: &str) -> &str {
    match policy {
        "learn_next" | "review_next" | "balanced" | "random" | "mastery_only"
        | "uncertainty_only" | "curriculum_linear" => policy,
        _ => "adaptive",
    }
}

fn rank_next_to_study(
    store: &OlmStore,
    top: Option<usize>,
    lambda: f64,
    readiness_threshold: f64,
    min_readiness: f64,
    readiness_gamma: f64,
    readiness_distance_delta: f64,
    ranking_policy: &str,
    soft_gate_k: Option<f64>,
    root_penalty: f64,
    stop_mastery: f64,
    exclude_set: &HashSet<String>,
    domain_filter: Option<&str>,
) -> (Vec<NextToStudyItem>, ScenarioDiagnostics) {
    let lam = clamp_01(lambda);
    let threshold = clamp_01(readiness_threshold);
    let min_ready = clamp_01(min_readiness);
    let gamma = clamp_01(readiness_gamma);
    let delta = clamp_01(readiness_distance_delta).max(0.01);
    let policy = ranking_policy_normalized(ranking_policy);
    let soft_k = soft_gate_k.unwrap_or(1.6).max(0.0);
    let root_score_penalty = clamp_01(root_penalty);

    let mut curriculum_order: Vec<String> = store.concepts.keys().cloned().collect();
    curriculum_order.sort();
    let curriculum_index: HashMap<String, usize> = curriculum_order
        .iter()
        .enumerate()
        .map(|(idx, concept_id)| (concept_id.clone(), idx))
        .collect();
    let curriculum_len = curriculum_order.len().max(1) as f64;

    // Pre-compute seeded-random scores when policy is "random".
    // Uses a seeded shuffle of concept positions so the ordering is a true
    // random permutation (reproducible per scenario+seed pair) rather than a
    // deterministic hash that never varies across runs.
    let random_scores: HashMap<String, f64> = if policy == "random" {
        let rng_seed = store.config.ranking_seed.unwrap_or(12345);
        let mut rng = StdRng::seed_from_u64(rng_seed);
        let n = curriculum_order.len().max(1);
        let mut indices: Vec<usize> = (0..n).collect();
        indices.shuffle(&mut rng);
        curriculum_order
            .iter()
            .zip(indices.iter())
            .map(|(concept_id, &pos)| {
                let score = 1.0 - (pos as f64 / n as f64);
                (concept_id.clone(), score)
            })
            .collect()
    } else {
        HashMap::new()
    };

    let mut state_by_id: HashMap<String, (f64, f64)> = HashMap::new();
    let mut raw_mastery_by_id: HashMap<String, f64> = HashMap::new();
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
        raw_mastery_by_id.insert(concept.id.clone(), mastery(&raw_state));
        let concept_state = decay_state_if_needed(&raw_state, &store.config);
        state_by_id.insert(
            concept.id.clone(),
            (mastery(&concept_state), uncertainty(&concept_state)),
        );
    }

    let mut prereq_graph: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &store.edges {
        prereq_graph
            .entry(edge.target_id.clone())
            .or_default()
            .push(edge.prereq_id.clone());
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

        let direct_prereqs = prereq_graph
            .get(&concept.id)
            .map(|parents| parents.len())
            .unwrap_or(0);

        // Stop rule: root concepts with mastery above stop_mastery are removed
        if direct_prereqs == 0 && concept_mastery > stop_mastery {
            continue;
        }

        let ancestor_dist = ancestor_distances(&concept.id, &prereq_graph);

        let mut prereq_details: Vec<(String, f64)> = ancestor_dist
            .iter()
            .map(|(ancestor_id, distance)| {
                let (m, u) = state_by_id.get(ancestor_id).copied().unwrap_or((0.5, 1.0));
                let rp = prereq_readiness(m, u, gamma);
                let attenuation = delta.powi(*distance as i32);
                let effective_r = clamp_01(attenuation * rp);
                let name = store
                    .concepts
                    .get(ancestor_id)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| ancestor_id.clone());
                (name, effective_r)
            })
            .collect();

        let readiness = if prereq_details.is_empty() {
            1.0
        } else {
            let eta = clamp_01(store.config.readiness_blend_eta);
            let readiness_min = prereq_details.iter().map(|(_, r)| *r).fold(1.0, f64::min);
            let readiness_mean =
                prereq_details.iter().map(|(_, r)| *r).sum::<f64>() / (prereq_details.len() as f64);
            clamp_01((eta * readiness_min) + ((1.0 - eta) * readiness_mean))
        };

        // Keep the 3 weakest prereqs for explanation
        prereq_details.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        prereq_details.truncate(3);

        readiness_distribution.push(readiness);

        if readiness < min_ready {
            candidates_excluded_min_readiness += 1;
            continue;
        }

        let policy_ignores_gate = matches!(
            policy,
            "mastery_only" | "uncertainty_only" | "curriculum_linear" | "random"
        );

        let is_ready = policy_ignores_gate || readiness >= threshold;
        if is_ready {
            candidates_after_gate += 1;
        }

        let gate_factor = if policy_ignores_gate {
            1.0
        } else {
            readiness.powf(soft_k)
        };

        if gate_factor <= 0.0 {
            continue;
        }

        let root_multiplier = if direct_prereqs == 0 {
            1.0 - root_score_penalty
        } else {
            1.0
        };

        let learn_score = gate_factor * (1.0 - concept_mastery) * root_multiplier;

        let raw_mastery = raw_mastery_by_id
            .get(&concept.id)
            .copied()
            .unwrap_or(concept_mastery);
        let decay_signal = concept_decay_signal(raw_mastery, concept_mastery);
        let inconsistency_signal = concept_inconsistency_signal(store.evidence.get(&concept.id));
        let review_signal = clamp_01(
            (0.60 * concept_uncertainty) + (0.25 * decay_signal) + (0.15 * inconsistency_signal),
        );

        let review_score = gate_factor * review_signal * root_multiplier;

        let balanced_score = gate_factor
            * ((lam * (1.0 - concept_mastery)) + ((1.0 - lam) * concept_uncertainty))
            * root_multiplier;

        let event_count = store
            .evidence
            .get(&concept.id)
            .map(|e| e.len())
            .unwrap_or(0);

        // Seeded-shuffle random score: pre-computed per concept above.
        // Falls back to a simple hash if ranking_seed was not set (should not happen
        // for simulation, but safe for any direct API call without a seed).
        let random_score = random_scores.get(&concept.id).copied().unwrap_or_else(|| {
            let mut hasher = DefaultHasher::new();
            concept.id.hash(&mut hasher);
            event_count.hash(&mut hasher);
            (hasher.finish() % 10_000) as f64 / 10_000.0
        });

        let curriculum_score = {
            let idx = *curriculum_index.get(&concept.id).unwrap_or(&0) as f64;
            let order_weight = 1.0 - (idx / curriculum_len);
            if concept_mastery >= stop_mastery {
                0.0
            } else {
                clamp_01(order_weight)
            }
        };

        let score = match policy {
            "learn_next" => learn_score,
            "review_next" => review_score,
            "balanced" => balanced_score,
            "mastery_only" => (1.0 - concept_mastery) * root_multiplier,
            "uncertainty_only" => concept_uncertainty * root_multiplier,
            "curriculum_linear" => curriculum_score,
            "random" => random_score,
            _ => {
                if event_count == 0 || concept_mastery <= threshold {
                    learn_score
                } else {
                    review_score
                }
            }
        };

        // ── Enriched explainability why ────────────────────────────────────
        let mut why = Vec::new();

        // 1. Prereq readiness with names
        if prereq_details.is_empty() {
            why.push("Sem pré-requisitos: acesso direto ao conceito".to_string());
        } else {
            let weakest = prereq_details.first().unwrap();
            if is_ready {
                let others: Vec<String> = prereq_details
                    .iter()
                    .skip(1)
                    .map(|(n, r)| format!("{} (r={:.2})", n, r))
                    .collect();
                let others_str = if others.is_empty() {
                    String::new()
                } else {
                    format!("; outros: {}", others.join(", "))
                };
                why.push(format!(
                    "Pré-requisitos suficientemente consolidados: '{}' ({:.2}){} — prontidão estrutural {:.2} ≥ limiar",
                    weakest.0, weakest.1, others_str, readiness
                ));
            } else {
                why.push(format!(
                    "Ainda condicionado pelo pré-requisito '{}' — prontidão insuficiente ({:.2})",
                    weakest.0, weakest.1
                ));
            }
        }

        // 2. Conceptual gap
        why.push(format!(
            "Necessidade de aprendizagem: domínio estimado {:.2}; lacuna por consolidar {:.2}",
            concept_mastery,
            1.0 - concept_mastery
        ));

        // 3. Active policy + scores
        why.push(format!(
            "Política de decisão '{}': prioridade para aprender {:.3}; prioridade para rever {:.3}",
            policy, learn_score, review_score
        ));

        // 4. Uncertainty
        if concept_uncertainty >= 0.3 {
            why.push(format!(
                "Prioridade aumentada porque a estimativa ainda é incerta ({:.3})",
                concept_uncertainty
            ));
        } else if concept_uncertainty < 0.05 {
            why.push(format!(
                "Estimativa estável: incerteza muito baixa ({:.3})",
                concept_uncertainty
            ));
        }

        // 5. Decay penalization
        if decay_signal > 0.02 {
            why.push(format!(
                "Penalização por esquecimento temporal: domínio efetivo reduzido em {:.3} devido à antiguidade da evidência",
                decay_signal
            ));
        }

        // 6. Inconsistency
        if inconsistency_signal > 0.15 {
            why.push(format!(
                "Resultados recentes pouco consistentes ({:.2}); recomenda-se recolher nova evidência",
                inconsistency_signal
            ));
        }

        // 7. Soft gate
        if !is_ready {
            why.push(format!(
                "Abaixo do limiar ({:.2}): mantido por soft-gating (k={:.2})",
                threshold, soft_k
            ));
        }

        // 8. Root penalty
        if direct_prereqs == 0 && root_score_penalty > 0.0 {
            why.push(format!(
                "Penalização de raiz aplicada: conceito sem pré-requisitos (×{:.2})",
                root_multiplier
            ));
        }

        let decomposition = ScoreDecomposition {
            readiness,
            gap: 1.0 - concept_mastery,
            uncertainty: concept_uncertainty,
            decay_signal,
            inconsistency: inconsistency_signal,
            gate_factor,
            root_multiplier,
            policy_used: policy.to_string(),
            weakest_prereqs: prereq_details,
        };

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
            score_decomposition: decomposition,
        });
        candidates_ranked += 1;
    }

    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.concept_id.cmp(&b.concept_id))
    });

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
    expected_any: &[String],
) -> (Option<usize>, bool, bool, f64, f64) {
    let rank_of_first_expected = ranked.iter().position(|item| {
        expected_any
            .iter()
            .any(|expected| expected == item.concept_id.as_str())
    });

    let hit_at_1 = rank_of_first_expected == Some(0);
    let hit_at_3 = rank_of_first_expected.map(|idx| idx < 3).unwrap_or(false);
    let mrr = rank_of_first_expected
        .map(|idx| 1.0 / ((idx + 1) as f64))
        .unwrap_or(0.0);

    let expected_set: HashSet<&str> = expected_any.iter().map(String::as_str).collect();
    let dcg_at_3: f64 = ranked
        .iter()
        .take(3)
        .enumerate()
        .filter(|(_, item)| expected_set.contains(item.concept_id.as_str()))
        .map(|(idx, _)| 1.0 / ((idx + 2) as f64).log2())
        .sum();
    let ideal_relevant = expected_any.len().min(3);
    let idcg_at_3: f64 = (0..ideal_relevant)
        .map(|idx| 1.0 / ((idx + 2) as f64).log2())
        .sum();
    let ndcg_at_3 = if idcg_at_3 > 0.0 {
        dcg_at_3 / idcg_at_3
    } else {
        0.0
    };

    (
        rank_of_first_expected.map(|idx| idx + 1),
        hit_at_1,
        hit_at_3,
        mrr,
        ndcg_at_3,
    )
}

fn average_rank_of_expected(ranked: &[NextToStudyItem], expected_any: &[String]) -> f64 {
    if ranked.is_empty() || expected_any.is_empty() {
        return 0.0;
    }

    let mut ranks = Vec::new();
    for expected in expected_any {
        if let Some(rank) = ranked
            .iter()
            .position(|item| item.concept_id == *expected)
            .map(|idx| idx + 1)
        {
            ranks.push(rank as f64);
        }
    }

    if ranks.is_empty() {
        (ranked.len() + 1) as f64
    } else {
        ranks.iter().sum::<f64>() / (ranks.len() as f64)
    }
}

fn simulate_longitudinal_cycles(
    scenario: &ScenarioBlueprint,
    approach: &DynApproach,
    seed: u64,
) -> Result<LongitudinalMetrics, String> {
    let mut store = OlmStore::default();
    materialize_scenario(
        &mut store,
        scenario,
        approach.meta_strength,
        approach.apply_mapping_penalty,
    )?;
    apply_seeded_scenario_noise(&mut store, seed, &format!("{}-long", scenario.id));
    store.config.decay_enabled = approach.decay_enabled;
    store.config.decay_half_life_days = approach.decay_half_life_days;
    store.config.ranking_seed = Some(seeded_scenario_u64(seed, &format!("{}-long", scenario.id)));

    let (start_mastery, _) = average_state(&store);
    let mut bad_recommendations = 0usize;
    let mut prereq_violations = 0usize;
    let mut time_to_mastery = None;
    let cycles = 8usize;

    for cycle in 0..cycles {
        // Set simulated_now so decay uses the scenario timeline (Feb 2026) rather than
        // the real wall-clock time, which would collapse mastery for historical timestamps.
        let cycle_ts_str = format!("2026-02-{:02}T00:00:00Z", cycle + 1);
        store.config.simulated_now = DateTime::parse_from_rfc3339(&cycle_ts_str)
            .ok()
            .map(|dt| dt.with_timezone(&Utc));

        let exclude = HashSet::new();
        let (ranked, _) = rank_next_to_study(
            &store,
            None,
            approach.lambda,
            approach.readiness_threshold,
            approach.min_readiness,
            approach.readiness_gamma,
            approach.readiness_distance_delta,
            &approach.ranking_policy,
            approach.soft_gate_k,
            approach.root_penalty,
            approach.stop_mastery,
            &exclude,
            None,
        );

        let Some(top) = ranked.first() else {
            break;
        };

        if top.readiness < approach.readiness_threshold {
            prereq_violations += 1;
        }

        let latent = scenario
            .true_mastery
            .get(&top.concept_id)
            .copied()
            .unwrap_or(0.5);
        if latent > 0.70 {
            bad_recommendations += 1;
        }

        let practice_score =
            (0.35 + (0.45 * latent) + (0.20 * ((cycle + 1) as f64 / cycles as f64)))
                .clamp(0.0, 1.0);
        let ts = cycle_ts_str;
        let followup = sim_event_at(
            &format!("{}-L", scenario.id),
            (cycle + 1) as u32,
            ts,
            "longitudinal",
            "practice_attempt",
            vec![top.concept_id.as_str()],
            None,
            json!({
                "correct": if practice_score >= 0.5 { 1.0 } else { 0.0 },
                "total": 1.0,
                "confidence": (latent + 0.15).clamp(0.0, 1.0),
                "effort": 0.8,
                "perceived_score": practice_score
            }),
        );

        ingest_into_store(&mut store, &followup, approach.meta_strength)?;

        let (avg_mastery_now, _) = average_state(&store);
        if time_to_mastery.is_none() && avg_mastery_now >= 0.75 {
            time_to_mastery = Some(cycle + 1);
        }
    }

    let (end_mastery, _) = average_state(&store);
    let mastery_gain = end_mastery - start_mastery;

    Ok(LongitudinalMetrics {
        learning_gain: mastery_gain,
        post_test_score: end_mastery,
        mastery_gain,
        time_to_mastery,
        number_of_bad_recommendations: bad_recommendations,
        prerequisite_violation_rate: prereq_violations as f64 / cycles as f64,
    })
}

fn aggregate_subset_metrics(
    scenarios: &[SimulationScenarioResult],
    subset_ids: &[String],
) -> (f64, f64, f64, f64) {
    let subset_id_set: HashSet<&str> = subset_ids.iter().map(|id| id.as_str()).collect();
    let subset: Vec<&SimulationScenarioResult> = scenarios
        .iter()
        .filter(|scenario| subset_id_set.contains(scenario.scenario_id.as_str()))
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

fn build_calibration_split(scenarios: &[ScenarioBlueprint]) -> (Vec<String>, Vec<String>) {
    let ids: Vec<String> = scenarios
        .iter()
        .map(|scenario| scenario.id.clone())
        .collect();
    let id_set: HashSet<&str> = ids.iter().map(|id| id.as_str()).collect();

    let has_canonical = [
        "S1", "S2", "S3", "S4", "S5", "S6", "S7", "S8", "S9", "S10", "S11", "S12",
    ]
    .iter()
    .all(|id| id_set.contains(id));

    if has_canonical {
        let mut train = vec![
            "S1".to_string(),
            "S2".to_string(),
            "S3".to_string(),
            "S4".to_string(),
            "S5".to_string(),
            "S6".to_string(),
            "S7".to_string(),
            "S8".to_string(),
        ];
        let mut test = vec![
            "S9".to_string(),
            "S10".to_string(),
            "S11".to_string(),
            "S12".to_string(),
        ];

        // Generated and hard-case scenarios must not disappear from evaluation.
        // Keep the split deterministic by scenario id so a scenario family never
        // moves between train and test merely because HashMap/order changes.
        let canonical_ids: HashSet<String> = train.iter().chain(test.iter()).cloned().collect();
        let mut extras: Vec<String> = ids
            .iter()
            .filter(|id| !canonical_ids.contains(*id))
            .cloned()
            .collect();
        extras.sort();
        for (idx, id) in extras.into_iter().enumerate() {
            if matches!(id.as_str(), "H1" | "H2" | "H3") {
                train.push(id);
            } else if matches!(id.as_str(), "H4" | "H5" | "H6") {
                test.push(id);
            } else if idx % 10 < 7 {
                train.push(id);
            } else {
                test.push(id);
            }
        }
        return (train, test);
    }

    let split_idx = ((ids.len() as f64) * 0.7).round() as usize;
    let split_idx = split_idx.clamp(1, ids.len().saturating_sub(1));

    let train = ids[..split_idx].to_vec();
    let test = ids[split_idx..].to_vec();
    (train, test)
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
    sim_event_at(
        id,
        step,
        "2026-01-01T00:00:00Z".to_string(),
        source,
        event_type,
        concept_ids,
        content_id,
        payload,
    )
}

fn sim_event_at(
    id: &str,
    step: u32,
    timestamp: String,
    source: &str,
    event_type: &str,
    concept_ids: Vec<&str>,
    content_id: Option<&str>,
    payload: Value,
) -> StudyEvent {
    StudyEvent {
        event_id: format!("{}-{}", id, step),
        timestamp,
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

/// Derive the expected ground-truth concepts to study next.
///
/// Priority 1 — "weak and unblocked": concepts with `true_mastery < 0.55`
/// whose *direct* prerequisites all have `true_mastery >= 0.60`. This aligns
/// the ground truth with pedagogically correct sequencing: the system should
/// surface concepts that are genuinely learnable right now, not blocked ones.
///
/// Priority 2 — fallback to any weak concept (< 0.55) when no unblocked weak
/// concept exists (e.g. all weak concepts have at least one weak prerequisite).
///
/// Priority 3 — if no concept has `true_mastery < 0.55`, return the single
/// concept with the lowest mastery.
fn expected_any_from_true_mastery(
    true_mastery: &HashMap<String, f64>,
    edges: &[(String, String)],
) -> Vec<String> {
    // Build direct-prerequisite map: target → {direct prereqs}
    let mut direct_prereqs: HashMap<&str, Vec<&str>> = HashMap::new();
    for (prereq, target) in edges {
        direct_prereqs
            .entry(target.as_str())
            .or_default()
            .push(prereq.as_str());
    }

    let mut pairs: Vec<(&String, &f64)> = true_mastery.iter().collect();
    pairs.sort_by(|a, b| {
        a.1.partial_cmp(b.1)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.0.cmp(b.0)) // stable tie-break by concept id
    });

    let prereqs_dominated = |id: &str| -> bool {
        let prereqs = direct_prereqs.get(id).map(|v| v.as_slice()).unwrap_or(&[]);
        prereqs
            .iter()
            .all(|p| true_mastery.get(*p).copied().unwrap_or(0.0) >= 0.60)
    };

    // Priority 1: weak + all direct prereqs dominated
    let unblocked_weak: Vec<String> = pairs
        .iter()
        .filter(|(id, mastery)| **mastery < 0.55 && prereqs_dominated(id.as_str()))
        .map(|(id, _)| (*id).clone())
        .take(3)
        .collect();

    if !unblocked_weak.is_empty() {
        return unblocked_weak;
    }

    // Priority 2: any weak concept (prereqs may not be dominated)
    let any_weak: Vec<String> = pairs
        .iter()
        .filter_map(|(id, mastery)| {
            if **mastery < 0.55 {
                Some((*id).clone())
            } else {
                None
            }
        })
        .take(3)
        .collect();

    if !any_weak.is_empty() {
        return any_weak;
    }

    // Priority 3: lowest mastery overall
    pairs
        .first()
        .map(|(id, _)| vec![(*id).clone()])
        .unwrap_or_default()
}

fn build_chain_with_shortcuts(
    concepts: &[String],
    density: f64,
    rng: &mut StdRng,
) -> Vec<(String, String)> {
    let mut edges = Vec::new();
    for idx in 1..concepts.len() {
        edges.push((concepts[idx - 1].clone(), concepts[idx].clone()));
    }

    for target in 2..concepts.len() {
        for prereq in 0..target - 1 {
            if rng.gen_bool(density.clamp(0.0, 1.0) * 0.4) {
                edges.push((concepts[prereq].clone(), concepts[target].clone()));
            }
        }
    }

    edges
}

fn canonical_true_mastery(
    profile: &str,
    concepts: &[String],
    rng: &mut StdRng,
) -> HashMap<String, f64> {
    let mut map = HashMap::new();
    let n = concepts.len().max(1) as f64;

    for (idx, concept_id) in concepts.iter().enumerate() {
        let t = idx as f64 / n;
        let base = match profile {
            "linear" => (0.88 - (0.55 * t)).clamp(0.15, 0.95),
            "blocked" => {
                if idx < 2 {
                    0.20
                } else {
                    0.60
                }
            }
            "core_errors" => {
                if idx == concepts.len() / 2 {
                    0.15
                } else {
                    0.65
                }
            }
            "high_uncertainty" => {
                // Deterministic gradient: avoids HashMap-order ties in expected_any.
                let n = concepts.len().max(1) as f64;
                (0.45 + 0.08 * (idx as f64 / n)).clamp(0.40, 0.60)
            }
            "overconfidence" => rng.gen_range(0.20_f64..0.45_f64),
            "underconfidence" => rng.gen_range(0.70_f64..0.92_f64),
            "multi_weak" => {
                if idx % 2 == 0 {
                    0.30
                } else {
                    0.55
                }
            }
            "advanced_blocked" => {
                if idx + 1 == concepts.len() {
                    0.20
                } else if idx + 2 == concepts.len() {
                    0.25
                } else {
                    0.70
                }
            }
            "almost_mastered" => rng.gen_range(0.84_f64..0.96_f64),
            "all_unknown" => rng.gen_range(0.08_f64..0.26_f64),
            "weak_mapping" => rng.gen_range(0.25_f64..0.60_f64),
            "strong_mapping" => rng.gen_range(0.30_f64..0.70_f64),
            _ => rng.gen_range(0.25_f64..0.75_f64),
        };
        map.insert(concept_id.clone(), clamp_01(base));
    }

    map
}

fn baseline_scenario_blueprints(seed: u64) -> Vec<ScenarioBlueprint> {
    #[derive(Clone)]
    struct CanonicalPreset {
        id: &'static str,
        name: &'static str,
        profile: &'static str,
        graph_size: usize,
        event_count: usize,
        prereq_density: f64,
        mapping_quality: f64,
        old_timestamps: bool,
    }

    let presets = vec![
        CanonicalPreset {
            id: "S1",
            name: "Progressao linear simples",
            profile: "linear",
            graph_size: 12,
            event_count: 28,
            prereq_density: 0.30,
            mapping_quality: 0.90,
            old_timestamps: false,
        },
        CanonicalPreset {
            id: "S2",
            name: "Pre-requisito bloqueado",
            profile: "blocked",
            graph_size: 14,
            event_count: 32,
            prereq_density: 0.50,
            mapping_quality: 0.90,
            old_timestamps: false,
        },
        CanonicalPreset {
            id: "S3",
            name: "Conceito core com erros repetidos",
            profile: "core_errors",
            graph_size: 14,
            event_count: 36,
            prereq_density: 0.50,
            mapping_quality: 0.90,
            old_timestamps: false,
        },
        CanonicalPreset {
            id: "S4",
            name: "Alta incerteza por pouca evidencia",
            profile: "high_uncertainty",
            graph_size: 12,
            event_count: 20,
            prereq_density: 0.38,
            mapping_quality: 0.90,
            old_timestamps: false,
        },
        CanonicalPreset {
            id: "S5",
            name: "Overconfidence",
            profile: "overconfidence",
            graph_size: 15,
            event_count: 40,
            prereq_density: 0.50,
            mapping_quality: 0.88,
            old_timestamps: false,
        },
        CanonicalPreset {
            id: "S6",
            name: "Underconfidence",
            profile: "underconfidence",
            graph_size: 15,
            event_count: 40,
            prereq_density: 0.50,
            mapping_quality: 0.88,
            old_timestamps: false,
        },
        CanonicalPreset {
            id: "S7",
            name: "Varios conceitos fracos",
            profile: "multi_weak",
            graph_size: 16,
            event_count: 45,
            prereq_density: 0.55,
            mapping_quality: 0.90,
            old_timestamps: false,
        },
        CanonicalPreset {
            id: "S8",
            name: "Conceito avancado com pre-requisito fraco",
            profile: "advanced_blocked",
            graph_size: 16,
            event_count: 42,
            prereq_density: 0.55,
            mapping_quality: 0.90,
            old_timestamps: false,
        },
        CanonicalPreset {
            id: "S9",
            name: "Todos os conceitos quase dominados",
            profile: "almost_mastered",
            graph_size: 14,
            event_count: 30,
            prereq_density: 0.50,
            mapping_quality: 0.92,
            old_timestamps: true,
        },
        CanonicalPreset {
            id: "S10",
            name: "Todos os conceitos desconhecidos",
            profile: "all_unknown",
            graph_size: 14,
            event_count: 30,
            prereq_density: 0.50,
            mapping_quality: 0.92,
            old_timestamps: true,
        },
        CanonicalPreset {
            id: "S11",
            name: "Conteudo com mapeamento fraco",
            profile: "weak_mapping",
            graph_size: 12,
            event_count: 28,
            prereq_density: 0.45,
            mapping_quality: 0.35,
            old_timestamps: false,
        },
        CanonicalPreset {
            id: "S12",
            name: "Conteudo com mapeamento correcto",
            profile: "strong_mapping",
            graph_size: 12,
            event_count: 28,
            prereq_density: 0.45,
            mapping_quality: 0.95,
            old_timestamps: false,
        },
    ];

    let mut blueprints = Vec::new();

    for preset in presets {
        let mut rng = StdRng::seed_from_u64(seeded_scenario_u64(seed, preset.id));
        let concepts: Vec<String> = (0..preset.graph_size)
            .map(|i| format!("{}_c{}", preset.id.to_ascii_lowercase(), i + 1))
            .collect();

        let edges = build_chain_with_shortcuts(&concepts, preset.prereq_density, &mut rng);
        let true_mastery = canonical_true_mastery(preset.profile, &concepts, &mut rng);
        let expected_any = expected_any_from_true_mastery(&true_mastery, &edges);

        let event_mix = scenario_event_mix(&mut rng);
        let mut content_items = Vec::new();
        let mut content_concepts = Vec::new();

        if preset.profile == "weak_mapping" || preset.profile == "strong_mapping" {
            for item_idx in 0..3 {
                let content_id =
                    format!("{}_lesson_{}", preset.id.to_ascii_lowercase(), item_idx + 1);
                content_items.push(ContentItem {
                    id: content_id.clone(),
                    item_type: "lesson".to_string(),
                    title: format!("{} Lesson {}", preset.id, item_idx + 1),
                    domain_id: None,
                });
                for concept_id in concepts.iter().skip(item_idx).step_by(2) {
                    content_concepts.push(ContentConceptMap {
                        content_id: content_id.clone(),
                        concept_id: concept_id.clone(),
                        coverage_weight: (rng.gen_range(0.2_f64..1.0_f64) * preset.mapping_quality)
                            .clamp(0.05, 1.0),
                    });
                }
            }
        }

        let mut events = Vec::new();
        let weak_concepts: Vec<String> = concepts
            .iter()
            .filter(|c| true_mastery.get(*c).copied().unwrap_or(0.5) < 0.55)
            .cloned()
            .collect();

        for step in 0..preset.event_count {
            let event_type = sample_event_type(&mut rng, &event_mix);
            let use_weak = !weak_concepts.is_empty() && rng.gen_bool(0.65);
            let concept_id = if use_weak {
                weak_concepts[rng.gen_range(0..weak_concepts.len())].clone()
            } else {
                concepts[rng.gen_range(0..concepts.len())].clone()
            };

            let true_level = true_mastery.get(&concept_id).copied().unwrap_or(0.5);
            let observed = (true_level + rng.gen_range(-0.25_f64..0.25_f64)).clamp(0.0, 1.0);
            let confidence = match preset.profile {
                "overconfidence" => rng.gen_range(0.80_f64..0.98_f64),
                "underconfidence" => rng.gen_range(0.10_f64..0.35_f64),
                _ => (observed + rng.gen_range(-0.15_f64..0.15_f64)).clamp(0.0, 1.0),
            };
            let effort = rng.gen_range(0.35_f64..0.95_f64);
            let perceived_score = match preset.profile {
                "overconfidence" => (confidence + rng.gen_range(0.0_f64..0.15_f64)).clamp(0.0, 1.0),
                "underconfidence" => {
                    (confidence + rng.gen_range(-0.1_f64..0.1_f64)).clamp(0.0, 1.0)
                }
                _ => (observed + rng.gen_range(-0.10_f64..0.10_f64)).clamp(0.0, 1.0),
            };

            let payload = match event_type.as_str() {
                "quiz_attempt" | "practice_attempt" => {
                    json!({
                        "correct": if observed >= 0.5 { 1.0 } else { 0.0 },
                        "total": 1.0,
                        "confidence": confidence,
                        "effort": effort,
                        "perceived_score": perceived_score
                    })
                }
                _ => json!({
                    "confidence": confidence,
                    "effort": effort,
                    "perceived_score": perceived_score
                }),
            };

            let timestamp = if preset.old_timestamps {
                match step % 3 {
                    0 => "2025-01-01T00:00:00Z".to_string(),
                    1 => "2025-06-01T00:00:00Z".to_string(),
                    _ => "2026-01-01T00:00:00Z".to_string(),
                }
            } else {
                format!("2026-01-01T00:{:02}:{:02}Z", (step / 60) % 60, step % 60)
            };

            let use_content = !content_items.is_empty() && rng.gen_bool(0.65);
            let selected_content_id = if use_content {
                Some(
                    content_items[rng.gen_range(0..content_items.len())]
                        .id
                        .as_str(),
                )
            } else {
                None
            };
            let concept_ids = if use_content {
                vec![]
            } else {
                vec![concept_id.as_str()]
            };

            events.push(sim_event_at(
                preset.id,
                (step + 1) as u32,
                timestamp,
                "canonical",
                &event_type,
                concept_ids,
                selected_content_id,
                payload,
            ));
        }

        blueprints.push(ScenarioBlueprint {
            id: preset.id.to_string(),
            name: preset.name.to_string(),
            expected_any,
            true_mastery,
            concepts,
            edges,
            content_items,
            content_concepts,
            events,
            metadata: ScenarioMetadata {
                mode: "canonical".to_string(),
                difficulty: if matches!(preset.id, "S4" | "S5" | "S6" | "S7" | "S8" | "S11" | "S12")
                {
                    "hard".to_string()
                } else {
                    "easy".to_string()
                },
                case_tags: vec![preset.profile.to_string()],
                seed,
                graph_size: preset.graph_size,
                prereq_density: preset.prereq_density,
                depth: 4,
                event_count: preset.event_count,
                event_mix,
                error_rate: 0.25,
                confidence_level: 0.65,
                effort_level: 0.70,
                metacognitive_alignment: 0.65,
                mapping_quality: preset.mapping_quality,
            },
        });
    }

    blueprints
}

fn hard_case_blueprints(seed: u64) -> Vec<ScenarioBlueprint> {
    let cases: Vec<(&str, &str, &str, Vec<f64>)> = vec![
        (
            "H1",
            "Confianca alta com desempenho baixo",
            "overconfidence_low_performance",
            vec![0.85, 0.82, 0.15, 0.35, 0.50, 0.62, 0.70, 0.76],
        ),
        (
            "H2",
            "Confianca baixa com desempenho alto",
            "underconfidence_high_performance",
            vec![0.86, 0.80, 0.85, 0.20, 0.42, 0.58, 0.66, 0.72],
        ),
        (
            "H3",
            "Conceito avancado bloqueado por pre-requisito fraco",
            "weak_prerequisite_advanced_candidate",
            vec![0.20, 0.84, 0.10, 0.64, 0.68, 0.30, 0.72, 0.78],
        ),
        (
            "H4",
            "Revisao incerta versus progressao",
            "review_vs_progression",
            vec![0.82, 0.80, 0.70, 0.35, 0.52, 0.60, 0.68, 0.74],
        ),
        (
            "H5",
            "Multiplos candidatos elegiveis",
            "multiple_eligible_candidates",
            vec![0.88, 0.84, 0.18, 0.22, 0.26, 0.45, 0.55, 0.65],
        ),
        (
            "H6",
            "Sinais objetivos e subjetivos contraditorios",
            "contradictory_signals",
            vec![0.86, 0.82, 0.20, 0.30, 0.40, 0.54, 0.64, 0.72],
        ),
    ];

    let mut blueprints = Vec::new();
    for (case_id, name, tag, levels) in cases {
        let concepts: Vec<String> = (1..=8)
            .map(|idx| format!("{}_c{}", case_id.to_ascii_lowercase(), idx))
            .collect();
        // Two mastered foundations feed six parallel candidates.  This avoids
        // the trivial single-candidate shape of a pure chain.
        let mut edges = Vec::new();
        for target in 2..8 {
            let prereq = if target < 5 { 0 } else { 1 };
            edges.push((concepts[prereq].clone(), concepts[target].clone()));
        }
        let true_mastery: HashMap<String, f64> = concepts
            .iter()
            .cloned()
            .zip(levels.iter().copied())
            .collect();
        let mut expected_any = expected_any_from_true_mastery(&true_mastery, &edges);
        if case_id == "H4" {
            // Independent pedagogical oracle: either consolidate the unstable
            // prior concept or progress to the next weak concept is acceptable.
            expected_any = vec![concepts[2].clone(), concepts[3].clone()];
        }

        let mut events = Vec::new();
        let mut step = 0u32;
        for concept_idx in 0..concepts.len() {
            for repeat in 0..6 {
                if case_id == "H4" && concept_idx == 3 {
                    continue;
                }
                step += 1;
                let latent = levels[concept_idx];
                let mut correct = if latent >= 0.5 { 1.0 } else { 0.0 };
                let mut confidence = if correct > 0.5 { 0.75 } else { 0.30 };
                let mut perceived = confidence;

                match case_id {
                    "H1" if concept_idx == 2 => {
                        correct = 0.0;
                        confidence = 0.95;
                        perceived = 0.95;
                    }
                    "H2" if concept_idx == 2 => {
                        correct = 1.0;
                        confidence = 0.15;
                        perceived = 0.20;
                    }
                    "H4" if concept_idx == 2 => {
                        correct = if repeat % 2 == 0 { 1.0 } else { 0.0 };
                        confidence = 0.80;
                        perceived = if repeat % 2 == 0 { 0.85 } else { 0.75 };
                    }
                    "H6" if concept_idx >= 2 => {
                        correct = if repeat % 3 == 0 { 1.0 } else { 0.0 };
                        confidence = if correct > 0.5 { 0.20 } else { 0.90 };
                        perceived = confidence;
                    }
                    _ => {}
                }

                let timestamp = if case_id == "H4" && concept_idx == 2 {
                    format!("2025-01-{:02}T00:00:00Z", repeat + 1)
                } else {
                    format!("2026-02-01T00:{:02}:{:02}Z", (step / 60) % 60, step % 60)
                };
                let event_type = if case_id == "H6" {
                    match repeat % 4 {
                        0 => "quiz_attempt",
                        1 => "practice_attempt",
                        2 => "study_read",
                        _ => "self_assessment",
                    }
                } else {
                    "practice_attempt"
                };
                events.push(sim_event_at(
                    case_id,
                    step,
                    timestamp,
                    "hard_case",
                    event_type,
                    vec![concepts[concept_idx].as_str()],
                    None,
                    json!({
                        "correct": correct,
                        "total": 1.0,
                        "confidence": confidence,
                        "perceived_score": perceived,
                        "effort": 0.75,
                        "score": confidence,
                        "duration_sec": 600.0,
                        "target_duration_sec": 600.0
                    }),
                ));
            }
        }

        blueprints.push(ScenarioBlueprint {
            id: case_id.to_string(),
            name: name.to_string(),
            expected_any,
            true_mastery,
            concepts,
            edges,
            content_items: vec![],
            content_concepts: vec![],
            events,
            metadata: ScenarioMetadata {
                mode: "hard_case".to_string(),
                difficulty: "hard".to_string(),
                case_tags: vec![tag.to_string(), "multi_candidate".to_string()],
                seed: seeded_scenario_u64(seed, case_id),
                graph_size: 8,
                prereq_density: 0.25,
                depth: 2,
                event_count: step as usize,
                event_mix: HashMap::from([("practice_attempt".to_string(), 1.0)]),
                error_rate: 0.35,
                confidence_level: 0.65,
                effort_level: 0.75,
                metacognitive_alignment: if matches!(case_id, "H1" | "H2" | "H6") {
                    0.20
                } else {
                    0.75
                },
                mapping_quality: 1.0,
            },
        });
    }

    blueprints
}

fn generated_scenario_blueprints(seed: u64, options: &SimulationOptions) -> Vec<ScenarioBlueprint> {
    let mut blueprints = Vec::new();

    for idx in 0..options.generated_scenarios {
        let scenario_seed = seed.wrapping_add((idx as u64) * 7919);
        let mut local_rng = StdRng::seed_from_u64(scenario_seed);
        let graph_size = options.graph_size.max(6);
        let depth = options.depth.max(3).min(graph_size);
        let prereq_density = options.prereq_density.clamp(0.0, 1.0);
        let event_count = options.event_count.max(10);
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

        // Isolado por cenário para evitar correlação artificial intra-seed.
        let event_mix = scenario_event_mix(&mut local_rng);
        let confidence_level = local_rng.gen_range(0.4_f64..0.95_f64);
        let effort_level = local_rng.gen_range(0.35_f64..0.95_f64);
        let metacognitive_alignment = local_rng.gen_range(0.2_f64..0.95_f64);

        let true_mastery: HashMap<String, f64> = concepts
            .iter()
            .map(|concept_id| (concept_id.clone(), local_rng.gen_range(0.1_f64..0.9_f64)))
            .collect();
        let expected_any = expected_any_from_true_mastery(&true_mastery, &edges);

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
            for concept_id in &concepts {
                if local_rng.gen_bool(0.5) {
                    let weight =
                        (local_rng.gen_range(0.2_f64..1.0_f64) * mapping_quality).clamp(0.05, 1.0);
                    content_concepts.push(ContentConceptMap {
                        content_id: content_id.clone(),
                        concept_id: concept_id.clone(),
                        coverage_weight: weight,
                    });
                }
            }
        }

        let mut events = Vec::new();
        for step in 0..event_count {
            let event_type = sample_event_type(&mut local_rng, &event_mix);
            let concept_id = concepts[local_rng.gen_range(0..graph_size)].clone();
            let latent = true_mastery.get(&concept_id).copied().unwrap_or(0.5);
            let observed = (latent + local_rng.gen_range(-0.30_f64..0.30_f64)).clamp(0.0, 1.0);

            let perceived = (observed * metacognitive_alignment)
                + ((1.0 - metacognitive_alignment) * local_rng.gen_range(0.0_f64..1.0_f64));
            let confidence =
                (confidence_level + local_rng.gen_range(-0.18_f64..0.18_f64)).clamp(0.0, 1.0);
            let effort = (effort_level + local_rng.gen_range(-0.15_f64..0.15_f64)).clamp(0.0, 1.0);
            let payload = match event_type.as_str() {
                "quiz_attempt" | "practice_attempt" => json!({
                    "correct": if observed >= 0.5 { 1.0 } else { 0.0 },
                    "total": 1.0,
                    "confidence": confidence,
                    "effort": effort,
                    "perceived_score": perceived
                }),
                _ => json!({
                    "confidence": confidence,
                    "effort": effort,
                    "perceived_score": perceived
                }),
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

            events.push(sim_event_at(
                &format!("G{}", idx + 1),
                (step + 1) as u32,
                format!("2026-01-01T00:{:02}:{:02}Z", (step / 60) % 60, step % 60),
                "generated",
                &event_type,
                concept_ids,
                content_id,
                payload,
            ));
        }

        blueprints.push(ScenarioBlueprint {
            id: format!("G{}", idx + 1),
            name: format!("Generated scenario {}", idx + 1),
            expected_any,
            true_mastery,
            concepts,
            edges,
            content_items,
            content_concepts,
            events,
            metadata: ScenarioMetadata {
                mode: "generated".to_string(),
                difficulty: if graph_size >= 8 || metacognitive_alignment < 0.45 {
                    "hard".to_string()
                } else {
                    "easy".to_string()
                },
                case_tags: vec!["generated".to_string(), "multi_candidate".to_string()],
                seed: scenario_seed,
                graph_size,
                prereq_density,
                depth,
                event_count,
                event_mix,
                error_rate: 0.25,
                confidence_level,
                effort_level,
                metacognitive_alignment,
                mapping_quality,
            },
        });
    }

    blueprints
}

fn seeded_scenario_u64(seed: u64, scenario_id: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    scenario_id.hash(&mut hasher);
    hasher.finish()
}

fn apply_seeded_scenario_noise(store: &mut OlmStore, seed: u64, scenario_id: &str) {
    let mut rng = StdRng::seed_from_u64(seeded_scenario_u64(seed, scenario_id));

    // Apply light deterministic perturbation so multi-seed runs are reproducible with small variance.
    for state in store.concept_state.values_mut() {
        let alpha_scale = rng.gen_range(0.98_f64..1.02_f64);
        let beta_scale = rng.gen_range(0.98_f64..1.02_f64);
        state.alpha = (state.alpha * alpha_scale).clamp(0.1, 1_000.0);
        state.beta = (state.beta * beta_scale).clamp(0.1, 1_000.0);
    }
}

fn build_simulation_scenarios(seed: u64, options: &SimulationOptions) -> Vec<ScenarioBlueprint> {
    let mode = options.scenario_mode.trim().to_ascii_lowercase();

    match mode.as_str() {
        "canonical" | "baseline" => {
            let mut scenarios = baseline_scenario_blueprints(seed);
            scenarios.extend(hard_case_blueprints(seed));
            scenarios
        }
        "hard" | "hard_cases" => hard_case_blueprints(seed),
        "generated" => generated_scenario_blueprints(seed, options),
        _ => {
            let mut scenarios = baseline_scenario_blueprints(seed);
            scenarios.extend(hard_case_blueprints(seed));
            scenarios.extend(generated_scenario_blueprints(seed, options));
            scenarios
        }
    }
}

fn materialize_scenario(
    store: &mut OlmStore,
    scenario: &ScenarioBlueprint,
    meta_strength: f64,
    apply_mapping_penalty: bool,
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
        if apply_mapping_penalty {
            match ingest_into_store(store, event, meta_strength) {
                Ok(_) => {}
                Err(err)
                    if err.contains(
                        "Event must include concept_ids or a content_id mapped to concepts",
                    ) =>
                {
                    continue;
                }
                Err(err) => return Err(err),
            }
            continue;
        }

        // No-mapping-penalty ablation: resolve content mappings to explicit concept_ids
        // so ingest path skips mapping-quality penalty branch.
        let mut adapted_event = event.clone();
        if adapted_event.concept_ids.is_empty() {
            if let Some(content_id) = adapted_event.content_id.as_ref() {
                let mapped: Vec<String> = store
                    .content_concepts
                    .iter()
                    .filter(|map| &map.content_id == content_id)
                    .map(|map| map.concept_id.clone())
                    .collect();
                if !mapped.is_empty() {
                    adapted_event.concept_ids = mapped;
                }
            }
        }

        if adapted_event.concept_ids.is_empty() && adapted_event.content_id.is_some() {
            // Some synthetic events intentionally stress weak content mappings.
            // For this ablation, drop unresolved mapping-only events instead of failing.
            continue;
        }

        match ingest_into_store(store, &adapted_event, meta_strength) {
            Ok(_) => {}
            Err(err)
                if err.contains(
                    "Event must include concept_ids or a content_id mapped to concepts",
                ) =>
            {
                continue;
            }
            Err(err) => return Err(err),
        }
    }

    Ok(())
}

/// Owned runtime approach config used inside the simulation loop.
/// Built-in `ApproachConfig` (with `&'static str`) converts into this.
#[derive(Debug, Clone)]
struct DynApproach {
    id: String,
    name: String,
    description: String,
    lambda: f64,
    readiness_threshold: f64,
    min_readiness: f64,
    readiness_gamma: f64,
    readiness_distance_delta: f64,
    ranking_policy: String,
    root_penalty: f64,
    meta_strength: f64,
    soft_gate_k: Option<f64>,
    stop_mastery: f64,
    decay_enabled: bool,
    decay_half_life_days: f64,
    apply_mapping_penalty: bool,
}

impl From<&ApproachConfig> for DynApproach {
    fn from(a: &ApproachConfig) -> Self {
        Self {
            id: a.id.to_string(),
            name: a.name.to_string(),
            description: a.description.to_string(),
            lambda: a.lambda,
            readiness_threshold: a.readiness_threshold,
            min_readiness: a.min_readiness,
            readiness_gamma: a.readiness_gamma,
            readiness_distance_delta: a.readiness_distance_delta,
            ranking_policy: a.ranking_policy.to_string(),
            root_penalty: a.root_penalty,
            meta_strength: a.meta_strength,
            soft_gate_k: a.soft_gate_k,
            stop_mastery: a.stop_mastery,
            decay_enabled: a.decay_enabled,
            decay_half_life_days: a.decay_half_life_days,
            apply_mapping_penalty: a.apply_mapping_penalty,
        }
    }
}

fn resolve_custom_approach(custom: &CustomApproachConfig) -> DynApproach {
    let builtin = approach_definitions();
    let requested_base = custom.base.as_deref().unwrap_or("no_metacognition");
    let base = builtin
        .iter()
        .find(|a| a.id == requested_base)
        .map(DynApproach::from)
        .or_else(|| {
            builtin
                .iter()
                .find(|a| a.id == "no_metacognition")
                .map(DynApproach::from)
        })
        .unwrap_or_else(|| DynApproach::from(&builtin[0]));
    DynApproach {
        id: custom.id.clone(),
        name: custom.name.clone(),
        description: custom
            .description
            .clone()
            .unwrap_or_else(|| format!("Custom: {}", custom.id)),
        lambda: custom.lambda.unwrap_or(base.lambda),
        readiness_threshold: custom
            .readiness_threshold
            .unwrap_or(base.readiness_threshold),
        min_readiness: custom.min_readiness.unwrap_or(base.min_readiness),
        readiness_gamma: custom.readiness_gamma.unwrap_or(base.readiness_gamma),
        readiness_distance_delta: custom
            .readiness_distance_delta
            .unwrap_or(base.readiness_distance_delta),
        ranking_policy: custom.ranking_policy.clone().unwrap_or(base.ranking_policy),
        root_penalty: custom.root_penalty.unwrap_or(base.root_penalty),
        meta_strength: custom.meta_strength.unwrap_or(base.meta_strength),
        soft_gate_k: custom.soft_gate_k.or(base.soft_gate_k),
        stop_mastery: custom.stop_mastery.unwrap_or(base.stop_mastery),
        decay_enabled: base.decay_enabled,
        decay_half_life_days: base.decay_half_life_days,
        apply_mapping_penalty: base.apply_mapping_penalty,
    }
}

fn snapshot_concept_states(store: &OlmStore) -> Vec<ConceptStateView> {
    let mut rows: Vec<ConceptStateView> = store
        .concepts
        .values()
        .map(|concept| {
            let raw = store
                .concept_state
                .get(&concept.id)
                .cloned()
                .unwrap_or(ConceptState {
                    alpha: 1.0,
                    beta: 1.0,
                    last_update: None,
                });
            let effective = decay_state_if_needed(&raw, &store.config);
            ConceptStateView {
                concept_id: concept.id.clone(),
                name: concept.name.clone(),
                mastery: mastery(&effective),
                uncertainty: uncertainty(&effective),
                alpha: effective.alpha,
                beta: effective.beta,
                last_update: raw.last_update,
            }
        })
        .collect();
    rows.sort_by(|a, b| a.concept_id.cmp(&b.concept_id));
    rows
}

fn item_from_any_approach<'a>(
    approach_results: &'a [(String, Vec<NextToStudyItem>)],
    concept_id: &str,
) -> Option<&'a NextToStudyItem> {
    for (_, ranked) in approach_results {
        if let Some(item) = ranked.iter().find(|i| i.concept_id == concept_id) {
            return Some(item);
        }
    }
    None
}

fn build_scenario_comparison(
    scenario: &ScenarioBlueprint,
    approach_results: &[(String, Vec<NextToStudyItem>)],
) -> ScenarioApproachComparison {
    let mut concept_ids = scenario.concepts.clone();
    concept_ids.sort();

    let concepts = concept_ids
        .iter()
        .map(|cid| {
            let name = item_from_any_approach(approach_results, cid)
                .map(|i| i.name.clone())
                .unwrap_or_else(|| cid.clone());

            let slices = approach_results
                .iter()
                .map(|(approach_id, ranked)| {
                    let idx = ranked.iter().position(|item| &item.concept_id == cid);
                    let item = ranked.iter().find(|item| &item.concept_id == cid);
                    ConceptApproachSlice {
                        approach_id: approach_id.clone(),
                        rank: idx.map(|i| i + 1),
                        score: item.map(|i| i.score).unwrap_or(0.0),
                        mastery: item.map(|i| i.mastery).unwrap_or(0.0),
                        uncertainty: item.map(|i| i.uncertainty).unwrap_or(0.0),
                        readiness: item.map(|i| i.readiness).unwrap_or(0.0),
                        gate_factor: item.map(|i| i.gate_factor).unwrap_or(0.0),
                        gap: item.map(|i| i.score_decomposition.gap).unwrap_or(0.0),
                        decay_signal: item
                            .map(|i| i.score_decomposition.decay_signal)
                            .unwrap_or(0.0),
                        inconsistency: item
                            .map(|i| i.score_decomposition.inconsistency)
                            .unwrap_or(0.0),
                        policy_used: item
                            .map(|i| i.score_decomposition.policy_used.clone())
                            .unwrap_or_default(),
                    }
                })
                .collect();

            ConceptCrossApproachRow {
                concept_id: cid.clone(),
                name,
                slices,
            }
        })
        .collect();

    ScenarioApproachComparison {
        scenario_id: scenario.id.clone(),
        scenario_name: scenario.name.clone(),
        concepts,
    }
}

fn approach_definitions() -> Vec<ApproachConfig> {
    vec![
        ApproachConfig {
            id: "baseline",
            name: "Simple Baseline",
            description: "Baseline isolado: menor domínio estimado, sem pré-requisitos, metacognição, incerteza ou penalizações avançadas",
            lambda: 1.0,
            readiness_threshold: 0.0,
            min_readiness: 0.0,
            readiness_gamma: 0.0,
            readiness_distance_delta: 1.0,
            ranking_policy: "mastery_only",
            root_penalty: 0.0,
            meta_strength: 0.0,
            soft_gate_k: Some(0.0),
            stop_mastery: 1.0,
            decay_enabled: false,
            decay_half_life_days: 30.0,
            apply_mapping_penalty: false,
        },
        ApproachConfig {
            id: "random_baseline",
            name: "Random Baseline",
            description: "Escolha aleatoria de conceito recomendavel",
            lambda: 0.7,
            readiness_threshold: 0.5,
            min_readiness: 0.1,
            readiness_gamma: 0.35,
            readiness_distance_delta: 0.85,
            ranking_policy: "random",
            root_penalty: 0.12,
            meta_strength: 0.0,
            soft_gate_k: Some(1.6),
            stop_mastery: 0.85,
            decay_enabled: false,
            decay_half_life_days: 30.0,
            apply_mapping_penalty: true,
        },
        ApproachConfig {
            id: "mastery_only",
            name: "Mastery Only",
            description: "score = 1 - mastery, sem readiness e sem metacognicao",
            lambda: 0.7,
            readiness_threshold: 0.5,
            min_readiness: 0.0,
            readiness_gamma: 0.35,
            readiness_distance_delta: 0.85,
            ranking_policy: "mastery_only",
            root_penalty: 0.0,
            meta_strength: 0.0,
            soft_gate_k: Some(0.0),
            stop_mastery: 0.85,
            decay_enabled: false,
            decay_half_life_days: 30.0,
            apply_mapping_penalty: true,
        },
        ApproachConfig {
            id: "uncertainty_only",
            name: "Uncertainty Only",
            description: "score = uncertainty, estrategia exploratoria",
            lambda: 0.7,
            readiness_threshold: 0.5,
            min_readiness: 0.0,
            readiness_gamma: 0.35,
            readiness_distance_delta: 0.85,
            ranking_policy: "uncertainty_only",
            root_penalty: 0.0,
            meta_strength: 0.0,
            soft_gate_k: Some(0.0),
            stop_mastery: 0.85,
            decay_enabled: false,
            decay_half_life_days: 30.0,
            apply_mapping_penalty: true,
        },
        ApproachConfig {
            id: "mastery_only_gated",
            name: "Mastery Only (Gated)",
            description: "score = (1-mastery) * readiness^k — ablacao limpa gap vs gap+readiness: balanced com lambda=1.0 para que gate_factor seja aplicado",
            lambda: 1.0,             // score = gate * gap (sem uncertainty)
            readiness_threshold: 0.5,
            min_readiness: 0.1,
            readiness_gamma: 0.35,
            readiness_distance_delta: 0.85,
            ranking_policy: "balanced", // balanced aplica gate_factor = readiness^soft_k
            root_penalty: 0.0,       // identico a mastery_only para isolar apenas o efeito do gating
            meta_strength: 0.0,
            soft_gate_k: Some(1.6),
            stop_mastery: 0.85,
            decay_enabled: false,
            decay_half_life_days: 30.0,
            apply_mapping_penalty: true,
        },
        ApproachConfig {
            id: "curriculum_linear",
            name: "Curriculum Linear",
            description: "Primeiro conceito nao dominado segundo ordem curricular",
            lambda: 0.7,
            readiness_threshold: 0.5,
            min_readiness: 0.0,
            readiness_gamma: 0.35,
            readiness_distance_delta: 0.85,
            ranking_policy: "curriculum_linear",
            root_penalty: 0.0,
            meta_strength: 0.0,
            soft_gate_k: Some(0.0),
            stop_mastery: 0.85,
            decay_enabled: false,
            decay_half_life_days: 30.0,
            apply_mapping_penalty: true,
        },
        ApproachConfig {
            id: "no_root_penalty",
            name: "No Root Penalty",
            description: "Ablacao explicita sem penalizacao para conceitos raiz",
            lambda: 0.7,
            readiness_threshold: 0.5,
            min_readiness: 0.1,
            readiness_gamma: 0.35,
            readiness_distance_delta: 0.85,
            ranking_policy: "balanced",
            root_penalty: 0.0,
            meta_strength: 0.0,
            soft_gate_k: Some(1.6),
            stop_mastery: 0.85,
            decay_enabled: false,
            decay_half_life_days: 30.0,
            apply_mapping_penalty: true,
        },
        ApproachConfig {
            id: "no_prereq_gating",
            name: "No Prereq Gating",
            description: "Mesma logica base sem gating de pre-requisitos",
            lambda: 0.7,
            readiness_threshold: 0.5,
            min_readiness: 0.0,
            readiness_gamma: 0.35,
            readiness_distance_delta: 0.85,
            ranking_policy: "balanced",
            root_penalty: 0.12,
            meta_strength: 0.0,
            soft_gate_k: Some(0.0),
            stop_mastery: 0.85,
            decay_enabled: false,
            decay_half_life_days: 30.0,
            apply_mapping_penalty: true,
        },
        ApproachConfig {
            id: "no_mapping_penalty",
            name: "No Mapping Penalty",
            description: "Ablacao explicita sem penalizacao de qualidade de mapeamento",
            lambda: 0.7,
            readiness_threshold: 0.5,
            min_readiness: 0.1,
            readiness_gamma: 0.35,
            readiness_distance_delta: 0.85,
            ranking_policy: "balanced",
            root_penalty: 0.12,
            meta_strength: 0.0,
            soft_gate_k: Some(1.6),
            stop_mastery: 0.85,
            decay_enabled: false,
            decay_half_life_days: 30.0,
            apply_mapping_penalty: false,
        },
        ApproachConfig {
            id: "no_metacognition",
            name: "No Metacognition",
            description: "Ablacao explicita sem sinal metacognitivo",
            lambda: 0.7,
            readiness_threshold: 0.5,
            min_readiness: 0.1,
            readiness_gamma: 0.35,
            readiness_distance_delta: 0.85,
            ranking_policy: "balanced",
            root_penalty: 0.12,
            meta_strength: 0.0,
            soft_gate_k: Some(1.6),
            stop_mastery: 0.85,
            decay_enabled: false,
            decay_half_life_days: 30.0,
            apply_mapping_penalty: true,
        },
        ApproachConfig {
            id: "metacog_balanced",
            name: "Metacognition Balanced",
            description: "Metacognicao equilibrada (meta_strength=0.6)",
            lambda: 0.7,
            readiness_threshold: 0.5,
            min_readiness: 0.1,
            readiness_gamma: 0.35,
            readiness_distance_delta: 0.85,
            ranking_policy: "balanced",
            root_penalty: 0.12,
            meta_strength: 0.6,
            soft_gate_k: Some(1.6),
            stop_mastery: 0.85,
            decay_enabled: false,
            decay_half_life_days: 30.0,
            apply_mapping_penalty: true,
        },
        ApproachConfig {
            id: "metacog_strict",
            name: "Metacognition Strong",
            description: "Metacognicao forte (meta_strength=1.0), mesmos parametros estruturais que no_metacognition",
            lambda: 0.7,
            readiness_threshold: 0.5,
            min_readiness: 0.1,
            readiness_gamma: 0.35,
            readiness_distance_delta: 0.85,
            ranking_policy: "balanced",
            root_penalty: 0.12,
            meta_strength: 1.0,
            soft_gate_k: Some(1.6),
            stop_mastery: 0.85,
            decay_enabled: false,
            decay_half_life_days: 30.0,
            apply_mapping_penalty: true,
        },
        ApproachConfig {
            id: "decay_hl7",
            name: "Decay HL 7d",
            description: "Com metacognicao moderada e decay ativo (7 dias)",
            lambda: 0.7,
            readiness_threshold: 0.5,
            min_readiness: 0.1,
            readiness_gamma: 0.35,
            readiness_distance_delta: 0.85,
            ranking_policy: "balanced",
            root_penalty: 0.12,
            meta_strength: 0.6,
            soft_gate_k: Some(1.6),
            stop_mastery: 0.85,
            decay_enabled: true,
            decay_half_life_days: 7.0,
            apply_mapping_penalty: true,
        },
        ApproachConfig {
            id: "decay_hl30",
            name: "Decay HL 30d",
            description: "Com metacognicao moderada e decay ativo (30 dias)",
            lambda: 0.7,
            readiness_threshold: 0.5,
            min_readiness: 0.1,
            readiness_gamma: 0.35,
            readiness_distance_delta: 0.85,
            ranking_policy: "balanced",
            root_penalty: 0.12,
            meta_strength: 0.6,
            soft_gate_k: Some(1.6),
            stop_mastery: 0.85,
            decay_enabled: true,
            decay_half_life_days: 30.0,
            apply_mapping_penalty: true,
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

    // Cycle detection: reject if target_id is already an ancestor of prereq_id.
    let mut prereq_graph: HashMap<String, Vec<String>> = HashMap::new();
    for e in &store.edges {
        prereq_graph
            .entry(e.target_id.clone())
            .or_default()
            .push(e.prereq_id.clone());
    }
    let ancestors = ancestor_distances(&edge.prereq_id, &prereq_graph);
    if ancestors.contains_key(&edge.target_id) {
        return Err(
            "Adding this prerequisite would create a cycle in the knowledge graph".to_string(),
        );
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
pub fn olm_remove_content_concept_maps(
    state: State<OlmState>,
    content_id: String,
) -> Result<usize, String> {
    let mut store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    if content_id.trim().is_empty() {
        return Err("content_id is required".to_string());
    }
    let before = store.content_concepts.len();
    store
        .content_concepts
        .retain(|m| m.content_id != content_id);
    Ok(before - store.content_concepts.len())
}

#[tauri::command]
pub fn olm_ingest_event(state: State<OlmState>, event: StudyEvent) -> Result<IngestResult, String> {
    let mut store = state.inner.lock().map_err(|_| "State lock poisoned")?;
    let meta_strength = store.config.meta_strength;

    ingest_into_store(&mut store, &event, meta_strength)
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
                uncertainty: uncertainty(&effective_state),
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
        lambda.unwrap_or(config.ranking_lambda_resolved()),
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
    let (train_split, test_split) = build_calibration_split(&scenarios);
    let generated_at = format!("simulated-seed-{}", seed);

    // Build effective approach list: built-in three + any custom ones.
    let mut approaches: Vec<DynApproach> = if options.custom_approaches_only {
        vec![]
    } else {
        approach_definitions()
            .iter()
            .map(DynApproach::from)
            .collect()
    };
    for custom in &options.custom_approaches {
        approaches.push(resolve_custom_approach(custom));
    }
    if approaches.is_empty() {
        return Err(
            "No approaches to run. Provide at least one built-in or custom approach.".to_string(),
        );
    }

    let mut approach_reports: Vec<SimulationApproachResult> = Vec::new();
    // Per-scenario per-approach full rankings, used to build the comparison table.
    let mut comparison_data: HashMap<String, Vec<(String, Vec<NextToStudyItem>)>> = HashMap::new();

    for approach in &approaches {
        let mut scenario_results = Vec::new();
        let mut passed = 0usize;
        let mut hit_at_3_count = 0usize;
        let mut mrr_sum = 0.0;
        let mut ndcg_sum = 0.0;
        let mut avg_rank_sum = 0.0;
        let mut longitudinal_gain_sum = 0.0;
        let mut post_test_sum = 0.0;
        let mut mastery_gain_sum = 0.0;
        let mut bad_recs_sum = 0usize;
        let mut prereq_violation_sum = 0.0;
        let mut time_to_mastery_values = Vec::new();
        let mut candidate_count_sum = 0usize;
        let mut tie_count = 0usize;
        let mut easy_total = 0usize;
        let mut easy_passed = 0usize;
        let mut hard_total = 0usize;
        let mut hard_passed = 0usize;

        for scenario in scenarios.iter() {
            let mut store = OlmStore::default();
            materialize_scenario(
                &mut store,
                scenario,
                approach.meta_strength,
                approach.apply_mapping_penalty,
            )?;
            apply_seeded_scenario_noise(&mut store, seed, &scenario.id);
            store.config.decay_enabled = approach.decay_enabled;
            store.config.decay_half_life_days = approach.decay_half_life_days;
            store.config.stop_mastery = approach.stop_mastery;
            // Give random_baseline a reproducible per-(scenario, seed) shuffle seed
            // so each scenario+seed pair produces a different random ordering.
            store.config.ranking_seed = Some(seeded_scenario_u64(seed, &scenario.id));

            let sim_exclude: HashSet<String> = HashSet::new();
            // Rank ALL concepts (no top limit) so full_ranking and comparison are complete.
            let (all_ranked, diagnostics) = rank_next_to_study(
                &store,
                None,
                approach.lambda,
                approach.readiness_threshold,
                approach.min_readiness,
                approach.readiness_gamma,
                approach.readiness_distance_delta,
                &approach.ranking_policy,
                approach.soft_gate_k,
                approach.root_penalty,
                approach.stop_mastery,
                &sim_exclude,
                None,
            );

            let concept_states = snapshot_concept_states(&store);
            let (avg_mastery, avg_uncertainty) = average_state(&store);

            let top_recommendation = all_ranked.first().map(|item| item.concept_id.clone());
            let top_tie = all_ranked
                .get(0)
                .zip(all_ranked.get(1))
                .map(|(first, second)| (first.score - second.score).abs() <= 1e-9)
                .unwrap_or(false);
            let candidate_count = all_ranked.len();
            let top_recommendations: Vec<String> = all_ranked
                .iter()
                .take(3)
                .map(|item| item.concept_id.clone())
                .collect();
            let (rank_of_first_expected, pass, hit_at_3, mrr, ndcg_at_3) =
                scenario_ranking_metrics(&all_ranked, &scenario.expected_any);
            let avg_rank_expected = average_rank_of_expected(&all_ranked, &scenario.expected_any);
            let long_metrics = simulate_longitudinal_cycles(scenario, approach, seed)?;

            if pass {
                passed += 1;
            }
            if scenario.metadata.difficulty == "hard" {
                hard_total += 1;
                if pass {
                    hard_passed += 1;
                }
            } else {
                easy_total += 1;
                if pass {
                    easy_passed += 1;
                }
            }
            candidate_count_sum += candidate_count;
            if top_tie {
                tie_count += 1;
            }
            if hit_at_3 {
                hit_at_3_count += 1;
            }
            mrr_sum += mrr;
            ndcg_sum += ndcg_at_3;
            avg_rank_sum += avg_rank_expected;
            longitudinal_gain_sum += long_metrics.learning_gain;
            post_test_sum += long_metrics.post_test_score;
            mastery_gain_sum += long_metrics.mastery_gain;
            bad_recs_sum += long_metrics.number_of_bad_recommendations;
            prereq_violation_sum += long_metrics.prerequisite_violation_rate;
            if let Some(ttm) = long_metrics.time_to_mastery {
                time_to_mastery_values.push(ttm as f64);
            }

            // Collect for comparison table (full, untruncated).
            comparison_data
                .entry(scenario.id.clone())
                .or_default()
                .push((approach.id.clone(), all_ranked.clone()));

            // Apply top_n limit to what we store in full_ranking.
            let full_ranking = if options.top_n == 0 {
                all_ranked
            } else {
                all_ranked.into_iter().take(options.top_n).collect()
            };

            scenario_results.push(SimulationScenarioResult {
                scenario_id: scenario.id.clone(),
                scenario_name: scenario.name.clone(),
                expected_any: scenario.expected_any.clone(),
                true_weak_concepts: expected_any_from_true_mastery(&scenario.true_mastery, &scenario.edges),
                scenario_metadata: scenario.metadata.clone(),
                top_recommendation,
                top_recommendations,
                rank_of_first_expected,
                pass,
                hit_at_3,
                candidate_count,
                top_tie,
                mrr,
                ndcg_at_3,
                avg_mastery,
                avg_uncertainty,
                diagnostics,
                average_rank_of_expected: avg_rank_expected,
                learning_gain: long_metrics.learning_gain,
                post_test_score: long_metrics.post_test_score,
                mastery_gain: long_metrics.mastery_gain,
                time_to_mastery: long_metrics.time_to_mastery,
                number_of_bad_recommendations: long_metrics.number_of_bad_recommendations,
                prerequisite_violation_rate: long_metrics.prerequisite_violation_rate,
                notes: format!(
                    "approach={}, lambda={:.2}, threshold={:.2}, min_readiness={:.2}, readiness_gamma={:.2}, root_penalty={:.2}, meta_strength={:.2}, soft_gate_k={}, decay_enabled={}, decay_half_life_days={:.1}",
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
                        .unwrap_or_else(|| "none".to_string()),
                    approach.decay_enabled,
                    approach.decay_half_life_days
                ),
                full_ranking,
                concept_states,
            });
        }

        let n = scenarios.len();
        let pass_rate = if n == 0 {
            0.0
        } else {
            passed as f64 / n as f64
        };
        let hit_at_3_rate = if n == 0 {
            0.0
        } else {
            hit_at_3_count as f64 / n as f64
        };
        let avg_mrr = if n == 0 { 0.0 } else { mrr_sum / n as f64 };
        let avg_ndcg_at_3 = if n == 0 { 0.0 } else { ndcg_sum / n as f64 };
        let avg_rank_expected = if n == 0 { 0.0 } else { avg_rank_sum / n as f64 };
        let avg_learning_gain = if n == 0 {
            0.0
        } else {
            longitudinal_gain_sum / n as f64
        };
        let avg_post_test_score = if n == 0 {
            0.0
        } else {
            post_test_sum / n as f64
        };
        let avg_mastery_gain = if n == 0 {
            0.0
        } else {
            mastery_gain_sum / n as f64
        };
        let avg_bad_recommendations = if n == 0 {
            0.0
        } else {
            bad_recs_sum as f64 / n as f64
        };
        let prereq_violation_rate = if n == 0 {
            0.0
        } else {
            prereq_violation_sum / n as f64
        };
        let avg_time_to_mastery = if time_to_mastery_values.is_empty() {
            None
        } else {
            Some(time_to_mastery_values.iter().sum::<f64>() / (time_to_mastery_values.len() as f64))
        };
        let candidate_count_avg = if n == 0 {
            0.0
        } else {
            candidate_count_sum as f64 / n as f64
        };
        let tie_rate = if n == 0 {
            0.0
        } else {
            tie_count as f64 / n as f64
        };
        let easy_case_pass_rate = if easy_total == 0 {
            0.0
        } else {
            easy_passed as f64 / easy_total as f64
        };
        let hard_case_pass_rate = if hard_total == 0 {
            0.0
        } else {
            hard_passed as f64 / hard_total as f64
        };

        let (train_pass_rate, _, train_avg_mrr, _) =
            aggregate_subset_metrics(&scenario_results, &train_split);
        let (test_pass_rate, _, test_avg_mrr, _) =
            aggregate_subset_metrics(&scenario_results, &test_split);

        approach_reports.push(SimulationApproachResult {
            approach_id: approach.id.clone(),
            approach_name: approach.name.clone(),
            description: approach.description.clone(),
            hit_at_1_rate: pass_rate,
            pass_rate,
            hit_at_3_rate,
            avg_mrr,
            avg_ndcg_at_3,
            average_rank_of_expected: avg_rank_expected,
            train_pass_rate,
            test_pass_rate,
            train_avg_mrr,
            test_avg_mrr,
            random_baseline_delta: 0.0,
            mastery_baseline_delta: 0.0,
            uncertainty_baseline_delta: 0.0,
            metacognition_rank_shift: 0.0,
            avg_learning_gain,
            avg_post_test_score,
            avg_mastery_gain,
            avg_time_to_mastery,
            avg_bad_recommendations,
            prerequisite_violation_rate: prereq_violation_rate,
            candidate_count_avg,
            tie_rate,
            easy_case_pass_rate,
            hard_case_pass_rate,
            decision_divergence_rate: 0.0,
            ranking_delta_vs_baseline: 0.0,
            meta_influence_rate: 0.0,
            scenarios: scenario_results,
        });
    }

    let random_baseline = approach_reports
        .iter()
        .find(|r| r.approach_id == "random_baseline")
        .map(|r| r.hit_at_1_rate)
        .unwrap_or(0.0);
    let mastery_baseline = approach_reports
        .iter()
        .find(|r| r.approach_id == "mastery_only")
        .map(|r| r.hit_at_1_rate)
        .unwrap_or(0.0);
    let uncertainty_baseline = approach_reports
        .iter()
        .find(|r| r.approach_id == "uncertainty_only")
        .map(|r| r.hit_at_1_rate)
        .unwrap_or(0.0);
    let no_meta_rank = approach_reports
        .iter()
        .find(|r| r.approach_id == "no_metacognition")
        .map(|r| r.average_rank_of_expected)
        .unwrap_or(0.0);

    for report in &mut approach_reports {
        report.random_baseline_delta = report.hit_at_1_rate - random_baseline;
        report.mastery_baseline_delta = report.hit_at_1_rate - mastery_baseline;
        report.uncertainty_baseline_delta = report.hit_at_1_rate - uncertainty_baseline;
        report.metacognition_rank_shift = no_meta_rank - report.average_rank_of_expected;
    }

    let decisions_by_approach: HashMap<String, Vec<Option<String>>> = approach_reports
        .iter()
        .map(|report| {
            (
                report.approach_id.clone(),
                report
                    .scenarios
                    .iter()
                    .map(|scenario| scenario.top_recommendation.clone())
                    .collect(),
            )
        })
        .collect();
    let simple_baseline_mrr = approach_reports
        .iter()
        .find(|report| report.approach_id == "baseline")
        .map(|report| report.avg_mrr)
        .unwrap_or(0.0);
    let simple_baseline_decisions = decisions_by_approach
        .get("baseline")
        .cloned()
        .unwrap_or_default();
    let no_meta_decisions = decisions_by_approach
        .get("no_metacognition")
        .cloned()
        .unwrap_or_default();

    for report in &mut approach_reports {
        let decisions = decisions_by_approach
            .get(&report.approach_id)
            .cloned()
            .unwrap_or_default();
        let compared = decisions.len().min(simple_baseline_decisions.len());
        let changed = decisions
            .iter()
            .zip(simple_baseline_decisions.iter())
            .take(compared)
            .filter(|(current, baseline)| current != baseline)
            .count();
        report.decision_divergence_rate = if compared == 0 {
            0.0
        } else {
            changed as f64 / compared as f64
        };
        report.ranking_delta_vs_baseline = report.avg_mrr - simple_baseline_mrr;

        let meta_compared = decisions.len().min(no_meta_decisions.len());
        let meta_changed = decisions
            .iter()
            .zip(no_meta_decisions.iter())
            .take(meta_compared)
            .filter(|(current, no_meta)| current != no_meta)
            .count();
        report.meta_influence_rate = if !matches!(
            report.approach_id.as_str(),
            "metacog_balanced" | "metacog_strict"
        ) || meta_compared == 0
        {
            0.0
        } else {
            meta_changed as f64 / meta_compared as f64
        };
    }

    let divergence_pairs = [
        ("baseline", "no_metacognition"),
        ("no_metacognition", "metacog_balanced"),
        ("no_metacognition", "metacog_strict"),
        ("metacog_balanced", "metacog_strict"),
    ];
    let decision_divergence: Vec<DecisionDivergenceSummary> = divergence_pairs
        .iter()
        .filter_map(|(reference_id, compared_id)| {
            let reference = decisions_by_approach.get(*reference_id)?;
            let compared = decisions_by_approach.get(*compared_id)?;
            let scenarios_compared = reference.len().min(compared.len());
            let changed_decisions = reference
                .iter()
                .zip(compared.iter())
                .take(scenarios_compared)
                .filter(|(left, right)| left != right)
                .count();
            Some(DecisionDivergenceSummary {
                reference_approach_id: (*reference_id).to_string(),
                compared_approach_id: (*compared_id).to_string(),
                changed_decisions,
                scenarios_compared,
                rate: if scenarios_compared == 0 {
                    0.0
                } else {
                    changed_decisions as f64 / scenarios_compared as f64
                },
            })
        })
        .collect();

    // Build cross-approach comparison table now that all approaches have run.
    let scenario_comparisons: Vec<ScenarioApproachComparison> = scenarios
        .iter()
        .map(|scenario| {
            let rankings = comparison_data.remove(&scenario.id).unwrap_or_default();
            build_scenario_comparison(scenario, &rankings)
        })
        .collect();

    let best_approach_id = approach_reports
        .iter()
        .max_by(|a, b| {
            a.pass_rate
                .partial_cmp(&b.pass_rate)
                .unwrap_or(Ordering::Equal)
        })
        .map(|report| report.approach_id.clone())
        .unwrap_or_else(|| "none".to_string());

    let calibration_candidates: Vec<&SimulationApproachResult> = approach_reports
        .iter()
        .filter(|report| {
            matches!(
                report.approach_id.as_str(),
                "no_metacognition" | "metacog_balanced" | "metacog_strict"
            )
        })
        .collect();
    let selected_for_calibration = calibration_candidates
        .iter()
        .copied()
        .max_by(|a, b| {
            a.train_pass_rate
                .partial_cmp(&b.train_pass_rate)
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    a.train_avg_mrr
                        .partial_cmp(&b.train_avg_mrr)
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| {
                    a.hit_at_3_rate
                        .partial_cmp(&b.hit_at_3_rate)
                        .unwrap_or(Ordering::Equal)
                })
        })
        .cloned();

    let train_pass_values: Vec<f64> = calibration_candidates
        .iter()
        .map(|report| report.train_pass_rate)
        .collect();
    let train_mrr_values: Vec<f64> = calibration_candidates
        .iter()
        .map(|report| report.train_avg_mrr)
        .collect();
    let metric_range = |values: &[f64]| -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let min = values.iter().copied().fold(f64::INFINITY, f64::min);
        let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        max - min
    };
    let max_meta_influence = calibration_candidates
        .iter()
        .map(|report| report.meta_influence_rate)
        .fold(0.0_f64, f64::max);
    let calibration_discriminative = metric_range(&train_pass_values) >= 0.02
        || (metric_range(&train_mrr_values) >= 0.02 && max_meta_influence >= 0.05);

    let calibration = if let Some(selected) = selected_for_calibration {
        let (selected_approach_id, status, rationale) = if calibration_discriminative {
            (
                selected.approach_id.clone(),
                "selected".to_string(),
                "As abordagens diferem nas métricas de treino; seleção por pass rate e MRR de treino.".to_string(),
            )
        } else {
            (
                "inconclusive".to_string(),
                "inconclusive".to_string(),
                "As abordagens de meta_strength empataram nas métricas de treino; não existe evidência discriminativa para recomendar um valor.".to_string(),
            )
        };
        SimulationCalibrationSummary {
            train_scenarios: train_split.clone(),
            test_scenarios: test_split.clone(),
            selection_metric: "train_pass_rate_then_train_avg_mrr_then_hit_at_3_rate".to_string(),
            selected_approach_id,
            selected_test_pass_rate: selected.test_pass_rate,
            selected_test_avg_mrr: selected.test_avg_mrr,
            status,
            rationale,
        }
    } else {
        SimulationCalibrationSummary {
            train_scenarios: vec![],
            test_scenarios: vec![],
            selection_metric: "none".to_string(),
            selected_approach_id: "none".to_string(),
            selected_test_pass_rate: 0.0,
            selected_test_avg_mrr: 0.0,
            status: "inconclusive".to_string(),
            rationale: "Não existem abordagens elegíveis para calibração.".to_string(),
        }
    };

    let mut evaluation_warnings = Vec::new();
    if approach_reports
        .iter()
        .any(|report| report.candidate_count_avg <= 3.0)
    {
        evaluation_warnings.push(
            "hit@3 pode estar inflacionado: pelo menos uma abordagem avaliou em média três ou menos candidatos."
                .to_string(),
        );
    }
    if !calibration_discriminative {
        evaluation_warnings.push(
            "Calibração não discriminativa: meta_strength não alterou pass rate/MRR de treino."
                .to_string(),
        );
    }

    Ok(SimulationReport {
        generated_at,
        scenarios_count: scenarios.len(),
        approaches: approach_reports,
        best_approach_id,
        calibration,
        scenario_comparisons,
        decision_divergence,
        evaluation_warnings,
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
        lambda.unwrap_or(config.ranking_lambda_resolved()),
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
        config.ranking_lambda_resolved(),
        config.theta,
        config.min_readiness,
        config.gamma,
        config.readiness_distance_delta,
        &config.ranking_policy,
        Some(config.soft_gate_k),
        config.root_penalty,
        config.stop_mastery,
        &exclude_set,
        domain_filter.as_deref(),
    );
    Ok(RankingDebugResponse {
        candidates,
        diagnostics,
    })
}

/// Caso determinístico usado na documentação académica do domínio Python.
/// Executa o mesmo pipeline de ingestão e ranking usado pela aplicação.
pub fn run_python_domain_demonstration() -> Result<Value, String> {
    let domain = "introducao-a-programacao-python";
    let concept_specs = vec![
        (
            "percurso-de-fundamentos-de-python",
            "Percurso de fundamentos de Python",
            "Visão geral e sequência recomendada da coletânea.",
        ),
        (
            "execucao-e-sintaxe-de-python",
            "Execução e sintaxe de Python",
            "Execução de programas, comentários, blocos e indentação.",
        ),
        (
            "variaveis-e-tipos-basicos-em-python",
            "Variáveis e tipos básicos em Python",
            "Representação, atribuição e conversão de valores básicos.",
        ),
        (
            "operadores-e-interacao-em-python",
            "Operadores e interação em Python",
            "Expressões aritméticas, comparações, lógica, entrada e saída.",
        ),
        (
            "condicionais-em-python",
            "Condicionais em Python",
            "Seleção de comportamento com if, elif e else.",
        ),
        (
            "colecoes-em-python",
            "Coleções em Python",
            "Organização de dados em listas, tuplos, dicionários e conjuntos.",
        ),
        (
            "ciclos-em-python",
            "Ciclos em Python",
            "Repetição controlada com for e while.",
        ),
        (
            "funcoes-em-python",
            "Funções em Python",
            "Encapsulamento e reutilização de comportamento.",
        ),
        (
            "modulos-em-python",
            "Módulos em Python",
            "Organização e reutilização de código entre ficheiros.",
        ),
        (
            "excecoes-em-python",
            "Exceções em Python",
            "Tratamento explícito de situações de erro.",
        ),
        (
            "ficheiros-em-python",
            "Ficheiros em Python",
            "Leitura e escrita persistente de dados.",
        ),
        (
            "classes-e-objetos-em-python",
            "Classes e objetos em Python",
            "Modelação simples de entidades, estado e comportamento.",
        ),
    ];
    let edge_specs = vec![
        (
            "execucao-e-sintaxe-de-python",
            "variaveis-e-tipos-basicos-em-python",
        ),
        (
            "variaveis-e-tipos-basicos-em-python",
            "operadores-e-interacao-em-python",
        ),
        ("operadores-e-interacao-em-python", "condicionais-em-python"),
        ("variaveis-e-tipos-basicos-em-python", "colecoes-em-python"),
        ("condicionais-em-python", "ciclos-em-python"),
        ("colecoes-em-python", "ciclos-em-python"),
        ("ciclos-em-python", "funcoes-em-python"),
        ("funcoes-em-python", "modulos-em-python"),
        ("funcoes-em-python", "excecoes-em-python"),
        ("condicionais-em-python", "excecoes-em-python"),
        ("excecoes-em-python", "ficheiros-em-python"),
        ("ciclos-em-python", "ficheiros-em-python"),
        ("funcoes-em-python", "classes-e-objetos-em-python"),
        ("colecoes-em-python", "classes-e-objetos-em-python"),
    ];
    let content_specs = vec![
        (
            "00-indice.md",
            "Python — fundamentos",
            "percurso-de-fundamentos-de-python",
            "leitura",
        ),
        (
            "01-primeiros-passos.md",
            "Primeiros passos",
            "execucao-e-sintaxe-de-python",
            "leitura, exercício",
        ),
        (
            "02-variaveis-e-tipos.md",
            "Variáveis e tipos básicos",
            "variaveis-e-tipos-basicos-em-python",
            "leitura, quiz, exercício",
        ),
        (
            "03-operadores-entrada-saida.md",
            "Operadores, entrada e saída",
            "operadores-e-interacao-em-python",
            "leitura, quiz, exercício",
        ),
        (
            "04-condicionais.md",
            "Decisões com condicionais",
            "condicionais-em-python",
            "leitura, quiz, exercício",
        ),
        (
            "05-colecoes.md",
            "Coleções",
            "colecoes-em-python",
            "leitura, quiz, exercício",
        ),
        (
            "06-ciclos.md",
            "Ciclos",
            "ciclos-em-python",
            "leitura, exercício",
        ),
        (
            "07-funcoes.md",
            "Funções",
            "funcoes-em-python",
            "leitura, exercício",
        ),
        (
            "08-modulos.md",
            "Módulos e importações",
            "modulos-em-python",
            "leitura, exercício",
        ),
        (
            "09-erros-e-excecoes.md",
            "Erros e exceções",
            "excecoes-em-python",
            "leitura, quiz, exercício",
        ),
        (
            "10-ficheiros.md",
            "Leitura e escrita de ficheiros",
            "ficheiros-em-python",
            "leitura, exercício",
        ),
        (
            "11-classes-e-objetos.md",
            "Classes e objetos",
            "classes-e-objetos-em-python",
            "leitura, exercício",
        ),
    ];

    let full_id = |slug: &str| format!("{}.inline.{}", domain, slug);
    let mut store = OlmStore::default();
    store.config.ranking_policy = "adaptive".to_string();
    store.config.decay_enabled = false;

    for (slug, name, description) in &concept_specs {
        let id = full_id(slug);
        store.concepts.insert(
            id.clone(),
            Concept {
                id,
                name: (*name).to_string(),
                description: Some((*description).to_string()),
            },
        );
    }
    for (prereq, target) in &edge_specs {
        store.edges.push(ConceptEdge {
            prereq_id: full_id(prereq),
            target_id: full_id(target),
        });
    }
    for (file, title, concept_slug, _) in &content_specs {
        let content_id = format!("Introdução à Programação Python/{}", file);
        store.content_items.insert(
            content_id.clone(),
            ContentItem {
                id: content_id.clone(),
                item_type: "note".to_string(),
                title: (*title).to_string(),
                domain_id: Some(domain.to_string()),
            },
        );
        store.content_concepts.push(ContentConceptMap {
            content_id,
            concept_id: full_id(concept_slug),
            coverage_weight: 1.0,
        });
    }

    let initial_states = vec![
        ("percurso-de-fundamentos-de-python", 9.0, 1.0),
        ("execucao-e-sintaxe-de-python", 9.0, 2.0),
        ("variaveis-e-tipos-basicos-em-python", 8.0, 2.0),
        ("operadores-e-interacao-em-python", 7.0, 3.0),
        ("condicionais-em-python", 2.5, 3.5),
        ("colecoes-em-python", 4.0, 3.0),
        ("ciclos-em-python", 2.0, 3.0),
        ("funcoes-em-python", 1.5, 3.5),
        ("modulos-em-python", 1.0, 2.0),
        ("excecoes-em-python", 1.2, 2.8),
        ("ficheiros-em-python", 1.0, 2.0),
        ("classes-e-objetos-em-python", 1.0, 2.5),
    ];
    for (slug, positive_evidence, difficulty_evidence) in initial_states {
        store.concept_state.insert(
            full_id(slug),
            ConceptState {
                alpha: positive_evidence,
                beta: difficulty_evidence,
                last_update: Some("2026-06-22T09:00:00Z".to_string()),
            },
        );
    }

    let target_id = full_id("condicionais-em-python");
    let before = store
        .concept_state
        .get(&target_id)
        .cloned()
        .ok_or_else(|| "Estado inicial do conceito não encontrado".to_string())?;
    let event = StudyEvent {
        event_id: "demo-practice-conditionals-001".to_string(),
        timestamp: "2026-06-22T10:00:00Z".to_string(),
        source: "demo_academica".to_string(),
        event_type: "practice_attempt".to_string(),
        content_id: Some("Introdução à Programação Python/04-condicionais.md".to_string()),
        concept_ids: vec![target_id.clone()],
        payload: json!({ "correct": 4.0, "total": 5.0, "confidence": 0.75, "duration_sec": 420.0 }),
    };
    let raw_event_score = event_score(&event);
    let meta_strength = store.config.meta_strength;
    ingest_into_store(&mut store, &event, meta_strength)?;
    let after = store
        .concept_state
        .get(&target_id)
        .cloned()
        .ok_or_else(|| "Estado atualizado do conceito não encontrado".to_string())?;
    let evidence = store
        .evidence
        .get(&target_id)
        .and_then(|rows| rows.last())
        .cloned()
        .ok_or_else(|| "Evidência da simulação não encontrada".to_string())?;

    let exclude = HashSet::new();
    let (ranking, _) = rank_next_to_study(
        &store,
        Some(6),
        store.config.ranking_lambda_resolved(),
        store.config.theta,
        store.config.min_readiness,
        store.config.gamma,
        store.config.readiness_distance_delta,
        &store.config.ranking_policy,
        Some(store.config.soft_gate_k),
        store.config.root_penalty,
        store.config.stop_mastery,
        &exclude,
        Some("introducao-a-programacao-python."),
    );

    let concept_rows: Vec<Value> = concept_specs
        .iter()
        .map(|(slug, name, description)| {
            let prerequisites: Vec<String> = edge_specs
                .iter()
                .filter(|(_, target)| target == slug)
                .filter_map(|(prereq, _)| {
                    concept_specs
                        .iter()
                        .find(|(candidate, _, _)| candidate == prereq)
                        .map(|(_, prereq_name, _)| (*prereq_name).to_string())
                })
                .collect();
            let contents: Vec<String> = content_specs
                .iter()
                .filter(|(_, _, concept, _)| concept == slug)
                .map(|(file, _, _, _)| (*file).to_string())
                .collect();
            json!({
                "id": full_id(slug), "name": name, "description": description,
                "prerequisites": prerequisites, "contents": contents,
            })
        })
        .collect();

    let content_rows: Vec<Value> = content_specs
        .iter()
        .map(|(file, title, concept_slug, evidence_types)| {
            let concept_name = concept_specs
                .iter()
                .find(|(slug, _, _)| slug == concept_slug)
                .map(|(_, name, _)| *name)
                .unwrap_or(*concept_slug);
            json!({
                "file": file, "title": title, "concepts": [concept_name],
                "content_type": "Nota Markdown interativa", "possible_evidence": evidence_types,
            })
        })
        .collect();

    let ranking_rows: Vec<Value> = ranking.iter().enumerate().map(|(index, candidate)| {
        let penalty = 1.0 - clamp_01(candidate.gate_factor * candidate.score_decomposition.root_multiplier);
        json!({
            "rank": index + 1, "concept_id": candidate.concept_id, "name": candidate.name,
            "structural_readiness": candidate.readiness, "current_mastery": candidate.mastery,
            "uncertainty": candidate.uncertainty, "penalty": penalty,
            "final_score": candidate.score,
            "decision": if index == 0 { "Recomendar como próxima opção" } else { "Alternativa" },
            "why": candidate.why,
        })
    }).collect();

    Ok(json!({
        "simulation": {
            "title": "Atualização de Condicionais em Python após tentativa prática",
            "deterministic": true, "timestamp": event.timestamp, "event": event,
            "raw_event_score": raw_event_score,
            "event_type_base_weight": event_type_weight("practice_attempt"),
        },
        "concepts": concept_rows,
        "contents": content_rows,
        "update": {
            "concept_id": target_id, "concept_name": "Condicionais em Python",
            "positive_evidence_before": before.alpha, "difficulty_evidence_before": before.beta,
            "mastery_before": mastery(&before), "uncertainty_before": uncertainty(&before),
            "score": evidence.score, "applied_weight": evidence.applied_weight,
            "positive_evidence_delta": evidence.delta_alpha,
            "difficulty_evidence_delta": evidence.delta_beta,
            "positive_evidence_after": after.alpha, "difficulty_evidence_after": after.beta,
            "mastery_after": mastery(&after), "uncertainty_after": uncertainty(&after),
            "metacognitive_weight": evidence.metacognitive_weight,
            "metacognitive_alignment": evidence.metacognitive_alignment,
            "effect": "O desempenho de 4/5 produz evidência favorável ao domínio, mas o limitador inicial mantém a atualização conservadora; uma única tentativa não consolida o conceito.",
        },
        "recommendation_ranking": ranking_rows,
        "formulas": {
            "mastery": "evidência favorável / (evidência favorável + evidência de dificuldade)",
            "uncertainty": "αβ / ((α+β)^2(α+β+1))",
            "event_score": "respostas corretas / total de respostas",
            "state_update": "α' = α + peso×score; β' = β + peso×(1-score)",
        }
    }))
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_domain_demonstration_updates_state_and_ranks_conditionals_first() {
        let report = run_python_domain_demonstration().expect("demo should run");
        let update = &report["update"];
        assert!(
            update["positive_evidence_after"].as_f64().unwrap()
                > update["positive_evidence_before"].as_f64().unwrap()
        );
        assert!(
            update["mastery_after"].as_f64().unwrap() > update["mastery_before"].as_f64().unwrap()
        );
        assert!(
            update["uncertainty_after"].as_f64().unwrap()
                < update["uncertainty_before"].as_f64().unwrap()
        );
        assert_eq!(
            report["recommendation_ranking"][0]["name"].as_str(),
            Some("Condicionais em Python")
        );
    }

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
        // 0.5 + 0.10 * (150/300) = 0.55
        assert!((event_score(&e) - 0.55).abs() < 1e-9);
    }

    #[test]
    fn test_event_score_review() {
        let e = make_event(
            "review",
            json!({"duration_sec": 300.0, "target_duration_sec": 600.0}),
        );
        // 0.5 + 0.15 * (300/600) = 0.575
        assert!((event_score(&e) - 0.575).abs() < 1e-9);
    }

    #[test]
    fn test_event_score_note_taking() {
        let e = make_event(
            "note_taking",
            json!({"chars_written": 150.0, "target_chars": 300.0}),
        );
        // 0.5 + 0.15 * (150/300) = 0.575
        assert!((event_score(&e) - 0.575).abs() < 1e-9);
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
        // 1 - lambda * |confidence-objective| = 1 - 0.7 * |0.8-1.0| = 0.86
        assert!((confidence_weight(&e, Some(1.0), 0.7) - 0.86).abs() < 1e-9);
    }

    #[test]
    fn test_confidence_weight_absent() {
        let e = make_event("quiz_attempt", json!({}));
        assert_eq!(confidence_weight(&e, Some(1.0), 0.7), 1.0);
    }

    #[test]
    fn test_mastery_uncertainty_basic() {
        let s = make_state_cs(3.0, 1.0);
        assert!((mastery(&s) - 0.75).abs() < 1e-9);
        assert!((uncertainty(&s) - 0.45).abs() < 1e-9);
    }

    #[test]
    fn test_mastery_initial() {
        let s = make_state_cs(1.0, 1.0);
        assert!((mastery(&s) - 0.5).abs() < 1e-9);
        // Beta(1,1) is the uniform prior: maximum uncertainty, normalized to 1.0.
        assert!((uncertainty(&s) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_uncertainty_is_canonical_beta_variance() {
        let s = make_state_cs(3.0, 1.0); // total = 4
        assert!((uncertainty(&s) - 0.45).abs() < 1e-9);
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
            0.7,
            "balanced",
            Some(1.6),
            0.0,
            0.99,
            &exclude,
            None,
        );
        // With penalty=0.5
        let (ranked_penalty, _) = rank_next_to_study(
            &store,
            None,
            0.7,
            0.5,
            0.1,
            0.35,
            0.7,
            "balanced",
            Some(1.6),
            0.5,
            0.99,
            &exclude,
            None,
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
            0.7,
            "balanced",
            Some(1.6),
            0.12,
            0.7,
            &exclude,
            None,
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
            0.7,
            "balanced",
            Some(1.6),
            0.12,
            0.7,
            &exclude,
            None,
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
            0.7,
            "balanced",
            Some(1.6),
            0.12,
            0.99,
            &exclude,
            None,
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
            0.7,
            "balanced",
            Some(1.6),
            0.12,
            0.99,
            &exclude,
            Some("math"),
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
            0.7,
            "balanced",
            Some(2.0),
            0.0,
            0.99,
            &exclude,
            None,
        );
        // k=3 strict gate
        let (ranked_strict, _) = rank_next_to_study(
            &store,
            None,
            0.7,
            0.5,
            0.1,
            0.35,
            0.7,
            "balanced",
            Some(3.0),
            0.0,
            0.99,
            &exclude,
            None,
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
    fn test_multi_hop_readiness_uses_ancestors() {
        let mut store = OlmStore::default();
        for id in ["a", "b", "c"] {
            store.concepts.insert(
                id.to_string(),
                Concept {
                    id: id.to_string(),
                    name: id.to_uppercase(),
                    description: None,
                },
            );
        }

        // a -> b -> c, so c must account for ancestor a (distance 2) as well.
        store.edges.push(ConceptEdge {
            prereq_id: "a".to_string(),
            target_id: "b".to_string(),
        });
        store.edges.push(ConceptEdge {
            prereq_id: "b".to_string(),
            target_id: "c".to_string(),
        });

        // Ancestor a is unready (very low mastery), b is fully ready (rp ~ 1).
        store.concept_state.insert(
            "a".to_string(),
            ConceptState {
                alpha: 1.0,
                beta: 40.0,
                last_update: None,
            },
        );
        store.concept_state.insert(
            "b".to_string(),
            ConceptState {
                alpha: 10.0,
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
            0.2,
            0.35,
            0.85,
            "balanced",
            Some(1.6),
            0.0,
            0.99,
            &exclude,
            None,
        );

        assert!(
            ranked.iter().all(|row| row.concept_id != "c"),
            "c should be excluded by low multi-hop readiness due to ancestor a"
        );
    }

    #[test]
    fn test_adaptive_policy_prefers_review_for_mastered_concept() {
        let mut store = OlmStore::default();
        store.concepts.insert(
            "stable".to_string(),
            Concept {
                id: "stable".to_string(),
                name: "Stable".to_string(),
                description: None,
            },
        );

        store.concept_state.insert(
            "stable".to_string(),
            ConceptState {
                alpha: 12.0,
                beta: 1.0,
                last_update: None,
            },
        );

        // Contradictory recent evidence should raise inconsistency and make review intent visible.
        store.evidence.insert(
            "stable".to_string(),
            vec![
                EvidenceChunk {
                    event_id: "e1".to_string(),
                    delta_alpha: 0.1,
                    delta_beta: 0.0,
                    event_type: "quiz_attempt".to_string(),
                    score: 1.0,
                    applied_weight: 0.1,
                    metacognitive_weight: 1.0,
                    metacognitive_alignment: 1.0,
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                },
                EvidenceChunk {
                    event_id: "e2".to_string(),
                    delta_alpha: 0.0,
                    delta_beta: 0.1,
                    event_type: "quiz_attempt".to_string(),
                    score: 0.0,
                    applied_weight: 0.1,
                    metacognitive_weight: 1.0,
                    metacognitive_alignment: 1.0,
                    created_at: "2026-01-01T00:01:00Z".to_string(),
                },
            ],
        );

        let exclude = HashSet::new();
        let (ranked, _) = rank_next_to_study(
            &store,
            None,
            0.7,
            0.5,
            0.1,
            0.35,
            0.85,
            "adaptive",
            Some(1.6),
            0.0,
            0.99,
            &exclude,
            None,
        );

        let stable = ranked
            .iter()
            .find(|row| row.concept_id == "stable")
            .unwrap();
        assert!(
            stable.why.iter().any(|line| line.contains("adaptive")),
            "adaptive policy diagnostics should be present"
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
        assert!(cfg.ranking_lambda.is_none());
        assert!(cfg.confidence_mismatch_lambda.is_none());
        assert!((cfg.ranking_lambda_resolved() - 0.7).abs() < 1e-9);
        assert!((cfg.confidence_mismatch_lambda_resolved() - 0.7).abs() < 1e-9);
        assert!((cfg.gamma - 0.35).abs() < 1e-9);
        assert!((cfg.readiness_blend_eta - 0.7).abs() < 1e-9);
        assert!((cfg.readiness_distance_delta - 0.85).abs() < 1e-9);
        assert_eq!(cfg.ranking_policy, "adaptive");
        assert!((cfg.stop_mastery - 0.85).abs() < 1e-9);
        assert!(!cfg.decay_enabled);
        assert!(cfg.exclude_concepts.is_empty());
    }

    #[test]
    fn test_split_lambda_resolution_prefers_specific_values() {
        let mut cfg = OlmConfig::default();
        cfg.lambda = 0.4;

        // Backward-compatible fallback to legacy shared lambda.
        assert!((cfg.ranking_lambda_resolved() - 0.4).abs() < 1e-9);
        assert!((cfg.confidence_mismatch_lambda_resolved() - 0.4).abs() < 1e-9);

        cfg.ranking_lambda = Some(0.8);
        cfg.confidence_mismatch_lambda = Some(0.2);

        assert!((cfg.ranking_lambda_resolved() - 0.8).abs() < 1e-9);
        assert!((cfg.confidence_mismatch_lambda_resolved() - 0.2).abs() < 1e-9);
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

    #[test]
    fn test_build_simulation_scenarios_generated_mode_only_returns_generated() {
        let options = SimulationOptions {
            generated_scenarios: 3,
            scenario_mode: "generated".to_string(),
            ..SimulationOptions::default()
        };

        let scenarios = build_simulation_scenarios(11, &options);
        assert_eq!(scenarios.len(), 3);
        assert!(scenarios
            .iter()
            .all(|scenario| scenario.metadata.mode == "generated"));
    }

    #[test]
    fn test_build_simulation_scenarios_canonical_mode_includes_hard_cases() {
        let options = SimulationOptions {
            generated_scenarios: 10,
            scenario_mode: "canonical".to_string(),
            ..SimulationOptions::default()
        };

        let scenarios = build_simulation_scenarios(7, &options);
        assert_eq!(scenarios.len(), 18);
        assert_eq!(
            scenarios
                .iter()
                .filter(|scenario| scenario.metadata.mode == "canonical")
                .count(),
            12
        );
        assert_eq!(
            scenarios
                .iter()
                .filter(|scenario| scenario.metadata.mode == "hard_case")
                .count(),
            6
        );
        assert!(scenarios
            .iter()
            .filter(|scenario| scenario.metadata.mode == "hard_case")
            .all(|scenario| scenario.metadata.difficulty == "hard"
                && scenario
                    .metadata
                    .case_tags
                    .contains(&"multi_candidate".to_string())));
    }

    #[test]
    fn test_calibration_split_is_disjoint_and_keeps_all_scenarios() {
        let options = SimulationOptions {
            generated_scenarios: 8,
            scenario_mode: "mixed".to_string(),
            ..SimulationOptions::default()
        };
        let scenarios = build_simulation_scenarios(19, &options);
        let (train, test) = build_calibration_split(&scenarios);
        let train_set: HashSet<&str> = train.iter().map(String::as_str).collect();
        let test_set: HashSet<&str> = test.iter().map(String::as_str).collect();

        assert!(train_set.is_disjoint(&test_set));
        assert_eq!(train.len() + test.len(), scenarios.len());
        assert!(train_set.contains("H1"));
        assert!(test_set.contains("H4"));
        assert!(train
            .iter()
            .chain(test.iter())
            .any(|id| id.starts_with('G')));
    }

    #[test]
    fn test_meta_calibration_is_inconclusive_without_material_separation() {
        let report = olm_run_synthetic_scenarios_seeded(0).expect("simulation should run");
        assert_eq!(report.calibration.status, "inconclusive");
        assert_eq!(report.calibration.selected_approach_id, "inconclusive");
        assert!(report
            .decision_divergence
            .iter()
            .any(
                |row| row.reference_approach_id == "no_metacognition" && row.changed_decisions > 0
            ));
    }

    #[test]
    fn test_remove_content_concept_maps_retain_logic() {
        let mut store = OlmStore::default();
        store.content_concepts.push(ContentConceptMap {
            content_id: "doc/a.md".to_string(),
            concept_id: "c1".to_string(),
            coverage_weight: 1.0,
        });
        store.content_concepts.push(ContentConceptMap {
            content_id: "doc/a.md".to_string(),
            concept_id: "c2".to_string(),
            coverage_weight: 1.0,
        });
        store.content_concepts.push(ContentConceptMap {
            content_id: "doc/b.md".to_string(),
            concept_id: "c3".to_string(),
            coverage_weight: 1.0,
        });

        store
            .content_concepts
            .retain(|m| m.content_id != "doc/a.md");

        assert_eq!(store.content_concepts.len(), 1);
        assert_eq!(store.content_concepts[0].content_id, "doc/b.md");
    }

    #[test]
    fn test_ingest_uses_mapping_quality_penalty_for_content_only_mapping() {
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
        store.content_items.insert(
            "doc/a.md".to_string(),
            ContentItem {
                id: "doc/a.md".to_string(),
                item_type: "note".to_string(),
                title: "A".to_string(),
                domain_id: Some("root".to_string()),
            },
        );
        store.content_concepts.push(ContentConceptMap {
            content_id: "doc/a.md".to_string(),
            concept_id: "c1".to_string(),
            coverage_weight: 0.95,
        });
        store.content_concepts.push(ContentConceptMap {
            content_id: "doc/a.md".to_string(),
            concept_id: "c2".to_string(),
            coverage_weight: 0.05,
        });

        let event = StudyEvent {
            event_id: "evt-quality-1".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            source: "test".to_string(),
            event_type: "quiz_attempt".to_string(),
            content_id: Some("doc/a.md".to_string()),
            concept_ids: vec![],
            payload: json!({"correct": 1.0, "total": 1.0, "confidence": 1.0}),
        };

        ingest_into_store(&mut store, &event, 0.0).unwrap();

        let applied_c1 = store
            .evidence
            .get("c1")
            .and_then(|rows| rows.last())
            .map(|row| row.applied_weight)
            .unwrap_or(0.0);
        let applied_c2 = store
            .evidence
            .get("c2")
            .and_then(|rows| rows.last())
            .map(|row| row.applied_weight)
            .unwrap_or(0.0);

        let quality_penalty = content_mapping_quality(1.0, 2).penalty;
        // With meta=0 and confidence=1, only safety gate and mapping quality affect weight.
        // The dominant concept should reflect the quality penalty factor.
        assert!(applied_c1 > 0.0);
        assert!(applied_c2 > 0.0);
        assert!(quality_penalty < 1.0);
        assert!(
            applied_c1 < 1.0,
            "quality penalty and safety should bound applied weight"
        );
    }

    #[test]
    fn test_ingest_explicit_concepts_bypass_mapping_quality_penalty() {
        let mut store = OlmStore::default();
        store.concepts.insert(
            "c1".to_string(),
            Concept {
                id: "c1".to_string(),
                name: "C1".to_string(),
                description: None,
            },
        );

        let event = StudyEvent {
            event_id: "evt-explicit-1".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            source: "test".to_string(),
            event_type: "quiz_attempt".to_string(),
            content_id: Some("doc/a.md".to_string()),
            concept_ids: vec!["c1".to_string()],
            payload: json!({"correct": 1.0, "total": 1.0, "confidence": 1.0}),
        };

        ingest_into_store(&mut store, &event, 0.0).unwrap();

        let applied = store
            .evidence
            .get("c1")
            .and_then(|rows| rows.last())
            .map(|row| row.applied_weight)
            .unwrap_or(0.0);
        assert!(applied > 0.0);
    }

    // ── Fix 3: olm_remove_content_concept_maps ──────────────────────────────

    #[test]
    fn test_remove_content_concept_maps_removes_only_target_content() {
        let mut store = OlmStore::default();
        store.content_concepts.push(ContentConceptMap {
            content_id: "file-a".to_string(),
            concept_id: "c1".to_string(),
            coverage_weight: 1.0,
        });
        store.content_concepts.push(ContentConceptMap {
            content_id: "file-a".to_string(),
            concept_id: "c2".to_string(),
            coverage_weight: 0.5,
        });
        store.content_concepts.push(ContentConceptMap {
            content_id: "file-b".to_string(),
            concept_id: "c1".to_string(),
            coverage_weight: 0.8,
        });

        let before = store.content_concepts.len();
        store.content_concepts.retain(|m| m.content_id != "file-a");
        let removed = before - store.content_concepts.len();

        assert_eq!(removed, 2, "should remove both file-a entries");
        assert_eq!(
            store.content_concepts.len(),
            1,
            "file-b entry should remain"
        );
        assert_eq!(store.content_concepts[0].content_id, "file-b");
    }

    #[test]
    fn test_remove_content_concept_maps_noop_on_unknown_content() {
        let mut store = OlmStore::default();
        store.content_concepts.push(ContentConceptMap {
            content_id: "file-a".to_string(),
            concept_id: "c1".to_string(),
            coverage_weight: 1.0,
        });
        let before = store.content_concepts.len();
        store
            .content_concepts
            .retain(|m| m.content_id != "unknown-file");
        assert_eq!(
            store.content_concepts.len(),
            before,
            "nothing removed for unknown content_id"
        );
    }

    // ── Fix 4: mapping quality penalty on ingest ─────────────────────────────

    #[test]
    fn test_ingest_with_content_id_applies_quality_penalty() {
        let build_store = || {
            let mut store = OlmStore::default();
            store.concepts.insert(
                "c1".to_string(),
                Concept {
                    id: "c1".to_string(),
                    name: "C1".to_string(),
                    description: None,
                },
            );
            store.content_items.insert(
                "file-a".to_string(),
                ContentItem {
                    id: "file-a".to_string(),
                    item_type: "note".to_string(),
                    title: "File A".to_string(),
                    domain_id: None,
                },
            );
            // Single-concept mapping -> raw_weight_sum = 1.0, breadth = 0.6 -> quality = 0.88 -> penalty = 0.946
            store.content_concepts.push(ContentConceptMap {
                content_id: "file-a".to_string(),
                concept_id: "c1".to_string(),
                coverage_weight: 1.0,
            });
            store
        };

        // Event via explicit concept_ids (Tier 1) — no quality penalty applied
        let event_tier1 = StudyEvent {
            event_id: "tier1-1".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            source: "test".to_string(),
            event_type: "review".to_string(),
            content_id: Some("file-a".to_string()),
            concept_ids: vec!["c1".to_string()],
            payload: json!({}),
        };

        // Event via content_id only (Tier 2) — quality penalty should reduce applied_weight
        let event_tier2 = StudyEvent {
            event_id: "tier2-1".to_string(),
            timestamp: "2026-01-01T00:00:01Z".to_string(),
            source: "test".to_string(),
            event_type: "review".to_string(),
            content_id: Some("file-a".to_string()),
            concept_ids: vec![],
            payload: json!({}),
        };

        let mut store1 = build_store();
        let mut store2 = build_store();
        ingest_into_store(&mut store1, &event_tier1, 0.0).unwrap();
        ingest_into_store(&mut store2, &event_tier2, 0.0).unwrap();

        let evidence1 = store1.evidence.get("c1").unwrap();
        let evidence2 = store2.evidence.get("c1").unwrap();

        assert!(
            evidence2[0].applied_weight < evidence1[0].applied_weight,
            "Tier 2 (content_id mapping) should have lower applied_weight than Tier 1 (explicit concept_ids) due to quality penalty. tier2={:.4} tier1={:.4}",
            evidence2[0].applied_weight,
            evidence1[0].applied_weight,
        );
    }

    #[test]
    fn test_quality_penalty_breadth_increases_with_more_concepts() {
        // Verify the existing content_mapping_quality breadth logic:
        // 1 concept → penalty lower than 3 concepts
        let q1 = content_mapping_quality(1.0, 1);
        let q3 = content_mapping_quality(1.0, 3);
        assert!(
            q3.penalty > q1.penalty,
            "more concepts should yield higher quality penalty (less penalised): q1={:.3} q3={:.3}",
            q1.penalty,
            q3.penalty
        );
    }

    // ── Fix 1: coverage_weight passthrough ───────────────────────────────────

    #[test]
    fn test_custom_coverage_weight_stored_and_normalised() {
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
        store.content_items.insert(
            "note".to_string(),
            ContentItem {
                id: "note".to_string(),
                item_type: "note".to_string(),
                title: "Note".to_string(),
                domain_id: None,
            },
        );
        store.content_concepts.push(ContentConceptMap {
            content_id: "note".to_string(),
            concept_id: "c1".to_string(),
            coverage_weight: 0.8,
        });
        store.content_concepts.push(ContentConceptMap {
            content_id: "note".to_string(),
            concept_id: "c2".to_string(),
            coverage_weight: 0.2,
        });

        let maps = normalize_maps(
            &store,
            &StudyEvent {
                event_id: "e1".to_string(),
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                source: "test".to_string(),
                event_type: "review".to_string(),
                content_id: Some("note".to_string()),
                concept_ids: vec![],
                payload: json!({}),
            },
        );

        let c1_weight = maps
            .iter()
            .find(|(id, _)| id == "c1")
            .map(|(_, w)| *w)
            .unwrap();
        let c2_weight = maps
            .iter()
            .find(|(id, _)| id == "c2")
            .map(|(_, w)| *w)
            .unwrap();

        assert!(
            (c1_weight - 0.8).abs() < 1e-9,
            "c1 weight should be 0.8 after normalisation, got {}",
            c1_weight
        );
        assert!(
            (c2_weight - 0.2).abs() < 1e-9,
            "c2 weight should be 0.2 after normalisation, got {}",
            c2_weight
        );
    }
}
