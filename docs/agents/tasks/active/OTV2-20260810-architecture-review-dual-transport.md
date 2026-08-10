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
updated_at: 2026-08-10T21:18:00+02:00
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
  - unambiguous architecture continuation and transport implementation spike
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

Persist the owner-requested multi-perspective architecture review and the accepted transport direction without implementing runtime code: keep the existing Oteryn-v2 technical foundation, reduce delivery scope toward evidence-producing vertical slices, add explicit product/channel/determinism/admin/SRE/tooling guardrails, and supersede the TCP-only transport preference with one `protocol-oteryn` application protocol over TCP+TLS 1.3 by default plus player-opt-in QUIC preference with secure TCP fallback.

## Architecture and source of truth

- `PROVEN`: current `main` at task start is `9794e9a6307b6f9db193ca2ce08607eb065b7d7e`.
- `PROVEN`: there were no open PRs at task start; this task opened draft PR #145.
- `PROVEN`: PR #63 is merged but its active task record still declared ownership of `FOUNDATION_PROGRAMME_CURRENT_STATUS.md`; this task archives that stale record as an ownership/lifecycle correction without changing historical architecture evidence.
- `PROVEN`: `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` marks FND-02/FND-03/FND-04, DUR-01 and ANL-01 accepted while runtime implementation remains separately unauthorized.
- `OWNER_ACCEPTED`: preserve the native Rust stack, one authoritative `protocol-oteryn`, Platform/game boundary, CharacterLease/fencing, one-writer channel authority, PostgreSQL durability direction, transactional outbox/audit, multichannel-first identity/ownership, native World Project/Bundle direction and read-only Game Intelligence.
- `OWNER_ACCEPTED`: initial gameplay transport policy is TCP+TLS 1.3 default with QUIC v1 + TLS 1.3 available as a player preference; QUIC failure may fall back only for transport-level failures and must never bypass authentication, certificate, ALPN, ticket, lease, version, entitlement or policy rejection.
- `OWNER_ACCEPTED`: `QUIC_ONLY` is diagnostic/developer-only; 0-RTT and QUIC DATAGRAM are excluded from baseline v1.
- `DERIVED`: production preference for QUIC requires benchmark/fault evidence and a safe TCP-only kill switch.

## Acceptance criteria

- [x] Archive the stale merged-PR #63 task and release its advisory ownership without rewriting historical evidence.
- [x] Add a canonical 2026-08-10 architecture review refinement covering software, systems, development, engine, networking, security, SRE, producer, design, MMO operations, tooling and player perspectives.
- [x] Add an explicit architecture status model separating decision, delivery and implementation states.
- [x] Add ADR-0014 plus a machine-readable transport policy for TCP default / QUIC opt-in / secure fallback.
- [x] Record GAME-VISION, GAME-CHANNEL, GAME-CHAR, GAME-ITEM and deterministic-simulation ordering/guardrails without prematurely implementing gameplay.
- [x] Split the broad first proof into small ordered vertical slices that still traverse real system boundaries.
- [x] Refresh the canonical current-status overlay and architecture index; older stale backlog/register execution prose remains historical under the overlay instead of being mass-rewritten.
- [x] Do not implement protocol, server, persistence, QUIC runtime, Platform changes or production activation.
- [ ] Verify the final changed-file set, exact-head documentation/governance CI and independent audit before merge readiness.

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

- Refreshed root `AGENTS.md`, `AGENTS.override.md`, nested architecture/governance policy and GitHub-only execution rules before writes.
- Verified GitHub connector access with admin/push permission; the earlier claim that this session lacked GitHub write access was incorrect.
- Created dedicated branch and draft PR #145 from exact `main` SHA `9794e9a6307b6f9db193ca2ce08607eb065b7d7e`.
- Archived the stale PR #63 task and released its advisory ownership.
- Added ADR-0014, machine-readable transport policy, three-axis status model, canonical architecture index and multi-perspective programme refinement.
- Refreshed `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` so the dependency ordering and transport decision are visible without rewriting historical ADR evidence.
- `NET-TRANSPORT-01` explicitly supersedes only FND-02 transport-choice clauses; FND-02 remains authority for one `protocol-oteryn` application protocol and its sequencing/security/framing semantics.

## Validation

### Focused

- changed-file discovery: GitHub PR #145 reports exactly the declared documentation/task/contract scope.
- PR diff inspection: performed through GitHub connector; no runtime/workflow/dependency files are changed.
- machine-readable contract: covered by the repository Agent Governance validation on the PR head; no JSON/governance failure reported.

### Component/integration

- command/run: `NOT_APPLICABLE` — documentation/contract task only
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable runtime behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending after this task checkpoint commit
- trigger source: pull request
- workflow/run/job: prior head `ba39311d8bb8d42ffb88016e04bc8b8a449b8184` had Agent Governance PASS (`31423159049`), Dependency Review PASS (`31423159735`), CodeQL in progress (`31423158441`); these results do not substitute for the new final head.
- runner assignment: pending final-head observation
- classification: pending
- result: pending

## Independent audit

- exact head: pending final head
- method/auditor: independent reviewer/workflow evidence required by repository policy; current coordinator review is not claimed as independent
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: current diff inspected; final-head recheck pending after this metadata commit
- unresolved review threads: pending final-head inspection
- related/superseded PRs: PR #63 merged; stale lifecycle record corrected
- protected auto-merge: not enabled while audit/final-head gates remain pending
- merge commit/result: pending
- ownership release: pending merge/archive

## Context checkpoint

```yaml
last_progress: Architecture and dual-transport artifacts are on draft PR #145; scope is frozen except for validation/closeout metadata.
status: validating
branch: docs/OTV2-20260810-architecture-review-dual-transport
head_sha: null
pr: 145
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: final-head-pending
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
next_action: Verify final-head CI and obtain independent audit evidence for PR #145 without moving the head.
```
