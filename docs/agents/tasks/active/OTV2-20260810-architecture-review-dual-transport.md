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
updated_at: 2026-08-10T23:05:00+02:00
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
  - merged stale FND-ID lifecycle cleanup PR 147
blocks:
  - unambiguous architecture continuation and a future bounded QUIC profile-reconciliation task
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

Persist the owner-requested multi-perspective architecture review and transport direction without implementing runtime code: keep the existing Oteryn-v2 technical foundation, reduce delivery scope toward evidence-producing vertical slices, add explicit product/channel/determinism/admin/SRE/tooling guardrails, retain TCP+TLS 1.3 profile `1` as the current default/authoritative transport, and record QUIC v1 + TLS 1.3 as the future player-opt-in target with secure TCP fallback after the protocol/admission profile contracts and ordering evidence are explicitly reconciled.

## Architecture and source of truth

- `PROVEN`: current `main` at task start was `9794e9a6307b6f9db193ca2ce08607eb065b7d7e`; PR #145 owns this bounded architecture delivery.
- `PROVEN`: PR #63 was merged while its active task record remained; this task archives that stale record and releases advisory ownership without changing historical architecture evidence.
- `PROVEN`: FND-02 currently registers only TCP transport profile `1`; FND-04 fresh admission requires profile `1`, and the accepted recovery profile currently supports exact profile `1`.
- `PROVEN`: FND-02 Section 5 explicitly says QUIC is deferred, not rejected, and requires measured latency/head-of-line/roaming evidence; Section 24 also records `QUIC v1` as deferred pending measured benefit.
- `PROVEN`: FND-02 server-visible `server_sequence` and snapshot publication barrier require ordered application semantics that cannot be assumed across independent QUIC streams.
- `PROVEN`: Game Login Ticket redemption remains owned by Oteryn Platform Game Gateway; GameNode consumes purpose-separated pre-admission/recovery material and must not accept the Game Login Ticket.
- `OWNER_ACCEPTED`: preserve the native Rust stack, one authoritative `protocol-oteryn`, Platform/game boundary, CharacterLease/fencing, one-writer channel authority, PostgreSQL durability direction, transactional outbox/audit, multichannel-first identity/ownership, native World Project/Bundle direction and read-only Game Intelligence.
- `OWNER_ACCEPTED`: TCP+TLS 1.3 stays the initial default/safe baseline; QUIC is the intended later player preference with safe TCP fallback, not a second application protocol.
- `REPAIRED`: QUIC player admission is explicitly blocked until a later accepted delivery registers a stable QUIC transport profile and reconciles both FND-04 fresh/recovery grant profiles. `PREFER_QUIC` is therefore not a functional production-client option yet.
- `REPAIRED`: ADR-0014 does not mischaracterize FND-02 as TCP-only; it refines only the future direction of FND-02's explicitly deferred QUIC alternative while retaining the measured-benefit prerequisite and all current profile/ordering/security clauses.
- `REPAIRED`: the future QUIC baseline puts every `SERVER_SEQUENCED` message plus SnapshotBegin/Chunk/Commit on one reliable ordered server-authoritative lane; cross-lane authoritative resequencing is disabled unless a later reviewed bounded mechanism proves equivalence.
- `REPAIRED`: the canonical status overlay uses independent architecture/delivery/implementation axes and distinguishes `PLANNED` future gates from active delivery.
- `PROVEN`: review of the status model exposed stale FND-ID support tasks. The branch conservatively marked FND-ID lifecycle `OPEN` while that external bookkeeping remained unresolved. Separate low-risk PR #147 then archived/released the ten proven-merged stale support tasks and squash-merged as `81db47966d76709a0e44dfbf1bc3979f38a24ffa`.
- `PROVEN`: after PR #147, the external lifecycle dependency is closed, so the status overlay is synchronized back to `FND-ID-01 / ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED` with the cleanup merge recorded. This is a post-review dependency-state synchronization, not a new transport/security design change.
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
- [x] Resolve stale FND-ID lifecycle ownership through separate merged PR #147 and synchronize the status overlay to the resulting current state.
- [x] Do not implement protocol, server, persistence, QUIC runtime, Platform changes or production activation.
- [ ] Verify the final post-dependency-sync changed-file set, exact-head CI, mandatory self-review and required independent review before merge readiness.

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
- Created dedicated branch and PR #145 from exact start SHA `9794e9a6307b6f9db193ca2ce08607eb065b7d7e`.
- Archived stale PR #63 lifecycle ownership.
- Added ADR-0014, transport-policy registry, status model, architecture index and multi-perspective programme refinement.
- First Codex review found two P1 contract conflicts: missing registered QUIC profile compatibility and incorrect Game Login Ticket redemption ordering. Repair cycle 1 blocked QUIC activation and restored the Gateway boundary.
- Second Codex review found three P1 architecture-discipline/ordering issues: QUIC lane ordering, missing options/trade-offs analysis and imprecise FND-02 supersession. Repair cycle 2 corrected all three.
- Third review sequence found status-model consistency issues: explicit axes, `PLANNED` delivery state and lifecycle-closed FND-02/FND-03. Repair cycle 3 corrected those status-model semantics without runtime/transport activation.
- A later status audit exposed external stale FND-ID lifecycle records. Instead of expanding this transport PR to archive unrelated historical task files, the status was conservatively held `OPEN` until a separate bounded cleanup could prove and release them.
- Under the subsequently merged risk-based review governance, low-risk PR #147 archived those ten stale records with direct GitHub evidence and merged as `81db47966d76709a0e44dfbf1bc3979f38a24ffa`.
- The current branch now performs only the resulting dependency-state synchronization: FND-ID returns to `LIFECYCLE_CLOSED`. This synchronization does not alter accepted transport, admission, security, runtime or product semantics and is not counted as an additional material design repair cycle.
- `repair_cycles_for_current_gate` remains exhausted at `3`; no further review-driven semantic repair is allowed. Any new material finding in the final required independent review blocks/rotates the task instead of being patched here.

## Validation

### Focused

- changed-file discovery/full diff: PR #145; documentation/task/contract paths only.
- transport contract revision 3 preserves FND-02 relationship, blocked QUIC activation, Gateway boundary and authoritative QUIC lane ordering.
- architecture status model distinguishes `PLANNED`, `OPEN`, `IN_REVIEW`, `MERGED` and `LIFECYCLE_CLOSED`.
- stale FND-ID dependency evidence: cleanup PR #147 merged `81db47966d76709a0e44dfbf1bc3979f38a24ffa` and removed the ten proven-terminal support records from `tasks/active/`.
- runtime/workflow/dependency/production implementation changes: none.

### Component/integration

- result: `NOT_APPLICABLE` — documentation/contract task only.

### E2E

- result: `NOT_APPLICABLE` — no executable runtime behavior changes.

### Exact-head CI

- earlier heads passed their exact-head checks but were superseded by review or dependency-state synchronization.
- final post-dependency-sync head: pending after this task checkpoint commit; prior checks do not substitute.

## Self-review

- exact head: pending final post-dependency-sync head
- method/reviewer: implementing/coordinating agent full-diff review
- material findings: pending final pass
- verdict: pending

## Independent review

- required: `YES` — transport/admission/security architecture is high risk and the task also started under governance requiring independent audit
- method/auditor: Codex independent PR review is the available mechanism chosen only because this independent review is genuinely required
- prior reviews: material findings repaired through the allowed three design/status repair cycles; stale FND-ID lifecycle dependency resolved externally
- final exact head: pending
- material findings: pending
- verdict: pending; any new material finding blocks because design repair budget is exhausted

## PR and closeout

- changed-file review: pending final post-dependency-sync head
- unresolved review threads: all prior threads resolved/outdated; final review may not leave a material thread open
- related/superseded PRs: PR #147 merged and closes the FND-ID lifecycle dependency; PR #146 governance change is merged independently
- merge: pending exact-head checks + self-review + required independent review
- ownership release: pending merge + later lifecycle archive

## Context checkpoint

```yaml
last_progress: FND-ID lifecycle dependency closed by merged PR #147 and status overlay synchronized back to LIFECYCLE_CLOSED without changing transport/security semantics.
status: validating
branch: docs/OTV2-20260810-architecture-review-dual-transport
head_sha: null
pr: 145
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: post-dependency-sync-head-pending
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
next_action: Freeze the post-dependency-sync head, run mandatory exact-head self-review and CI, then invoke the one required final independent Codex review; squash-merge only if no material finding remains.
```
