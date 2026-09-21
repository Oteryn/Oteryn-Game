> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #721 exact candidate `72f035d4dd7d22058cfbb0b6f593d6514ecb73f6` passed real Merge Queue run `35615585400` with aggregate `game-gate` job `106388137988` SUCCESS and integrated as `29c21553f919bd143e6b60264a7381b66bab2fa9`. Protected `main@5a28b53108b5a9e6c570e6dd3dbb2e9bad0a9555` contains the accepted cleanup chain. Any nonterminal/checkpoint wording below is historical provenance only.

# OTV2-20260921-agent-prompt-cleanup-715

```yaml
task_id: OTV2-20260921-agent-prompt-cleanup-715
title: Retire Terra and slim remaining broad-startup prompts
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/agent-prompt-cleanup-715
issue: 715
pr: 721
base_sha: 256aa3b152c944cb8451906effe1f0090c5b798d
head_sha: 72f035d4dd7d22058cfbb0b6f593d6514ecb73f6
final_head_sha: 72f035d4dd7d22058cfbb0b6f593d6514ecb73f6
final_head_frozen_at: null
owner: ChatGPT
created_at: 2026-09-21T12:24:24Z
updated_at: 2026-09-21T13:45:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/prompts/README.md
  - docs/agents/prompts/OTV2_TERRA_GAME_CONTROL_PLANE.md
  - docs/agents/prompts/OTV2_REFERENCE_INVESTIGATOR.md
  - docs/agents/prompts/OTV2_OWNER_EXECUTION_STATUS_ADVISOR.md
  - docs/agents/prompts/OTV2_CONTENT_WORLD_INDEPENDENT_AUDIT.md
  - docs/agents/prompts/OTV2_DEFECT_DISCOVERY_SUPERVISOR.md
  - docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
  - docs/agents/prompts/OTV2_INDEPENDENT_PROGRAMME_ARCHITECTURE_AUDIT.md
  - docs/agents/prompts/OTV2_SOL_EXECUTION_ARCHITECTURE_CONTINUATION.md
  - docs/agents/prompts/OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR.md
  - docs/agents/programs/OTERYN_GAME_AGENT_OPERATOR_RUNBOOK.md
  - docs/agents/programs/OTERYN_V2_TERRA_SOL_EXECUTION_SCHEDULER.md
  - docs/agents/programs/OTERYN_REFERENCE_INVESTIGATION_OPERATOR_RUNBOOK_20260910.md
  - docs/agents/programs/OTERYN_GLOBAL_REFERENCE_FIRST_AGENT_LAUNCH_LINES_20260909.md
  - docs/agents/programs/OTV2_RUNTIME_ACTOR_CARRIER_PREPRODUCTION_ALLOCATION_20260911.md
  - tools/agents/tests/test_meta_agent_policy_adoption.py
  - docs/agents/tasks/active/OTV2-20260921-agent-prompt-cleanup-715.md
public_contracts:
  - reusable prompt dispatchability and control-plane routing
depends_on:
  - pr:712
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Remove the obsolete second control-plane choice, keep Work as the single active/reusable Game control plane, and make the remaining owner/reference/audit prompts resolve only target-specific current context by default.

## Proven basis

- #162 durable task selects `OTV2_WORK_DELIVERY_COORDINATOR` as the active control plane.
- Terra is reusable but inactive and has no unique mutation authority; scheduler/runbooks already forbid it as a concurrent coordinator.
- Terra exists primarily as the 2026-08-27 execution configuration for Terra High + Sol leads.
- #712 established targeted context routing and ordinary alias reuse without full registry/prompt-eval reconstruction.
- Issue #715 records the remaining broad-startup hotspots.

## Acceptance

- [x] Terra lifecycle entry is retired/non-reusable (`1.2`) and superseded by Work for operational control-plane dispatch.
- [x] Historical Terra prompt/design remains provenance with an explicit DO NOT DISPATCH banner.
- [x] Operator runbook and retained scheduler expose one mutating control plane: Work.
- [x] Reference Investigator resolves requested lane before loading lane-specific sources.
- [x] Owner/status and Content audit aliases resolve only their selected lifecycle entry, not the full registry.
- [x] Ordinary supervisor/coordinator startup is target/allocated-worker bounded; explicit whole-programme audit carries a documented breadth exception.
- [x] Per-invocation prompt-eval reads are removed where the task is not prompt authoring/evaluation.
- [x] META adoption regressions cover retired Terra, Work-only scheduling and targeted startup.
- [ ] Exact-head governance/META/merge-gate validation passes.

## Excluded

No runtime/gameplay/protocol/persistence/production/external-repository mutation. No mass retirement of reusable executor templates. No change to Sol lane technical authority.

## Context checkpoint

```yaml
last_progress: >-
  retired Terra dispatchability, simplified the Work + Sol scheduler/runbook, bounded remaining
  broad-startup prompts, reconciled all current runbook references to Terra, repaired the Work Auditor selector residue and
  prompt-version drift found by independent review, and added Work-only/version-sync regression coverage
status: validating
branch: docs/agent-prompt-cleanup-715
head_sha: null
pr: null
next_action: run final exact-head CI and one final independent re-review; mark Ready only if both are clean
```


## Terminal closeout

- issue: #715
- delivery PR: #721
- frozen exact head: `72f035d4dd7d22058cfbb0b6f593d6514ecb73f6`
- real Merge Queue run: `35615585400`
- merge-group aggregate `game-gate`: job `106388137988` — SUCCESS
- integrated commit: `29c21553f919bd143e6b60264a7381b66bab2fa9`
- final protected-main readback for this cleanup chain: `5a28b53108b5a9e6c570e6dd3dbb2e9bad0a9555`
- ownership: released
