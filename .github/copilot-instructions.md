# Copilot Repository Instructions

## Documentation Sync Is Mandatory

Whenever a code change alters behavior, contracts, formulas, data flow, or operational usage, update documentation in the same task/PR.

Do not finish an implementation task with stale docs.

## Canonical Documentation Map

- Architecture and implementation truth: `docs/architecture-deep-dive.md`
- OLM formulas and scoring logic: `OLM_LOGIC_MAP.md` (repo root)
- Operational usage/runbook: `docs/PROJECT_GUIDE.md`
- Module map / where-to-change reference: `docs/IMPLEMENTATION_CONTEXT.md`

## Update Rules

1. If backend/frontend logic changes recommendation, scoring, or event ingestion:

- Update `OLM_LOGIC_MAP.md`
- Update `docs/architecture-deep-dive.md`

2. If commands, APIs, fields, or payloads change:

- Update `docs/architecture-deep-dive.md`
- Update `docs/IMPLEMENTATION_CONTEXT.md`

3. If user-visible flow, setup, or run commands change:

- Update `docs/PROJECT_GUIDE.md`

4. If no doc updates are needed, explicitly state why in the final change summary.

## PR/Task Completion Checklist (Required)

Before marking a task done, verify:

- [ ] Relevant docs were updated for behavior/contract changes.
- [ ] File references in docs match current implementation paths.
- [ ] Old or duplicated statements were removed.
- [ ] New risks/limitations introduced by the change were documented.
