# OTV2-20260811-dual-transport-closeout-repair

```yaml
task_id: OTV2-20260811-dual-transport-closeout-repair
title: Close final dual-transport architecture review findings
mode: REPAIR
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260811-dual-transport-closeout-repair
pr: 149
base_sha: 9794e9a6307b6f9db193ca2ce08607eb065b7d7e
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture closeout coordinator
created_at: 2026-08-11T00:35:00+02:00
updated_at: 2026-08-11T01:30:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - README.md
  - docs/agents/tasks/active/OTV2-20260807-protocol-contract-reconciliation.md
  - docs/agents/tasks/archive/OTV2-20260807-protocol-contract-reconciliation.md
  - docs/agents/tasks/active/OTV2-20260811-dual-transport-closeout-repair.md
  - docs/agents/tasks/archive/OTV2-20260810-architecture-review-dual-transport.md
  - docs/agents/tasks/archive/OTV2-20260810-dual-transport-final-repair.md
  - docs/architecture/ADR-0014-dual-gameplay-transport-tcp-default-quic-opt-in.md
  - docs/architecture/ADR-0015-gamenode-implementation-shape-not-yet-frozen.md
  - docs/architecture/ADR-0016-gameplay-transport-client-mode-runtime-readiness.md
  - docs/architecture/ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md
  - docs/architecture/ARCHITECTURE_STATUS_MODEL.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/README.md
  - docs/contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json
public_contracts:
  - docs/architecture/ADR-0014-dual-gameplay-transport-tcp-default-quic-opt-in.md
  - docs/architecture/ADR-0015-gamenode-implementation-shape-not-yet-frozen.md
  - docs/architecture/ADR-0016-gameplay-transport-client-mode-runtime-readiness.md
  - docs/contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json
depends_on:
  - closed unmerged PR 145 at 9bf162e9d78f41706e92253c41f36d745e33382e
  - closed unmerged PR 148 at c36e0eb1127c5689a23ea810c802766fe79d8050
  - merged governance PR 146
  - merged FND-ID lifecycle cleanup PR 147
blocks:
  - final merge of the owner-accepted architecture and dual-transport documentation programme
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

Deliver the inherited architecture package and close the bounded review findings without authorizing runtime implementation. The final package must keep one `protocol-oteryn`, preserve ADR-0009's `GameNode = one game-server process` identity, treat the modular-monolith shape only as a nonbinding internal-decomposition starting hypothesis, and keep every gameplay transport mode unavailable until separately implemented and proven.

## Architecture and source of truth

- `PROVEN`: original trusted package base is `main@9794e9a6307b6f9db193ca2ce08607eb065b7d7e`.
- `PROVEN`: current strict base after branch synchronization is `main@81db47966d76709a0e44dfbf1bc3979f38a24ffa`.
- `PROVEN`: PR #148 exact head `c36e0eb1127c5689a23ea810c802766fe79d8050` passed its exact-head checks/self-review; final independent review `4901458913` found the two bounded findings inherited here.
- `PROVEN`: PR #148 is closed unmerged after exhausting repair budget `3/3`.
- `PROVEN`: PR #149 repair cycle 1 closed the status-overlay topology ambiguity.
- `PROVEN`: PR #149 repair cycle 2 preserved ADR-0009's one-process GameNode boundary and aligned `docs/architecture/README.md` with that scope.
- `PROVEN`: post-sync head `747a68dd009ace5757f0354c75588f815bc84a96` was strictly up to date with main (`behind_by=0`), had the intended 14-path diff, passed self-review `4901701858`, Agent Governance `31442112915`, Dependency Review `31442112903`, and CodeQL `31442112916`.
- `PROVEN`: required independent review on `747a68dd...` found one final bounded P2: ADR-0014/0015/0016 were accepted but missing from `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`, contrary to that register's own decision discipline.
- `DERIVED`: repair cycle 3 must be limited to registering ADR-0014/0015/0016 globally, removing the stale implication that QUIC still belongs to the generic advanced-scaling gate, and updating this task ownership/evidence. No fourth repair is permitted in this task.
- `OWNER_ACCEPTED`: transport/admission/security invariants remain unchanged: one `protocol-oteryn`; TCP+TLS profile 1 is architecture registration/default intent only; QUIC is future opt-in and blocked on profile/FND-04/ordering/resource/fault/measured-benefit/implementation evidence; Gateway-only Game Login Ticket redemption; no cross-profile grant reuse; fail-closed fallback; no 0-RTT/DATAGRAM baseline.

## Acceptance criteria

- [x] Make the modular-monolith GameNode shape a nonbinding preferred starting hypothesis rather than frozen internal decomposition.
- [x] Preserve ADR-0009's accepted one-process GameNode identity/process/container boundary.
- [x] Keep later evidence-driven decisions open only for internal module/crate decomposition and genuinely separate adjacent services.
- [x] Make `TCP_ONLY` and all gameplay transport client modes runtime-unavailable until implemented and proven.
- [x] Add ADR-0015/0016 to the canonical architecture index with explicit narrow precedence.
- [x] Archive/rotate exhausted PR #148 task and close PR #148 unmerged.
- [x] Include the status-overlay modular-monolith sentence in ADR-0015's narrow supersession scope.
- [x] Register accepted ADR-0014/0015/0016 in `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` and keep ADR-0009/transport-readiness precedence explicit.
- [ ] Final repair-cycle-3 diff contains only declared documentation/task/contract paths and preserves reviewed invariants.
- [ ] Mandatory repair-cycle-3 exact-head self-review passes with zero material findings.
- [ ] Repair-cycle-3 exact-head Agent Governance, Dependency Review and CodeQL pass.
- [ ] One clean required independent final review passes on the unchanged repair-cycle-3 head with zero material findings.
- [ ] Squash merge succeeds on the exact reviewed head.

## Excluded scope

- no runtime implementation or transport listener;
- no QUIC transport profile registration;
- no FND-04 grant implementation;
- no Platform repository write;
- no production/live activation;
- no change to ADR-0009's one-process GameNode identity;
- no broad cleanup of pre-existing stale programme prose outside the bounded global-register registration repair;
- no fourth repair cycle.

## Implementation / findings

- `ADR-0015` makes internal modular-monolith decomposition nonbinding while preserving ADR-0009's one-process GameNode identity.
- `ADR-0016` makes transport-profile registration and future client-mode vocabulary explicitly distinct from runtime availability.
- `PROTOCOL_OTERYN_TRANSPORT_POLICY.json` remains machine-readable current authority for runtime availability.
- `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` now records ADR-0014 through ADR-0016, binds them narrowly in decision discipline, and removes QUIC from the generic `EXP-SCALE-01` ownership bucket in favor of ADR-0014/`NET-TRANSPORT-02`.
- Repair cycles 1-3 are bounded review repairs only; none adds runtime implementation or changes Platform/production authority.

## Validation

### Focused

- original trusted package base: `9794e9a6307b6f9db193ca2ce08607eb065b7d7e`
- strict current-main base after synchronization: `81db47966d76709a0e44dfbf1bc3979f38a24ffa`
- repair-cycle-1 exact-head checks: PASS before superseding review repair
- repair-cycle-2/post-sync exact-head checks: PASS before final register finding
- repair-cycle-3 full current-main diff: pending frozen head

### Component/integration

- result: `NOT_APPLICABLE` — documentation/contract repair only

### E2E

- result: `NOT_APPLICABLE` — no executable runtime behavior

### Exact-head CI

- final head: pending after repair-cycle-3 checkpoint commit
- trigger source: pull_request/synchronize
- result: pending

## Self-review

- exact head: pending repair-cycle-3 head
- material findings: global-register omission repaired in cycle 3
- verdict: pending

## Independent review

- required: `YES` — inherited package contains high-risk transport/admission/security architecture
- auditor: one genuinely independent final reviewer; Codex only because this gate genuinely requires independence and is available
- earlier PR #149 reviews: bounded P2 findings repaired in cycles 1-3
- final verdict: pending; zero material findings required and no further repair is permitted in this task

## PR and closeout

- changed-file review: pending repair-cycle-3 head
- unresolved review thread: global-register P2 to resolve after repaired content is visible and self-review confirms the scope
- related/superseded PRs: PR #145 and PR #148 closed unmerged/rotated
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Repair cycle 3 registers ADR-0014/0015/0016 in the global architecture decision register and removes the stale generic QUIC scaling ownership implication.
status: validating
branch: docs/OTV2-20260811-dual-transport-closeout-repair
head_sha: null
pr: 149
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: repair-3-head-pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Freeze repair-cycle-3 PR #149 head, run exact-head self-review and CI, resolve the global-register finding, then obtain one clean final independent review; squash-merge unchanged head if clean. No further content repair is permitted in this task.
```
