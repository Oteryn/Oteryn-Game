# OTV2-20260904-gameplay-server-seam-allocation

```yaml
task_id: OTV2-20260904-gameplay-server-seam-allocation
title: Allocate production gameplay Server Seam
mode: COORDINATE
status: blocked
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
updated_at: 2026-09-04T20:00:00+02:00
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
  - issue: 280
  - merged_pr: 117
  - merged_pr: 151
  - merged_pr: 252
  - merged_pr: 290
blocks:
  - OTV2-20260904-gameplay-server-seam
external_repositories: []
```

## Outcome

Prepare one fresh docs-only allocation for the accepted production gameplay Server Seam, while withholding every runtime/shared-path write right until the live post-#252 authority API floor is terminally repaired and read back from protected `main`.

PR #294 may remain open Draft and may receive deterministic docs/control-plane qualification while the dependency is active. It MUST NOT integrate, release the worker, create `agent/otv2-gameplay-server-seam-01`, or authorize any Server Seam runtime write while Issue #280 / PR #289 is nonterminal.

This PR changes no runtime behavior, wire bytes, session/durable state, Cargo dependency, workflow/protection, production setting, secret or external repository.

## Architecture and source of truth

- `PROVEN` — allocation admission `main` is `68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705`.
- `PROVEN` — Issue #162 is the live Work coordinator lifecycle; its prior checkpoint selected a fresh Server Seam allocation after historical Durability closeout.
- `PROVEN` — Issue #247 is open and its allocation checkpoint grants no implementation write authority by itself.
- `PROVEN` — preparation #96 / PR #117 merged as `4079804b7f1f29cc2b7db2e746d4da2861bff084` and defines the accepted conditional Server Seam topology, shared paths, negative tests and QA evidence boundary.
- `PROVEN` — FND-04 verifier/consumer Issue #115 is terminal; archived task `OTV2-20260825-fnd04-verifier-consumer` records PR #151 merged as `2d0e951ce37c2e28773c22966bb816c00bebaa0a` with ownership released.
- `PROVEN` — Issue #116 is closed and the current Resource Limits Registry contains the accepted Server Seam NET03 ceilings.
- `PROVEN` — historical Durability implementation/archival work #252/#290 is merged and released.
- `PROVEN` — live Issue #280 is OPEN / `IMPLEMENTING` and PR #289 is OPEN Draft on branch `refactor/authority-api-floor-280`; this is a later post-#252 authority-contract-floor repair, not historical Durability ownership.
- `PROVEN` — current PR #289 head observed during this allocation review is `ddbb44d2644c6f66bf86aba837d7712b01878fac` and its `admission_recovery_inner.rs` patch makes `ReconnectCandidateBindingV1::from_record(...)` test-only/internal for expected matching.
- `PROVEN` — on that same current #289 head, `CharacterWorldEligibilityClaimV1::from_identity(&ReconnectIdentityV1)` remains production-public and directly derives a supposedly current eligibility claim from immutable reconnect identity.
- `PROVEN` — on that same current #289 head, `AccountPresenceClaimV1::from_identity(&ReconnectIdentityV1)` remains production-public and directly derives a supposedly current presence claim from immutable reconnect identity.
- `DERIVED` — because Server Seam will consume FND-04/current-authority/reconnect boundaries, the nonterminal #280 sibling-family repair is a semantic release dependency even though PR #289 does not own the exact Server Seam files.
- `PROVEN` — PR #289 itself states final integration qualification is pending later convergence before its own terminal review/integration; therefore PR #294 cannot infer #280 completion from #289 being mergeable or from a historical green head.
- `PROVEN` — current `apps/game-server/src/main.rs` has no gameplay listener; ordinary gameplay execution remains fail-closed.
- `PROVEN` — current `apps/game-server/src/foundation/protocol.rs` owns FND-02 inbound wire validation and remains the accepted shared lease for only the minimum Server Seam typed bridge/registered server encoders.
- `PROVEN` — current open non-Dependabot path scan found no exact Server Seam path collision. #289 is a semantic authority dependency, not a path-ownership collision.
- `PROVEN` — Dependabot #259/#260/#261 touch root Cargo files but have no task/allocation ownership; they remain later reconciliation candidates.
- `CONFLICT` — `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md` contains stale release-candidate prose referring to nonexistent PR #314. It does not override the newer live coordinator/Issue state and is outside this allocation repair.

Accepted technical authority remains merged #117, current FND-02/FND-03/FND-04 contracts/registries, current protected-main Foundation/Durability implementation, current high-risk authority qualification policy and QA-E2E evidence ownership. A lower implementation artifact may not widen them.

## Blocking dependency gate

The following gate is mandatory before PR #294 can leave Draft for protected integration:

1. Issue #280 is terminally completed and its final repair is integrated to protected `main` through repository controls.
2. PR #289 (or its explicitly superseding terminal repair) has protected-main readback; a Draft/open/green branch head is not enough.
3. Work refreshes protected `main`, Issue #280, PR #289/successor, exact current authority API declarations, current Server Seam path ownership, Dependabot Cargo candidates and all required checks/review state.
4. Work repeats the authority sibling-family sweep for production-public record/identity-derived convenience constructors relevant to current authority.
5. Only if that sweep leaves no unresolved P0/P1 authority-floor finding does Work recompute PR #294's plan/leases against the new protected-main APIs.
6. Any material API/lease/plan change invalidates prior PR #294 exact-head review/qualification and receives a fresh stable-candidate generation before integration.

No amount of green CI on this docs-only allocation can substitute for that semantic dependency.

## Evidence ownership correction from coordinator self-review

Merged #117 requires the Server Seam implementation to traverse the actual production listener/composition path in real local TCP/TLS integration tests, while formal ADR-0007 QA Tier 1/Tier 2 remains owned by a separate post-merge QA lane. Therefore the worker may prove **Server Seam physical integration** for its candidate, but must report QA Tier 1/Tier 2 as `NOT_EVALUATED` until separately allocated QA proves them.

The child plan also carries #117's golden/cross-oracle server encoding, wrong protocol/profile, invalid/expired/replayed/wrong-binding FND-04 material, concurrent fresh-admission replay, unsupported gameplay and non-silent shutdown handling of already-authoritative reserved work.

## Allocation decision

When — and only when — the blocking dependency gate above is fully proven and PR #294 itself subsequently qualifies/merges, Work may release:

```text
lane_id: OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
task_id: OTV2-20260904-gameplay-server-seam
worker: Oteryn: sol server seam lead
branch: agent/otv2-gameplay-server-seam-01
```

The worker branch is created from exactly the eventual allocation merge SHA read back from protected `main`, never from this admission SHA or a cached SHA.

### Primary worker paths after lawful release

- `apps/game-server/src/gameplay_transport/mod.rs`
- `apps/game-server/src/gameplay_transport/tcp_tls.rs`
- `apps/game-server/src/gameplay_transport/connection.rs`
- `apps/game-server/tests/gameplay_server_seam.rs`

### Serialized shared paths after lawful release

- `apps/game-server/src/foundation/protocol.rs`
- `apps/game-server/src/lib.rs`
- `apps/game-server/src/main.rs`
- `apps/game-server/Cargo.toml`
- `Cargo.toml`
- `Cargo.lock`

Worker governance metadata:

- `docs/agents/tasks/active/OTV2-20260904-gameplay-server-seam.md`

The shared lease authorizes only the accepted Server Seam consumer. It does not authorize #280/#289 repair, unrelated Foundation/Durability refactoring, bot upgrades, stable registry changes or workspace restructuring.

## Dependabot reconciliation

Before first Cargo mutation and before final worker integration, re-read #259/#260/#261 plus every active non-Dependabot owner. Never merge bot branches into the worker lane merely for convenience. A predecessor merge is reconciled from fresh protected `main` with normal history-preserving integration and requalification.

## Mandatory execution order

1. Keep PR #294 docs-only and Draft while #280 is nonterminal.
2. Do not mutate #280/#289 paths; their existing writer retains single-writer authority.
3. Observe #280/#289 only for terminal handoff/integration evidence.
4. After protected-main #280 terminal readback, refresh current authority APIs and rerun the sibling-family sweep.
5. Reconcile PR #294 plan/lease metadata if the final authority API shape changes any Server Seam assumption.
6. Qualify the resulting stable PR #294 head with deterministic exact-head checks, whole-diff Work self-review and one genuinely independent deep review.
7. Integrate only a clean, current, expected-head allocation through protected controls.
8. Read allocation merge SHA from protected `main`; then create the worker branch from exactly that SHA and release one Sol writer.
9. Worker performs fresh RED -> minimal GREEN, preserves canonical Foundation/Durability authority, proves physical Server Seam integration without claiming QA Tier 1/Tier 2 completion, completes family sweep/exact-head CI/independent review and returns `READY_FOR_INTEGRATION` without self-merging.
10. Work independently verifies/integrates the worker and only then recomputes Client and QA readiness.

## Acceptance criteria

- [ ] PR #294 changes exactly these three docs paths and no runtime/Cargo/workflow/registry path.
- [ ] Issue #280 / PR #289 or its explicit successor is terminal on protected `main` before PR #294 integration/release.
- [ ] Fresh post-#280 authority sibling-family sweep finds no production-public record/identity-derived current-authority convenience that violates the active authority API floor.
- [ ] Any final #280 API change is reconciled into the Server Seam plan/task before final allocation review.
- [ ] No active non-Dependabot writer owns any final Server Seam primary/shared path.
- [ ] Primary/shared lease remains within accepted #117 topology and excludes `workspace-boundaries.toml`, stable registries, workflows and architecture contracts.
- [ ] Worker task binds `AuthorityInvariant × ConsumerBoundary × MutationOperator` and independent current facts.
- [ ] Child plan retains fresh RED -> minimal GREEN, bounded framing/resource proof, FND-04 authority/fencing, golden/cross-oracle encoding, replay/concurrency, unsupported-gameplay fail-close, bounded drain and production-path seam integration.
- [ ] Formal ADR-0007 QA Tier 1/Tier 2 remains separate post-merge QA authority.
- [ ] No production endpoint/certificate/key/secret/deployment topology, gameplay IDs/semantics, Reference fact or Content-format decision is selected.
- [ ] Final allocation stable head has all required current-head checks, self-review, independent review, zero unresolved required threads and no actionable P0/P1.
- [ ] Worker branch is not created before both dependency readback and allocation protected-main merge/readback.

## Excluded scope

No runtime/test-runtime/Cargo/lockfile/Foundation implementation/Durability implementation/#280 repair/registry/workspace policy/workflow/ruleset/production environment/secret/certificate/production endpoint/live account/session/data/Client/QA execution/Movement/Combat/Ability/Interaction/AI/Channel/Analytics/Content-format/Platform/Atlas/META or other external-repository mutation is allowed in this allocation PR.

A required unowned path is `SHARED_LEASE_REQUIRED`; a material public API/protocol/trust/fencing/persistence/resource/evidence-ownership decision is `ARCHITECTURE_ESCALATION_REQUIRED` rather than implicit expansion.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: pending new exact-head generation after dependency-gate correction

### Component/integration

- command/run: `python tools/repository/validate_repository_policy.py`
- result: pending new exact-head generation after dependency-gate correction

### E2E

- scenario: `NOT_APPLICABLE` — allocation PR changes docs/control-plane authority only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: null
- trigger source: pull_request
- workflow/run/job: prior generations are historical after this tracked dependency-gate correction
- classification: docs-only high-risk allocation/control-plane change
- result: pending new current-head generation

## Self-review

- exact head: pending new stable candidate
- method/reviewer: ChatGPT Work Delivery Coordinator whole-diff review against merged #117 plus live authority programme
- material findings:
  - repaired pre-qualification provenance typo in historical Durability text
  - repaired worker task schema omission by binding Issue #247
  - repaired QA evidence-ownership overclaim and literal #117 negative/golden/shutdown coverage
  - `P1 accepted`: path-only collision analysis missed the semantic dependency on active authority-floor Issue #280 / PR #289; direct exact-head inspection proved both `CharacterWorldEligibilityClaimV1::from_identity(...)` and `AccountPresenceClaimV1::from_identity(...)` remain production-public on current #289 head, so allocation integration/release is now gated on terminal #280 readback and a fresh sibling-family sweep
- verdict: `BLOCKED_DEPENDENCY` pending #280 terminal readback; no Server Seam runtime authority released

## Independent review

- required: `YES` for the eventual stable integrable allocation candidate
- exact head: null
- method/auditor: one genuinely independent deep exact-head review under current policy
- material findings: all prior requests are historical after tracked corrections; do not count them for final integration
- verdict: pending after #280 terminal reconciliation

## PR and closeout

- changed-file review: exactly three allocated docs paths expected
- unresolved review threads: must be re-read for final candidate
- related PRs: #289 is a semantic prerequisite but owns no Server Seam path; #259/#260/#261 are non-owning Cargo candidates
- protected integration: `BLOCKED` while Issue #280 is nonterminal
- merge/result: none
- ownership release: none; worker write authority remains withheld

## Context checkpoint

```yaml
last_progress: live semantic dependency review proved authority-floor Issue #280 / PR #289 is still implementing and both AccountPresenceClaimV1::from_identity(...) and CharacterWorldEligibilityClaimV1::from_identity(...) remain production-public on current PR #289 head; PR #294 was therefore converted from merely validating to dependency-blocked before any integration or worker release
status: blocked
branch: coord/gameplay-server-seam-allocation-247
head_sha: null
pr: 294
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending_dependency_gate_head
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
blocker: authority_api_floor_issue_280_pr_289_nonterminal
next_action: leave PR #294 Draft; observe the existing #280/#289 single-writer lane to terminal protected-main integration/readback, then refresh main and authority APIs, rerun the sibling-family sweep, reconcile this allocation if necessary, and only then perform final exact-head qualification/review/integration
```
