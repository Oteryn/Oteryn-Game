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
updated_at: 2026-08-10T21:35:00+02:00
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

Persist the owner-requested multi-perspective architecture review and transport direction without implementing runtime code: keep the existing Oteryn-v2 technical foundation, reduce delivery scope toward evidence-producing vertical slices, add explicit product/channel/determinism/admin/SRE/tooling guardrails, retain TCP+TLS 1.3 profile `1` as the current default/authoritative transport, and record QUIC v1 + TLS 1.3 as the future player-opt-in target with secure TCP fallback after the protocol/admission profile contracts are explicitly reconciled.

## Architecture and source of truth

- `PROVEN`: current `main` at task start is `9794e9a6307b6f9db193ca2ce08607eb065b7d7e`.
- `PROVEN`: there were no open PRs at task start; this task opened PR #145.
- `PROVEN`: PR #63 was merged while its active task record remained; this task archives that stale record and releases advisory ownership without changing historical architecture evidence.
- `PROVEN`: FND-02 currently registers only TCP transport profile `1`; FND-04 fresh admission requires profile `1`, and the accepted recovery profile currently supports exact profile `1`.
- `PROVEN`: Game Login Ticket redemption remains owned by Oteryn Platform Game Gateway; GameNode consumes purpose-separated pre-admission/recovery material and must not accept the Game Login Ticket.
- `OWNER_ACCEPTED`: preserve the native Rust stack, one authoritative `protocol-oteryn`, Platform/game boundary, CharacterLease/fencing, one-writer channel authority, PostgreSQL durability direction, transactional outbox/audit, multichannel-first identity/ownership, native World Project/Bundle direction and read-only Game Intelligence.
- `OWNER_ACCEPTED`: TCP+TLS 1.3 stays the initial default/safe baseline; QUIC is the intended later player preference with safe TCP fallback, not a second application protocol.
- `REPAIRED`: QUIC player admission is explicitly **blocked** until a later accepted delivery registers a stable QUIC transport profile and reconciles both FND-04 fresh/recovery grant profiles. `PREFER_QUIC` is therefore not a functional production-client option yet.
- `OWNER_ACCEPTED`: `QUIC_ONLY` is diagnostic/developer-only after QUIC exists; 0-RTT and QUIC DATAGRAM are excluded from the baseline.

## Acceptance criteria

- [x] Archive the stale merged-PR #63 task and release its advisory ownership without rewriting historical evidence.
- [x] Add a canonical 2026-08-10 architecture review refinement covering software, systems, development, engine, networking, security, SRE, producer, design, MMO operations, tooling and player perspectives.
- [x] Add an explicit architecture status model separating decision, delivery and implementation states.
- [x] Add ADR-0014 plus a machine-readable transport policy for TCP current/default plus future QUIC opt-in direction and safe fallback.
- [x] Keep Game Login Ticket redemption at Platform Game Gateway and separate it from transport-bound pre-admission material.
- [x] Make QUIC admission/recovery activation explicitly blocked until FND-02/FND-04 transport-profile reconciliation.
- [x] Record GAME-VISION, GAME-CHANNEL, GAME-CHAR, GAME-ITEM and deterministic-simulation ordering/guardrails without prematurely implementing gameplay.
- [x] Split the broad first proof into small ordered vertical slices that still traverse real system boundaries.
- [x] Refresh the canonical current-status overlay and architecture index without mass-rewriting historical evidence.
- [x] Do not implement protocol, server, persistence, QUIC runtime, Platform changes or production activation.
- [ ] Verify the repaired final changed-file set, exact-head CI and independent Codex/audit state before merge readiness.

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
- Initial exact-head CI passed, but Codex exact-head review on `ef1fa2cb7fec66e33b547a3343c6015654ba6e17` found two P1 contract conflicts:
  1. QUIC had been described as accepted/player-available while the canonical FND-02 registry and FND-04 grant profiles support TCP profile `1` only.
  2. the initial transport policy incorrectly implied transport selection before Game Login Ticket redemption, conflicting with the accepted Gateway redemption boundary.
- Repair cycle 1 corrected both findings without broadening scope: QUIC is now an accepted future strategy but admission/recovery activation remains blocked; Game Login Ticket redemption remains at Gateway; fallback requires fresh Gateway-authorized transport-bound pre-admission material rather than cross-profile reuse.

## Validation

### Focused

- changed-file discovery and diff inspection: GitHub PR #145.
- machine-readable contract: `PROTOCOL_OTERYN_TRANSPORT_POLICY.json` revision 2 now records `ACCEPTED_STRATEGY_QUIC_ACTIVATION_BLOCKED`, current registered profile `[1]`, Gateway-owned Game Login Ticket redemption and no cross-profile pre-admission grant reuse.
- runtime/workflow/dependency changes: none.

### Component/integration

- result: `NOT_APPLICABLE` — documentation/contract task only.

### E2E

- result: `NOT_APPLICABLE` — no executable runtime behavior changes.

### Exact-head CI

- prior pre-repair head `ef1fa2cb7fec66e33b547a3343c6015654ba6e17`: Agent Governance `31423308902` PASS; Dependency Review `31423310091` PASS; CodeQL `31423310164` PASS.
- repaired final head: pending exact-head runs after this checkpoint commit; prior results do not substitute.

## Independent audit

- Codex review `4900259919` on pre-repair head found two material P1 findings; both are repaired in the current branch.
- final repaired exact-head Codex/audit result: pending.

## PR and closeout

- changed-file review: repaired scope remains documentation/task/contract only.
- unresolved review threads: two pre-repair P1 threads remain until the repaired exact head is re-reviewed/resolved.
- merge: forbidden until repaired exact-head CI and review/audit are clean.
- ownership release: pending merge + lifecycle archive.

## Context checkpoint

```yaml
last_progress: Repaired both Codex P1 findings by preserving Gateway ticket redemption and blocking QUIC admission until protocol/FND-04 transport profiles are reconciled.
status: validating
branch: docs/OTV2-20260810-architecture-review-dual-transport
head_sha: null
pr: 145
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: repaired-head-pending
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
next_action: Verify repaired exact-head CI and rerun Codex review on PR #145; merge only if all material findings are closed.
```
