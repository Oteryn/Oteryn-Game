# OTV2-20260810-architecture-review-dual-transport

```yaml
task_id: OTV2-20260810-architecture-review-dual-transport
title: Consolidate architecture review and dual gameplay transport decision
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260810-architecture-review-dual-transport
pr: 145
base_sha: 9794e9a6307b6f9db193ca2ce08607eb065b7d7e
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-10T21:06:00+02:00
updated_at: 2026-08-10T22:10:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - README.md
  - docs/agents/tasks/active/OTV2-20260810-architecture-review-dual-transport.md
  - docs/agents/tasks/active/OTV2-20260807-protocol-contract-reconciliation.md
  - docs/agents/tasks/archive/OTV2-20260807-protocol-contract-reconciliation.md
  - docs/architecture/ADR-0014-dual-gameplay-transport-tcp-default-quic-opt-in.md
  - docs/architecture/ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md
  - docs/architecture/ARCHITECTURE_STATUS_MODEL.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/README.md
  - docs/contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json
public_contracts:
  - docs/architecture/ADR-0014-dual-gameplay-transport-tcp-default-quic-opt-in.md
  - docs/contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json
depends_on:
  - accepted FND-02 protocol-oteryn v1 architecture
  - accepted FND-03 runtime ownership architecture
  - accepted FND-04 admission/reconnect/session architecture
  - accepted DUR-01 and ANL-01 contracts
blocks:
  - unambiguous architecture continuation and a future bounded QUIC profile-reconciliation task
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

Persist the owner-requested multi-perspective architecture review and transport direction without implementing runtime code: keep the existing Oteryn-v2 technical foundation, reduce delivery scope toward evidence-producing vertical slices, add explicit product/channel/determinism/admin/SRE/tooling guardrails, retain TCP+TLS 1.3 profile `1` as the current default/authoritative transport, and record QUIC v1 + TLS 1.3 as the future player-opt-in target with secure TCP fallback after the protocol/admission profile contracts and ordering evidence are explicitly reconciled.

## Architecture and source of truth

- `PROVEN`: current `main` at task start is `9794e9a6307b6f9db193ca2ce08607eb065b7d7e`.
- `PROVEN`: there were no open PRs at task start; this task opened PR #145.
- `PROVEN`: PR #63 was merged while its active task record remained; this task archives that stale record and releases advisory ownership without changing historical architecture evidence.
- `PROVEN`: FND-02 currently registers only TCP transport profile `1`; FND-04 fresh admission requires profile `1`, and the accepted recovery profile currently supports exact profile `1`.
- `PROVEN`: FND-02 Section 5 explicitly says QUIC is deferred, not rejected, and requires measured latency/head-of-line/roaming evidence; Section 24 also records `QUIC v1` as deferred pending measured benefit.
- `PROVEN`: FND-02 server-visible `server_sequence` and snapshot publication barrier require ordered application semantics that cannot be assumed across independent QUIC streams.
- `PROVEN`: Game Login Ticket redemption remains owned by Oteryn Platform Game Gateway; GameNode consumes purpose-separated pre-admission/recovery material and must not accept the Game Login Ticket.
- `OWNER_ACCEPTED`: preserve the native Rust stack, one authoritative `protocol-oteryn`, Platform/game boundary, CharacterLease/fencing, one-writer channel authority, PostgreSQL durability direction, transactional outbox/audit, multichannel-first identity/ownership, native World Project/Bundle direction and read-only Game Intelligence.
- `OWNER_ACCEPTED`: TCP+TLS 1.3 stays the initial default/safe baseline; QUIC is the intended later player preference with safe TCP fallback, not a second application protocol.
- `REPAIRED`: QUIC player admission is explicitly blocked until a later accepted delivery registers a stable QUIC transport profile and reconciles both FND-04 fresh/recovery grant profiles. `PREFER_QUIC` is therefore not a functional production-client option yet.
- `REPAIRED`: ADR-0014 no longer mischaracterizes FND-02 as TCP-only; it refines only the future direction of FND-02's explicitly deferred QUIC alternative while retaining the measured-benefit prerequisite and all current profile/ordering/security clauses.
- `REPAIRED`: the future QUIC baseline now puts every `SERVER_SEQUENCED` message plus SnapshotBegin/Chunk/Commit on one reliable ordered server-authoritative lane; cross-lane authoritative resequencing is disabled unless a later reviewed bounded mechanism proves equivalence.
- `REPAIRED`: the canonical status overlay now uses the three independent architecture/delivery/implementation axes, distinguishes `PLANNED` future gates from concrete `OPEN` delivery, and reports archived FND-02/FND-03 delivery as `LIFECYCLE_CLOSED`.
- `OWNER_ACCEPTED`: `QUIC_ONLY` is diagnostic/developer-only after QUIC exists; 0-RTT and QUIC DATAGRAM are excluded from the baseline.

## Acceptance criteria

- [x] Archive the stale merged-PR #63 task and release its advisory ownership without rewriting historical evidence.
- [x] Add a canonical 2026-08-10 architecture review refinement covering software, systems, development, engine, networking, security, SRE, producer, design, MMO operations, tooling and player perspectives.
- [x] Add an explicit architecture status model separating decision, delivery and implementation states, including `PLANNED` for registered gates without active delivery.
- [x] Add ADR-0014 plus a machine-readable transport policy for TCP current/default plus future QUIC opt-in direction and safe fallback.
- [x] Preserve FND-02's exact current TCP profile and measured-benefit prerequisite; identify only the deferred QUIC direction as refined.
- [x] Document realistic transport options, trade-offs, risks and recommendation per architecture decision discipline.
- [x] Keep Game Login Ticket redemption at Platform Game Gateway and separate it from transport-bound pre-admission material.
- [x] Make QUIC admission/recovery activation explicitly blocked until FND-02/FND-04 transport-profile reconciliation.
- [x] Preserve FND-02 server-sequence and snapshot-barrier ordering across future QUIC by defining the baseline ordered-lane rule and activation proof.
- [x] Record GAME-VISION, GAME-CHANNEL, GAME-CHAR, GAME-ITEM and deterministic-simulation ordering/guardrails without prematurely implementing gameplay.
- [x] Split the broad first proof into small ordered vertical slices that still traverse real system boundaries.
- [x] Refresh the canonical current-status overlay and architecture index without mass-rewriting historical evidence.
- [x] Do not implement protocol, server, persistence, QUIC runtime, Platform changes or production activation.
- [ ] Verify the final repair-3 changed-file set, exact-head CI and independent Codex/audit state before merge readiness.

## Excluded scope

- no runtime implementation;
- no production or protected-environment changes;
- no writes to `blakinio/Oteryn-Platform`;
- no QUIC transport-profile ID registration in this delivery;
- no FND-04 grant-profile revision in this delivery;
- no QUIC library selection freeze;
- no QUIC 0-RTT or DATAGRAM baseline;
- no autonomous AI sanctions;
- no microservice split or Kubernetes requirement;
- no proprietary Tibia/CipSoft assets.

## Implementation / findings

- Refreshed root and nested repository instructions before writes and verified the GitHub connector has write/admin permission for `blakinio/Oteryn-v2`.
- Created dedicated branch and PR #145 from exact `main` SHA `9794e9a6307b6f9db193ca2ce08607eb065b7d7e`.
- Archived stale PR #63 lifecycle ownership.
- Added ADR-0014, transport-policy registry, status model, architecture index and multi-perspective programme refinement.
- First Codex review on `ef1fa2cb7fec66e33b547a3343c6015654ba6e17` found two P1 contract conflicts: missing registered QUIC profile compatibility and incorrect Game Login Ticket redemption ordering. Repair cycle 1 blocked QUIC activation and restored the Gateway boundary.
- Second Codex review on `1d9c9bcbcdf54bc27324ab22268405a0a9b5639d` found three P1 architecture-discipline/ordering issues: QUIC lane ordering, missing options/trade-offs analysis and imprecise FND-02 supersession. Repair cycle 2 corrected all three.
- Third/final-status review sequence found status-model consistency issues: the canonical overlay needed the three explicit axes, a `PLANNED` delivery state for gates with no active task, and lifecycle-closed delivery for already archived FND-02/FND-03. Repair cycle 3 corrects those bookkeeping semantics without changing runtime or transport authority.

## Validation

### Focused

- changed-file discovery and full-diff inspection: GitHub PR #145.
- machine-readable transport contract revision 3 records FND-02 relationship, blocked QUIC activation, Gateway boundary and authoritative QUIC lane ordering.
- architecture status model now distinguishes `PLANNED`, `OPEN`, `IN_REVIEW`, `MERGED` and `LIFECYCLE_CLOSED` truthfully.
- runtime/workflow/dependency changes: none.

### Component/integration

- result: `NOT_APPLICABLE` — documentation/contract task only.

### E2E

- result: `NOT_APPLICABLE` — no executable runtime behavior changes.

### Exact-head CI

- earlier heads passed their exact-head checks but were superseded by review repairs.
- pre-repair status head `b349864ee9b75d3a2438c226304ab533b5b89db6`: Agent Governance `31426588720` PASS; Dependency Review `31426589866` PASS; CodeQL `31426589610` PASS; superseded by repair cycle 3.
- repair-3 final head: pending exact-head runs after this checkpoint commit; earlier results do not substitute.

## Independent audit

- Codex review was used because the task started under governance requiring an independent audit and the transport/admission contract is high risk.
- All prior material P1/P2 findings are repaired or resolved.
- final repair-3 exact-head Codex/audit result: pending.

## PR and closeout

- changed-file review: scope remains documentation/task/contract only.
- two final status-model P2 threads require reply/resolution after this repair and final re-review.
- merge: forbidden until repair-3 exact-head CI and final independent review are clean.
- ownership release: pending merge + lifecycle archive.

## Context checkpoint

```yaml
last_progress: Repair cycle 3 distinguishes planned versus active delivery and corrects lifecycle-closed status for archived FND-02/FND-03.
status: validating
branch: docs/OTV2-20260810-architecture-review-dual-transport
head_sha: null
pr: 145
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
next_action: Verify repair-3 exact-head CI and obtain final Codex review on PR #145; squash-merge only if every material thread is resolved and the head stays unchanged.
```
