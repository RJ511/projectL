#[path = "../commands/olm.rs"]
#[allow(dead_code)]
mod olm;

use serde::Serialize;
use std::cmp::Ordering;
use std::collections::HashSet;

#[derive(Debug, Clone)]
struct SearchSpace {
    lambda: Vec<f64>,
    gamma: Vec<f64>,
    theta: Vec<f64>,
    meta_strength: Vec<f64>,
    soft_gate_k: Vec<f64>,
    root_penalty: Vec<f64>,
}

impl Default for SearchSpace {
    fn default() -> Self {
        Self {
            lambda: vec![0.65, 0.7],
            gamma: vec![0.35],
            theta: vec![0.5, 0.55],
            meta_strength: vec![0.0, 0.6, 1.0],
            soft_gate_k: vec![1.6, 1.8],
            root_penalty: vec![0.12],
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct TunedConfig {
    config_id: String,
    lambda: f64,
    gamma: f64,
    theta: f64,
    meta_strength: f64,
    soft_gate_k: f64,
    root_penalty: f64,
}

#[derive(Debug, Clone, Serialize)]
struct ConfigSeedMetrics {
    config_id: String,
    seed: u64,
    pass_rate: f64,
    hit_at_3: f64,
    mrr: f64,
    ndcg_at_3: f64,
}

#[derive(Debug, Clone, Serialize)]
struct ConfigAggregate {
    config: TunedConfig,
    seeds: Vec<u64>,
    pass_rate_avg: f64,
    hit_at_3_avg: f64,
    mrr_avg: f64,
    ndcg_at_3_avg: f64,
    pass_rate_stability: f64,
    hit_at_3_stability: f64,
    mrr_stability: f64,
    ndcg_at_3_stability: f64,
    composed_score: f64,
}

#[derive(Debug, Clone, Serialize)]
struct FailureCase {
    config_id: String,
    seed: u64,
    scenario_id: String,
    scenario_name: String,
    top_1: Option<String>,
    expected_any: Vec<String>,
    top_3: Vec<String>,
    rank_of_first_expected: Option<usize>,
    notes: String,
}

#[derive(Debug, Serialize)]
struct SearchOutput {
    seeds: Vec<u64>,
    mode: String,
    total_configs: usize,
    ranking: Vec<ConfigAggregate>,
    metrics_table: Vec<ConfigSeedMetrics>,
    failure_cases: Vec<FailureCase>,
}

fn parse_f64_range(spec: &str) -> Result<Vec<f64>, String> {
    if spec.contains(':') {
        let parts: Vec<&str> = spec.split(':').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid range '{}'. Use start:end:step", spec));
        }
        let start: f64 = parts[0]
            .parse()
            .map_err(|_| format!("Invalid start in '{}'", spec))?;
        let end: f64 = parts[1]
            .parse()
            .map_err(|_| format!("Invalid end in '{}'", spec))?;
        let step: f64 = parts[2]
            .parse()
            .map_err(|_| format!("Invalid step in '{}'", spec))?;
        if step <= 0.0 {
            return Err(format!("Step must be > 0 in '{}'", spec));
        }
        let mut out = Vec::new();
        let mut value = start;
        while value <= end + 1e-9 {
            out.push((value * 1_000_000.0).round() / 1_000_000.0);
            value += step;
        }
        return Ok(out);
    }

    let mut out = Vec::new();
    for item in spec.split(',').filter(|x| !x.trim().is_empty()) {
        out.push(
            item.trim()
                .parse()
                .map_err(|_| format!("Invalid number '{}'.", item))?,
        );
    }
    if out.is_empty() {
        return Err("Empty range/list".to_string());
    }
    Ok(out)
}

fn average(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn std_dev(values: &[f64]) -> f64 {
    if values.len() <= 1 {
        return 0.0;
    }
    let avg = average(values);
    let variance = values.iter().map(|v| (v - avg).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut output_path: Option<String> = None;
    let mut seeds: Vec<u64> = vec![0];
    let mut space = SearchSpace::default();
    let mut random_search_count: Option<usize> = None;

    let mut index = 1usize;
    while index < args.len() {
        match args[index].as_str() {
            "--save" => {
                output_path = Some("results.json".to_string());
                index += 1;
            }
            "--out" | "-o" => {
                if index + 1 >= args.len() {
                    eprintln!("Missing file path after {}", args[index]);
                    std::process::exit(1);
                }
                output_path = Some(args[index + 1].clone());
                index += 2;
            }
            "--seeds" => {
                if index + 1 >= args.len() {
                    eprintln!("Missing count after --seeds");
                    std::process::exit(1);
                }
                let count: usize = args[index + 1].parse().unwrap_or(1);
                seeds = (0..count as u64).collect();
                index += 2;
            }
            "--seed-range" => {
                if index + 2 >= args.len() {
                    eprintln!("Missing start/end after --seed-range");
                    std::process::exit(1);
                }
                let start: u64 = args[index + 1].parse().unwrap_or(0);
                let end: u64 = args[index + 2].parse().unwrap_or(1);
                seeds = (start..=end).collect();
                index += 3;
            }
            "--lambda" => {
                space.lambda = parse_f64_range(args.get(index + 1).unwrap_or(&"".to_string()))
                    .unwrap_or_else(|e| {
                        eprintln!("--lambda: {}", e);
                        std::process::exit(1)
                    });
                index += 2;
            }
            "--gamma" => {
                space.gamma = parse_f64_range(args.get(index + 1).unwrap_or(&"".to_string()))
                    .unwrap_or_else(|e| {
                        eprintln!("--gamma: {}", e);
                        std::process::exit(1)
                    });
                index += 2;
            }
            "--theta" => {
                space.theta = parse_f64_range(args.get(index + 1).unwrap_or(&"".to_string()))
                    .unwrap_or_else(|e| {
                        eprintln!("--theta: {}", e);
                        std::process::exit(1)
                    });
                index += 2;
            }
            "--meta-strength" => {
                space.meta_strength = parse_f64_range(
                    args.get(index + 1).unwrap_or(&"".to_string()),
                )
                .unwrap_or_else(|e| {
                    eprintln!("--meta-strength: {}", e);
                    std::process::exit(1)
                });
                index += 2;
            }
            "--soft-gate-k" => {
                space.soft_gate_k = parse_f64_range(args.get(index + 1).unwrap_or(&"".to_string()))
                    .unwrap_or_else(|e| {
                        eprintln!("--soft-gate-k: {}", e);
                        std::process::exit(1)
                    });
                index += 2;
            }
            "--root-penalty" => {
                space.root_penalty = parse_f64_range(
                    args.get(index + 1).unwrap_or(&"".to_string()),
                )
                .unwrap_or_else(|e| {
                    eprintln!("--root-penalty: {}", e);
                    std::process::exit(1)
                });
                index += 2;
            }
            "--random-search" => {
                if index + 1 >= args.len() {
                    eprintln!("Missing count after --random-search");
                    std::process::exit(1);
                }
                random_search_count = Some(args[index + 1].parse().unwrap_or(10));
                index += 2;
            }
            _ => index += 1,
        }
    }

    let mut candidates = Vec::new();
    for &lambda in &space.lambda {
        for &gamma in &space.gamma {
            for &theta in &space.theta {
                for &meta_strength in &space.meta_strength {
                    for &soft_gate_k in &space.soft_gate_k {
                        for &root_penalty in &space.root_penalty {
                            candidates.push(TunedConfig {
                                config_id: format!(
                                    "l{:.3}_g{:.3}_t{:.3}_m{:.3}_k{:.3}_r{:.3}",
                                    lambda, gamma, theta, meta_strength, soft_gate_k, root_penalty
                                ),
                                lambda,
                                gamma,
                                theta,
                                meta_strength,
                                soft_gate_k,
                                root_penalty,
                            });
                        }
                    }
                }
            }
        }
    }

    let mode = if let Some(limit) = random_search_count {
        let mut selected = Vec::new();
        let mut seen = HashSet::new();
        let mut rng = rand::thread_rng();
        use rand::seq::SliceRandom;
        while selected.len() < limit.min(candidates.len()) {
            if let Some(choice) = candidates.choose(&mut rng).cloned() {
                if seen.insert(choice.config_id.clone()) {
                    selected.push(choice);
                }
            }
        }
        candidates = selected;
        "random".to_string()
    } else {
        "grid".to_string()
    };

    if candidates.is_empty() {
        eprintln!("No candidate configurations generated.");
        std::process::exit(1);
    }

    let mut table = Vec::<ConfigSeedMetrics>::new();
    let mut failure_cases = Vec::<FailureCase>::new();
    let mut aggregate = Vec::<ConfigAggregate>::new();

    for config in &candidates {
        let approach = olm::SimulationApproachConfig {
            id: config.config_id.clone(),
            name: "OLM search candidate".to_string(),
            description: format!(
                "lambda={:.3}, gamma={:.3}, theta={:.3}, meta={:.3}, k={:.3}, root={:.3}",
                config.lambda,
                config.gamma,
                config.theta,
                config.meta_strength,
                config.soft_gate_k,
                config.root_penalty
            ),
            lambda: config.lambda,
            theta: config.theta,
            min_readiness: 0.1,
            gamma: config.gamma,
            root_penalty: config.root_penalty,
            meta_strength: config.meta_strength,
            soft_gate_k: Some(config.soft_gate_k),
            stop_mastery: 0.99,
        };

        let mut pass_values = Vec::new();
        let mut hit_values = Vec::new();
        let mut mrr_values = Vec::new();
        let mut ndcg_values = Vec::new();

        for &seed in &seeds {
            let report =
                olm::olm_run_synthetic_scenarios_seeded_with_approaches(seed, &[approach.clone()])
                    .unwrap_or_else(|err| {
                        eprintln!(
                            "Simulation failed for config={} seed={}: {}",
                            config.config_id, seed, err
                        );
                        std::process::exit(1)
                    });
            let metrics = report.approaches.first().expect("one approach expected");

            pass_values.push(metrics.pass_rate);
            hit_values.push(metrics.hit_at_3_rate);
            mrr_values.push(metrics.avg_mrr);
            ndcg_values.push(metrics.avg_ndcg_at_3);

            table.push(ConfigSeedMetrics {
                config_id: config.config_id.clone(),
                seed,
                pass_rate: metrics.pass_rate,
                hit_at_3: metrics.hit_at_3_rate,
                mrr: metrics.avg_mrr,
                ndcg_at_3: metrics.avg_ndcg_at_3,
            });

            for s in &metrics.scenarios {
                let top_1_ok = s
                    .top_recommendation
                    .as_ref()
                    .map(|top| s.expected_any.iter().any(|expected| expected == top))
                    .unwrap_or(false);

                if !top_1_ok {
                    failure_cases.push(FailureCase {
                        config_id: config.config_id.clone(),
                        seed,
                        scenario_id: s.scenario_id.clone(),
                        scenario_name: s.scenario_name.clone(),
                        top_1: s.top_recommendation.clone(),
                        expected_any: s.expected_any.clone(),
                        top_3: s.top_recommendations.clone(),
                        rank_of_first_expected: s.rank_of_first_expected,
                        notes: s.notes.clone(),
                    });
                }
            }
        }

        let pass_avg = average(&pass_values);
        let hit_avg = average(&hit_values);
        let mrr_avg = average(&mrr_values);
        let ndcg_avg = average(&ndcg_values);

        let pass_stability = std_dev(&pass_values);
        let hit_stability = std_dev(&hit_values);
        let mrr_stability = std_dev(&mrr_values);
        let ndcg_stability = std_dev(&ndcg_values);

        let composed_score = 0.35 * pass_avg + 0.25 * hit_avg + 0.20 * mrr_avg + 0.10 * ndcg_avg
            - 0.10 * (pass_stability + mrr_stability);

        aggregate.push(ConfigAggregate {
            config: config.clone(),
            seeds: seeds.clone(),
            pass_rate_avg: pass_avg,
            hit_at_3_avg: hit_avg,
            mrr_avg,
            ndcg_at_3_avg: ndcg_avg,
            pass_rate_stability: pass_stability,
            hit_at_3_stability: hit_stability,
            mrr_stability,
            ndcg_at_3_stability: ndcg_stability,
            composed_score,
        });
    }

    aggregate.sort_by(|a, b| {
        b.composed_score
            .partial_cmp(&a.composed_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                b.pass_rate_avg
                    .partial_cmp(&a.pass_rate_avg)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| b.mrr_avg.partial_cmp(&a.mrr_avg).unwrap_or(Ordering::Equal))
    });

    let output = SearchOutput {
        seeds,
        mode,
        total_configs: aggregate.len(),
        ranking: aggregate,
        metrics_table: table,
        failure_cases,
    };

    let payload = serde_json::to_string_pretty(&output).unwrap_or_else(|err| {
        eprintln!("Failed to serialize search output: {}", err);
        std::process::exit(1)
    });

    if let Some(path) = output_path {
        std::fs::write(&path, payload).unwrap_or_else(|err| {
            eprintln!("Failed to write output '{}': {}", path, err);
            std::process::exit(1)
        });
        println!("Saved search report to {}", path);
    } else {
        println!("{}", payload);
    }
}
