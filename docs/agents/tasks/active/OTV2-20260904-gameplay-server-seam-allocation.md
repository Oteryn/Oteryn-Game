# OTV2-20260904-gameplay-server-seam-allocation

```yaml
task_id: OTV2-20260904-gameplay-server-seam-allocation
title: Allocate production gameplay Server Seam
mode: COORDINATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/gameplay-server-seam-allocation-247
pr: 294
issue: 247
parent_coordinator_issue: 162
preparation_issue: 96
preparation_pr: 117
preparation_merge_sha: 4079804b7f1f29cc2b7db2e746d4da2861bff084
base_sha: 68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705
head_sha: null
final_head_sha: null
final_head_frozen_at: null
control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
owner: ChatGPT Work Delivery Coordinator
lane_id: OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
worker_alias: "Oteryn: sol server seam lead"
worker_task_id: OTV2-20260904-gameplay-server-seam
worker_branch: agent/otv2-gameplay-server-seam-01
worker_pr: null
created_at: 2026-09-04T19:27:00+02:00
updated_at: 2026-09-04T19:27:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260904-gameplay-server-seam-allocation.md
  - docs/agents/tasks/active/OTV2-20260904-gameplay-server-seam.md
  - docs/superpowers/plans/2026-08-24-oteryn-production-gameplay-server-seam.md
public_contracts:
  - OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM allocation authority
  - serialized Server Seam shared-path ownership
  - accepted production gameplay server-seam plan from PR #117
runtime_write_authority: none
production_authority: none
depends_on:
  - issue: 247
  - issue: 162
  - merged_pr: 117
  - merged_pr: 151
  - merged_pr: 252
  - merged_pr: 290
blocks:
  - OTV2-20260904-gameplay-server-seam
external_repositories: []
```

## Outcome

Create one fresh exact-base, docs-only allocation for the accepted production gameplay Server Seam. Only after this allocation itself passes all required gates, merges through protected repository controls and Work reads the merge SHA back from protected `main` may one Sol worker receive runtime/shared-path write authority.

This PR changes no runtime behavior, wire bytes, durable/session state, Cargo dependency, workflow/protection, production setting, secret or external repository.

## Architecture and source of truth

- `PROVEN` — allocation admission `main` is `68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705`.
- `PROVEN` — Issue #162 is the live Work coordinator lifecycle and its latest durable checkpoint names a fresh Server Seam allocation as the deterministic next action after Durability closeout.
- `PROVEN` — Issue #247 is open and classified `READY_FOR_FRESH_ALLOCATION`; the Issue itself grants no implementation authority.
- `PROVEN` — preparation #96 / PR #117 merged as `4079804b7f1f29cc2b7db2e746d4da2861bff084` and defines the exact conditional Server Seam topology, leases, negative tests and evidence boundary.
- `PROVEN` — the #117 FND-04 prerequisite is terminal: archived task `OTV2-20260825-fnd04-verifier-consumer` records PR #151 merged as `2d0e951ce37c2e28773c22966bb816c00bebaa0a` with released ownership.
- `PROVEN` — Issue #116 is closed and current `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` contains the accepted Server Seam NET03 ceilings: pre-admission 256, handshake/auth 64, outbound entries 64/session, outbound bytes 1,048,576/session, pending writes 8/session and drain tasks 256/batch.
- `PROVEN` — Durability is terminal: current coordinator checkpoints record implementation PR #252 merged as `b67f4425e9e9c5bbf9f7bc94c422cd7478edcdd3`, archival PR #290 merged as `47faa84152ffdcac00d8e2173d582aa54be2cbcf`, and ownership released.
- `PROVEN` — current `apps/game-server/src/main.rs` has no gameplay listener; `--smoke` remains the only successful bootstrap mode and normal gameplay execution fails closed.
- `PROVEN` — current `apps/game-server/src/foundation/protocol.rs` owns inbound FND-02 wire validation; the accepted #117 decision intentionally leases that exact path for only the missing typed bootstrap/resume bridge and registered server encoders.
- `PROVEN` — current open non-Dependabot PR scan found no active task-owned Server Seam runtime/shared path. #287/#291/#293 are workflow/governance/docs surfaces; #289 owns Durability/Foundation recovery paths but not the allocated protocol/composition/Cargo paths.
- `PROVEN` — Dependabot #259/#260/#261 each touch root `Cargo.toml`/`Cargo.lock` but have no task/allocation ownership. They remain future integration candidates and are explicitly serialized/re-read by the worker.
- `CONFLICT` — `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md` contains stale release-candidate prose referring to nonexistent PR #314. Newer Issue #162/#247 checkpoints, the current scheduler, reusable Work lifecycle and merged #117 decision are the governing live sources. This unrelated stale prose is not repaired by this allocation.
- `DERIVED` — all accepted #117 prerequisites are terminal on the admission base and no active non-Dependabot writer owns the exact Server Seam allowlist, so one serialized lane is allocatable after this PR merges/readback.

Accepted technical authority remains:

- merged #117 preparation decision;
- FND-02 wire contract/registry;
- FND-03 runtime/resource contract and current NET03 registry rows;
- FND-04 verifier/admission/GameSession/CharacterLease/reconnect/fencing authority;
- current Durability journal/replacement/reconciliation implementation;
- QA-E2E contracts, including evidence ownership.

A lower implementation artifact may not widen those decisions.

## Evidence ownership correction from coordinator self-review

The accepted #117 packet distinguishes two claims:

1. the Server Seam implementation MUST traverse the actual production listener/composition path in real local TCP/TLS integration tests; and
2. formal ADR-0007 QA Tier 1/Tier 2 remains owned by a separately allocated QA lane after the seam is merged.

Therefore the worker must prove **Server Seam physical integration** for its exact candidate, while reporting `QA Tier 1: NOT_EVALUATED` and `QA Tier 2: NOT_EVALUATED`. It must not label its implementation test as final QA Tier 1 `PROVEN`.

The same self-review aligned the child plan with #117's literal mandatory evidence for golden/cross-oracle server encoding, wrong protocol/profile, invalid/expired/replayed/wrong-binding admission material, concurrent fresh-admission replay, post-admission unsupported gameplay, and non-silent shutdown handling of already-authoritative reserved work.

## Allocation decision

Only after PR #294 merges and Work reads the exact merge SHA from protected `main`, release:

```text
lane_id: OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
task_id: OTV2-20260904-gameplay-server-seam
worker: Oteryn: sol server seam lead
branch: agent/otv2-gameplay-server-seam-01
```

The branch MUST be created from exactly the allocation merge SHA, never from this admission SHA or a cached chat SHA.

### Primary worker paths after allocation merge

- `apps/game-server/src/gameplay_transport/mod.rs`
- `apps/game-server/src/gameplay_transport/tcp_tls.rs`
- `apps/game-server/src/gameplay_transport/connection.rs`
- `apps/game-server/tests/gameplay_server_seam.rs`

### Serialized shared paths after allocation merge

- `apps/game-server/src/foundation/protocol.rs`
- `apps/game-server/src/lib.rs`
- `apps/game-server/src/main.rs`
- `apps/game-server/Cargo.toml`
- `Cargo.toml`
- `Cargo.lock`

Worker-governance metadata:

- `docs/agents/tasks/active/OTV2-20260904-gameplay-server-seam.md`

The shared lease authorizes only changes required by the accepted Server Seam consumer. It does not authorize unrelated dependency upgrades, workspace restructuring, stable registry edits, Durability redesign or general Foundation refactoring.

## Dependabot reconciliation

Before first Cargo mutation and before final integration the worker MUST re-read #259/#260/#261 plus all active non-Dependabot ownership. It must not merge bot branches into the lane for convenience. If a bot/shared predecessor merges first, reconcile from fresh protected `main` using normal history-preserving integration and rerun invalidated qualification.

## Mandatory execution order

1. Keep this allocation PR docs-only.
2. Qualify its stable exact head with governance/repository/architecture/merge gates, Work whole-diff self-review and one genuinely independent deep review because this allocation grants later high-risk authority.
3. Integrate only a clean, current, expected-head candidate through protected controls.
4. Read the allocation merge SHA from protected `main`.
5. Create the worker branch from exactly that SHA and release one Sol writer.
6. Worker starts with fresh Foundation/framing RED; it does not begin by opening a listener.
7. Worker preserves Foundation/Durability authority, proves real production-path TCP/TLS Server Seam integration without claiming QA Tier 1/Tier 2 completion, completes authority-family qualification, exact-head CI and independent review, and returns `READY_FOR_INTEGRATION` without self-merging.
8. Work independently verifies/integrates the worker only when every predicate remains proven.
9. After Server Seam merge/readback, recompute Client and separately allocated QA readiness from fresh protected `main`; do not pre-release them here.

## Acceptance criteria

- [ ] PR #294 changes exactly these three docs paths and no runtime/Cargo/workflow/registry path.
- [ ] Admission base remains the recorded provenance SHA; if `main` advances before integration, current repository policy is followed and invalidated checks are rerun.
- [ ] #115/#116/Durability prerequisites remain terminal at final integration preflight.
- [ ] No active non-Dependabot writer owns any exact primary/shared Server Seam path.
- [ ] Primary/shared path lease equals the accepted #117 topology and excludes `workspace-boundaries.toml`, stable registries, workflows and architecture contracts.
- [ ] Worker task binds `AuthorityInvariant × ConsumerBoundary × MutationOperator` and independently current facts before runtime authority is released.
- [ ] Child plan requires fresh RED -> minimal GREEN, bounded untrusted framing, registered resource-boundary proof, FND-04 admission/reconnect/fencing, golden/cross-oracle encoding, replay/concurrency cases, fail-closed unsupported gameplay, bounded backpressure/drain and real production-path seam integration.
- [ ] Child plan explicitly reserves formal ADR-0007 QA Tier 1/Tier 2 evidence for separately allocated QA after the seam merge.
- [ ] No production endpoint/certificate/key/secret/deployment topology, gameplay IDs/semantics, Reference fact or Content-format decision is selected.
- [ ] Allocation stable exact head has green required checks, whole-diff self-review, one genuinely independent exact-head review, zero unresolved required threads and no unresolved actionable P0/P1.
- [ ] Worker branch is not created before protected-main allocation merge/readback.

## Excluded scope

No runtime/test runtime/Cargo/lockfile/Foundation implementation/registry/workspace policy/workflow/ruleset/production environment/secret/certificate/production endpoint/live account/session/data/Client/QA execution/Movement/Combat/Ability/Interaction/AI/Channel/Analytics/Content-format/Platform/Atlas/META or other external-repository mutation is allowed in this allocation PR.

Future worker authority also excludes `workspace-boundaries.toml`; a proven need for an unowned path is `SHARED_LEASE_REQUIRED`, not implicit expansion.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: pending final exact-head candidate after self-review correction

### Component/integration

- command/run: `python tools/repository/validate_repository_policy.py`
- result: pending final exact-head candidate after self-review correction

### E2E

- scenario: `NOT_APPLICABLE` — allocation PR changes docs/control-plane authority only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending final stable allocation candidate
- trigger source: pull_request
- workflow/run/job: pending current generation
- classification: docs-only high-risk allocation/control-plane change
- result: pending current generation

## Self-review

- exact head: pending final stable candidate
- method/reviewer: ChatGPT Work Delivery Coordinator whole-diff review against merged #117 and live prerequisites
- material findings:
  - repaired pre-qualification provenance typo: removed a spurious non-authoritative Durability SHA
  - repaired task-schema omission: worker task now directly names Issue #247
  - repaired evidence-ownership mismatch: local physical Server Seam integration no longer claims formal QA Tier 1/Tier 2 completion
  - aligned child plan with literal #117 golden/cross-oracle, protocol/profile, admission replay/binding, unsupported gameplay and shutdown-reserved-work evidence
- verdict: pending requalification of the new exact head

## Independent review

- required: `YES`
- exact head: pending final stable candidate
- method/auditor: one genuinely independent deep exact-head review under current META/Game policy
- material findings: prior review generations are historical after this material tracked-doc correction
- verdict: pending fresh current-head review

## PR and closeout

- changed-file review: exactly three allocated docs paths expected
- unresolved review threads: pending final exact head
- related/superseded PRs: no Server Seam implementation PR at allocation admission; Dependabot #259/#260/#261 are non-owning Cargo candidates
- protected integration: only after every current-head gate and authority predicate is proven
- merge/result: pending
- ownership release: allocation task archives after protected-main readback and worker release

## Context checkpoint

```yaml
last_progress: coordinator self-review against merged #117 found and repaired QA evidence-ownership overclaim plus missing literal #117 negative/golden/shutdown cases; all original #117 prerequisites were re-verified terminal before the correction
status: validating
branch: coord/gameplay-server-seam-allocation-247
head_sha: null
pr: 294
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending_post_self_review_head
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: freeze the resulting remote PR #294 head, verify exact three-path diff and behind_by=0 against fresh main, run exact-head deterministic qualification, then request one fresh independent deep review for that exact stable head
```
