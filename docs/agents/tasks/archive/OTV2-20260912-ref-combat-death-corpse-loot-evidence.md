> Lifecycle closeout: **ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #576 merged as `674c998ca949422ac018f17f177b590f965a28f7`. Any active/checkpoint language below is historical provenance only; live GitHub and protected current state supersede it.

# OTV2-20260912-ref-combat-death-corpse-loot-evidence

```yaml
task_id: OTV2-20260912-ref-combat-death-corpse-loot-evidence
title: Close Reference ordinary death/corpse/loot evidence for #506/#513
mode: AUDIT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/ref-combat-death-corpse-loot-evidence-506-513
pr: 576
original_base_sha: 1a9cb71f424a821633fd42f8a1a19920ffeff2c3
reconciled_protected_main_sha: 1995bd97460774ea9fc136959d5548471b81c987
merge_up_commit: 3f1052fcf92d979325a28621df7f183818035bb1
content_reconciliation_commit: 449de73d1a61d8a4a2245589b64dc3eaa42134f0
final_head_sha: LIVE_PR_READBACK_REQUIRED
final_head_frozen_at: LIVE_PR_READBACK_REQUIRED
owner: ChatGPT GPT-5.6 Sol
created_at: 2026-09-12T09:13:00+02:00
updated_at: 2026-09-16T20:41:00+02:00
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260912-ref-combat-death-corpse-loot-evidence.md
  - docs/agents/evidence/OTV2-20260912-reference-combat-death-corpse-loot-chain.md
public_contracts: []
depends_on:
  - "#483"
  - "#506"
  - "#513"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Retain one official-first evidence pack for the ordinary single-player creature path required by #506/#513:

`committed lethal result -> death -> XP consequence + corpse/loot selection -> protected corpse interaction -> durable pickup boundary`.

The evidence pack preserves the immutable Reference target `global-tibia-observable-2026-07-28-post-server-save`, keeps party XP/PvP/boss/quick-loot breadth out of scope, and does not invent Global durability ownership or transaction internals.

The same canonical lineage and PR are preserved. No replacement branch, replacement PR or duplicate worker was created.

## 2026-09-16 protected-main reconciliation

Fresh protected state used for this continuation:

```text
protected main = 1995bd97460774ea9fc136959d5548471b81c987
prior PR head = f7b990b8169b38f74fe50fb6a47641911043526c
prior compare = diverged / ahead 3 / behind 34
```

The same branch was reconciled with protected main through an ordinary two-parent, non-force merge-up:

```text
merge commit = 3f1052fcf92d979325a28621df7f183818035bb1
parent 1 = f7b990b8169b38f74fe50fb6a47641911043526c
parent 2 = 1995bd97460774ea9fc136959d5548471b81c987
force update = false
```

Before merge-up, fresh compare proved the branch's effective changes were exactly the two already-owned Markdown paths and protected main did not contain either path. No runtime/manifest/schema/registry/Cargo/WP3/WP4/WP5/Server-Seam path was introduced by the merge reconciliation.

## Architecture and source-of-truth reconciliation

- `PROVEN`: live #506 binds the first Combat death workflow and owner separation.
- `PROVEN`: live #513 binds the DUR-03 materialization/pickup boundary and durable item/value ownership.
- `PROVEN`: #483 binds the official-first source hierarchy and immutable 2026-07-28 target.
- `DERIVED HIGH`: protected `docs/architecture/reviews/OTERYN_REFERENCE_FIRST_CREATURE_FIXTURE_RAT_2026-09-12.md` selects Rat as the first creature fixture, including HP `20`, base XP `5`, `Dead Rat` corpse family and ordinary candidate item types `Gold Coin`/`Cheese`; exact natural loot probabilities remain `UNKNOWN`.
- `PROVEN Oteryn-native DECLARED_DIFFERENCE`: protected `docs/architecture/OTERYN_REFERENCE_DEATH_XP_SPAN_OWNER_BASELINE_2026-09-09.md` fixes Character/player-death XP-loss basis to `LevelXPSpan(current_level)` and fixes `DeathSkillLoss = 0`, `DeathMagicLevelLoss = 0`. These are not Global truth and do not alter creature-kill reward XP.
- `PROVEN target-boundary`: Iceplume Strider `8,150` remains a direct 2026-07-28 XP anchor, but it is no longer described as the selected first creature fixture.
- `DERIVED` strong: ordinary Global corpse authority uses the highest-damage principal during the protected 10-second interval, with corpse immovability, from a long official continuity chain.
- `DERIVED` strong fixture precondition: the refreshed first fixture uses `20h` remaining stamina as an interior neutral point instead of merely `>14h`, avoiding both the final-14h half-XP/no-loot state and the Premium first-three-hours XP bonus band; explicit boost/event/prey/boosted-creature modifiers are disabled as fixture preconditions.
- `UNKNOWN`: exact server-tick/rounding semantics at the 10-second expiry boundary.
- `UNKNOWN`: Rat exact loot probabilities/RNG, exact flee threshold and numeric corpse item ID remain unpromoted.
- `UNKNOWN`: proprietary/internal Global durable item identity, commit, custody and retry implementation.
- `CONFLICT`: last hit as universal loot owner, one generic killer owner for XP+loot, all loot necessarily rolled at death, `stamina > 14h` as sufficient proof of neutral XP, Iceplume as the current first-creature fixture, or Oteryn zero skill/magic player-death loss as Global truth.

## #514 manifest-v4 boundary

Fresh #514 remains:

```text
manifest_mutation_authority = NONE_UNTIL_EXACT_ALLOCATION
allocation_authority = 162_ONLY
```

This worker does not mutate or activate manifest-v4, its schema, registry, parity fixtures or runtime. Rat and the accepted Reference death declared difference are evidence inputs only. #514 must be re-evaluated after #576 is protected-integrated, and mutation may begin only if #162 grants the exact allocation.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: docs-only read-only evidence retention; no production mutation, PREPARE/COMMIT authority, controller installation, durable write, recovery interpretation or live-state mutation
```

## Acceptance criteria

- [x] Same canonical branch/PR preserved; no replacement lineage created.
- [x] Current protected main merged up non-force into the existing branch.
- [x] Evidence is restricted to ordinary single-player creature death/corpse/loot semantics required by #506/#513.
- [x] Protected Rat first-creature fixture is reconciled without promoting `DERIVED HIGH` fields to `PROVEN`.
- [x] Accepted Oteryn Reference player-death declared difference is fenced from Global truth and from creature-kill reward semantics.
- [x] Neutral XP/stamina fixture preconditions use an interior `20h` stamina point and disable separate reward modifiers instead of assuming `>14h` is neutral.
- [x] Atomic claims preserve `PROVEN`, `DERIVED`, `UNKNOWN` and `CONFLICT` boundaries.
- [x] Exact target continuity is distinguished from current/historical official evidence.
- [x] Kill attribution, XP consequence, corpse creation/access, protected window/immovability, ordinary corpse interaction and loot-selection/durable-pickup boundary are covered.
- [x] Questions owned by #506 and #513 are separated explicitly.
- [x] Party/shared XP, PvP, bosses/events and quick-loot breadth are excluded.
- [x] No Global durability ownership or proprietary transaction internals are invented.
- [x] No runtime/manifest/schema/registry/Cargo/WP3/WP4/WP5/Server-Seam mutation.
- [ ] Fresh exact-head Agent Governance, Architecture Semantic Audit and Merge Gate terminal readback for the post-reconciliation PR head.
- [ ] Governed integration through the bound native exact-head Merge Queue route, followed by real `merge_group` aggregate `game-gate` and protected-main readback.

## Excluded scope

No runtime/client/protocol/Cargo/workflow/registry/contract/manifest/persistence/schema/migration/production mutation. No WP3/WP4/WP5 or Server Seam mutation. No party/shared XP breadth, PvP/player-death implementation breadth, bosses/events, broad quick-loot aggregation, exact corpse decay timing, or Evolved mechanics. No OTS implementation is used as Reference truth.

## Implementation / findings

The reconciled research is retained in `docs/agents/evidence/OTV2-20260912-reference-combat-death-corpse-loot-chain.md`.

The core owner separation remains:

```text
Global observable semantics
  -> kill/death consequence
  -> XP contribution consequence
  -> corpse + protected access rule
  -> ordinary corpse/item selection

Oteryn authority boundary
  Combat loot-selection intent
  -> DUR-03 materialization
  -> LOOT_READY
  -> pickup TRANSFER of the same durable ItemInstance
```

No durability ownership is inferred from Global public behavior.

## Validation

### Focused content validation

Fresh reconciliation checks performed before the final tracked task update:

```text
- protected main fresh-read: PASS
- #162/#483/#506/#513/#514 fresh-read: PASS
- PR #576 exact branch/head readback: PASS
- changed-file inventory: exactly two owned Markdown paths
- protected Rat fixture readback: PASS
- accepted Reference death declared-difference readback: PASS
- official stamina rule readback: confirms 42h total, Premium +50% first three hours down to hour 39, half XP final 14h and no loot at/below 14h for highest-damage character
```

No local command runner was used because the local execution container could not resolve GitHub network access. Repository-native GitHub operations remained available and were used for live state and mutation.

### Component/integration

- command/run: `NOT_APPLICABLE` — documentation/evidence only
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no runtime behavior changed
- result: `NOT_APPLICABLE`

### Exact-head CI

The pre-reconciliation PR head `f7b990b8169b38f74fe50fb6a47641911043526c` had terminal SUCCESS for the applicable exact-head Architecture Semantic Audit, Agent Governance and Merge Gate. That evidence became stale when the branch head changed and is **not** reused as qualification for the refreshed candidate.

This tracked task file deliberately does not write its own final commit SHA into itself. After this update, the authoritative exact head and its checks must be read live from PR #576. The required post-reconciliation state is:

```text
Architecture Semantic Audit = terminal SUCCESS on exact current head
Agent Governance            = terminal SUCCESS on exact current head
Merge Gate                  = terminal SUCCESS on exact current head
unresolved review threads   = 0
```

Until live GitHub proves those facts for the exact current head, qualification remains `PENDING_EXACT_HEAD_READBACK`.

## Self-review

- method/reviewer: implementing/coordinating agent; fresh protected authority readback plus base/head effective-diff review.
- material findings repaired: stale Iceplume-as-first-fixture wording; insufficient `>14h` neutral stamina condition; missing accepted Reference player-death declared-difference fence; stale CI/task metadata.
- preserved boundaries: all prior `UNKNOWN`/`CONFLICT` semantics retained or made more explicit; Rat remains `DERIVED HIGH`, not `PROVEN`.
- verdict: PASS for bounded content reconciliation; exact-head repository CI still governs qualification.

## Independent review

- required by this task: NO — docs-only retained evidence, no contract/runtime/authority mutation.
- repository review/check policy remains live authority if the classifier requires anything further.

## PR and closeout

- canonical PR: #576
- canonical branch: `agent/ref-combat-death-corpse-loot-evidence-506-513`
- changed-file scope: exactly the two owned Markdown paths
- replacement PR/branch/worker: none
- direct merge: forbidden
- generic auto-merge: forbidden
- force update: forbidden and not used
- integration route: bound META 3.1 native exact-head `merge-async` with `merge_action="merge_queue"` only
- integration terminal proof: real `merge_group` aggregate `game-gate` SUCCESS + protected-main readback
- merge result: pending governed capability and qualification
- ownership release: pending terminal disposition

## Context checkpoint

```yaml
last_progress: same #576 lineage merged up non-force to protected main and both owned Markdown paths reconciled to Rat/death-difference/neutral-stamina evidence
status: validating
branch: agent/ref-combat-death-corpse-loot-evidence-506-513
pr: 576
protected_main_reconciled: 1995bd97460774ea9fc136959d5548471b81c987
merge_up_commit: 3f1052fcf92d979325a28621df7f183818035bb1
content_reconciliation_commit: 449de73d1a61d8a4a2245589b64dc3eaa42134f0
final_head_sha: LIVE_PR_READBACK_REQUIRED
ci_trigger_source: pull_request
ci_check_generation: POST_RECONCILIATION
ci_checks_for_current_head: LIVE_READBACK_REQUIRED
ci_run_ids: LIVE_READBACK_REQUIRED
runner_assignment_state: LIVE_READBACK_REQUIRED
owner_action_required: null
blocker: null
next_action: read exact current #576 head, run/read applicable governance/semantic/Merge Gate checks, resolve only material findings, then use only governed Merge Queue route if capability and eligibility are proven
```
