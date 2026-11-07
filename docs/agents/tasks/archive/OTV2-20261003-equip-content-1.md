# OTV2-20261003-equip-content-1

```yaml
task_id: OTV2-20261003-equip-content-1
title: "EQUIP-CONTENT-1: typed Equipment abilities and the Canary fallback"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/equip-content-1-20261003
issue: 162
coordination: 1622
pr: 1719
head_sha: "exact frozen head in the control plane FREEZE_SHA entry"
merge: "squash merge of the PR"
owner: oteryn-hard-worker (CP session_013KJX6mv8LQveCKKXYgAX94)
created_at: 2026-10-04
updated_at: 2026-10-04
packet: docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ITEM_EQUIP_PACKETS_2026-10-03.md (§2.9; decision EQUIP-0 §3.1-§3.2)
migration_lease: none
owned_paths:
  - tools/content-schema/item-authoring/{lower_equip_abilities_packet.py,test_lower_equip_abilities_packet.py}   # added by CP ruling (a), 04:28Z
  - tools/content-schema/item-authoring/README.md
  - tools/content-schema/item-authoring/{lower_wiki_stats_packet.py,test_lower_wiki_stats_packet.py}             # byte-unchanged (CP ruling a)
  - apps/game-server/src/content/{item_abilities.rs,mod.rs}
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/world/** and the content tree (regenerated)
  - docs/agents/evidence/OTV2-20261003-equip-abilities-v1.json
  - docs/agents/evidence/OTV2-20261003-equip-abilities-sources-v1.json   # tool-only provenance; CP D471 (Codex P1 repair)
  - docs/agents/tasks/archive/OTV2-20261003-equip-content-1.md
public_contracts: []   # no codec, schema or wire change
jira: null   # sync pending (coordinator batch)
```

## Control-plane rulings

- **04:21Z, ruling (a).** The six abilities are a derived typed view in `item_abilities.rs`. The
  Canary fallback lives in the lowerer, with an evidence file, and there is no codec or schema
  change. STAT_BOOST and LIGHT get the type but no rows: "no source in TibiaWiki snapshot nor
  Canary items.xml (D384 pin)". Whether they need a source is a follow-up question for EQUIP-RT-1.
- **04:28Z, ruling (a).** The lowering is a new tool, `lower_equip_abilities_packet.py`, with its
  own test, added to the owned paths. `lower_wiki_stats_packet.py` stays byte-unchanged. The
  reason: the stats packet embeds its compiler digest, and the elemental, Mantra-Bond and
  numeric-17 chains and the successor-8 receipts pin that packet.

- **Codex P1 4176303392, round 1 (accepted).** `apps/game-server/AGENTS.md` forbids Canary
  compatibility code in the server. Repair: the tool lowers everything, and the server embeds a
  source-free facts packet (Item key, field path, canonical value). Provenance, holds and the
  timed/listing invariants move to a tool-only sources record and the tool's `--check`.
  The control plane approved the sources record as an owned path (D471).

## Outcome

- **Derived view (`item_abilities.rs`).** `item_ability_profile` maps an Item definition to
  `ItemAbilityProfile { timed, extra_slot, abilities }`.
  - SKILL_BOOST covers the six weapon skills and magic level.
  - SPEED is signed, in displayed units.
  - PROTECTION is an element with a signed percent in [-100, 100]. `FIRE_FIELD` becomes Fire with
    `field_only`.
  - SUPPRESS covers drown and drunk.
  - STAT_BOOST and LIGHT are typed, with no rows.
  - `timed` holds when `charges.count` or `temporal.duration` is Known. `extra_slot` holds when a
    pattern has `primary_slot == Extra`. Neither has a flag of its own.
  - An ability-kind entry without a Known, well-typed value fails closed.
- **Application.** `apply_equip_abilities_v1` runs last in the materializer.
  - It pins the facts packet digest and its counts: 27 Items, 32 fields.
  - Each fact fills only an Unknown leaf.
  - Every Item's derived view must decode.
  - The server names no source.
- **Facts packet** (`OTV2-20261003-equip-abilities-v1.json`, `OTERYN_EQUIP_ABILITY_FACTS/v1`).
  It holds Item key, field path and canonical value only: 21 modifier lists and 11 resistance
  lists over 27 Items.
- **Sources record** (`OTV2-20261003-equip-abilities-sources-v1.json`). Only the tool reads it.
  - It lists 761 Items with their sources: TibiaWiki page and revision plus values, and the
    Canary top-level attributes (D384 pin, OTS_HYPOTHESIS_ONLY). 153 of them are timed.
  - Fallback applies only where every wiki page is silent on the group.
  - 12 are held, because their Canary group carries a key outside the mapped abilities. These are
    mantra, elemental bond, gain/ticks, mana shield and invisibility.
  - The tool fails when a materialized Item with an ability has no source.
- **Speed-unit deviation from the packet text.** The packet expected Canary speed in a doubled
  unit. In the pinned items.xml, speed agrees 1:1 with the wiki on all 31 Items that state both
  (boots of haste: 20 and 20). The factor is therefore 1, and any disagreement fails the tool.

## Validation

- `python3 -m unittest tools/content-schema/item-authoring/test_lower_equip_abilities_packet.py` pass
- `python3 tools/content-schema/item-authoring/lower_equip_abilities_packet.py --check` pass
- `python3 tools/content-schema/item-authoring/test_lower_wiki_stats_packet.py` pass (the tool is unchanged)
- `cargo test --locked -p oteryn-game-server --test content_world_project_repository --quiet` pass
- `cargo test --locked -p oteryn-game-server --lib item_abilities` pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings` pass
- `cargo fmt --all --check` pass
- `python3 tools/content-migration/regenerate_content.py` pass (idempotent, clean tree)
- `ruff check` / `ruff format --check` on the two new tool files pass
- `python tools/agents/validate_governance.py` pass
- `python -m unittest discover -s tools/agents/tests` OK

Review: independent review on the final frozen head (CP routes).
