import fs from "node:fs";
import path from "node:path";
import { execSync } from "node:child_process";

const APPROACH_META_STRENGTH = {
  baseline: 0.0,
  metacog_balanced: 0.6,
  metacog_strict: 1.0,
};

function parseArgs(argv) {
  const args = {
    start: 0,
    end: 29,
    outDir: path.join("docs", "reports"),
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
  lines.push("# Meta Strength Calibration Report");
  lines.push("");
  lines.push(`Generated at: ${summary.generated_at}`);
  lines.push(
    `Seed range: ${summary.seed_start}..${summary.seed_end} (${summary.seed_count} seeds)`,
  );
  lines.push("");
  lines.push("## Recommendation");
  lines.push("");
  lines.push(
    `- Recommended approach: \`${summary.recommendation.approach_id}\``,
  );
  lines.push(
    `- Recommended meta_strength: \`${summary.recommendation.meta_strength}\``,
  );
  lines.push(
    `- Selection rationale: highest training pass rate, then highest training MRR, then highest hit@3.`,
  );
  lines.push("");
  lines.push("## Aggregated Metrics");
  lines.push("");
  lines.push(
    "| Approach | meta_strength | Avg pass | Avg hit@3 | Avg MRR | Avg nDCG@3 | Avg test pass | Avg test MRR | Stability (test MRR stdev) | Composite | Calibration picks |",
  );
  lines.push("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");

  for (const row of summary.approaches) {
    lines.push(
      `| ${row.approach_id} | ${row.meta_strength.toFixed(2)} | ${toPct(row.avg_pass_rate)} | ${toPct(row.avg_hit_at_3_rate)} | ${toNum(row.avg_mrr)} | ${toNum(row.avg_ndcg_at_3)} | ${toPct(row.avg_test_pass_rate)} | ${toNum(row.avg_test_mrr)} | ${toNum(row.std_test_mrr)} | ${toNum(row.composite_score)} | ${row.calibration_selected_count}/${summary.seed_count} |`,
    );
  }

  lines.push("");
  lines.push("## Notes");
  lines.push("");
  lines.push("- This report is based on synthetic scenarios only.");
  lines.push(
    "- Real anonymized telemetry should be merged in a second pass before locking production defaults.",
  );

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

  const cargoCommand = `cargo run --bin olm_sim -- --seed-range ${args.start} ${args.end} --out "${rawJsonPath}"`;
  execSync(cargoCommand, {
    cwd: path.resolve("src-tauri"),
    stdio: "inherit",
  });

  const payload = JSON.parse(fs.readFileSync(rawJsonPath, "utf8"));
  const byApproach = new Map();
  const selectedCount = new Map();

  for (const seedEntry of payload) {
    const report = seedEntry.report;
    const selected = report?.calibration?.selected_approach_id;
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
        });
      }

      const row = byApproach.get(id);
      row.pass_rate.push(approach.pass_rate ?? 0);
      row.hit_at_3_rate.push(approach.hit_at_3_rate ?? 0);
      row.avg_mrr.push(approach.avg_mrr ?? 0);
      row.avg_ndcg_at_3.push(approach.avg_ndcg_at_3 ?? 0);
      row.test_pass_rate.push(approach.test_pass_rate ?? 0);
      row.test_avg_mrr.push(approach.test_avg_mrr ?? 0);
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
      std_test_mrr: stddev(row.test_avg_mrr),
      calibration_selected_count: selectedCount.get(row.approach_id) || 0,
    };

    return {
      ...summary,
      composite_score: scoreApproach(summary),
    };
  });

  approaches.sort((a, b) => {
    if (b.calibration_selected_count !== a.calibration_selected_count) {
      return b.calibration_selected_count - a.calibration_selected_count;
    }
    if (b.avg_test_pass_rate !== a.avg_test_pass_rate)
      return b.avg_test_pass_rate - a.avg_test_pass_rate;
    if (b.avg_test_mrr !== a.avg_test_mrr)
      return b.avg_test_mrr - a.avg_test_mrr;
    return b.composite_score - a.composite_score;
  });

  if (!approaches.length) {
    throw new Error("No approach data found in calibration payload.");
  }

  const recommendation = {
    approach_id: approaches[0].approach_id,
    meta_strength: approaches[0].meta_strength,
  };

  const summary = {
    generated_at: new Date().toISOString(),
    seed_start: args.start,
    seed_end: args.end,
    seed_count: args.end - args.start + 1,
    recommendation,
    approaches,
  };

  fs.writeFileSync(summaryJsonPath, JSON.stringify(summary, null, 2));
  fs.writeFileSync(markdownPath, buildMarkdown(summary));

  console.log(`Saved raw calibration payload: ${rawJsonPath}`);
  console.log(`Saved summary JSON: ${summaryJsonPath}`);
  console.log(`Saved report Markdown: ${markdownPath}`);
}

main();
