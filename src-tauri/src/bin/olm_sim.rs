#[path = "../commands/mod.rs"]
mod commands;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut as_json = false;
    let mut output_path: Option<String> = None;

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
            _ => {
                index += 1;
            }
        }
    }

    match commands::olm_run_synthetic_scenarios() {
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

            for approach in report.approaches {
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

                for scenario in approach.scenarios {
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
        Err(err) => {
            eprintln!("Simulation failed: {}", err);
            std::process::exit(1);
        }
    }
}
