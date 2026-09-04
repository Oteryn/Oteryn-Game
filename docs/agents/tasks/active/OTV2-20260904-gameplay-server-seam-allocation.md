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
  - merged_pr: 252
  - merged_pr: 290
blocks:
  - OTV2-20260904-gameplay-server-seam
external_repositories: []
```

## Outcome

Create one fresh exact-base, docs-only allocation that releases the already-accepted production gameplay Server Seam to exactly one Sol lane lead after this allocation itself lawfully merges and protected-main readback proves the merge. This allocation changes no runtime behavior, protocol bytes, session state, durable state, workflow, protection, production setting or external repository.

## Architecture and source of truth

- `PROVEN` — protected `main` at allocation admission is `68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705`.
- `PROVEN` — Issue #162 remains the live coordinator lifecycle for `Oteryn: work coordinator`; its latest durable checkpoint names a fresh Server Seam allocation as the deterministic next action after Durability terminal closeout.
- `PROVEN` — Issue #247 is open and its latest checkpoint classifies the lane `READY_FOR_FRESH_ALLOCATION`, explicitly stating that the Issue itself grants no implementation write authority.
- `PROVEN` — Durability prerequisite Issue #250 is completed; live coordinator checkpoints record implementation PR #252 merged as `b67f4425e9e9c5bbf9f7bc94c422cd7478edcdd3`, followed by archival PR #290 merged as `47faa84152ffdcac00d8e2173d582aa54be2cbcf`, with the Durability task archived and ownership released.
- `PROVEN` — Server Seam preparation Issue #96 is completed and PR #117 merged as `4079804b7f1f29cc2b7db2e746d4da2861bff084`; `docs/architecture/reviews/OTERYN_GAME_PRODUCTION_GAMEPLAY_SERVER_SEAM_PLAN_2026-08-24.md` is the accepted implementation shape.
- `PROVEN` — current `apps/game-server/src/main.rs` has no gameplay listener: `--smoke` runs `bootstrap_smoke()`, while normal execution reports gameplay unavailable and exits non-zero.
- `PROVEN` — current `apps/game-server/src/foundation/protocol.rs` owns FND-02 wire validation and exposes `WireEnvelopeView`/`decode_wire_envelope`, while the accepted #117 decision records the still-missing typed bootstrap/resume extraction and server-side Foundation encoders required by the Server Seam.
- `PROVEN` — `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` contains the Server Seam limits accepted through #116/#140/#144, including `NET03-PREADMISSION-CONNECTIONS=256`, `NET03-HANDSHAKE-AUTH-WORK=64`, `NET03-OUTBOUND-QUEUE-ENTRIES=64`, `NET03-OUTBOUND-QUEUE-BYTES=1048576`, `NET03-PENDING-WRITES=8` and `NET03-DRAIN-TASKS=256`, plus the FND-02 wire/message limits.
- `PROVEN` — the current open non-Dependabot PR scan found no task-owned Server Seam runtime/shared path in the exact allowlist below. PR #289 owns Durability/Foundation recovery paths but not `foundation/protocol.rs`, `src/lib.rs`, `src/main.rs` or Cargo shared paths; #287/#291/#293 own workflow/governance/docs surfaces only.
- `PROVEN` — Dependabot PRs #259, #260 and #261 are open and each changes `Cargo.toml`/`Cargo.lock`, but they have no coordinator task/allocation ownership and are not active implementation writers. Repository precedent distinguishes active non-Dependabot ownership from unallocated Dependabot candidates. They remain real later integration candidates and receive no shared lease from this task.
- `CONFLICT` — `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md` still carries a stale release-candidate header referring to nonexistent PR #314. This stale marker does not override the newer live Issue #162/#247 checkpoints, current Terra/Sol scheduler, merged Work orchestration architecture, reusable prompt lifecycle or accepted PR #117 Server Seam decision. This allocation does not repair that unrelated stale programme prose.
- `DERIVED` — with Durability terminal and no active non-Dependabot writer on the exact Server Seam allowlist, one serialized lane can be released without concurrent ownership ambiguity after this allocation merges.

The accepted technical source of truth is:

- `docs/architecture/reviews/OTERYN_GAME_PRODUCTION_GAMEPLAY_SERVER_SEAM_PLAN_2026-08-24.md`;
- FND-02 protocol contract and registry;
- FND-03 runtime execution contract and registered NET03 limits;
- FND-04 admission/GameSession/CharacterLease/reconnect/fencing contracts and current verifier/consumer implementation;
- current Durability admission/reconnect journal and terminal replacement implementation on protected `main`;
- current QA-E2E real-boundary requirements.

No lower source may widen the accepted architecture or convert test/development configuration into production deployment authority.

## Allocation decision

Only after this allocation PR merges and Work reads its merge SHA back from protected `main`, `Oteryn: sol server seam lead` becomes the exclusive mutating worker for:

```text
lane_id: OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
task_id: OTV2-20260904-gameplay-server-seam
branch: agent/otv2-gameplay-server-seam-01
```

The worker branch MUST be created from exactly the allocation merge SHA, not from this allocation's admission SHA and not from a cached chat SHA.

### Primary worker paths after allocation merge

- `apps/game-server/src/gameplay_transport/mod.rs`
- `apps/game-server/src/gameplay_transport/tcp_tls.rs`
- `apps/game-server/src/gameplay_transport/connection.rs`
- `apps/game-server/tests/gameplay_server_seam.rs`

### Serialized shared paths after allocation merge

These are one-writer-at-a-time leases for the Server Seam lane only:

- `apps/game-server/src/foundation/protocol.rs`
- `apps/game-server/src/lib.rs`
- `apps/game-server/src/main.rs`
- `apps/game-server/Cargo.toml`
- `Cargo.toml`
- `Cargo.lock`

Worker-governance metadata additionally owned after merge:

- `docs/agents/tasks/active/OTV2-20260904-gameplay-server-seam.md`

The worker may edit a shared path only for the accepted Server Seam consumer. It may not use the lease to perform unrelated dependency upgrades, workspace restructuring, protocol registry allocation, durability redesign or general Foundation refactoring.

### Dependabot shared-path reconciliation

PRs #259/#260/#261 remain open, non-owning Cargo candidates. The lane MUST:

1. re-read their state before its first Cargo shared write and again before final integration;
2. never treat their branch heads as a dependency or merge them into the Server Seam branch merely to resolve ownership;
3. keep the Server Seam dependency diff limited to dependencies actually required by the accepted TLS/transport implementation;
4. if a Dependabot PR merges first, reconcile from fresh protected `main` with normal history-preserving integration and rerun invalidated qualification;
5. if a bot candidate conflicts after Server Seam integration, leave its later rebase/recreation to the normal dependency-update lifecycle.

## Mandatory execution order

1. This allocation candidate remains docs-only and exact-base.
2. Work performs whole-diff self-review, exact-head governance/repository qualification and one genuinely independent deep review because the allocation grants high-risk protocol/session/admission/fencing and shared Cargo/composition write authority.
3. Only a clean, current, expected-head candidate may integrate through protected repository controls.
4. Work reads the resulting allocation merge SHA from protected `main`.
5. Work creates `agent/otv2-gameplay-server-seam-01` from exactly that merge SHA and releases `Oteryn: sol server seam lead`.
6. The worker starts from fresh RED evidence for the allocated seam before implementing production transport behavior.
7. The worker preserves Foundation as the sole protocol/session/admission/reconnect authority, consumes current Durability rather than cloning it, proves a real local TCP/TLS Tier 1 journey, performs authority/recovery finding-family qualification, completes exact-head CI and independent review, then returns `READY_FOR_INTEGRATION` without self-merging.
8. Work independently verifies the handoff and integrates only when all exact-head predicates are proven.
9. After Server Seam merge/readback, recompute Client/QA readiness from fresh protected `main`; do not pre-release Client from this allocation.

## Acceptance criteria

- [ ] Allocation diff contains exactly the allocation task, worker task and required implementation plan; no runtime path is changed.
- [ ] Exact admission base is `68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705`; any main advance before integration is reconciled normally and invalidated checks are rerun.
- [ ] Durability terminal evidence is current and no active non-Dependabot writer owns the exact Server Seam primary/shared paths.
- [ ] Worker primary paths are exactly the four accepted #117 paths.
- [ ] Worker shared leases are exactly `foundation/protocol.rs`, game-server `lib.rs`/`main.rs`/`Cargo.toml`, root `Cargo.toml` and `Cargo.lock`.
- [ ] `workspace-boundaries.toml`, workflow files, stable protocol/event/resource registries and architecture contracts receive no write authority.
- [ ] The worker task binds the current AuthorityInvariant × ConsumerBoundary × MutationOperator qualification discipline before material freeze.
- [ ] The worker task requires fresh TDD RED → minimal GREEN, bounded framing/input rejection, resource-limit boundary proof, admission/reconnect fencing, backpressure/drain/shutdown behavior and real TCP/TLS Tier 1 evidence.
- [ ] No production address/port/certificate/key/secret/deployment topology, Reference semantics or gameplay IDs are selected.
- [ ] Allocation receives exact-head governance/repository/architecture/merge-gate evidence as applicable, Work whole-diff self-review and one genuinely independent deep review with no unresolved actionable P0/P1 finding.
- [ ] Protected-main readback occurs before the worker branch is created or any runtime write begins.

## Excluded scope

No runtime code, test runtime, Cargo/lockfile, Foundation implementation, protocol registry, resource registry, workspace policy, workflow, ruleset, production/protected environment, secret, certificate, production endpoint, live account/session/data, Client, Movement, Combat, Ability, Interaction, AI, Channel, Analytics, Content-format selection, Platform/Atlas/META or other external-repository mutation is permitted in this allocation PR.

The future worker allocation also excludes `workspace-boundaries.toml` unless a later exact authority decision proves it necessary. Discovery that this excluded path is materially required is `SHARED_LEASE_REQUIRED`/control-plane reconciliation, not implicit worker expansion.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: pending exact-head candidate evidence

### Component/integration

- command/run: `python tools/repository/validate_repository_policy.py`
- result: pending exact-head candidate evidence

### E2E

- scenario: `NOT_APPLICABLE` — this PR allocates authority only and changes no runtime
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending stable allocation candidate
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: repository-managed/GitHub-hosted as routed by current policy
- classification: docs-only high-risk allocation/control-plane change
- result: pending

## Self-review

- exact head: pending stable allocation candidate
- method/reviewer: ChatGPT Work Delivery Coordinator whole-diff review
- material findings: one pre-PR self-review finding repaired before qualification — removed a spurious non-authoritative SHA from the Durability provenance sentence; no runtime or authority scope changed
- verdict: pending exact stable candidate review

## Independent review

- required: `YES` — this docs-only change grants later high-risk protocol/session/admission/fencing runtime authority and serialized shared Cargo/composition ownership
- exact head: pending stable allocation candidate
- method/auditor: one genuinely independent deep exact-head review under current protected-main policy
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: expected exactly three allocated docs paths
- unresolved review threads: pending
- related/superseded PRs: no Server Seam implementation PR exists at allocation admission; Dependabot #259/#260/#261 are non-owning shared-path candidates
- protected auto-merge: pending policy-qualified candidate only
- merge commit/result: pending
- ownership release: allocation task archives after protected-main readback and worker authority release

## Context checkpoint

```yaml
last_progress: Draft allocation PR #294 is open with exactly the intended three docs paths; task metadata is being synchronized before exact-head qualification
status: validating
branch: coord/gameplay-server-seam-allocation-247
head_sha: null
pr: 294
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending_metadata_head
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: synchronize PR #294 into the worker task, then freeze the resulting remote head for exact-path/main-relation verification, hosted CI and one independent deep review
```
