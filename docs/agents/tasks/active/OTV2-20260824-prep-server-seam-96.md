# OTV2-20260824-prep-server-seam-96

```yaml
task_id: OTV2-20260824-prep-server-seam-96
title: Prepare production gameplay server listener seam
mode: COORDINATE
status: validating
repository: Oteryn/Oteryn-Game
issue: 96
base_branch: main
branch: docs/otv2-prep-server-seam-96
pr: 117
base_sha: 22a3eb866dae19d048969edff1e1fa5012a429b6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chat-github-20260824-prep-server-seam
created_at: 2026-08-24T20:30:00+02:00
updated_at: 2026-08-24T21:23:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_PRODUCTION_GAMEPLAY_SERVER_SEAM_PLAN_2026-08-24.md
  - docs/agents/tasks/active/OTV2-20260824-prep-server-seam-96.md
public_contracts: []
depends_on:
  - Oteryn-Game#94
  - Oteryn-Game#115
  - Oteryn-Game#116
blocks:
  - OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Produce the exact decision/allocation packet for the smallest production TCP/TLS gameplay listener/client-entry seam required before Native Client and real Tier-1 QA can be released, without implementing runtime code or inventing gameplay IDs/limits.

Terminal preparation verdict: `BLOCKED_BEFORE_SERVER_SEAM_ALLOCATION` with exact conditional implementation paths and tracked prerequisites #94, #115 and #116.

## Architecture and source of truth

- **PROVEN:** exact rebased task base is `main@22a3eb866dae19d048969edff1e1fa5012a429b6`; Issue #96 remains open and no active Server Seam implementation PR/branch was found at preflight.
- **PROVEN:** intervening `main` change after the initial candidate was the disjoint Content Format Spike allocation; this task was rebased before final-head validation.
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

- final head: recorded by PR #117/check evidence after the final content commit; do not self-reference it in this commit.
- trigger source: PR #117
- workflow/run/job: pending exact-head checks
- runner assignment: pending exact-head checks
- classification: documentation/preparation only
- result: pending

## Self-review

- exact head: repeat against final PR #117 head after this commit
- method/reviewer: implementing/coordinating agent
- material findings: 2 candidate findings repaired before publication: avoid overstating whole FND-03 implementation; avoid inventing an unsupported-gameplay error semantic.
- verdict: candidate diff PASS; final exact-head PR diff review required before readiness.

## Independent review

- required: NO — this preparation delivery changes no runtime/public contract/registry/security policy and grants no implementation authority; the eventual protocol/session/admission/fencing implementation requires genuinely independent exact-head review.
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending exact-head PR diff
- unresolved review threads: pending
- related/superseded PRs: none for Issue #96 preparation
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending after terminal preparation delivery

## Context checkpoint

```yaml
last_progress: Rebased onto current main and published PR #117 with conditional Server Seam topology plus tracked blockers #94/#115/#116
status: validating
branch: docs/otv2-prep-server-seam-96
head_sha: recorded externally by PR #117 after final commit
pr: 117
final_head_sha: recorded externally by PR/check evidence
final_head_frozen_at: after this content commit
ci_trigger_source: pull_request
ci_check_generation: pending
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
blocker: Server Seam implementation waits for #115, the #94 durable journal dependency, #116 and a serialized shared-path lease
next_action: perform exact-head PR diff/review-thread/CI validation and merge the preparation delivery if all gates pass
```
