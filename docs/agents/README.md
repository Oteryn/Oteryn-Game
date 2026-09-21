# Oteryn Game agent governance

This directory contains routed procedures and retained coordination evidence for `Oteryn/Oteryn-Game`. The preserved `blakinio/Oteryn-v2` repository is legacy/migration provenance and is read-only by default.

## Procedure catalog

This list is an index, not a mandatory reading bundle. Start with root and nearest instructions, then load only entries whose operation or domain applies.

Shared execution, authority/trust, recovery, checkpoint/handoff and communication semantics come from the bound META policy plus root and nearest `AGENTS.md`. They are intentionally not copied into Game-local procedure files.

- `AGENTS.md` — rules for this directory and task records.
- `REPOSITORY_MAP.md` — current/planned repository layout and source-of-truth boundaries.
- `CONTEXT_ROUTING.md` — which documents to load for each task class.
- `BUILD_TEST_MATRIX.md` — validation selection.
- `ANTI_STALL_AND_EXECUTION_BUDGET.md` — bounded autonomous execution.
- `AUTONOMOUS_PROGRAM_CONTINUATION.md` — programme/coordinator continuation.
- `DELIVERY_COMPLETENESS_AND_CLOSEOUT.md` — completion and merge gate.
- `GITHUB_ONLY_EXECUTION.md` — GitHub/Actions fallback execution.
- `PROMPTING_STANDARD.md`, `PROMPTING_HANDOVER.md`, `PROMPT_EVAL_STANDARD.md` — prompt quality and handover.
- `END_TO_END_FEATURE_COMPLETENESS.md` — end-to-end feature acceptance.
- `CROSS_REPO_CONTRACTS.md` — Oteryn Platform/Otheryn/otclient migration boundaries.
- `GOVERNANCE_CONTRACT.json`, `PROJECT_LANES.json` — machine-readable policy.

## Reusable programme prompts

- `prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md` — autonomous coordinator prompt for continuing the staged global architecture decision programme from the canonical foundation checkpoint and global decision register.
- `prompts/OTV2_INDEPENDENT_PROGRAMME_ARCHITECTURE_AUDIT.md` — independent read-only programme and architecture audit prompt for exact-SHA, phase-aware review of Game direction, active workstreams, evidence, legacy contamination and the next evidence-producing milestone; short invocation: `Oteryn: audyt`.
- `prompts/OTV2_WP3_WRITER.md` — single mutating WP3 SQLx-driver-accounting writer; short invocation: `Oteryn: wp3 writer`.
- `prompts/OTV2_WP3_TLS_AUDITOR.md` — strict read-only TLS allocation/custody auditor for the exact live WP3 candidate; short invocation: `Oteryn: wp3 tls audit`.
- `prompts/OTV2_WP3_QUALIFICATION_AUDITOR.md` — strict read-only WP3 qualification/integration-prerequisite auditor; short invocation: `Oteryn: wp3 qualification audit`.
- `programs/OTERYN_WP3_MULTIAGENT_LAUNCH_RUNBOOK_20260910.md` — one-writer + two-read-only launch order, packet relay and recommended model/effort guidance for the current WP3 lineage.

Reusable prompts are task deltas, not project state. Apply current authority, task checkpoints, ADRs/contracts and live PR/CI state when they are material to the requested operation.

## Task lifecycle

Substantial work uses:

- `tasks/active/OTV2-YYYYMMDD-short-slug.md` while active;
- `tasks/archive/` after terminal completion;
- `tasks/TASK_TEMPLATE.md` as the required template.

Do not use chat history as project state. A replacement agent must be able to continue from Git, the task checkpoint and live PR/CI state.

## Bootstrap note

The repository is greenfield. Planned code paths in `REPOSITORY_MAP.md` are not proof that those paths already exist. Agents must inspect the exact tree before selecting commands or claiming implementation state.
