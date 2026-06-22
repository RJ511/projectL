#[path = "../commands/olm.rs"]
#[allow(dead_code)]
mod olm;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut as_json = false;
    let mut output_path: Option<String> = None;
    let mut seeds: Vec<u64> = vec![0];
    let mut generated_scenarios: usize = 0;
    let mut graph_size: usize = 6;
    let mut event_count: usize = 24;
    let mut prereq_density: f64 = 0.35;
    let mut depth: usize = 3;
    let mut mapping_quality: f64 = 0.8;
    let mut scenario_mode = "mixed".to_string();
    let mut approaches_json_path: Option<String> = None;
    let mut custom_approaches_only = false;
    let mut top_n: usize = 0;
    let mut show_compare = false;

    let mut index = 1usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--save" => {
                as_json = true;
                output_path = Some("results.json".to_string());
                index += 1;
            }
            "--out" | "-o" => {
                if index + 1 >= args.len() {
                    eprintln!("Missing file path after {}", args[index]);
                    std::process::exit(1);
                }
                as_json = true;
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
            "--generated-scenarios" => {
                if index + 1 >= args.len() {
                    eprintln!("Missing count after --generated-scenarios");
                    std::process::exit(1);
                }
                generated_scenarios = args[index + 1].parse().unwrap_or(0);
                index += 2;
            }
            "--profile" => {
                if index + 1 >= args.len() {
                    eprintln!("Missing profile after --profile");
                    std::process::exit(1);
                }
                match args[index + 1].as_str() {
                    "quick" => {
                        generated_scenarios = 8;
                        graph_size = 6;
                        event_count = 20;
                        prereq_density = 0.30;
                        depth = 3;
                        mapping_quality = 0.75;
                    }
                    "balanced" => {
                        generated_scenarios = 24;
                        graph_size = 14;
                        event_count = 55;
                        prereq_density = 0.50;
                        depth = 5;
                        mapping_quality = 0.82;
                    }
                    "stress" => {
                        generated_scenarios = 60;
                        graph_size = 12;
                        event_count = 48;
                        prereq_density = 0.45;
                        depth = 5;
                        mapping_quality = 0.88;
                    }
                    other => {
                        eprintln!(
                            "Unknown profile '{}'. Use: quick | balanced | stress",
                            other
                        );
                        std::process::exit(1);
                    }
                }
                index += 2;
            }
            "--graph-size" => {
                if index + 1 >= args.len() {
                    eprintln!("Missing value after --graph-size");
                    std::process::exit(1);
                }
                graph_size = args[index + 1].parse().unwrap_or(6);
                index += 2;
            }
            "--event-count" => {
                if index + 1 >= args.len() {
                    eprintln!("Missing value after --event-count");
                    std::process::exit(1);
                }
                event_count = args[index + 1].parse().unwrap_or(24);
                index += 2;
            }
            "--prereq-density" => {
                if index + 1 >= args.len() {
                    eprintln!("Missing value after --prereq-density");
                    std::process::exit(1);
                }
                prereq_density = args[index + 1].parse().unwrap_or(0.35);
                index += 2;
            }
            "--depth" => {
                if index + 1 >= args.len() {
                    eprintln!("Missing value after --depth");
                    std::process::exit(1);
                }
                depth = args[index + 1].parse().unwrap_or(3);
                index += 2;
            }
            "--mapping-quality" => {
                if index + 1 >= args.len() {
                    eprintln!("Missing value after --mapping-quality");
                    std::process::exit(1);
                }
                mapping_quality = args[index + 1].parse().unwrap_or(0.8);
                index += 2;
            }
            "--scenario-mode" => {
                if index + 1 >= args.len() {
                    eprintln!("Missing value after --scenario-mode");
                    std::process::exit(1);
                }
                scenario_mode = args[index + 1].clone();
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
            "--approaches-json" => {
                if index + 1 >= args.len() {
                    eprintln!("Missing file path after --approaches-json");
                    std::process::exit(1);
                }
                approaches_json_path = Some(args[index + 1].clone());
                index += 2;
            }
            "--approach-override" => {
                custom_approaches_only = true;
                index += 1;
            }
            "--top-n" => {
                if index + 1 >= args.len() {
                    eprintln!("Missing value after --top-n");
                    std::process::exit(1);
                }
                top_n = args[index + 1].parse().unwrap_or(0);
                index += 2;
            }
            "--compare" => {
                show_compare = true;
                index += 1;
            }
            _ => {
                index += 1;
            }
        }
    }

    // Load custom approaches from JSON file if provided.
    let custom_approaches: Vec<olm::CustomApproachConfig> =
        if let Some(ref path) = approaches_json_path {
            match std::fs::read_to_string(path) {
                Ok(raw) => match serde_json::from_str(&raw) {
                    Ok(parsed) => parsed,
                    Err(err) => {
                        eprintln!("Failed to parse approaches JSON '{}': {}", path, err);
                        std::process::exit(1);
                    }
                },
                Err(err) => {
                    eprintln!("Failed to read approaches JSON '{}': {}", path, err);
                    std::process::exit(1);
                }
            }
        } else {
            vec![]
        };

    let sim_options = olm::SimulationOptions {
        generated_scenarios,
        graph_size,
        event_count,
        prereq_density,
        depth,
        mapping_quality,
        scenario_mode,
        custom_approaches,
        custom_approaches_only,
        top_n,
    };

    if seeds.len() == 1 {
        run_single(seeds[0], as_json, show_compare, output_path, sim_options);
    } else {
        run_multi_seed(&seeds, show_compare, output_path, sim_options);
    }
}

fn run_single(
    seed: u64,
    as_json: bool,
    show_compare: bool,
    output_path: Option<String>,
    sim_options: olm::SimulationOptions,
) {
    match olm::olm_run_synthetic_scenarios_seeded_with_options(seed, sim_options) {
        Ok(report) => {
            if as_json {
                match serde_json::to_string_pretty(&report) {
                    Ok(payload) => {
                        if let Some(path) = output_path {
                            if let Err(err) = std::fs::write(&path, payload) {
                                eprintln!("Failed to write report file '{}': {}", path, err);
                                std::process::exit(1);
                            }
                            println!("Saved report to {}", path);
                        } else {
                            println!("{}", payload);
                        }
                    }
                    Err(err) => {
                        eprintln!("Failed to serialize report: {}", err);
                        std::process::exit(1);
                    }
                }
                return;
            }

            print_report(&report, show_compare);
        }
        Err(err) => {
            eprintln!("Simulation failed: {}", err);
            std::process::exit(1);
        }
    }
}

fn run_multi_seed(
    seeds: &[u64],
    show_compare: bool,
    output_path: Option<String>,
    sim_options: olm::SimulationOptions,
) {
    let _ = show_compare; // comparison table is per-seed in JSON; text mode aggregates only
    let mut all_reports = Vec::new();

    for &seed in seeds {
        match olm::olm_run_synthetic_scenarios_seeded_with_options(seed, sim_options.clone()) {
            Ok(report) => all_reports.push((seed, report)),
            Err(err) => {
                eprintln!("Simulation failed for seed {}: {}", seed, err);
                std::process::exit(1);
            }
        }
    }

    // Aggregate metrics across seeds
    println!("Multi-seed report ({} seeds)", seeds.len());

    // Collect unique approach ids from first report
    let approach_ids: Vec<String> = all_reports
        .first()
        .map(|(_, r)| r.approaches.iter().map(|a| a.approach_id.clone()).collect())
        .unwrap_or_default();

    for approach_id in &approach_ids {
        let approach_reports: Vec<_> = all_reports
            .iter()
            .filter_map(|(_, r)| r.approaches.iter().find(|a| &a.approach_id == approach_id))
            .collect();

        let n = approach_reports.len() as f64;
        let avg_hit1 = approach_reports.iter().map(|a| a.hit_at_1_rate).sum::<f64>() / n;
        let avg_hit3 = approach_reports
            .iter()
            .map(|a| a.hit_at_3_rate)
            .sum::<f64>()
            / n;
        let avg_mrr = approach_reports.iter().map(|a| a.avg_mrr).sum::<f64>() / n;
        let avg_ndcg = approach_reports
            .iter()
            .map(|a| a.avg_ndcg_at_3)
            .sum::<f64>()
            / n;
        let avg_test_pass = approach_reports
            .iter()
            .map(|a| a.test_pass_rate)
            .sum::<f64>()
            / n;
        let avg_test_mrr = approach_reports.iter().map(|a| a.test_avg_mrr).sum::<f64>() / n;
        let avg_learning_gain = approach_reports
            .iter()
            .map(|a| a.avg_learning_gain)
            .sum::<f64>()
            / n;

        println!(
            "  [{}] hit@1={:.2}% hit@3={:.2}% mrr={:.3} ndcg@3={:.3} | test_pass={:.2}% test_mrr={:.3} | learning_gain={:.3}",
            approach_id,
            avg_hit1 * 100.0,
            avg_hit3 * 100.0,
            avg_mrr,
            avg_ndcg,
            avg_test_pass * 100.0,
            avg_test_mrr,
            avg_learning_gain
        );
    }

    if let Some(path) = output_path {
        let payload: Vec<serde_json::Value> = all_reports
            .iter()
            .map(|(seed, report)| {
                serde_json::json!({
                    "seed": seed,
                    "report": report,
                })
            })
            .collect();
        match serde_json::to_string_pretty(&payload) {
            Ok(json) => {
                if let Err(err) = std::fs::write(&path, json) {
                    eprintln!("Failed to write multi-seed report '{}': {}", path, err);
                    std::process::exit(1);
                }
                println!("Saved multi-seed report to {}", path);
            }
            Err(err) => {
                eprintln!("Failed to serialize multi-seed report: {}", err);
                std::process::exit(1);
            }
        }
    }
}

fn print_report(report: &olm::SimulationReport, show_compare: bool) {
    println!("OLM Synthetic Simulation Report");
    println!("Scenarios: {}", report.scenarios_count);
    println!("Best approach: {}", report.best_approach_id);
    println!(
        "Calibration pick (train): {} | test hit@1(pass_rate): {:.2}% | test MRR: {:.3}",
        report.calibration.selected_approach_id,
        report.calibration.selected_test_pass_rate * 100.0,
        report.calibration.selected_test_avg_mrr
    );
    println!();

    for approach in &report.approaches {
        println!(
            "- {} [{}] | hit@1: {:.2}% | hit@3: {:.2}% | mrr: {:.3} | ndcg@3: {:.3}",
            approach.approach_name,
            approach.approach_id,
            approach.hit_at_1_rate * 100.0,
            approach.hit_at_3_rate * 100.0,
            approach.avg_mrr,
            approach.avg_ndcg_at_3
        );
        println!("  {}", approach.description);
        println!(
            "  train pass: {:.2}% | test pass: {:.2}% | train mrr: {:.3} | test mrr: {:.3} | avg_rank_expected={:.2}",
            approach.train_pass_rate * 100.0,
            approach.test_pass_rate * 100.0,
            approach.train_avg_mrr,
            approach.test_avg_mrr,
            approach.average_rank_of_expected
        );
        println!(
            "  delta_vs_random={:+.2}% | delta_vs_mastery={:+.2}% | delta_vs_uncertainty={:+.2}% | metacog_rank_shift={:+.3}",
            approach.random_baseline_delta * 100.0,
            approach.mastery_baseline_delta * 100.0,
            approach.uncertainty_baseline_delta * 100.0,
            approach.metacognition_rank_shift
        );
        println!(
            "  long: learning_gain={:.3} post_test={:.3} mastery_gain={:.3} bad_recs={:.2} prereq_violation={:.2}%",
            approach.avg_learning_gain,
            approach.avg_post_test_score,
            approach.avg_mastery_gain,
            approach.avg_bad_recommendations,
            approach.prerequisite_violation_rate * 100.0
        );

        for scenario in &approach.scenarios {
            let status = if scenario.pass { "PASS" } else { "FAIL" };
            let top = scenario
                .top_recommendation
                .clone()
                .unwrap_or_else(|| "none".to_string());

            println!(
                "    {} {} [{} seed={}] -> top={} top3={:?} expected_any={:?} true_weak={:?} rank={:?} | hit@3={} mrr={:.3} ndcg@3={:.3} avg_rank={:.2} | hard={}/{} ranked={} excluded_min={} | avg_m={:.2} avg_u={:.2} | gain={:.3} ttm={:?} bad={} prereq_v={:.2}%",
                status,
                scenario.scenario_id,
                scenario.scenario_metadata.mode,
                scenario.scenario_metadata.seed,
                top,
                scenario.top_recommendations,
                scenario.expected_any,
                scenario.true_weak_concepts,
                scenario.rank_of_first_expected,
                scenario.hit_at_3,
                scenario.mrr,
                scenario.ndcg_at_3,
                scenario.average_rank_of_expected,
                scenario.diagnostics.candidates_after_gate,
                scenario.diagnostics.candidates_before_gate,
                scenario.diagnostics.candidates_ranked,
                scenario.diagnostics.candidates_excluded_min_readiness,
                scenario.avg_mastery,
                scenario.avg_uncertainty,
                scenario.learning_gain,
                scenario.time_to_mastery,
                scenario.number_of_bad_recommendations,
                scenario.prerequisite_violation_rate * 100.0
            );
        }

        println!();
    }

    // Cross-approach comparison tables (opt-in via --compare).
    if show_compare && !report.scenario_comparisons.is_empty() {
        let approach_ids: Vec<&str> = report
            .approaches
            .iter()
            .map(|a| a.approach_id.as_str())
            .collect();

        for cmp in &report.scenario_comparisons {
            println!("── Compare: {} ──────────────────────", cmp.scenario_name);

            // Header row
            let col_w = 20usize;
            let approach_col_w = 36usize;
            print!("  {:<col_w$}", "Concept");
            for aid in &approach_ids {
                print!("  {:<approach_col_w$}", aid);
            }
            println!();
            print!("  {:<col_w$}", "");
            for _ in &approach_ids {
                print!("  {:<approach_col_w$}", "rank | score  mastery readiness gate");
            }
            println!();
            println!("  {}", "-".repeat(col_w + approach_ids.len() * (approach_col_w + 2)));

            for row in &cmp.concepts {
                let short_name = if row.name.len() > col_w - 1 {
                    format!("{}…", &row.name[..col_w - 2])
                } else {
                    row.name.clone()
                };
                print!("  {:<col_w$}", short_name);
                for aid in &approach_ids {
                    let slice = row
                        .slices
                        .iter()
                        .find(|s| s.approach_id == *aid);
                    match slice {
                        Some(s) => {
                            let rank_str = s
                                .rank
                                .map(|r| format!("#{}", r))
                                .unwrap_or_else(|| "--".to_string());
                            let cell = format!(
                                "{:<3} s={:.3} m={:.2} r={:.2} g={:.2}",
                                rank_str, s.score, s.mastery, s.readiness, s.gate_factor
                            );
                            print!("  {:<approach_col_w$}", cell);
                        }
                        None => print!("  {:<approach_col_w$}", "-"),
                    }
                }
                println!();
            }
            println!();
        }
    }
}
