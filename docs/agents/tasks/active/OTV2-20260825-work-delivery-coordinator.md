# OTV2-20260825-work-delivery-coordinator

```yaml
task_id: OTV2-20260825-work-delivery-coordinator
title: Coordinate the post-blocker gameplay vertical slice
mode: COORDINATE
status: COORDINATING
programme_state: WAITING_DEPENDENCY
active_control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 162
pr: null
protected_main_sha: a8678d4a94e479a9aa2a92920379a4b32f95143b
server_seam_issue: 247
server_seam_allocation_pr: 294
server_seam_allocation_merge_sha: bc9f5dac5642b56135cce31f91b9ed23e5258a70
server_seam_branch: agent/otv2-gameplay-server-seam-01
server_seam_head_sha: 9370b254c6ac4f6529e069c1968ae6bfa1e1750e
server_seam_state: WAITING_DEPENDENCY
architecture_escalation_issue: 313
nonblocking_governance_reconciliation_issue: 316
owner: ChatGPT Work Delivery Coordinator
created_at: 2026-08-25T23:13:10+02:00
updated_at: 2026-09-05
execution_budget_minutes: 720
large_budget_reason: coordinator lifecycle spanning independently reviewable lane allocations, integration and closeout; no single worker owns the programme
owned_paths:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md
  - docs/superpowers/plans/2026-09-05-foundation-fresh-admission-durability.md
  - docs/agents/tasks/active/OTV2-20260905-foundation-fresh-admission-318.md
  - docs/superpowers/plans/2026-08-25-oteryn-game-interaction-lifecycle.md
  - docs/superpowers/plans/2026-08-25-oteryn-game-ability-engine.md
  - docs/superpowers/plans/2026-08-25-oteryn-game-durability-journal.md
  - docs/superpowers/plans/2026-08-26-oteryn-game-ai-bootstrap.md
  - docs/agents/evidence/OTV2-20260826-wave-a-post-merge-review-reconciliation.md
  - docs/agents/tasks/active/OTV2-20260825-work-delivery-coordinator.md
public_contracts: []
depends_on:
  - foundation_child_issue: 318
  - separately allocated Child B persistence and Child C producer readiness
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

Current correction: #313 is completed after reviewed PR #317 passed Merge Queue run `33983003548` and integrated as `a8678d4a94e479a9aa2a92920379a4b32f95143b`, tree `d7224da9885fd1b55406c3f64d48f2c239508df8`. #247 is now `WAITING_DEPENDENCY` on A (#318), separately allocated B and C, and actual C producer readiness. Its worker checkpoint/path lease is preserved. The historical architecture-hold observations below remain provenance, not a current unresolved design.

The exact prospective A allocation names `agent/foundation-fresh-admission-318`, seven Foundation paths and its task; its immutable admission base is this allocation's future protected merge SHA. Work remains the sole control plane and gains no lane runtime ownership. The separately coordinated #247 task edit is metadata-only and preserves worker head, owned paths and all partial test evidence.

Historical pre-architecture checkpoint:

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

Accepted decision `FND-DUR-FRESH-ADMISSION-V1` is integrated; the missing implementation is now the blocker. Child A #318 supplies only Foundation semantic authorization/publication/completion APIs. Child B must persist them atomically; Child C must bind actual authenticated/owning producers and prove admission readiness. Existing test-only verifier authorities do not establish production sources. No A scope expansion or fabricated source can substitute for C.

## Next authorized sequence

1. Work integrates the exact Child A allocation and records that future protected merge SHA as immutable worker admission provenance.
2. Work creates `agent/foundation-fresh-admission-318` from that SHA and dispatches its sole worker after fresh ownership/readback checks.
3. Qualify/integrate/read back A; only then freshly allocate B. Qualify/integrate/read back B; only then freshly allocate C.
4. Require C's actual producer inventory, commit-before-publish, source nonrollback and readiness evidence; missing sources keep admission closed.
5. After A+B+C protected integrations and readiness, Work re-evaluates the preserved Server Seam branch and decides lawful resume/reconciliation. No reset/rebase/force/restart or partial-candidate integration is authorized.
6. Qualify Server Seam through its existing full production-path/review/CI lifecycle, then recompute compatible Client -> applicable real QA -> Movement -> Combat.

The architecture author's task is left unchanged; #313 and Work evidence record protected integration. No architecture-worker archive/release role is assumed by this bounded correction.

## Open-state disposition

A repository search may still find historical, template, draft, Dependabot or separately owned PR/task prose containing `READY_FOR_INTEGRATION` or old Durability/Server Seam states. Those strings are not current authority. A Work integration action requires a fresh lane return tied to the exact current allocation, exact head, changed-path allowlist, required review and CI generation.

The current #162 critical path has no integration-ready Server Seam return: #247 is `WAITING_DEPENDENCY` on A+B+C implementation/readiness; Client/QA/Movement/Combat remain blocked.

Issue #316's narrow coordinator cleanup qualifies the DAG historical release as `blakinio/Oteryn-v2#314` / `88f4fb754b5ae11243afd38a9e0b6a8e3b0a5815`, retaining source-retirement provenance. Only the release header/paragraph changes; DAG dependencies, lane semantics and existing implementation authority remain unchanged. The active Work control plane owns this bounded shared-programme correction; #316 itself grants no independent worker lease.

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
- [x] #313 architecture decision is reviewed, qualified and integrated to protected `main` through PR #317.
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

The current critical path is A #318 -> separately allocated B -> separately allocated C/real producer readiness -> Server Seam. #313 is completed. #316 release-provenance correction remains documentation-only. No other lane is promoted or implicitly allocated.

## Validation

### Focused

- validation target: coordinator task/control-plane consistency against protected `main`, Issues #162/#165/#166/#174/#247/#250/#313/#316, merged allocation #294 and protected correction #314
- result: live GitHub readback performed before mutation

### Component/integration

- scenario: documentation/control-plane reconciliation only
- result: runtime/component/E2E execution is `NOT_APPLICABLE`; this package makes an exact prospective Child A allocation; it performs no runtime, contract, schema or Cargo mutation

### E2E

- scenario: `NOT_APPLICABLE` — no executable gameplay behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- exact head: recorded in PR/CI evidence rather than self-referential task content
- required result: repository PR governance/checks green, then normal protected Merge Queue qualification and `game-gate`

## Self-review

- exact head: recorded in immutable PR evidence after the final material content change
- method: whole-diff review against the six-path coordinator allocation package
- verdict: pending exact package review; prospective Child A lease follows accepted architecture and activates only on protected integration; no runtime/Cargo/schema/workflow mutation

## Independent review

- required: `YES` for this exact allocation package under current protected root `AGENTS.md` AI review policy
- reason: the package prospectively allocates a high-risk admission/security authority boundary; independent non-author exact-head review must verify scope, accepted contracts, lease activation and preserved Server Seam ownership before integration
- enforcement remains: exact-head repository checks, `game-gate`, protection and Merge Queue

## PR and closeout

- changed-file allowlist: Child A task and plan, LIVE_ALLOCATIONS, executor DAG provenance, this Work task, and explicitly coordinated Server Seam task metadata only
- terminal merge authority: normal protected repository control plane only; no direct/self/bypass merge
- coordinator programme remains active after this checkpoint; this is not Movement/Combat programme closeout

## Context checkpoint

```yaml
last_progress: architecture #317 protected integration verified; prospective A #318 allocation and metadata-only #247 dependency correction prepared; #316 historical release provenance reconciled
status: COORDINATING
programme_state: WAITING_DEPENDENCY
active_control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
protected_main_sha: a8678d4a94e479a9aa2a92920379a4b32f95143b
server_seam_issue: 247
server_seam_branch: agent/otv2-gameplay-server-seam-01
server_seam_head_sha: 9370b254c6ac4f6529e069c1968ae6bfa1e1750e
server_seam_pr: null
server_seam_state: WAITING_DEPENDENCY
architecture_escalation_issue: 313
nonblocking_governance_reconciliation_issue: 316
owner_action_required: null
blocker: Foundation_318_then_Child_B_then_Child_C_actual_producer_readiness
next_action: Work integrates the exact Child A allocation and records its protected merge SHA before worker branch creation and dispatch
```
