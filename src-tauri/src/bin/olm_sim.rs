#[path = "../commands/olm.rs"]
#[allow(dead_code)]
mod olm;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut as_json = false;
    let mut output_path: Option<String> = None;
    let mut seeds: Vec<u64> = vec![0];

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
            _ => {
                index += 1;
            }
        }
    }

    if seeds.len() == 1 {
        run_single(seeds[0], as_json, output_path);
    } else {
        run_multi_seed(&seeds, output_path);
    }
}

fn run_single(seed: u64, as_json: bool, output_path: Option<String>) {
    match olm::olm_run_synthetic_scenarios_seeded(seed) {
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

            print_report(&report);
        }
        Err(err) => {
            eprintln!("Simulation failed: {}", err);
            std::process::exit(1);
        }
    }
}

fn run_multi_seed(seeds: &[u64], output_path: Option<String>) {
    let mut all_reports = Vec::new();

    for &seed in seeds {
        match olm::olm_run_synthetic_scenarios_seeded(seed) {
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
        let avg_pass = approach_reports.iter().map(|a| a.pass_rate).sum::<f64>() / n;
        let avg_hit3 = approach_reports.iter().map(|a| a.hit_at_3_rate).sum::<f64>() / n;
        let avg_mrr = approach_reports.iter().map(|a| a.avg_mrr).sum::<f64>() / n;
        let avg_ndcg = approach_reports.iter().map(|a| a.avg_ndcg_at_3).sum::<f64>() / n;
        let avg_test_pass = approach_reports.iter().map(|a| a.test_pass_rate).sum::<f64>() / n;
        let avg_test_mrr = approach_reports.iter().map(|a| a.test_avg_mrr).sum::<f64>() / n;

        println!(
            "  [{}] pass={:.2}% hit@3={:.2}% mrr={:.3} ndcg@3={:.3} | test_pass={:.2}% test_mrr={:.3}",
            approach_id,
            avg_pass * 100.0,
            avg_hit3 * 100.0,
            avg_mrr,
            avg_ndcg,
            avg_test_pass * 100.0,
            avg_test_mrr
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

fn print_report(report: &olm::SimulationReport) {
    println!("OLM Synthetic Simulation Report");
    println!("Scenarios: {}", report.scenarios_count);
    println!("Best approach: {}", report.best_approach_id);
    println!(
        "Calibration pick (train): {} | test pass_rate: {:.2}% | test MRR: {:.3}",
        report.calibration.selected_approach_id,
        report.calibration.selected_test_pass_rate * 100.0,
        report.calibration.selected_test_avg_mrr
    );
    println!();

    for approach in &report.approaches {
        println!(
            "- {} [{}] | pass_rate: {:.2}% | hit@3: {:.2}% | mrr: {:.3} | ndcg@3: {:.3}",
            approach.approach_name,
            approach.approach_id,
            approach.pass_rate * 100.0,
            approach.hit_at_3_rate * 100.0,
            approach.avg_mrr,
            approach.avg_ndcg_at_3
        );
        println!("  {}", approach.description);
        println!(
            "  train pass: {:.2}% | test pass: {:.2}% | train mrr: {:.3} | test mrr: {:.3}",
            approach.train_pass_rate * 100.0,
            approach.test_pass_rate * 100.0,
            approach.train_avg_mrr,
            approach.test_avg_mrr
        );

        for scenario in &approach.scenarios {
            let status = if scenario.pass { "PASS" } else { "FAIL" };
            let top = scenario
                .top_recommendation
                .clone()
                .unwrap_or_else(|| "none".to_string());

            println!(
                "    {} {} -> top={} top3={:?} expected_any={:?} rank={:?} | hit@3={} mrr={:.3} ndcg@3={:.3} | hard={}/{} ranked={} excluded_min={} | avg_m={:.2} avg_u={:.2}",
                status,
                scenario.scenario_id,
                top,
                scenario.top_recommendations,
                scenario.expected_any,
                scenario.rank_of_first_expected,
                scenario.hit_at_3,
                scenario.mrr,
                scenario.ndcg_at_3,
                scenario.diagnostics.candidates_after_gate,
                scenario.diagnostics.candidates_before_gate,
                scenario.diagnostics.candidates_ranked,
                scenario.diagnostics.candidates_excluded_min_readiness,
                scenario.avg_mastery,
                scenario.avg_uncertainty
            );
        }

        println!();
    }
}