# Meta Strength Calibration and Synthetic Evaluation Report

Generated at: 2026-06-23T10:30:36.406Z
Seed range: 0..9 (10 seeds)

## Evaluation diagnosis

- Calibration status: **inconclusive**.
- The meta-strength approaches are not materially separated (pass range=0.00%, MRR range=0.003, maximum decision influence=10.38%).
- Ground truth leakage: **not detected**. Targets are derived from latent true mastery and prerequisite constraints, not from the evaluated ranking function.
- Circularity risk: the oracle shares the pedagogical assumption that weak, unblocked concepts are desirable, but does not reuse readiness scores, observed states or ranking scores.
- Train/test: canonical, hard-case and generated scenario IDs are assigned to disjoint sets.
- Seed variation: 14/14 approaches have non-zero test-MRR variance across seeds; scenario generation and state perturbations are seed-dependent.

## Problems found and fixes

- The previous canonical split returned only S1-S12, silently excluding every generated scenario from both train and test. Extra scenarios are now assigned deterministically and the sets are checked for overlap.
- The previous selector always returned an approach after ties. Selection now requires a material training difference; otherwise the status is `inconclusive`.
- The former `baseline` represented the full recommendation stack with metacognition disabled. A separate simple baseline now ranks only by mastery gap and uses no prerequisites, readiness, uncertainty, metacognition, root penalty or mapping penalty.
- Chain-shaped scenarios often exposed one obvious candidate. H1-H6 use parallel candidates and contradictory evidence to make component effects observable.
- nDCG@3 now credits every relevant result in the first three positions and normalizes against the number of valid targets.

## Recommendation

- Recommended approach: **inconclusive**.
- No production `meta_strength` should be locked from this synthetic evaluation alone.

## Aggregated Metrics

| Approach | meta_strength | hit@1 | hit@3 | MRR | nDCG@3 | Test pass | Test MRR | Hard pass | Candidates | Tie rate | Prereq violations | Divergence vs baseline | MRR delta vs baseline | Meta influence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| curriculum_linear | — | 46.92% | 74.62% | 0.616 | 0.511 | 46.25% | 0.624 | 47.62% | 11.3 | 0.00% | 0.00% | 93.46% | 0.039 | 0.00% |
| metacog_strict | 1.00 | 46.92% | 75.38% | 0.600 | 0.515 | 46.25% | 0.617 | 47.62% | 8.7 | 0.00% | 0.00% | 93.46% | 0.023 | 9.23% |
| no_root_penalty | — | 46.92% | 73.46% | 0.602 | 0.512 | 46.25% | 0.613 | 47.62% | 8.7 | 0.00% | 0.00% | 93.46% | 0.025 | 0.00% |
| no_mapping_penalty | — | 46.92% | 74.23% | 0.601 | 0.517 | 46.25% | 0.612 | 47.62% | 8.7 | 0.00% | 0.00% | 93.46% | 0.024 | 0.00% |
| mastery_only_gated | — | 46.92% | 75.38% | 0.613 | 0.530 | 46.25% | 0.606 | 47.62% | 8.7 | 0.00% | 0.00% | 93.46% | 0.036 | 0.00% |
| metacog_balanced | 0.60 | 46.92% | 72.69% | 0.597 | 0.512 | 46.25% | 0.606 | 47.62% | 8.7 | 0.00% | 0.00% | 93.46% | 0.020 | 10.38% |
| decay_hl7 | — | 46.92% | 74.23% | 0.592 | 0.513 | 46.25% | 0.605 | 47.62% | 6.7 | 1.54% | 0.00% | 93.46% | 0.015 | 0.00% |
| no_metacognition | 0.00 | 46.92% | 71.92% | 0.598 | 0.511 | 46.25% | 0.604 | 47.62% | 8.7 | 0.00% | 0.00% | 93.46% | 0.021 | 0.00% |
| decay_hl30 | — | 46.92% | 73.08% | 0.591 | 0.511 | 46.25% | 0.600 | 47.62% | 6.7 | 0.00% | 0.00% | 93.46% | 0.014 | 0.00% |
| uncertainty_only | — | 24.23% | 43.46% | 0.399 | 0.267 | 40.00% | 0.548 | 29.52% | 11.3 | 15.00% | 90.48% | 90.38% | -0.178 | 0.00% |
| mastery_only | — | 42.31% | 67.31% | 0.584 | 0.482 | 35.00% | 0.517 | 41.90% | 11.3 | 0.00% | 93.22% | 25.38% | 0.006 | 0.00% |
| baseline | — | 40.77% | 68.08% | 0.577 | 0.480 | 31.25% | 0.496 | 40.00% | 11.3 | 0.00% | 0.00% | 0.00% | 0.000 | 0.00% |
| no_prereq_gating | — | 23.46% | 41.92% | 0.375 | 0.275 | 28.75% | 0.439 | 29.05% | 11.3 | 0.00% | 100.00% | 82.31% | -0.202 | 0.00% |
| random_baseline | — | 21.54% | 50.00% | 0.408 | 0.289 | 23.75% | 0.457 | 23.33% | 8.7 | 0.00% | 85.82% | 90.38% | -0.169 | 0.00% |

## Easy cases vs hard cases

| Approach | Easy pass | Hard pass | Difference | Test MRR stdev |
|---|---:|---:|---:|---:|
| curriculum_linear | 44.00% | 47.62% | -3.62% | 0.065 |
| metacog_strict | 44.00% | 47.62% | -3.62% | 0.056 |
| no_root_penalty | 44.00% | 47.62% | -3.62% | 0.058 |
| no_mapping_penalty | 44.00% | 47.62% | -3.62% | 0.059 |
| mastery_only_gated | 44.00% | 47.62% | -3.62% | 0.062 |
| metacog_balanced | 44.00% | 47.62% | -3.62% | 0.060 |
| decay_hl7 | 44.00% | 47.62% | -3.62% | 0.063 |
| no_metacognition | 44.00% | 47.62% | -3.62% | 0.061 |
| decay_hl30 | 44.00% | 47.62% | -3.62% | 0.060 |
| uncertainty_only | 2.00% | 29.52% | -27.52% | 0.091 |
| mastery_only | 44.00% | 41.90% | 2.10% | 0.104 |
| baseline | 44.00% | 40.00% | 4.00% | 0.067 |
| no_prereq_gating | 0.00% | 29.05% | -29.05% | 0.046 |
| random_baseline | 14.00% | 23.33% | -9.33% | 0.111 |

## Decision divergence

| Reference | Compared | Changed decisions | Scenarios | Rate |
|---|---|---:|---:|---:|
| baseline | no_metacognition | 24.3 | 26.0 | 93.46% |
| no_metacognition | metacog_balanced | 2.7 | 26.0 | 10.38% |
| no_metacognition | metacog_strict | 2.4 | 26.0 | 9.23% |
| metacog_balanced | metacog_strict | 2.9 | 26.0 | 11.15% |

## Example failures

- `baseline` — **S5 Overconfidence**: top=`s5_c4`, expected=`s5_c1`, candidates=15, MRR=0.071.
- `random_baseline` — **S4 Alta incerteza por pouca evidencia**: top=`s4_c4`, expected=`s4_c1`, candidates=11, MRR=0.143.
- `mastery_only` — **S5 Overconfidence**: top=`s5_c4`, expected=`s5_c1`, candidates=15, MRR=0.067.
- `curriculum_linear` — **S6 Underconfidence**: top=`s6_c1`, expected=`s6_c11`, candidates=15, MRR=0.333.
- `no_prereq_gating` — **S4 Alta incerteza por pouca evidencia**: top=`s4_c12`, expected=`s4_c1`, candidates=12, MRR=0.083.
- `no_metacognition` — **S6 Underconfidence**: top=`s6_c1`, expected=`s6_c11`, candidates=9, MRR=0.000.
- `metacog_balanced` — **S6 Underconfidence**: top=`s6_c1`, expected=`s6_c11`, candidates=9, MRR=0.000.
- `metacog_strict` — **S6 Underconfidence**: top=`s6_c1`, expected=`s6_c11`, candidates=8, MRR=0.000.

## Metric interpretation and inflation risks

- `hit@3` is weak when the candidate set is small; always interpret it with `candidate_count_avg`.
- A high hit@3 with low hit@1/MRR indicates that the target is present but poorly ordered.
- `tie_rate` exposes apparently strong metrics that depend on deterministic tie-breaking.
- `decision_divergence_rate` measures changed top decisions, not whether the change is beneficial; combine it with hard-case pass and MRR.
- Synthetic ground truth is an explicit pedagogical oracle, not observed learner benefit; longitudinal metrics remain simulated outcomes.

## Limitations

- This report is based on synthetic scenarios only.
- Scenario generators can still encode assumptions that favour prerequisite-aware policies.
- Confidence and performance noise are simplified approximations of real learner behaviour.
- Real anonymized telemetry and outcome-based validation should be merged before locking production defaults.
