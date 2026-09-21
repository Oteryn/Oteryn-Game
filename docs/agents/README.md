# Oteryn Game agent governance

This directory contains routed procedures and retained coordination evidence for `Oteryn/Oteryn-Game`. The preserved `blakinio/Oteryn-v2` repository is legacy/migration provenance and is read-only by default.

## Procedure catalog

This list is an index, not a mandatory reading bundle. Start with root and nearest instructions, then load only entries whose operation or domain applies.

- `AGENTS.md` — rules for this directory and task records.
- `REPOSITORY_MAP.md` — current physical repository layout and source-of-truth boundaries.
- `CONTEXT_ROUTING.md` — which documents to load for each task class.
- `CONTEXT_HANDOFF.md` — durable task checkpoints and resume procedure.
- `BUILD_TEST_MATRIX.md` — validation selection.
- `ANTI_STALL_AND_EXECUTION_BUDGET.md` — bounded autonomous execution.
- `AUTONOMOUS_PROGRAM_CONTINUATION.md` — programme/coordinator continuation.
- `DELIVERY_COMPLETENESS_AND_CLOSEOUT.md` — completion and merge gate.
- `GITHUB_ONLY_EXECUTION.md` — GitHub/Actions fallback execution.
- `SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md` — durable recovery and waiting state.
- `TERMINAL_ONLY_COMMUNICATION.md` — low-noise autonomous communication.
- `TRUST_AND_CONTEXT_BOUNDARIES.md` — authority and evidence boundaries.
- `PROMPTING_STANDARD.md`, `PROMPTING_HANDOVER.md`, `PROMPT_EVAL_STANDARD.md` — prompt quality and handover.
- `EXECUTION_PROTOCOL.md`, `END_TO_END_FEATURE_COMPLETENESS.md`, `TASK_CLOSEOUT_AUDIT_E2E.md` — execution and acceptance.
- `CROSS_REPO_CONTRACTS.md` — Oteryn Platform/Otheryn/otclient migration boundaries.
- `GOVERNANCE_CONTRACT.json`, `PROJECT_LANES.json` — machine-readable policy.

## Reusable programme prompts

- `prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md` — autonomous coordinator prompt for continuing the staged global architecture decision programme from the canonical foundation checkpoint and global decision register.
- `prompts/OTV2_INDEPENDENT_PROGRAMME_ARCHITECTURE_AUDIT.md` — independent read-only programme and architecture audit prompt for exact-SHA, phase-aware review of Game direction, active workstreams, evidence, legacy contamination and the next evidence-producing milestone; short invocation: `Oteryn: audyt`.
- `prompts/OTV2_ASTRA_WP3_V2_PROGRAMME_COORDINATOR.md` — current upstream-first WP3-v2 programme routing; short invocation: `Oteryn: astra wp3-v2 programme coordinator`.
- `prompts/OTV2_ASTRA_WP3_V2_IMPLEMENTATION_LEAD.md` — current allocated upstream-first WP3-v2 implementation profile; short invocation: `Oteryn: astra wp3-v2 implementation lead`.
- `prompts/OTV2_SOL_WP3_V2_EVIDENCE_AUDITOR.md` — current read-only exact-source/evidence support; short invocation: `Oteryn: sol wp3-v2 evidence auditor`.
- `programs/OTV2_WP3_V2_AGENT_LAUNCH_RUNBOOK.md` — current WP3-v2 launch order and gates.

Reusable prompts are task deltas, not project state. Apply current authority, task checkpoints, ADRs/contracts and live PR/CI state when they are material to the requested operation.

## Task lifecycle

Substantial work uses:

- `tasks/active/OTV2-YYYYMMDD-short-slug.md` while active;
- `tasks/archive/` after terminal completion;
- `tasks/TASK_TEMPLATE.md` as the required template.

Do not use chat history as project state. A replacement agent must be able to continue from Git, the task checkpoint and live PR/CI state.

## Bootstrap note

The repository contains active implementation. Plans remain non-authoritative for physical paths; inspect the exact tree and live lifecycle state before selecting commands or claiming implementation status.
