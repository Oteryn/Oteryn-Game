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
admission_main_sha: 68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705
integration_main_sha: 187c6b83c6945d79aabef2c5730c3ddba13fcab1
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
updated_at: 2026-09-05T13:44:00+02:00
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
depends_on:
  - issue: 162
  - issue: 247
  - merged_pr: 117
  - merged_pr: 151
  - merged_pr: 252
  - merged_pr: 289
blocks:
  - OTV2-20260904-gameplay-server-seam
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Create and qualify one docs-only control-plane allocation for the already accepted production gameplay Server Seam. This PR itself changes no runtime, protocol bytes, durable/session state, Cargo dependency, workflow/protection, production setting, secret or external repository.

Only after PR #294 integrates through protected controls and Work reads the resulting merge SHA from protected `main` may one mutating worker branch be created from exactly that merge SHA.

## Architecture and source of truth

- `PROVEN` — current protected `main` is `187c6b83c6945d79aabef2c5730c3ddba13fcab1`.
- `PROVEN` — Issue #162 remains open and names `OTV2_WORK_DELIVERY_COORDINATOR` as the canonical control plane; no later profile transfer is recorded.
- `PROVEN` — Issue #247 remains open and is `READY_FOR_FRESH_ALLOCATION`; it grants no implementation authority by itself.
- `PROVEN` — preparation #96 / PR #117 defines the exact conditional Server Seam topology and one-writer shared paths.
- `PROVEN` — FND-04 verifier/consumer #115 / PR #151 is terminal and released.
- `PROVEN` — NET/FND-03 Server Seam ceilings are registered: pre-admission connections 256, handshake/auth work 64, outbound queue 64 entries / 1,048,576 bytes per session, pending writes 8 and drain tasks 256, plus inherited FND-02 limits.
- `PROVEN` — Durability terminal replacement #252 and its lifecycle closeout are merged/released.
- `PROVEN` — authority API floor Issue #280 is closed `completed`; PR #289 merged as `be708dc5be5290274f635d534d83f62b2f14b732` with P0=0/P1=0/P2=0, exact-head CI/review and protected-main readback.
- `PROVEN` — on current protected `main`, `ReconnectCandidateBindingV1::from_record(...)`, `AccountPresenceClaimV1::from_identity(...)` and `CharacterWorldEligibilityClaimV1::from_identity(...)` are not production current-authority conveniences; the identity-derived siblings are `#[cfg(test)]` wrappers over private expected-value helpers.
- `PROVEN` — subsequent #302/#303 add independent authority/recovery test harness coverage only and explicitly preserve production behavior; they do not consume Server Seam runtime paths.
- `PROVEN` — `apps/game-server/src/main.rs` still only succeeds for `--smoke`; ordinary gameplay remains fail-closed and there is no production gameplay listener.
- `PROVEN` — the latest open-PR path scan finds no active non-Dependabot writer on the Server Seam primary/shared paths. #305 is workflow/classifier/docs; #295/#293 are docs; #288/#262 are workflow/governance; historical #243 is Durability-only.
- `PROVEN` — Dependabot #259/#260/#261 still touch `Cargo.toml`/`Cargo.lock` but carry no task/allocation ownership. They remain later shared-path reconciliation candidates, not active Server Seam writers.
- `PROVEN` — PR #294 was reconciled to current protected main by normal two-parent merge commit `2d7cc7f21e232b17ab20838f0be6516972357c25`, with no rebase/reset/force update. After that merge-up, compare is `behind_by=0` and the PR still changes exactly these three docs paths.
- `DERIVED` — the prior semantic dependency blocker is cleared; one Server Seam allocation candidate can now be qualified, subject to current-head deterministic checks, whole-diff self-review, independent deep review and review-thread reconciliation.

Accepted technical authority remains merged #117, current FND-02/FND-03/FND-04 contracts/registries, current protected-main Foundation/Durability implementation, current authority-qualification discipline and QA evidence ownership. A lower implementation artifact may not widen them.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: docs-only control-plane allocation; it performs no production authority-consuming mutation, but the released worker is high-risk and must complete the full model in its own task
```

## Allocation decision

After — and only after — this PR itself qualifies, integrates and is read back from protected `main`, Work may release:

```text
lane_id: OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
task_id: OTV2-20260904-gameplay-server-seam
worker: Oteryn: sol server seam lead
branch: agent/otv2-gameplay-server-seam-01
```

The worker branch MUST be created from exactly the allocation merge SHA, never from the historical admission SHA or a cached chat SHA.

### Primary worker paths

- `apps/game-server/src/gameplay_transport/mod.rs`
- `apps/game-server/src/gameplay_transport/tcp_tls.rs`
- `apps/game-server/src/gameplay_transport/connection.rs`
- `apps/game-server/tests/gameplay_server_seam.rs`

### Serialized shared paths

- `apps/game-server/src/foundation/protocol.rs`
- `apps/game-server/src/lib.rs`
- `apps/game-server/src/main.rs`
- `apps/game-server/Cargo.toml`
- `Cargo.toml`
- `Cargo.lock`

Worker governance metadata:

- `docs/agents/tasks/active/OTV2-20260904-gameplay-server-seam.md`

The shared lease authorizes only the accepted Server Seam consumer. It does not authorize unrelated Foundation/Durability refactoring, bot upgrades, stable registry changes, `workspace-boundaries.toml`, workflows or workspace restructuring.

## Dependabot reconciliation

Before the worker's first Cargo mutation and again before final integration, re-read #259/#260/#261 and every active non-Dependabot owner. Never absorb a bot upgrade merely to resolve the lease. If a predecessor merges first, reconcile from fresh protected `main` with normal history-preserving integration and rerun invalidated qualification.

## Review finding reconciliation

The earlier Codex review on historical head `8df5f60f...` produced four actionable threads, now all `outdated` because the plan changed:

- P1 `3936600517` — require independent golden/cross-oracle server encoder vectors;
- P1 `3936600524` — exercise wrong protocol-major / transport-profile rejection through the composed listener;
- P1 `3936600529` — add invalid/expired/replayed/wrong-binding and concurrent fresh-admission RED cases;
- P2 `3936600539` — prove shutdown does not silently lose already-authoritative reserved work.

`PROVEN` — the current child plan contains each of those requirements. The threads remain unresolved historical review records and MUST NOT be counted as final review evidence. Fresh independent review of the new stable exact head is required before their final disposition/resolution.

## Evidence ownership

The Server Seam worker must traverse the actual production listener/composition path in real local TCP/TLS integration tests. That proves the Server Seam boundary only. Formal ADR-0007 QA Tier 1/Tier 2 remains `NOT_EVALUATED` until separately allocated post-merge QA executes it.

## Acceptance criteria

- [ ] PR #294 changes exactly the three allocated docs paths and no runtime/Cargo/workflow/registry path.
- [ ] Final compare to protected `main` has `behind_by=0`.
- [ ] Issue #280 / PR #289 remains terminal on protected `main`; the current authority sibling-family remains production-safe.
- [ ] No active non-Dependabot writer owns any Server Seam primary/shared path.
- [ ] Primary/shared lease remains exactly within accepted #117 topology; `workspace-boundaries.toml`, stable registries, workflows and architecture contracts remain excluded.
- [ ] Worker task binds `AuthorityInvariant × ConsumerBoundary × MutationOperator` and independent current facts.
- [ ] Child plan retains fresh RED -> minimal GREEN, bounded framing/resource proof, FND-04 authority/fencing, golden/cross-oracle encoding, replay/concurrency, unsupported-gameplay fail-close, bounded drain and production-path seam integration.
- [ ] Formal ADR-0007 QA Tier 1/Tier 2 remains separate post-merge QA authority.
- [ ] No production endpoint/certificate/key/secret/deployment topology, gameplay IDs/semantics, Reference fact or Content-format decision is selected.
- [ ] Final allocation head has all current required checks, Work whole-diff self-review, one genuinely independent deep review, zero unresolved required threads and no actionable P0/P1/P2 disposition gap.
- [ ] Worker branch is not created before allocation protected-main merge/readback.

## Excluded scope

No runtime/test-runtime/Cargo/lockfile/Foundation implementation/Durability implementation/registry/workspace policy/workflow/ruleset/production environment/secret/certificate/production endpoint/live account/session/data/Client/QA execution/Movement/Combat/Ability/Interaction/AI/Channel/Analytics/Content-format/Platform/Atlas/META or other external-repository mutation is allowed in this allocation PR.

A required unowned path is `SHARED_LEASE_REQUIRED`; a material public API/protocol/trust/fencing/persistence/resource/evidence-ownership decision is `ARCHITECTURE_ESCALATION_REQUIRED` rather than implicit expansion.

## Implementation / findings

Current candidate reconciliation is control-plane only. The semantic dependency on #280 was correctly discovered before release; it is now terminal and verified on protected main. No runtime authority has been released during the wait.

Do not make bookkeeping-only commits after the next material freeze. Exact head/check/review evidence belongs on the PR/Issue once the stable tracked candidate exists.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: pending fresh exact-head generation after this current-main reconciliation

### Component/integration

- command/run: `python tools/repository/validate_repository_policy.py`
- result: pending fresh exact-head generation after this current-main reconciliation

### E2E

- scenario: `NOT_APPLICABLE` — allocation PR changes docs/control-plane authority only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending stable candidate produced by this reconciliation
- trigger source: pull_request
- workflow/run/job: pending fresh generation
- runner assignment: repository-managed/GitHub-hosted as routed by current protected policy
- classification: docs-only high-risk control-plane allocation
- result: pending

## Self-review

- exact head: pending stable candidate
- method/reviewer: ChatGPT Work Delivery Coordinator whole-diff review against merged #117 and current protected main
- material findings:
  - accepted/repaired historical provenance typo and worker task schema omission;
  - accepted/repaired QA evidence-ownership overclaim;
  - accepted/repaired four historical Codex P1/P2 plan findings listed above;
  - accepted semantic dependency on #280 before release; now terminally cleared by protected-main readback;
  - no current non-Dependabot path collision found after current-main rescan
- verdict: pending final exact-head review after this tracked reconciliation

## Independent review

- required: `YES` — this docs-only PR grants later high-risk protocol/session/admission/fencing and serialized shared-path authority
- exact head: pending stable candidate
- method/auditor: one genuinely independent deep exact-head review under current policy
- material findings: historical reviews are non-final after tracked repairs
- verdict: pending

## PR and closeout

- changed-file review: exactly three allocated docs paths expected
- unresolved review threads: four historical/outdated threads remain; final reconciliation follows fresh representative review
- related PRs: #289 terminal semantic prerequisite; #259/#260/#261 non-owning Cargo candidates
- protected integration: pending current-head qualification/review only; dependency blocker cleared
- merge/result: none
- ownership release: none; worker write authority remains withheld until allocation merge/readback

## Context checkpoint

```yaml
last_progress: protected main advanced to 187c6b83c6945d79aabef2c5730c3ddba13fcab1; #287/#291/#289 and subsequent authority test harness work are integrated; the authority sibling-family is safe on current main, no active non-Dependabot Server Seam path collision exists, and PR #294 was normally merged up to current main without force/rebase/reset
status: validating
branch: coord/gameplay-server-seam-allocation-247
head_sha: null
pr: 294
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending_reconciled_head
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 4
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: publish the matching current-main worker task reconciliation on this same branch, then freeze the resulting remote head for exact-path/main-relation checks, fresh hosted CI and one independent deep review
```
