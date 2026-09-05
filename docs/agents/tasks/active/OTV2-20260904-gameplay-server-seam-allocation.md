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
integration_main_sha: 62590071b7e47e3221af0e180c73bbc7cdf37c31
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
updated_at: 2026-09-05T15:27:42+02:00
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

Create and qualify one docs-only control-plane allocation for the already accepted production gameplay Server Seam. This allocation changes no runtime, wire bytes, durable/session state, Cargo dependency, workflow/protection, production setting, secret or external repository.

Only after PR #294 integrates through protected controls and Work reads the resulting merge SHA from protected `main` may one mutating worker branch be created from exactly that merge SHA.

## Architecture and source of truth

- `PROVEN` — current protected `main` is `62590071b7e47e3221af0e180c73bbc7cdf37c31`.
- `PROVEN` — Issue #162 remains open and names `OTV2_WORK_DELIVERY_COORDINATOR` as the canonical control plane; no later profile transfer is recorded.
- `PROVEN` — Issue #247 remains open and grants no implementation authority by itself; worker authority still depends on allocation PR #294 protected integration/readback.
- `PROVEN` — preparation #96 / PR #117 defines the exact conditional Server Seam topology, primary paths and serialized shared paths.
- `PROVEN` — FND-04 verifier/consumer #115 / PR #151 is terminal and released.
- `PROVEN` — accepted NET/FND-03 Server Seam ceilings are registered: pre-admission connections 256, handshake/auth work 64, outbound queue 64 entries / 1,048,576 bytes per session, pending writes 8 and drain tasks 256, plus inherited FND-02 limits.
- `PROVEN` — Durability terminal replacement #252 and lifecycle closeout are merged/released.
- `PROVEN` — authority API floor Issue #280 is closed `completed`; PR #289 merged as `be708dc5be5290274f635d534d83f62b2f14b732` with P0=0/P1=0/P2=0, independent review, exact-head CI and protected-main readback.
- `PROVEN` — current protected-main `ReconnectCandidateBindingV1::from_record(...)`, `AccountPresenceClaimV1::from_identity(...)` and `CharacterWorldEligibilityClaimV1::from_identity(...)` are not production current-authority conveniences; the identity-derived siblings are `#[cfg(test)]` wrappers over private expected-value helpers.
- `PROVEN` — subsequent #302/#303 extend independent current-authority/retry/restart/PostgreSQL test coverage and explicitly preserve production semantics.
- `PROVEN` — PR #305 changed standalone protected-main `rust.yml` lane selection only and explicitly left PR `game-gate`, Merge Queue workflow and ruleset unchanged.
- `PROVEN` — PR #306 merged as current main `62590071b7e47e3221af0e180c73bbc7cdf37c31` and adds exactly `apps/game-server/tests/server_ci_qualification.rs`; it changes no runtime source, Cargo/dependency, classifier, workflow, ruleset, Merge Queue or fan-in authority and creates no Server Seam path ownership.
- `PROVEN` — current `apps/game-server/src/main.rs` remains fail-closed outside `--smoke`; no production gameplay listener exists.
- `PROVEN` — latest open-PR scan finds no active non-Dependabot writer on any Server Seam primary/shared path. #295/#293 are docs; #288/#262 are workflow/governance; historical #243 is Durability-only.
- `PROVEN` — Dependabot #259/#260/#261 still touch `Cargo.toml`/`Cargo.lock` but carry no task/allocation ownership. They remain later shared-path reconciliation candidates, not active Server Seam writers.
- `PROVEN` — PR #294 was reconciled to protected main by normal two-parent merges without rebase/reset/force. The latest merge-up is `ffb10d5d263de4d2f0d81fead71442a1e49f66ae` with parents `252198ddd8ee9c605b343c4643b4cc0ffa3bef63` and current main `62590071b7e47e3221af0e180c73bbc7cdf37c31`; the branch ref update was fast-forward with `force=false`.
- `PROVEN` — compare immediately after the #306 merge-up is `behind_by=0` and exactly the same three allocated docs paths remain changed relative to current protected main.
- `DERIVED` — the historical #280 semantic blocker is cleared; #305/#306 do not alter the accepted Server Seam architecture, API lease or review repair, so the allocation can proceed to one final exact-head qualification/review generation after this authority-record synchronization.

Accepted technical authority remains merged #117, current FND-02/FND-03/FND-04 contracts/registries, current protected-main Foundation/Durability implementation, current authority-qualification discipline and QA evidence ownership. A lower implementation artifact may not widen them.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: docs-only allocation performs no production authority-consuming mutation; the released worker is high-risk and must complete the full model in its own task
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

The shared lease authorizes only the accepted Server Seam consumer. It does not authorize unrelated Foundation/Durability refactoring, bot upgrades, stable registries, `workspace-boundaries.toml`, workflows or workspace restructuring.

## Dependabot reconciliation

Before the worker's first Cargo mutation and again before final integration, re-read #259/#260/#261 and every active non-Dependabot owner. Never absorb a bot upgrade merely to resolve the lease. If a predecessor merges first, reconcile from fresh protected `main` with normal history-preserving integration and rerun invalidated qualification.

## Review finding reconciliation

Historical Codex review on head `8df5f60f...` produced four actionable threads, all now `outdated` because the plan changed:

- P1 `3936600517` — independent golden/cross-oracle server encoder vectors;
- P1 `3936600524` — wrong protocol-major / transport-profile rejection through the composed listener;
- P1 `3936600529` — invalid/expired/replayed/wrong-binding and concurrent fresh-admission RED cases;
- P2 `3936600539` — shutdown must not silently lose already-authoritative reserved work.

Later exact-head review generations found two further P1s:

- P1 `3940618846` — composed-listener wrong protocol-major/profile cases were not literally included in the focused listener suite; accepted and repaired before head `6b3a5d73...`;
- P1 `3940738538` — those listener-path cases were placed in Task 2 before the production composition existed; accepted and repaired in `252198ddd8ee9c605b343c4643b4cc0ffa3bef63` by keeping Task 2 transport-only and moving the production-listener RED/GREEN to Task 5 after connection/lifecycle prerequisites.

`PROVEN` — the current child plan contains all six required finding-family dispositions. All prior CI/review generations are historical after the tracked repairs and #306 main reconciliation. One fresh representative independent review of the final exact head is required before final thread reconciliation/resolution.

## Evidence ownership

Server Seam implementation must traverse the actual production listener/composition path in local TCP/TLS integration tests. That proves the Server Seam boundary only. Formal ADR-0007 QA Tier 1/Tier 2 remains `NOT_EVALUATED` until separately allocated post-merge QA executes it.

## Acceptance criteria

- [ ] PR #294 changes exactly the three allocated docs paths and no runtime/Cargo/workflow/registry path.
- [ ] Final compare to protected `main` has `behind_by=0`.
- [ ] Authority API floor remains terminal and production-safe.
- [ ] No active non-Dependabot writer owns any Server Seam primary/shared path.
- [ ] Primary/shared lease remains exactly within #117 and excludes `workspace-boundaries.toml`, stable registries, workflows and architecture contracts.
- [ ] Worker task binds `AuthorityInvariant × ConsumerBoundary × MutationOperator` and independently current facts.
- [ ] Child plan retains fresh RED -> minimal GREEN, bounded framing/resource proof, FND-04 authority/fencing, golden/cross-oracle encoding, replay/concurrency, unsupported-gameplay fail-close, bounded drain and production-path integration.
- [ ] Formal ADR-0007 QA Tier 1/Tier 2 remains separate post-merge QA authority.
- [ ] No production endpoint/certificate/key/secret/deployment topology, gameplay IDs/semantics, Reference fact or Content-format decision is selected.
- [ ] Final allocation head has all current required checks, Work whole-diff self-review, one genuinely independent deep review, zero unresolved required threads and no actionable P0/P1/P2 disposition gap.
- [ ] Worker branch is not created before allocation protected-main merge/readback.

## Excluded scope

No runtime/test-runtime/Cargo/lockfile/Foundation implementation/Durability implementation/registry/workspace policy/workflow/ruleset/production environment/secret/certificate/production endpoint/live account/session/data/Client/QA execution/Movement/Combat/Ability/Interaction/AI/Channel/Analytics/Content-format/Platform/Atlas/META or external-repository mutation is allowed in this allocation PR.

A required unowned path is `SHARED_LEASE_REQUIRED`; a material public API/protocol/trust/fencing/persistence/resource/evidence-ownership decision is `ARCHITECTURE_ESCALATION_REQUIRED` rather than implicit expansion.

## Implementation / findings

The earlier semantic dependency on #280 was correctly discovered before release and is now terminally cleared. No runtime authority was released while it was open.

The post-review plan repair `252198ddd8ee9c605b343c4643b4cc0ffa3bef63` resolved the verified Task-2/Task-5 ordering P1. Protected main then advanced through #306; `ffb10d5d263de4d2f0d81fead71442a1e49f66ae` normally merged that protected main into the branch without changing the final three-path PR scope. This task-record synchronization is the final current-main authority update before the next material freeze.

Do not make bookkeeping-only commits after that freeze. Exact head/check/review evidence belongs on the PR/Issue once the stable tracked candidate exists.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: pending fresh exact-head generation after final #306 authority-record synchronization

### Component/integration

- command/run: `python tools/repository/validate_repository_policy.py`
- result: pending fresh exact-head generation after final #306 authority-record synchronization

### E2E

- scenario: `NOT_APPLICABLE` — allocation PR changes docs/control-plane authority only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending stable candidate after final task-record synchronization
- trigger source: pull_request
- workflow/run/job: pending fresh generation
- runner assignment: repository-managed/GitHub-hosted as routed by protected policy
- classification: docs-only high-risk control-plane allocation
- result: pending

## Self-review

- exact head: pending stable candidate
- method/reviewer: ChatGPT Work Delivery Coordinator whole-diff review against merged #117 and current protected main
- material findings:
  - accepted/repaired historical provenance typo and worker-task schema omission;
  - accepted/repaired QA evidence-ownership overclaim;
  - accepted/repaired four historical Codex P1/P2 plan findings;
  - accepted/repaired exact-head listener-suite P1 `3940618846`;
  - accepted/repaired exact-head sequential-composition P1 `3940738538`;
  - accepted semantic dependency on #280 before release; terminally cleared by protected-main readback;
  - no current non-Dependabot path collision;
  - #305/#306 do not require Server Seam plan/lease expansion
- verdict: pending final exact-head review after this authority-record synchronization

## Independent review

- required: `YES` — docs-only PR grants later high-risk protocol/session/admission/fencing and serialized shared-path authority
- exact head: pending stable candidate
- method/auditor: one genuinely independent deep exact-head review under current policy
- material findings: prior reviews are non-final after tracked repairs/main reconciliation
- verdict: pending

## PR and closeout

- changed-file review: exactly three allocated docs paths expected
- unresolved review threads: historical/repaired threads remain; final reconciliation follows fresh representative review
- related PRs: #289 terminal semantic prerequisite; #305/#306 protected-main predecessors; #259/#260/#261 non-owning Cargo candidates
- protected integration: pending final exact-head qualification/review only
- merge/result: none
- ownership release: none; worker authority withheld until allocation merge/readback

## Context checkpoint

```yaml
last_progress: protected main advanced to 62590071b7e47e3221af0e180c73bbc7cdf37c31 via #306; #306 adds exactly one durable server-only CI qualification test and does not alter Server Seam runtime/API/lease or protected Merge Queue authority; PR #294 was normally merged up as ffb10d5d263de4d2f0d81fead71442a1e49f66ae and remains behind_by=0 with exactly the same three docs paths; this task-record sync records the current authority before final freeze
status: validating
branch: coord/gameplay-server-seam-allocation-247
head_sha: null
pr: 294
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending_post_306_task_record_sync
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 6
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: synchronize the matching waiting worker task to current main, then freeze the resulting remote head and perform exact-path/main-relation checks, fresh hosted CI, Work whole-diff self-review and one independent exact-head deep review
```
