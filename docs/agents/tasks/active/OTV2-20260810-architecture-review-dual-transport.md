# OTV2-20260810-architecture-review-dual-transport

```yaml
task_id: OTV2-20260810-architecture-review-dual-transport
title: Consolidate architecture review and dual gameplay transport decision
mode: CONTRACT
status: implementing
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260810-architecture-review-dual-transport
pr: null
base_sha: 9794e9a6307b6f9db193ca2ce08607eb065b7d7e
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-10T21:06:00+02:00
updated_at: 2026-08-10T21:06:00+02:00
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
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
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
  - unambiguous architecture continuation and transport implementation spike
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

Persist the owner-requested multi-perspective architecture review and the accepted transport direction without implementing runtime code: keep the existing Oteryn-v2 technical foundation, reduce delivery scope toward evidence-producing vertical slices, add explicit product/channel/determinism/admin/SRE/tooling guardrails, and supersede the TCP-only transport preference with one `protocol-oteryn` application protocol over TCP+TLS 1.3 by default plus player-opt-in QUIC preference with secure TCP fallback.

## Architecture and source of truth

- `PROVEN`: current `main` at task start is `9794e9a6307b6f9db193ca2ce08607eb065b7d7e`.
- `PROVEN`: there are no open PRs at task start.
- `PROVEN`: PR #63 is merged but its active task record still declares ownership of `FOUNDATION_PROGRAMME_CURRENT_STATUS.md`; this task includes only the ownership-correction/archive needed to release that stale claim.
- `PROVEN`: `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` marks FND-02/FND-03/FND-04, DUR-01 and ANL-01 accepted while runtime implementation remains separately unauthorized.
- `OWNER_ACCEPTED`: preserve the native Rust stack, one authoritative `protocol-oteryn`, Platform/game boundary, CharacterLease/fencing, one-writer channel authority, PostgreSQL durability direction, transactional outbox/audit, multichannel-first identity/ownership, native World Project/Bundle direction and read-only Game Intelligence.
- `OWNER_ACCEPTED`: initial gameplay transport policy is TCP+TLS 1.3 default with QUIC v1 + TLS 1.3 available as a player preference; QUIC failure may fall back only for transport-level failures and must never bypass authentication, certificate, ALPN, ticket, lease, version, entitlement or policy rejection.
- `OWNER_ACCEPTED`: `QUIC_ONLY` is diagnostic/developer-only; 0-RTT and QUIC DATAGRAM are excluded from baseline v1.
- `DERIVED`: production preference for QUIC requires benchmark/fault evidence and a safe TCP-only kill switch.

## Acceptance criteria

- [ ] Archive the stale merged-PR #63 task and release its advisory ownership without rewriting historical evidence.
- [ ] Add a canonical 2026-08-10 architecture review refinement covering software, systems, development, engine, networking, security, SRE, producer, design, MMO operations, tooling and player perspectives.
- [ ] Add an explicit architecture status model separating decision, delivery and implementation states.
- [ ] Add ADR-0014 plus a machine-readable transport policy for TCP default / QUIC opt-in / secure fallback.
- [ ] Record GAME-VISION, GAME-CHANNEL, GAME-CHAR, GAME-ITEM and deterministic-simulation ordering/guardrails without prematurely implementing gameplay.
- [ ] Split the broad first proof into small ordered vertical slices that still traverse real system boundaries.
- [ ] Update current status/backlog/register/index navigation narrowly and truthfully.
- [ ] Do not implement protocol, server, persistence, QUIC runtime, Platform changes or production activation.
- [ ] Review the complete changed-file set and verify applicable governance/link/JSON checks on the exact final head.

## Excluded scope

- no runtime implementation;
- no production or protected-environment changes;
- no writes to `blakinio/Oteryn-Platform`;
- no QUIC library selection freeze;
- no QUIC 0-RTT or DATAGRAM baseline;
- no autonomous AI sanctions;
- no microservice split or Kubernetes requirement;
- no proprietary Tibia/CipSoft assets.

## Implementation / findings

Task branch created from the exact current `main`. The root and nested instruction files were refreshed before writes. GitHub connector access is verified with admin/push permission. The previously reported lack of GitHub write access was incorrect for this session.

## Validation

### Focused

- command/run: pending GitHub diff/contract inspection
- result: pending

### Component/integration

- command/run: `NOT_APPLICABLE` — documentation/contract task only
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable runtime behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Independent audit

- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: PR #63 merged; stale task ownership correction included
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Dedicated branch and task record created after refreshing repository instructions and live GitHub state.
status: implementing
branch: docs/OTV2-20260810-architecture-review-dual-transport
head_sha: null
pr: null
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
next_action: Archive the stale merged-PR #63 task and then write the accepted architecture/transport artifacts.
```
