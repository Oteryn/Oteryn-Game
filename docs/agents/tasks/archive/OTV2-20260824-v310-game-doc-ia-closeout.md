# OTV2-20260824-v310-game-doc-ia-closeout

```yaml
task_id: OTV2-20260824-v310-game-doc-ia-closeout
title: Close Game v3.10 Documentation and Agent IA gaps
mode: GOVERNANCE
status: merge_conditioned_closeout
repository: Oteryn/Oteryn-Game
base_branch: main
branch: audit/101-v310-game-doc-ia-game-gate
issue: 101
pr: 104
base_sha: 7dd412b6dc6e493e18cc4ad6ca230e5a6cfbb563
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chatgpt-gpt-5.6-sol
created_at: 2026-08-24T18:52:56+02:00
updated_at: 2026-08-24T19:24:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/HANDOVER_LIFECYCLE.json
  - docs/agents/reports/OTERYN_V310_GAME_DOC_IA_CURRENT_HEAD_AUDIT.md
  - docs/agents/tasks/active/OTV2-20260824-v310-game-doc-ia-closeout.md
  - docs/agents/tasks/archive/OTV2-20260824-v310-game-doc-ia-closeout.md
  - docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md
  - docs/agents/tasks/active/OTV2-20260807-disconnect-forensic-evidence-analysis.md
  - docs/agents/tasks/active/OTV2-20260807-lag-disconnect-protection-analysis.md
  - docs/agents/tasks/active/OTV2-20260818-implementation-coordinator.md
  - docs/agents/tasks/active/OTV2-20260822-impl-qa-e2e.md
  - docs/agents/tasks/archive/OTV2-20260805-foundation-preimplementation-contracts.md
  - docs/agents/tasks/archive/OTV2-20260807-disconnect-forensic-evidence-analysis.md
  - docs/agents/tasks/archive/OTV2-20260807-lag-disconnect-protection-analysis.md
  - docs/agents/tasks/archive/OTV2-20260818-implementation-coordinator.md
  - docs/agents/tasks/archive/OTV2-20260822-impl-qa-e2e.md
  - tools/agents/validate_governance.py
  - tools/agents/tests/test_validate_governance_lifecycle.py
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: OTERYN-ORG-AUDIT-v3.10
external_repositories: []
```

## Outcome

Close only the Game-owned v3.10 Documentation/Agent IA gaps and prove the stable `game-gate` state without changing product/runtime behavior or unrelated repository settings.

## Architecture and source of truth

- `PROVEN`: refreshed audited protected `main` is `7dd412b6dc6e493e18cc4ad6ca230e5a6cfbb563`.
- `PROVEN`: Issue #101 and Draft PR #104 are the owning GitHub lifecycle records for this bounded closeout.
- `PROVEN`: repository ruleset `20991995` requires strict `game-gate`; merged PR #99 emitted `game-gate=SUCCESS` on exact head `da5bc4fbd5b6635a50642bdd118a99c9e7fc2c17`.
- `PROVEN`: Issue #100 completed through PR #103 / merge `a431ec9390759e28c6cb543b8228e4882ee07652`; intervening `main@7dd412b6dc6e493e18cc4ad6ca230e5a6cfbb563` already archived that task projection, so this PR preserves that concurrent closeout unchanged.

## Acceptance criteria

- [x] Current-head Game Documentation/Agent IA inventory is recorded with exact audited SHA.
- [x] Retained prompt lifecycle metadata is deterministic and validated.
- [x] Active task packets correspond to live owning Issue/PR state; terminal/stale packets are archived.
- [x] Retained handovers are explicitly non-authoritative with expiry/supersession semantics.
- [x] Game operations/release taxonomy receives evidence-backed `NOT_NEEDED` or a demonstrated canonical artifact.
- [x] Existing provider governance validation covers the new lifecycle invariants without a new external check identity.
- [ ] Exact final diff, exact-head CI/review state, squash merge and branch cleanup are verified.

## Excluded scope

No runtime/gameplay/protocol/content/deployment/dependency/runner changes; no unrelated workflow, repository-setting or organization-setting change; no implementation work from Issue #100; no writes outside `Oteryn/Oteryn-Game`.

## Validation

Pre-freeze validation evidence:

- TDD RED: lifecycle regression suite failed in all 3 tests because the three lifecycle validator functions did not exist.
- TDD GREEN: `python tools/agents/tests/test_validate_governance_lifecycle.py` ? 3/3 PASS.
- Existing provider validator: `python tools/agents/validate_governance.py` ? PASS (`25` required policy documents / `9` project lanes) on the remediated branch before final freeze.
- Runtime/gameplay E2E: `NOT_APPLICABLE` for this documentation/governance-only task.
- Exact final diff/exact-head CI/review/merge evidence: required after the final archive commit; GitHub remains authoritative.

## Context checkpoint

```yaml
last_progress: lifecycle remediation implemented; pre-freeze tests and provider governance validation pass
status: merge_conditioned_closeout
branch: audit/101-v310-game-doc-ia-game-gate
head_sha: null
pr: 104
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
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
next_action: freeze the final PR head and verify exact diff, exact-head CI, reviews, squash merge and branch deletion
```

## Lifecycle disposition (v3.10 closeout)

This archive placement is merge-conditioned: on the PR branch, Issue #101 and PR #104 remain the live authority until merge. If PR #104 squash-merges, this archived packet becomes the terminal historical projection atomically with the remediation; the exact final head and merge commit stay in immutable GitHub PR/check evidence rather than creating a self-referential follow-up commit. If PR #104 does not merge, this archive record is not terminal authority.
