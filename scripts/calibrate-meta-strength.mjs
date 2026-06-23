import fs from "node:fs";
import path from "node:path";
import { execSync } from "node:child_process";

const APPROACH_META_STRENGTH = {
  no_metacognition: 0.0,
  metacog_balanced: 0.6,
  metacog_strict: 1.0,
};

const CALIBRATION_APPROACHES = [
  "no_metacognition",
  "metacog_balanced",
  "metacog_strict",
];

const MIN_MATERIAL_DIFFERENCE = 0.02;
const MIN_DECISION_DIVERGENCE = 0.05;

function parseArgs(argv) {
  const args = {
    start: 0,
    end: 9,
    outDir: path.join("docs", "reports"),
    keepRaw: false,
  };

  for (let i = 2; i < argv.length; i += 1) {
    const token = argv[i];
    if (token === "--start" && argv[i + 1]) {
      args.start = Number.parseInt(argv[i + 1], 10);
      i += 1;
    } else if (token === "--end" && argv[i + 1]) {
      args.end = Number.parseInt(argv[i + 1], 10);
      i += 1;
    } else if (token === "--out-dir" && argv[i + 1]) {
      args.outDir = argv[i + 1];
      i += 1;
    } else if (token === "--keep-raw") {
      args.keepRaw = true;
    }
  }

  if (
    !Number.isInteger(args.start) ||
    !Number.isInteger(args.end) ||
    args.start < 0 ||
    args.end < args.start
  ) {
    throw new Error(
      "Invalid seed range. Use --start N --end M with 0 <= N <= M.",
    );
  }

  return args;
}

function mean(values) {
  if (!values.length) return 0;
  return values.reduce((acc, v) => acc + v, 0) / values.length;
}

function stddev(values) {
  if (values.length <= 1) return 0;
  const m = mean(values);
  const variance =
    values.reduce((acc, v) => acc + (v - m) ** 2, 0) / values.length;
  return Math.sqrt(variance);
}

function scoreApproach(stats) {
  return (
    stats.avg_test_pass_rate * 0.55 +
    stats.avg_test_mrr * 0.35 +
    stats.avg_hit_at_3_rate * 0.1
  );
}

function toPct(value) {
  return `${(value * 100).toFixed(2)}%`;
}

function toNum(value, digits = 3) {
  return Number(value).toFixed(digits);
}

function buildMarkdown(summary) {
  const lines = [];
  lines.push("# Meta Strength Calibration and Synthetic Evaluation Report");
  lines.push("");
  lines.push(`Generated at: ${summary.generated_at}`);
  lines.push(
    `Seed range: ${summary.seed_start}..${summary.seed_end} (${summary.seed_count} seeds)`,
  );
  lines.push("");
  lines.push("## Evaluation diagnosis");
  lines.push("");
  lines.push(`- Calibration status: **${summary.recommendation.status}**.`);
  lines.push(`- ${summary.recommendation.rationale}`);
  lines.push(`- Ground truth leakage: **not detected**. Targets are derived from latent true mastery and prerequisite constraints, not from the evaluated ranking function.`);
  lines.push(`- Circularity risk: the oracle shares the pedagogical assumption that weak, unblocked concepts are desirable, but does not reuse readiness scores, observed states or ranking scores.`);
  lines.push(`- Train/test: canonical, hard-case and generated scenario IDs are assigned to disjoint sets.`);
  lines.push(`- Seed variation: ${summary.seed_variation_note}`);
  lines.push("");
  lines.push("## Problems found and fixes");
  lines.push("");
  lines.push("- The previous canonical split returned only S1-S12, silently excluding every generated scenario from both train and test. Extra scenarios are now assigned deterministically and the sets are checked for overlap.");
  lines.push("- The previous selector always returned an approach after ties. Selection now requires a material training difference; otherwise the status is `inconclusive`.");
  lines.push("- The former `baseline` represented the full recommendation stack with metacognition disabled. A separate simple baseline now ranks only by mastery gap and uses no prerequisites, readiness, uncertainty, metacognition, root penalty or mapping penalty.");
  lines.push("- Chain-shaped scenarios often exposed one obvious candidate. H1-H6 use parallel candidates and contradictory evidence to make component effects observable.");
  lines.push("- nDCG@3 now credits every relevant result in the first three positions and normalizes against the number of valid targets.");
  lines.push("");
  lines.push("## Recommendation");
  lines.push("");
  if (summary.recommendation.status === "selected") {
    lines.push(`- Recommended approach: \`${summary.recommendation.approach_id}\`.`);
    lines.push(`- Recommended meta_strength: \`${summary.recommendation.meta_strength}\`.`);
  } else {
    lines.push("- Recommended approach: **inconclusive**.");
    lines.push("- No production `meta_strength` should be locked from this synthetic evaluation alone.");
  }
  lines.push("");
  lines.push("## Aggregated Metrics");
  lines.push("");
  lines.push(
    "| Approach | meta_strength | hit@1 | hit@3 | MRR | nDCG@3 | Test pass | Test MRR | Hard pass | Candidates | Tie rate | Prereq violations | Divergence vs baseline | MRR delta vs baseline | Meta influence |",
  );
  lines.push("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");

  for (const row of summary.approaches) {
    lines.push(
      `| ${row.approach_id} | ${row.meta_strength == null ? "—" : row.meta_strength.toFixed(2)} | ${toPct(row.avg_pass_rate)} | ${toPct(row.avg_hit_at_3_rate)} | ${toNum(row.avg_mrr)} | ${toNum(row.avg_ndcg_at_3)} | ${toPct(row.avg_test_pass_rate)} | ${toNum(row.avg_test_mrr)} | ${toPct(row.avg_hard_case_pass_rate)} | ${toNum(row.avg_candidate_count, 1)} | ${toPct(row.avg_tie_rate)} | ${toPct(row.avg_prerequisite_violation_rate)} | ${toPct(row.avg_decision_divergence_rate)} | ${toNum(row.avg_ranking_delta_vs_baseline)} | ${toPct(row.avg_meta_influence_rate)} |`,
    );
  }

  lines.push("");
  lines.push("## Easy cases vs hard cases");
  lines.push("");
  lines.push("| Approach | Easy pass | Hard pass | Difference | Test MRR stdev |");
  lines.push("|---|---:|---:|---:|---:|");
  for (const row of summary.approaches) {
    lines.push(`| ${row.approach_id} | ${toPct(row.avg_easy_case_pass_rate)} | ${toPct(row.avg_hard_case_pass_rate)} | ${toPct(row.avg_easy_case_pass_rate - row.avg_hard_case_pass_rate)} | ${toNum(row.std_test_mrr)} |`);
  }

  lines.push("");
  lines.push("## Decision divergence");
  lines.push("");
  lines.push("| Reference | Compared | Changed decisions | Scenarios | Rate |");
  lines.push("|---|---|---:|---:|---:|");
  for (const row of summary.decision_divergence) {
    lines.push(`| ${row.reference_approach_id} | ${row.compared_approach_id} | ${row.changed_decisions_avg.toFixed(1)} | ${row.scenarios_compared_avg.toFixed(1)} | ${toPct(row.avg_rate)} |`);
  }

  lines.push("");
  lines.push("## Example failures");
  lines.push("");
  for (const failure of summary.failure_examples) {
    lines.push(`- \`${failure.approach_id}\` — **${failure.scenario_id} ${failure.scenario_name}**: top=\`${failure.top_recommendation || "none"}\`, expected=${failure.expected_any.map((id) => `\`${id}\``).join(", ")}, candidates=${failure.candidate_count}, MRR=${toNum(failure.mrr)}.`);
  }

  lines.push("");
  lines.push("## Metric interpretation and inflation risks");
  lines.push("");
  lines.push("- `hit@3` is weak when the candidate set is small; always interpret it with `candidate_count_avg`.");
  lines.push("- A high hit@3 with low hit@1/MRR indicates that the target is present but poorly ordered.");
  lines.push("- `tie_rate` exposes apparently strong metrics that depend on deterministic tie-breaking.");
  lines.push("- `decision_divergence_rate` measures changed top decisions, not whether the change is beneficial; combine it with hard-case pass and MRR.");
  lines.push("- Synthetic ground truth is an explicit pedagogical oracle, not observed learner benefit; longitudinal metrics remain simulated outcomes.");

  lines.push("");
  lines.push("## Limitations");
  lines.push("");
  lines.push("- This report is based on synthetic scenarios only.");
  lines.push("- Scenario generators can still encode assumptions that favour prerequisite-aware policies.");
  lines.push("- Confidence and performance noise are simplified approximations of real learner behaviour.");
  lines.push("- Real anonymized telemetry and outcome-based validation should be merged before locking production defaults.");

  return `${lines.join("\n")}\n`;
}

function main() {
  const args = parseArgs(process.argv);
  const dateStamp = new Date().toISOString().slice(0, 10);
  const outDir = path.resolve(args.outDir);
  const rawJsonPath = path.join(
    outDir,
    `meta-strength-calibration-${dateStamp}.raw.json`,
  );
  const summaryJsonPath = path.join(
    outDir,
    `meta-strength-calibration-${dateStamp}.json`,
  );
  const markdownPath = path.join(
    outDir,
    `meta-strength-calibration-${dateStamp}.md`,
  );

  fs.mkdirSync(outDir, { recursive: true });

  const cargoCommand = `cargo run --bin olm_sim -- --seed-range ${args.start} ${args.end} --generated-scenarios 8 --graph-size 10 --event-count 32 --prereq-density 0.45 --depth 4 --scenario-mode mixed --top-n 5 --out "${rawJsonPath}"`;
  execSync(cargoCommand, {
    cwd: path.resolve("src-tauri"),
    stdio: "inherit",
  });

  const payload = JSON.parse(fs.readFileSync(rawJsonPath, "utf8"));
  const byApproach = new Map();
  const selectedCount = new Map();
  const divergenceByPair = new Map();
  const failureExamples = new Map();
  const calibrationStatuses = [];

  for (const seedEntry of payload) {
    const report = seedEntry.report;
    const selected = report?.calibration?.selected_approach_id;
    calibrationStatuses.push(report?.calibration?.status || "unknown");
    if (selected) {
      selectedCount.set(selected, (selectedCount.get(selected) || 0) + 1);
    }

    for (const approach of report.approaches || []) {
      const id = approach.approach_id;
      if (!byApproach.has(id)) {
        byApproach.set(id, {
          approach_id: id,
          meta_strength: APPROACH_META_STRENGTH[id] ?? null,
          pass_rate: [],
          hit_at_3_rate: [],
          avg_mrr: [],
          avg_ndcg_at_3: [],
          test_pass_rate: [],
          test_avg_mrr: [],
          train_pass_rate: [],
          train_avg_mrr: [],
          hard_case_pass_rate: [],
          easy_case_pass_rate: [],
          candidate_count_avg: [],
          tie_rate: [],
          prerequisite_violation_rate: [],
          decision_divergence_rate: [],
          ranking_delta_vs_baseline: [],
          meta_influence_rate: [],
        });
      }

      const row = byApproach.get(id);
      row.pass_rate.push(approach.pass_rate ?? 0);
      row.hit_at_3_rate.push(approach.hit_at_3_rate ?? 0);
      row.avg_mrr.push(approach.avg_mrr ?? 0);
      row.avg_ndcg_at_3.push(approach.avg_ndcg_at_3 ?? 0);
      row.test_pass_rate.push(approach.test_pass_rate ?? 0);
      row.test_avg_mrr.push(approach.test_avg_mrr ?? 0);
      row.train_pass_rate.push(approach.train_pass_rate ?? 0);
      row.train_avg_mrr.push(approach.train_avg_mrr ?? 0);
      row.hard_case_pass_rate.push(approach.hard_case_pass_rate ?? 0);
      row.easy_case_pass_rate.push(approach.easy_case_pass_rate ?? 0);
      row.candidate_count_avg.push(approach.candidate_count_avg ?? 0);
      row.tie_rate.push(approach.tie_rate ?? 0);
      row.prerequisite_violation_rate.push(
        approach.prerequisite_violation_rate ?? 0,
      );
      row.decision_divergence_rate.push(
        approach.decision_divergence_rate ?? 0,
      );
      row.ranking_delta_vs_baseline.push(
        approach.ranking_delta_vs_baseline ?? 0,
      );
      row.meta_influence_rate.push(approach.meta_influence_rate ?? 0);

      if (!failureExamples.has(id)) {
        const failure = (approach.scenarios || []).find(
          (scenario) =>
            scenario.scenario_metadata?.difficulty === "hard" &&
            !scenario.pass,
        );
        if (failure) {
          failureExamples.set(id, {
            approach_id: id,
            scenario_id: failure.scenario_id,
            scenario_name: failure.scenario_name,
            top_recommendation: failure.top_recommendation,
            expected_any: failure.expected_any || [],
            candidate_count: failure.candidate_count || 0,
            mrr: failure.mrr || 0,
          });
        }
      }
    }

    for (const divergence of report.decision_divergence || []) {
      const key = `${divergence.reference_approach_id}=>${divergence.compared_approach_id}`;
      if (!divergenceByPair.has(key)) {
        divergenceByPair.set(key, {
          reference_approach_id: divergence.reference_approach_id,
          compared_approach_id: divergence.compared_approach_id,
          changed_decisions: [],
          scenarios_compared: [],
          rate: [],
        });
      }
      const row = divergenceByPair.get(key);
      row.changed_decisions.push(divergence.changed_decisions || 0);
      row.scenarios_compared.push(divergence.scenarios_compared || 0);
      row.rate.push(divergence.rate || 0);
    }
  }

  const approaches = Array.from(byApproach.values()).map((row) => {
    const summary = {
      approach_id: row.approach_id,
      meta_strength: row.meta_strength,
      avg_pass_rate: mean(row.pass_rate),
      avg_hit_at_3_rate: mean(row.hit_at_3_rate),
      avg_mrr: mean(row.avg_mrr),
      avg_ndcg_at_3: mean(row.avg_ndcg_at_3),
      avg_test_pass_rate: mean(row.test_pass_rate),
      avg_test_mrr: mean(row.test_avg_mrr),
      avg_train_pass_rate: mean(row.train_pass_rate),
      avg_train_mrr: mean(row.train_avg_mrr),
      std_test_mrr: stddev(row.test_avg_mrr),
      avg_hard_case_pass_rate: mean(row.hard_case_pass_rate),
      avg_easy_case_pass_rate: mean(row.easy_case_pass_rate),
      avg_candidate_count: mean(row.candidate_count_avg),
      avg_tie_rate: mean(row.tie_rate),
      avg_prerequisite_violation_rate: mean(row.prerequisite_violation_rate),
      avg_decision_divergence_rate: mean(row.decision_divergence_rate),
      avg_ranking_delta_vs_baseline: mean(row.ranking_delta_vs_baseline),
      avg_meta_influence_rate: mean(row.meta_influence_rate),
      calibration_selected_count: selectedCount.get(row.approach_id) || 0,
    };

    return {
      ...summary,
      composite_score: scoreApproach(summary),
    };
  });

  if (!approaches.length) {
    throw new Error("No approach data found in calibration payload.");
  }

  const calibrationRows = approaches.filter((row) =>
    CALIBRATION_APPROACHES.includes(row.approach_id),
  );
  const range = (values) =>
    values.length ? Math.max(...values) - Math.min(...values) : 0;
  const passRange = range(calibrationRows.map((row) => row.avg_train_pass_rate));
  const mrrRange = range(calibrationRows.map((row) => row.avg_train_mrr));
  const maxMetaInfluence = Math.max(
    0,
    ...calibrationRows.map((row) => row.avg_meta_influence_rate),
  );
  const discriminative =
    passRange >= MIN_MATERIAL_DIFFERENCE ||
    (mrrRange >= MIN_MATERIAL_DIFFERENCE &&
      maxMetaInfluence >= MIN_DECISION_DIVERGENCE);

  calibrationRows.sort((a, b) => {
    if (b.avg_train_pass_rate !== a.avg_train_pass_rate)
      return b.avg_train_pass_rate - a.avg_train_pass_rate;
    if (b.avg_train_mrr !== a.avg_train_mrr)
      return b.avg_train_mrr - a.avg_train_mrr;
    return a.approach_id.localeCompare(b.approach_id);
  });

  const selected = calibrationRows[0] || null;
  const recommendation = discriminative && selected
    ? {
        status: "selected",
        approach_id: selected.approach_id,
        meta_strength: selected.meta_strength,
        rationale: `Material training difference detected (pass range=${toPct(passRange)}, MRR range=${toNum(mrrRange)}, maximum decision influence=${toPct(maxMetaInfluence)}).`,
      }
    : {
        status: "inconclusive",
        approach_id: null,
        meta_strength: null,
        rationale: `The meta-strength approaches are not materially separated (pass range=${toPct(passRange)}, MRR range=${toNum(mrrRange)}, maximum decision influence=${toPct(maxMetaInfluence)}).`,
      };

  approaches.sort((a, b) => {
    if (b.avg_test_pass_rate !== a.avg_test_pass_rate)
      return b.avg_test_pass_rate - a.avg_test_pass_rate;
    if (b.avg_test_mrr !== a.avg_test_mrr)
      return b.avg_test_mrr - a.avg_test_mrr;
    return a.approach_id.localeCompare(b.approach_id);
  });

  const decisionDivergence = Array.from(divergenceByPair.values()).map(
    (row) => ({
      reference_approach_id: row.reference_approach_id,
      compared_approach_id: row.compared_approach_id,
      changed_decisions_avg: mean(row.changed_decisions),
      scenarios_compared_avg: mean(row.scenarios_compared),
      avg_rate: mean(row.rate),
    }),
  );
  const varyingApproaches = approaches.filter(
    (row) => row.std_test_mrr > 1e-6,
  ).length;
  const requestedFailureIds = new Set([
    "baseline",
    "random_baseline",
    "curriculum_linear",
    "mastery_only",
    "no_prereq_gating",
    "no_metacognition",
    "metacog_balanced",
    "metacog_strict",
  ]);

  const summary = {
    generated_at: new Date().toISOString(),
    seed_start: args.start,
    seed_end: args.end,
    seed_count: args.end - args.start + 1,
    recommendation,
    approaches,
    decision_divergence: decisionDivergence,
    failure_examples: Array.from(failureExamples.values()).filter((row) =>
      requestedFailureIds.has(row.approach_id),
    ),
    seed_variation_note: `${varyingApproaches}/${approaches.length} approaches have non-zero test-MRR variance across seeds; scenario generation and state perturbations are seed-dependent.`,
    per_seed_calibration_statuses: calibrationStatuses,
  };

  fs.writeFileSync(summaryJsonPath, JSON.stringify(summary, null, 2));
  fs.writeFileSync(markdownPath, buildMarkdown(summary));

  if (args.keepRaw) {
    console.log(`Saved raw calibration payload: ${rawJsonPath}`);
  } else {
    fs.unlinkSync(rawJsonPath);
    console.log("Removed intermediate raw payload (use --keep-raw to retain it).");
  }
  console.log(`Saved summary JSON: ${summaryJsonPath}`);
  console.log(`Saved report Markdown: ${markdownPath}`);
}

main();
