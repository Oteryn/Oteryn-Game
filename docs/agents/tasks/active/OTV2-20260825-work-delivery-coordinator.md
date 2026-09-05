# OTV2-20260825-work-delivery-coordinator

```yaml
task_id: OTV2-20260825-work-delivery-coordinator
title: Coordinate the post-blocker gameplay vertical slice
mode: COORDINATE
status: COORDINATING
programme_state: WAITING_ARCHITECTURE
active_control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 162
pr: null
protected_main_sha: 730c8bd12cc8c8c2588a97ddcd28f595c0018bca
server_seam_issue: 247
server_seam_allocation_pr: 294
server_seam_allocation_merge_sha: bc9f5dac5642b56135cce31f91b9ed23e5258a70
server_seam_branch: agent/otv2-gameplay-server-seam-01
server_seam_head_sha: 9370b254c6ac4f6529e069c1968ae6bfa1e1750e
server_seam_state: WAITING_ARCHITECTURE
architecture_escalation_issue: 313
nonblocking_governance_reconciliation_issue: 316
owner: ChatGPT Work Delivery Coordinator
created_at: 2026-08-25T23:13:10+02:00
updated_at: 2026-09-05
execution_budget_minutes: 720
large_budget_reason: coordinator lifecycle spanning independently reviewable lane allocations, integration and closeout; no single worker owns the programme
owned_paths:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/superpowers/plans/2026-08-25-oteryn-game-interaction-lifecycle.md
  - docs/superpowers/plans/2026-08-25-oteryn-game-ability-engine.md
  - docs/superpowers/plans/2026-08-25-oteryn-game-durability-journal.md
  - docs/superpowers/plans/2026-08-26-oteryn-game-ai-bootstrap.md
  - docs/agents/evidence/OTV2-20260826-wave-a-post-merge-review-reconciliation.md
  - docs/agents/tasks/active/OTV2-20260825-work-delivery-coordinator.md
public_contracts: []
depends_on:
  - architecture_issue: 313
blocks:
  - compatible native Client release
  - real applicable Tier 1/Tier 2 gameplay QA
  - Movement allocation
  - Combat allocation
cross_repository_coordination_id: OTV2-WORK-DELIVERY-POST-BLOCKER
external_repositories: []
```

The coordinator owns creation and integration control for bounded child allocations, but never inherits a lane worker's runtime authority. Immediately after an allocation is protected-main integrated, the named worker exclusively owns its allocated task/branch paths until terminal return/release. Shared composition, Cargo, registry and governance surfaces remain serialized.

## Outcome

Drive the accepted Oteryn Game post-blocker programme to the first real authoritative Movement + Combat vertical slice. Recompute from protected GitHub state, integrate only exact-head qualified deliveries, and emit `ARCHITECTURE_ESCALATION_REQUIRED` rather than inventing material architecture or persistence semantics.

This packet records the **current** coordinator state. The previous Durability-recovery-era version remains immutable Git history and Issue #162 provenance; its old dispatch/status prose is historical and grants no current worker, merge or architecture authority.

## Current protected state — 2026-09-05

- `PROVEN` — protected `main` is `730c8bd12cc8c8c2588a97ddcd28f595c0018bca` after coordinator correction PR #314 passed the required GitHub Merge Queue and terminal `game-gate`.
- `PROVEN` — the current #162 lifecycle still names `OTV2_WORK_DELIVERY_COORDINATOR`; no merged selector/transfer changes the active mutating control-plane profile.
- `PROVEN` — Interaction Issue #165 is closed `completed`.
- `PROVEN` — Ability Issue #166 is closed `completed`.
- `PROVEN` — pure-local AI Issue #174 is closed `completed`.
- `PROVEN` — Durability terminal repair Issue #250 is closed `completed`; implementation PR #252 merged through Merge Queue as `b67f4425e9e9c5bbf9f7bc94c422cd7478edcdd3`, and archival PR #290 merged as `47faa84152ffdcac00d8e2173d582aa54be2cbcf`; the worker and serialized composition lease are released.
- `PROVEN` — Server Seam allocation PR #294 merged as `bc9f5dac5642b56135cce31f91b9ed23e5258a70` and lawfully released the Server Seam worker.
- `PROVEN` — Server Seam Issue #247 and protected task `OTV2-20260904-gameplay-server-seam` are `WAITING_ARCHITECTURE` on Issue #313.
- `PROVEN` — preserved Server Seam worker branch remains `agent/otv2-gameplay-server-seam-01@9370b254c6ac4f6529e069c1968ae6bfa1e1750e`, tree `3681b01f8a08fc5c9b210b06957834477502b16f`, with no implementation PR.
- `PROVEN` — the preserved worker checkpoint contains only partial Task 1 / initial Task 2 work; it is not integration-eligible and has no production gameplay listener/admission.
- `PROVEN` — Issue #313 is the canonical bounded `ARCHITECTURE_ESCALATION_REQUIRED` for the missing ownership-correct durable fresh-admission / initial `GameSession` boundary.
- `DERIVED` — compatible Client, physical gameplay QA, Movement and Combat cannot be truthfully released while #313 prevents completion/integration of Server Seam.

## Current critical-path blocker

Server Seam Task 3 needs one canonical production-durable fresh-admission / initial `GameSession` commit-and-reconciliation boundary. Current accepted Foundation and Durability surfaces do not provide an ownership-correct production adapter for that operation. The gap is material public authority/persistence architecture and is outside the Server Seam allocation.

Issue #313 therefore owns exactly one architecture decision. It must define the smallest versioned Foundation -> Durability split-phase request/evidence and typed-completion contract, durable linearization, restart/retry/lost-response reconciliation, independently current final-authority checks, forward-migration/schema disposition, implementation ownership and compatibility treatment of synchronous `ReconnectAttemptJournal<T>::commit_fresh`, without redesigning already-frozen reconnect semantics.

Holding action:

- preserve `agent/otv2-gameplay-server-seam-01@9370b254c6ac4f6529e069c1968ae6bfa1e1750e` unchanged;
- no reset/rebase/force/restart and no implementation PR for the partial candidate;
- no Foundation/Durability scope seizure, schema invention, lease widening or transport-local in-memory substitute;
- no Client, Movement or Combat allocation through the unresolved boundary;
- independent path-disjoint work may continue only under its own current allocation/control plane.

## Next authorized sequence

1. `Oteryn: sol supervising architect` resolves only Issue #313 and returns a reviewed/qualified architecture decision integrated to protected `main`.
2. Work re-reads the then-current protected `main`, active Issues/PRs/tasks, ownership and path overlaps.
3. If the architecture return requires Foundation/Durability implementation, Work creates and integrates the smallest fresh bounded allocation for the proper owner; the Server Seam lease is not widened to absorb it.
4. The owning Foundation/Durability worker implements and qualifies that prerequisite through normal protected controls; Work integrates it and verifies protected-main readback.
5. Work re-evaluates the preserved Server Seam branch against that exact main. Only then may the existing Server Seam worker reconcile normally and resume Task 3+.
6. When the Server Seam worker returns `READY_FOR_INTEGRATION`, Work performs the full exact-head diff/review/CI/Merge Queue/post-merge lifecycle and releases the lane.
7. Recompute compatible Client -> applicable real Tier 1/Tier 2 QA -> Movement -> Combat readiness from the new protected main.

No owner-only product decision is currently requested. If the Supervising Architect reports a genuine owner-only semantic choice, Work records the exact blocker instead of guessing.

## Open-state disposition

A repository search may still find historical, template, draft, Dependabot or separately owned PR/task prose containing `READY_FOR_INTEGRATION` or old Durability/Server Seam states. Those strings are not current authority. A Work integration action requires a fresh lane return tied to the exact current allocation, exact head, changed-path allowlist, required review and CI generation.

The current #162 critical path has no such Server Seam return: #247 is explicitly `WAITING_ARCHITECTURE`, and the downstream Client/QA/Movement/Combat chain remains dependency-blocked by that fact.

Issue #316 separately records a non-blocking shared-programme provenance defect discovered during this coordinator readback. Current `OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md` still carries an unqualified historical `PR #314` release predicate from the retired `blakinio/Oteryn-v2` repository. Fresh external-history readback proves that `blakinio/Oteryn-v2#314` lawfully merged the DAG as `88f4fb754b5ae11243afd38a9e0b6a8e3b0a5815`; current `Oteryn/Oteryn-Game#314` is unrelated. Nearest `docs/agents/AGENTS.md` explicitly makes active task/live PR state authoritative over stale shared indexes, so #316 does not block this checkpoint or #313. It must be reconciled before a future allocation treats the DAG header itself as release proof.

## Architecture and source of truth

Governing order:

1. protected-main repository governance and active selector;
2. current Issue/PR/CI/Merge Queue state;
3. merged exact allocation and lane-specific accepted contracts;
4. current worker branch/task evidence;
5. historical task prose and chat state only as locators/evidence.

Relevant protected authority:

- `AGENTS.md`
- `docs/agents/AGENTS.md`
- `docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md`
- `docs/agents/prompts/OTV2_IMPLEMENTATION_COORDINATOR.md`
- `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md`
- `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md`
- `docs/architecture/reviews/OTERYN_GAME_POST_BLOCKER_WORK_ORCHESTRATION_2026-08-25.md`
- `docs/superpowers/plans/2026-08-25-oteryn-game-work-delivery-orchestration.md`

## Acceptance criteria

- [x] Exactly one active mutating #162 control-plane profile is resolved: `OTV2_WORK_DELIVERY_COORDINATOR`.
- [x] Interaction, Ability and AI child Issues are terminal completed.
- [x] Durability terminal repair is protected-main integrated and its worker/shared lease is released.
- [x] Server Seam received a lawful merged exact allocation and its worker checkpoint is preserved.
- [x] Material fresh-admission persistence/authority gap was escalated as #313 instead of being guessed through.
- [x] Server Seam task/Issue/parent control-plane state was reconciled through PR #314 and protected-main readback.
- [x] Ambiguous historical executor-DAG PR-number provenance was isolated as non-blocking governance Issue #316 rather than silently reinterpreted.
- [ ] #313 architecture decision is reviewed, qualified and integrated to protected `main`.
- [ ] Any architecture-required Foundation/Durability prerequisite is freshly allocated, implemented, qualified, merged and read back.
- [ ] Server Seam is completed, independently reviewed, exact-head qualified and merged through protected controls.
- [ ] Compatible native Client is allocated/merged from the post-Server-Seam main.
- [ ] Applicable real Tier 1/Tier 2 gameplay QA is truthfully proven.
- [ ] Movement readiness checkpoint, resource/registry conditions and exact child allocation are completed; Movement merges through all gates.
- [ ] Combat readiness checkpoint is proven from merged Movement plus Ability/Interaction/Durability/Client/QA; Combat merges through all gates.
- [ ] Programme terminal reconciliation proves all used tasks/PRs/leases are terminal and protected-main readback matches the claim.

## Excluded scope

No gameplay/runtime implementation, Cargo/workspace mutation, registry/stable-ID mutation, public contract or schema decision, workflow/ruleset/protection weakening, production deployment, protected-environment/secret/live-data mutation, Platform/Atlas/META/external-repository write, Reference-parity claim or permanent Content-format decision is authorized merely because Work owns this coordinator packet.

The coordinator may create bounded allocations and integrate qualified worker returns only under current accepted authority. Material architecture remains the Supervising Architect's domain.

## Implementation / findings

This reconciliation removes obsolete active-state claims from the coordinator packet. It does not reopen or reinterpret completed Wave A work, does not ratify historical invalid Durability branches, and does not modify any worker branch or runtime path.

The current critical-path material finding is #313 only. Issue #316 is independent low-risk governance provenance cleanup and grants no write authority by itself. The Server Seam partial checkpoint is preserved as evidence, not as a merge candidate. No other lane is promoted merely to occupy concurrency capacity.

## Validation

### Focused

- validation target: coordinator task/control-plane consistency against protected `main`, Issues #162/#165/#166/#174/#247/#250/#313/#316, merged allocation #294 and protected correction #314
- result: live GitHub readback performed before mutation

### Component/integration

- scenario: documentation/control-plane reconciliation only
- result: runtime/component/E2E execution is `NOT_APPLICABLE`; this change grants no runtime, contract, schema, Cargo or worker authority

### E2E

- scenario: `NOT_APPLICABLE` — no executable gameplay behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- exact head: recorded in PR/CI evidence rather than self-referential task content
- required result: repository PR governance/checks green, then normal protected Merge Queue qualification and `game-gate`

## Self-review

- exact head: recorded in immutable PR evidence after the final material content change
- method: whole-diff review against the single owned coordinator packet path
- verdict: `PASS` at content freeze; zero authority expansion, zero worker lease mutation, zero architecture decision, zero runtime/Cargo/schema/workflow mutation

## Independent review

- required: `NO` for this low-risk coordinator metadata/docs reconciliation under current protected root `AGENTS.md` AI review policy
- reason: no runtime/protocol/persistence/value/production semantics, no safety reduction, no authority expansion and no gate weakening; external AI review is explicitly unnecessary for trivial docs/metadata
- enforcement remains: exact-head repository checks, `game-gate`, protection and Merge Queue

## PR and closeout

- changed-file allowlist for this reconciliation: `docs/agents/tasks/active/OTV2-20260825-work-delivery-coordinator.md` only
- terminal merge authority: normal protected repository control plane only; no direct/self/bypass merge
- coordinator programme remains active after this checkpoint; this is not Movement/Combat programme closeout

## Context checkpoint

```yaml
last_progress: Server Seam architecture hold is durably recorded on protected main by PR #314; Work coordinator packet is reconciled to the current #313-gated critical path and non-blocking DAG provenance defect is tracked as #316
status: COORDINATING
programme_state: WAITING_ARCHITECTURE
active_control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
protected_main_sha: 730c8bd12cc8c8c2588a97ddcd28f595c0018bca
server_seam_issue: 247
server_seam_branch: agent/otv2-gameplay-server-seam-01
server_seam_head_sha: 9370b254c6ac4f6529e069c1968ae6bfa1e1750e
server_seam_pr: null
server_seam_state: WAITING_ARCHITECTURE
architecture_escalation_issue: 313
nonblocking_governance_reconciliation_issue: 316
owner_action_required: null
blocker: architecture_issue_313_durable_fresh_admission_initial_GameSession_boundary
next_action: Oteryn: sol supervising architect resolves Issue #313 and returns a reviewed and qualified architecture decision integrated to protected main
```
