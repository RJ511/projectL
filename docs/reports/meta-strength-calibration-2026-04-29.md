# Meta Strength Calibration Report

Generated at: 2026-04-29T09:42:27.513Z
Seed range: 0..29 (30 seeds)

## Recommendation

- Recommended approach: `metacog_strict`
- Recommended meta_strength: `1`
- Selection rationale: highest training pass rate, then highest training MRR, then highest hit@3.

## Aggregated Metrics

| Approach | meta_strength | Avg pass | Avg hit@3 | Avg MRR | Avg nDCG@3 | Avg test pass | Avg test MRR | Stability (test MRR stdev) | Composite | Calibration picks |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| metacog_strict | 1.00 | 100.00% | 100.00% | 1.000 | 1.000 | 100.00% | 1.000 | 0.000 | 1.000 | 30/30 |
| baseline | 0.00 | 100.00% | 100.00% | 1.000 | 1.000 | 100.00% | 1.000 | 0.000 | 1.000 | 0/30 |
| metacog_balanced | 0.60 | 100.00% | 100.00% | 1.000 | 1.000 | 100.00% | 1.000 | 0.000 | 1.000 | 0/30 |

## Notes

- This report is based on synthetic scenarios only.
- Real anonymized telemetry should be merged in a second pass before locking production defaults.
