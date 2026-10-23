# ITEM-PIN-1

```yaml
task_id: ITEM-PIN-1
title: Repin reference_playable.rs in the item-authoring promotion chain
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy
pr: 2299
base_sha: 97060ec39071db647641843f561be32452b52ffb
owner: Claude integrator
created_at: 2026-10-22
updated_at: 2026-10-22
execution_policy: continuous_progress
owned_paths:
  - docs/agents/evidence/OTV2-20261002-item-mantra-bond-source-qualification-v5.json
  - docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-current-receipt-v5.json
  - docs/agents/evidence/OTV2-20261003-item-stack-default-successor8-current-parent-receipt-v6.json
  - docs/agents/evidence/OTV2-20261002-item-elemental-magic-modifier-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-mantra-bond-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-promotion-v1.json
  - tools/content-schema/item-authoring/lower_mantra_bond_modifier_packet.py
  - tools/content-schema/item-authoring/lower_numeric_modifier17_packet.py
  - tools/content-schema/item-authoring/lower_wiki_stack_default_successor8_packet.py
  - crates/gameplay/content/src/content/item_elemental_magic_modifier_promotion.rs
  - crates/gameplay/content/src/content/item_mantra_bond_modifier_promotion.rs
  - crates/gameplay/content/src/content/item_numeric_modifier_promotion.rs
  - crates/gameplay/content/src/content/item_stack_default_successor8_promotion.rs
  - docs/agents/tasks/archive/ITEM-PIN-1.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
registry: no
plan_row: integrator-found item-authoring CI repair
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **DONE**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2299](https://github.com/Oteryn/Oteryn-Game/pull/2299). Item-authoring promotion regenerates and verifies reference_playable.rs pins. Evidence: `crates/gameplay/content/src/content/reference_playable.rs`.

Tracking closeout: delivered on main; no active implementation ownership is retained by this historical card.

## Problem and root cause

`item-authoring` (workflow `item-authoring-schema.yml`) has been red on main
since #2280. That PR moved the `WorldId` import in
`crates/gameplay/content/src/content/reference_playable.rs` under ADR-0022.
Several promotion lowerings pin that file's SHA-256 through their source
receipts, so `lower_wiki_stack_default_packet.checked` now raises
`source digest drift: crates/gameplay/content/src/content/reference_playable.rs`.
The import move changes no Item value.

## Fix

Supersede the receipts that carry the stale pin. Do not edit them in place:

- mantra-bond source qualification v4 → v5
- numeric-modifier17 current receipt v4 → v5
- stack-default successor8 parent receipt v5 → v6

Each successor records the new `reference_playable.rs` digest and a `supersedes`
link to its predecessor. Where a receipt pins a lowering tool this repair edits,
it also records that tool's new digest. The three
lowering tools now point at their successor and its SHA. The four promotion
packets are regenerated with their tools, and the four `*_promotion.rs`
constants take the regenerated packet SHAs. `engine-items-repin-receipt-v1`
keeps its original pin as a historical record.
No Item value, schema, runtime path or public API changes.

## Validation

- Every `item-authoring` job step of `item-authoring-schema.yml`, run locally
  in workflow order on this head. The checkout stays clean.
- `cargo +1.95.0 test --locked -p oteryn-gameplay-content --lib promotion`.
- `python3 tools/agents/validate_governance.py`,
  `python3 -m unittest discover -s tools/agents/tests`,
  `python3 tools/repository/validate_repository_policy.py`, `git diff --check`.

## Handoff

The integrator publishes this on `claude/happy-davinci-ijz3wy` for exact-head CI.
