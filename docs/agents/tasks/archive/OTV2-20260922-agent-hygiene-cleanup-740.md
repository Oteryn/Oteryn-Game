> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #741 merged on 2026-09-22 as protected `8ad99922d7140016df2de38bc3b6ae3a8d4edc1e`. Real Merge Queue gate run `35696735029` succeeded. Post-merge Agent Governance run `35697371175` failed only because this terminal task packet still remained in `tasks/active`; this archive move is the bounded corrective closeout. Any prior nonterminal checkpoint wording below is historical provenance only.

# OTV2-20260922-agent-hygiene-cleanup-740

```yaml
task_id: OTV2-20260922-agent-hygiene-cleanup-740
title: Finish lifecycle and context hygiene cleanup
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/agent-hygiene-cleanup-740
issue: 740
pr: 741
base_sha: 40e9d723392b8fc1be652bdbb4b6d31b6729867b
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT
created_at: 2026-09-22T07:20:00+02:00
updated_at: 2026-09-22T07:20:00+02:00
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/**
  - docs/agents/tasks/archive/**
  - docs/agents/evidence/**
  - docs/agents/programs/**
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/tasks/TASK_TEMPLATE.md
  - tools/agents/validate_governance.py
  - tools/agents/tests/**
public_contracts:
  - Game agent lifecycle and hot-path context hygiene
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Remove proven terminal records from the active agent hot path, make lifecycle vocabulary machine-enforced, bound active task context size, compact the still-live Server Seam task without losing provenance, and establish evidence-based programme hygiene.

## Architecture and source of truth

- PROVEN: protected admission base is `40e9d723392b8fc1be652bdbb4b6d31b6729867b`.
- PROVEN: Issue #740 records the fresh audit findings and bounded cleanup scope.
- PROVEN: current push Agent Governance run `35661567246` fails because active Child B packet names merged canonical PR #335.
- PROVEN: Issue #247 remains open and its latest control-plane checkpoint keeps Server Seam `WAITING_DEPENDENCY` with release gate `WP4_PROTECTED && WP5_G0_READINESS_PROVEN`.
- PROVEN: live GitHub state outranks historical task/programme prose.

## High-risk authority/recovery qualification

NOT_APPLICABLE — documentation/governance lifecycle cleanup only; no runtime, protocol, persistence, production, account/session authority or external-repository mutation.

## Acceptance criteria

- [x] terminal #329/#335 packet is archived and absent from `tasks/active`;
- [x] every known noncanonical active status found by the audit is normalized and the validator now enforces the contract vocabulary;
- [x] governance validator enforces active status/mode and bounded active-task size;
- [x] Server Seam active packet is current-state-only and full prior body is retained under evidence;
- [x] programme cleanup archives only directly proven terminal/superseded records and registers them in `PROGRAM_LIFECYCLE.json`;
- [x] exact-head PR #741 qualification and real Merge Queue gate passed before protected integration; post-merge push failure was lifecycle-only because this terminal packet still remained under `tasks/active`.

## Excluded scope

No runtime/gameplay/protocol/persistence behavior, architecture decision, contract semantics, production environment, branch protection or merge primitive changes. No deletion of historical evidence. No programme retirement inferred only from age or filename.

## Context checkpoint

```yaml
last_progress: >-
  PR #741 integrated as protected `8ad99922d7140016df2de38bc3b6ae3a8d4edc1e`; this closeout archives the terminal task packet that caused post-merge Agent Governance run 35697371175 to fail
status: completed
branch: docs/agent-hygiene-cleanup-740
head_sha: null
pr: 741
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: freeze exact PR #741 head and require lifecycle/governance/META/repository exact-head validation
```
