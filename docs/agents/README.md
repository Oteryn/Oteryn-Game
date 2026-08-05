# Oteryn v2 agent governance

This directory is the durable operating system for autonomous and multi-agent work in `blakinio/Oteryn-v2`.

## Required core

- `AGENTS.md` — rules for this directory and task records.
- `REPOSITORY_MAP.md` — current/planned repository layout and source-of-truth boundaries.
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

Reusable prompts are execution contracts, not project state. Agents must still read trusted-base governance, live task checkpoints, current ADRs/contracts and live PR/CI state before acting.

## Task lifecycle

Substantial work uses:

- `tasks/active/OTV2-YYYYMMDD-short-slug.md` while active;
- `tasks/archive/` after terminal completion;
- `tasks/TASK_TEMPLATE.md` as the required template.

Do not use chat history as project state. A replacement agent must be able to continue from Git, the task checkpoint and live PR/CI state.

## Bootstrap note

The repository is greenfield. Planned code paths in `REPOSITORY_MAP.md` are not proof that those paths already exist. Agents must inspect the exact tree before selecting commands or claiming implementation state.
