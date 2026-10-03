# ITEM-SEM-2b-2 Slot, hands and requirements: lowering rulings and worker packet

```yaml
decision_id: ITEM-SEM-2B2-EQUIPMENT-REQUIREMENTS-V1
status: CANDIDATE
date: 2026-10-03
owner: Sol Supervising Architect
answers: ITEM-SEM-2b-1 record (archive), "2b-2: requirements (level, vocation, hands, slot)"
writes_on_other_prs: none
```

This document fixes how the pinned TibiaWiki snapshot `imports/tibiawiki/facts/items-stats.json`
lowers `slot`, `hands`, `levelrequired`, `vocrequired` and `mlrequired` into Item content. It closes
the content input that ITEM-MOVE-WIRE-1 (ITEM-MOVE-2a: "slot, hands, requirements in content") and
EQUIP-0 (EQUIP-CONTENT-1) depend on. It changes no code, no schema, no contract and no wire. Live PR
and Issue state governs; counts below are from the snapshot on `main` at 98a2f95c5.

## 1. Evidence

- Snapshot fields: `slot` 1,836, `levelrequired` 1,325, `vocrequired` 878, `hands` 760,
  `mlrequired` 53 observations; no Item with a requirement has more than one observation; every
  `levelrequired` value is a decimal integer.
- `item.schema.json` already has both targets:
  - `equipment` in the compact form (`slot`, `hands`, optional `reserved_slots`, `groups`,
    `vocations`, `min_level`), exclusive with `patterns[]`;
  - `requirements` (`min_level`, `min_magic_level`, `vocations`, required `enforcement_mode`).
  The schema mapping (`build_formal_schema.py`) maps `vocrequired` and `levelrequired` to both.
- `apps/game-server/src/domain/equipment.rs` (owner answer 4 in #162, manual §3.4.1): weapons occupy
  the right hand; shields, spellbooks and quivers the left hand; a two-handed weapon reserves both
  hands; a two-handed distance weapon and a shield or spellbook reserve the non-quiver left-hand
  group; the Extra item takes the `ammo` slot.
- ITEM-MOVE-WIRE-1 §4: the `ammo` slot is the manual's Extra slot, admits any whole item and runs
  no level, vocation or Premium check. RANGED-0 §3.3 and GAME-ITEM-01 §6.1 agree.
- A13 §4.3: a character before Dawnport has `Vocation::None`, stored as the key `none`.

## 2. Rulings

1. **Slot.** `Head`→`head`; `Neck`→`neck`; `Body`, `Torso`→`armor`; `Legs`→`legs`; `Feet`→`feet`;
   `Finger`→`ring`; `Container`→`back`; `Weapon Hand`, `Both Hands`→`right_hand`;
   `Shield Hand`, `Shield`→`left_hand`; `Extra Slot`→`ammo`. The schema value `extra` is not used.
   Snapshot `primarytype` `Ammunition` without a `slot` takes `ammo`. Anything else without a
   `slot` gets no `equipment` block.
2. **Form.** Always the compact form. `patterns[]` is not written by this slice: every snapshot
   Item has exactly one slot.
3. **Hands and occupancy.** `hands` is `2` for `Both Hands`, `1` for `Weapon Hand`, `Shield Hand`
   and `Shield`, else `0`. A snapshot `hands` value that disagrees (`One` on `Both Hands`, `Two` on
   `Weapon Hand`) is reported, and the Item gets no `equipment` block. Reservations follow
   `equipment.rs`:
   - two-handed, not `Distance Weapons`: `reserved_slots: [left_hand]`;
   - two-handed `Distance Weapons`: `groups: [non_quiver_left_hand]`;
   - `Shields`, `Spellbooks` (left hand): `groups: [non_quiver_left_hand]`;
   - `Quivers`: no group.
4. **Where requirements go.**
   - An Item in a slot other than `ammo`: level and vocation go to the compact `equipment.min_level`
     and `equipment.vocations` (checked at equip, ITEM-MOVE-WIRE-1 §4).
   - An Item in `ammo` or with no slot (runes, ammunition and the rest): level, vocation and magic
     level go to `requirements` with `enforcement_mode: on_use`, because the Extra slot runs no
     check and these requirements bind use (RANGED-0, RUNE-USE-0).
   - `mlrequired` (53, all runes) goes to `requirements.min_magic_level`. No schema amendment is
     needed. RUNE-USE-0 owns its enforcement.
   - `levelrequired` `0` writes nothing.
5. **Vocations.** Lowercase the text, split on `,` and `and`, and drop a trailing `s`. Map
   `knight`, `paladin`, `sorcerer`, `druid` and `monk` to their vocation keys. `without` maps to
   `none`, the A13 key for a character without a vocation (Star Ring, Mean Knight Sword, Mean
   Paladin Spear, the Sorcerer and Druid Staff). `paladins and without` maps to `[paladin, none]`.
   `None` means no restriction, so `vocations` is omitted (Bright Sword, Justice Seeker, Elvish
   Bow). A list of all five vocations is kept as written, not collapsed. Any other token is
   reported and the field is not written.
6. **Promoted vocations.** A base key also admits its promotion (Elite Knight and the others). This
   is the equip rule's job (ITEM-MOVE-2a), not content's. Content writes base keys only.
7. **Guard.** D303 (the patterns-only guard carried by #1599) must admit the compact form's
   `vocations` and `min_level`. If the guard on `main` after #1599 refuses them, the worker narrows
   it to `patterns` written by this slice, and to nothing more.

## 3. Worker packet

```yaml
task_id: OTV2-20261003-item-sem-2b2-equipment-requirements
worker: oteryn-impl-worker   # content lowering; no runtime, persistence or wire change
review: content review on the final frozen head
branch: claude/item-sem-2b2-20261003
base: main after #1599 merges
owned_paths:
  - tools/content-schema/item-authoring/{lower_wiki_stats_packet.py,test_lower_wiki_stats_packet.py,README.md}
  - docs/agents/evidence/OTV2-20261003-item-equipment-requirements-v1.json
  - apps/game-server/src/content/{item_stats_promotion.rs,mod.rs}
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/world/** and the content tree (regenerated)
  - docs/agents/tasks/archive/OTV2-20261003-item-sem-2b2-equipment-requirements.md
leases: none
depends_on: [#1599]
```

Acceptance:

- The packet lowers §2 deterministically; the promotion decodes rows strictly (closed slot and
  vocation keys, one row per field, existing Item only) and sets the fields, as in 2b-1.
- Tests cover: each slot mapping; a hands mismatch (reported, no block); `without`, `None` and
  `paladins and without`; case and singular forms; level `0`; a rune with `mlrequired`
  (`requirements`, `on_use`); ammunition without `slot` (`ammo`, level in `requirements`); a
  two-handed bow (`groups`) next to a two-handed sword (`reserved_slots`).
- The record lists the written counts and every reported row.
- Not in scope: Premium (`premium_only`), the runtime check, promoted-vocation matching and the
  skill-based weapon requirements.
