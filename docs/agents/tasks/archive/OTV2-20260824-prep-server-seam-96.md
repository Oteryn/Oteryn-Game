# OTV2-20260824-prep-server-seam-96

```yaml
task_id: OTV2-20260824-prep-server-seam-96
title: Prepare production gameplay server listener seam
mode: COORDINATE
status: completed
repository: Oteryn/Oteryn-Game
issue: 96
base_branch: main
branch: null
pr: 117
base_sha: 9369abaca8f28a02534b57dfd82ac1fbebecb02e
head_sha: 2535e2a868c5a5893b9cf55a1ef73af09c4fa2f3
final_head_sha: 2535e2a868c5a5893b9cf55a1ef73af09c4fa2f3
final_head_frozen_at: 2026-08-24T21:39:34+02:00
owner: released
created_at: 2026-08-24T20:30:00+02:00
updated_at: 2026-08-24T21:43:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths: []
public_contracts: []
depends_on:
  - Oteryn-Game#94
  - Oteryn-Game#115
  - Oteryn-Game#116
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Produce the exact decision/allocation packet for the smallest production TCP/TLS gameplay listener/client-entry seam required before Native Client and real Tier-1 QA can be released, without implementing runtime code or inventing gameplay IDs/limits.

Terminal preparation verdict: `BLOCKED_BEFORE_SERVER_SEAM_ALLOCATION` with exact conditional implementation paths and tracked prerequisites #94, #115 and #116.

## Architecture and source of truth

- **PROVEN:** exact rebased task base is `main@9369abaca8f28a02534b57dfd82ac1fbebecb02e`; Issue #96 was closed completed by delivery PR #117; no Server Seam implementation authority was granted by this preparation task.
- **PROVEN:** intervening `main` changes after the initial candidate were disjoint preparation allocations for Content Format Spike and Wave-2 resource limits; this task was rebased before final-head validation.
- **PROVEN:** the bounded Foundation implementation from PR #59 is merged: FND-02 ingress/framing, FND-03 generation/ordinal fencing and FND-04 admission/session/lease/reconnect semantics; `apps/game-server` ordinary execution remains explicitly gameplay-unavailable.
- **PROVEN:** NET-TRANSPORT-01 registers only TCP + TLS 1.3 profile `1`; QUIC runtime remains unavailable/unregistered.
- **PROVEN:** `protocol.rs` has inbound framing/envelope validation but no production Foundation outbound encoder for required server admission/recovery responses.
- **PROVEN:** every concrete `ReconnectAttemptJournal` implementation found on current `main` is test-only.
- **PROVEN:** current code search found no production FND-04 JWS/Ed25519 admission/recovery material verifier/consumer under `apps/**` or `crates/**`.
- **PROVEN:** current Resource Limits Registry has FND-02, ANL-01 and export entries, but no FND-03/NET listener connection/handshake/outbound/pending-work ceilings.
- **DERIVED:** the Foundation outbound wire bridge can be safely included in the eventual Server Seam allocation if `apps/game-server/src/foundation/protocol.rs` receives a serialized one-writer lease; the verifier/journal/resource-limit gaps are external prerequisites.

## Acceptance criteria

- [x] Exact production listener/transport/composition/test/Cargo paths proposed.
- [x] Existing Foundation ownership preserved; no second protocol/session owner proposed.
- [x] TLS profile/resource ownership identified without selecting new numeric policy.
- [x] Authority-before-mutation and reconnect-generation fencing order defined.
- [x] Unsupported gameplay remains fail-closed after successful admission.
- [x] Real Tier-1 QA physical journey boundary defined without claiming Tier-1 proof.
- [x] Malformed/oversized/unknown/stale-generation/authority-negative test obligations defined.
- [x] Eventual implementation risk requires genuinely independent exact-head protocol/session/admission/fencing review.
- [x] Child plan path fixed to `docs/superpowers/plans/2026-08-24-oteryn-production-gameplay-server-seam.md`.
- [x] Conditional `OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM` allocation proposal recorded.
- [x] External blockers are tracked in Issues #115/#116 and as a cross-domain finding on #94.

## Excluded scope

No listener/runtime implementation, Cargo dependency mutation, registry value, stable ID, production port/address/certificate/secret, Platform/external-repository mutation, QUIC activation, gameplay command/state/event ID, Movement, Combat, Ability, Interaction or AI implementation.

## Implementation / findings

Decision packet: `docs/architecture/reviews/OTERYN_GAME_PRODUCTION_GAMEPLAY_SERVER_SEAM_PLAN_2026-08-24.md`.

Preparation result is intentionally fail-closed. The eventual Server Seam worker must not start until #115, the #94 journal dependency, #116 and the shared-path lease are all satisfied on merged `main`.

## Verified delivery

- delivery PR: #117
- exact delivery head: `2535e2a868c5a5893b9cf55a1ef73af09c4fa2f3`
- delivery squash merge: `4079804b7f1f29cc2b7db2e746d4da2861bff084`
- merge time: `2026-08-24T21:42:35+02:00`
- changed delivery paths: exactly the decision packet and this task record
- exact-head Agent governance run `32769469387`: PASS
- exact-head Architecture semantic audit run `32769469461`: PASS
- exact-head Merge authority audit run `32769469492`: PASS
- exact-head Merge gate run `32769469639`: PASS, including canonical merge-gate aggregation
- unresolved review threads before merge: 0
- source branch `docs/otv2-prep-server-seam-96`: absent after merge
- Issue #96: closed with state reason `completed`
- runtime/component/E2E: `NOT_APPLICABLE` — preparation documentation only

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: PASS — 25 required policy documents / 9 project lanes on the complete candidate before publication.
- command/run: `git diff --check`
- result: PASS on the complete candidate before publication.
- command/run: placeholder scan on both owned files
- result: PASS — no `TBD`/`TODO`/`fill in`/`implement later`/template placeholders.

### Component/integration

- result: `NOT_APPLICABLE` — preparation documentation only; no executable product behavior changes.

### E2E

- result: `NOT_APPLICABLE` — real Tier 1 remains downstream of the blocked production seam implementation.

### Exact-head CI

- final head: `2535e2a868c5a5893b9cf55a1ef73af09c4fa2f3`
- trigger source: pull request #117
- runs: `32769469387`, `32769469461`, `32769469492`, `32769469639`
- classification: documentation/preparation only
- result: PASS

## Self-review

- exact head: `2535e2a868c5a5893b9cf55a1ef73af09c4fa2f3`
- method/reviewer: implementing/coordinating agent whole-diff review
- material findings: 2 candidate findings were repaired before final freeze; zero material findings remained on the final exact head
- verdict: PASS

## Independent review

- required: NO — this preparation delivery changes no runtime/public contract/registry/security policy and grants no implementation authority; the eventual protocol/session/admission/fencing implementation requires genuinely independent exact-head review.
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: PASS — exactly two declared preparation paths
- unresolved review threads: 0
- related/superseded PRs: none for Issue #96 preparation
- squash merge: PASS — `4079804b7f1f29cc2b7db2e746d4da2861bff084`
- Issue #96: closed completed
- source branch: deleted/absent
- ownership release: PASS — `owned_paths: []`

## Ownership release

The preparation-owned decision packet and task-record paths are released. This closeout grants no runtime, Cargo, registry, production, secret, Platform or external-repository write authority. The conditional Server Seam implementation remains separately blocked on #94, #115, #116 and an explicit serialized shared-path lease.

## Context checkpoint

```yaml
last_progress: PR #117 exact head 2535e2a868c5a5893b9cf55a1ef73af09c4fa2f3 passed all required gates and squash-merged as 4079804b7f1f29cc2b7db2e746d4da2861bff084; Issue #96 closed completed
status: completed
branch: null
head_sha: 2535e2a868c5a5893b9cf55a1ef73af09c4fa2f3
pr: 117
final_head_sha: 2535e2a868c5a5893b9cf55a1ef73af09c4fa2f3
final_head_frozen_at: 2026-08-24T21:39:34+02:00
ci_trigger_source: pull_request
ci_check_generation: exact_head_2535e2a868c5a5893b9cf55a1ef73af09c4fa2f3
ci_checks_for_current_head: 4
ci_run_ids:
  - 32769469387
  - 32769469461
  - 32769469492
  - 32769469639
ci_job_ids: []
runner_assignment_state: completed
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 4
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: none for this completed preparation task; downstream implementation prerequisites remain tracked in #94, #115 and #116
blocker: none for this completed preparation task
next_action: none — task lifecycle closed; Server Seam implementation remains a separate conditional downstream task
```
