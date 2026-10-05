# OTV2-20261004-monster-main-reconciliation

```yaml
task_id: OTV2-20261004-monster-main-reconciliation
title: Complete seven monster data and source-mechanic batches
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: feat/monster-source-mechanics-seven-20261004
base_sha: 0ec917e6b3031974233e9f0119715aaef1a66a05
pr: 1807
owner: root
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/
  - content/
  - imports/canary/
  - imports/crystalserver/
  - imports/spells/
  - imports/tibiawiki/bindings/items.json
  - rulesets/items/
  - tools/agents/qualify_monster_wiki_fields.py
  - tools/agents/tests/test_qualify_monster_wiki_fields.py
  - tools/content-schema/
  - tools/content-migration/
  - tools/monster-lab/
  - docs/agents/evidence/monster-final-seven-20261004/
  - docs/agents/tasks/archive/OTV2-20261004-monster-main-reconciliation.md
public_contracts: []
external_repositories: []
```

## Outcome

Seven implementation batches are complete within the existing runtime library:
16 data cases; 120 temporary appearance bindings; 82 source Item operations;
Icicle raw-damage callback; 28 implemented callbacks and five typed pending map
bindings; Herald quest/death processing and explicit pending permanent wardrobe;
24 healing-from-damage maps through existing HP, condition and death owners.
Source-only and accepted PROJECT approximations remain distinguished from Wiki.

PROVEN: 1863 canonical Creatures, 1870 native profiles including variants,
34043 Items and 62801 reference records, plus 1282 NPCs and 836 Dialogues. All ten local Item additions, the main
Item40450 addition and 700 protected Item authoring records are retained.

## Authority and recovery qualification

Applicable: Creature source fields, current native pins and immutable cause
receipts use the existing SQL scope owner and reject wrong world/channel/scope,
actor generations, source bodies, occurrences and replay. Existing Player cause
and cost checks remain intact. Character quest writes retain current revision,
session generation and lease fencing. Negative cases separately cover missing
facts, stale generation, mismatched provenance, repeated occurrence, atomic
rollback and current contributor selection. PostgreSQL component proof covers
real migrations through 0078 on the repair generation; it does not establish all 82 native-to-SQL paths.
No shipping activation or production mutation was performed.

## Validation

Library: 2370 passed / 0 failed / 24 ignored. Actual appearance: 6 passed.
Actual native-manifest qualification: 1 passed. Herald PostgreSQL: historical 1 passed at `e82070f81028c79e9bc8d3a8e57ac29e78379122`; not rerun after this merge.
Native capture regressions: 12 passed across callback, summon and death owners.
Repository package integration: historical 3 passed; not rerun on this head. Full test-target compilation passed.
Strict all-target Clippy and final Rust formatting: pass.
`python tools/agents/validate_governance.py`: pass.
`python -m unittest discover -s tools/agents/tests`: pass (59 tests).
Repository policy validation: pass. Portable closeout evidence retains current results. Ignored tests are not
counted as executed. The final Git head is recorded in the PR rather than here.

Evidence: [final evidence](../../evidence/monster-final-seven-20261004/final-status/README.md).

## Coordination and publication

Normal PR #1807: https://github.com/Oteryn/Oteryn-Game/pull/1807.
This archive move is its final metadata authoring step. Runtime implementation and content remain qualified at code head
7a3711a95ce2f2be7fd38567998d86672a0b01f0; the successor updates two
test fixtures and derived migration/validation metadata. Repaired fixtures
are verified separately before final publication.
The PR is not merged or enqueued by this task. Pending integration bindings and
accepted local policies are explicitly listed in
[decisions for #162](../../evidence/monster-final-seven-20261004/final-status/decisions-162.md).
Source-map relocation, Lord Retro permanent Character wardrobe, full library
activation at the existing Channel cadence, Herald portal binding and old-state
migration remain separate work. No claim of Global parity or live execution.

Post-merge execution binds main `d98968d1134eef7bac863d8b8a00611fc3e8f15d` and code `7a3711a95ce2f2be7fd38567998d86672a0b01f0`: 2370 library, 19 native/appearance/capture tests, strict all-target Clippy PASS. PostgreSQL history remains explicitly qualified at its original boundary.


Final publication qualification (2026-10-05): runtime execution is bound to code head58a8d64f03564407f7501e16037892127c70f677, binary06a1a32ca2199d5ce4c397eb8bf5f85682271e97f8e111ea0d534644aa8287d6:2370 library PASS/0 failed/24 ignored and19 individually executed native captures PASS. Repository integration3 PASS and reward binding11 PASS were freshly rerun at58a. Strict all-target Clippy passed atcaef6ec33b7ed2f502e071bc4e1232511ba79a23. Every Rust source file is identical between those heads; canonical reference/declarations/sources and Quest quests[] payload remain preserved. Successor Python reproduction, typed Unknown normalization and Quest source metadata are separately qualified, not described as a fresh runtime-binary execution. Content-tree gates and Quest510/0/1skip plus four RewardClaim gates PASS. Accepted stats and seven protected source-body variants remain preserved. Historical7a/e820 receipts retain their original scope. No production activation or Global1:1 parity claimed.

## Repair generation (2026-10-05)

Returned to AUTHORING from published b984e329 after verified P1 findings. Both are accepted: detached same-world source projects previously acquired callback/Bone shared-HP authority by copying runtime pins. Repair consumes the actual loader-decoded NativeGameplayState, checks its independent outer digest against current runtime and requires exact loaded Creature profile membership. The raw Bone helper becomes private. RED reproduction is retained; current native positive and negative tests qualify the repair. Reconciliation target is main7358a14d, including NPC rebuild1777, held-target spell1808 and map-wire1810. No merge or production authority is requested.

## Qualified repair and publication update

The P1 repair and NPC catalogue union are complete locally, with current main98ca reconciled. Incoming migration0077_map_item_materialization is retained and our unchanged source-ground-cause migration uses0078. Current qualified results and exact execution scopes are in [repair evidence](../../evidence/monster-final-seven-20261004/repair-20261005/README.md). Earlier validation sections above retain their historical source scopes. Library2395/0/25 and19native captures executed at fdf0559a. Strict all-target Clippy, repository/reward binding and actual map-item PostgreSQL integration pass at a2c9e914. Source-ground/Player SQL guard controls pass through0078. The normal existing PR1807 is updated using guarded fast-forward publication; independent review and exact-head GitHub checks govern later integration. No merge, queue or server deployment is authorized here.

## Latest main and final mechanic execution

Merged current main0ec917e6 with CREATURE-AI-1. Its target selection/think owner is preserved alongside the existing mutable damage/field-contact owner. The accepted one-second cadence and new test helpers are reconciled. Fresh current-source qualification at2081af9b:2413library PASS/0failed/25ignored,19individually executed native/appearance/capture PASS and strict all-target Clippy PASS. Actual a2c SQL/repository/reward receipts remain separately scoped with unchanged source/data/fixture proof. Crystal map candidates reproduce against the reconciled Creature catalogue:8former held groups/29points resolved, all prior candidate records and all remaining raw held data preserved, runtime admission disabled. [Latest portable evidence](../../evidence/monster-final-seven-20261004/latest-main-20261005/README.md). Normal PR1807 is updated; no protected merge/queue or production activation.
