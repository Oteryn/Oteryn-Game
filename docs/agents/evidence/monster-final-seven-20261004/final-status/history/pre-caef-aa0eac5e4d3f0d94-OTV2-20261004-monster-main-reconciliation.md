# OTV2-20261004-monster-main-reconciliation

```yaml
task_id: OTV2-20261004-monster-main-reconciliation
title: Complete seven monster data and source-mechanic batches
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: feat/monster-source-mechanics-seven-20261004
base_sha: d98968d1134eef7bac863d8b8a00611fc3e8f15d
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
34043 Items and 62457 reference records. All ten local Item additions, the main
Item40450 addition and 700 protected Item authoring records are retained.

## Authority and recovery qualification

Applicable: Creature source fields, current native pins and immutable cause
receipts use the existing SQL scope owner and reject wrong world/channel/scope,
actor generations, source bodies, occurrences and replay. Existing Player cause
and cost checks remain intact. Character quest writes retain current revision,
session generation and lease fencing. Negative cases separately cover missing
facts, stale generation, mismatched provenance, repeated occurrence, atomic
rollback and current contributor selection. PostgreSQL component proof covers
real migrations through 0077; it does not establish all 82 native-to-SQL paths.
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
