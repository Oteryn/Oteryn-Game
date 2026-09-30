# OTV2-20260930-item-prof-1b

```yaml
task_id: OTV2-20260930-item-prof-1b
title: ITEM-PROF-1b - weapon vocations for proficiency bindings
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-prof-1b
issue: 162
lane_id: content-world
pr: 1326
base_sha: c145b7f
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "ITEM-PROF-1b impl worker (allocated by claude-code-session-01EfiFA9LMuUuzoNkizLfR2R)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/item-authoring/item_weapon_proficiency.py
  - tools/content-schema/item-authoring/test_item_weapon_proficiency.py
  - tools/content-schema/item-authoring/samples/item-weapon-proficiency-15-30-7fea90ec.json
  - tools/content-schema/item-authoring/README.md
  - imports/crystalserver/facts/items-weapon-vocations.json
  - docs/agents/tasks/archive/OTV2-20260930-item-prof-1b.md
public_contracts: []
depends_on: ["ITEM-PROF-1 (#1322, merged)", "ITEM-SEM-2a (#1324, merged)"]
blocks: ["PROF-CONTENT-1", "PROF-2", "PROFICIENCY-1"]
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

ITEM-PROF-1 (#1322) left 424 of the 666 weapon proficiency bindings with threshold class `unknown` because its only
vocation evidence was the TibiaWiki wave-1 snapshot joined by item name. This task gives every binding a labelled
weapon vocation and re-applies D197-D200 per binding:

- **PROVEN:** TibiaWiki `vocrequired` from `imports/tibiawiki/facts/items-stats.json` (ITEM-SEM-2a, #1324), read
  by the numeric `item_id` so both the current and the re-keyed (`claude/item-sem-2a-fixup`) key forms give a
  byte-identical artifact (verified locally against the fixup branch file). Several differing observations for
  one id are UNKNOWN, not picked. The wave-1 name join is removed.
- **DERIVED:** where TibiaWiki has no value, the Crystal ff7ede5 `data/items/items.xml` weapon `vocation`
  attribute (sha256 `c847293e...`), staged by the new `--stage-crystal ITEMS_XML` mode into
  `imports/crystalserver/facts/items-weapon-vocations.json` (`OTERYN_CRYSTAL_ITEM_WEAPON_VOCATION_FACTS/v1`,
  `OtsHypothesisOnly`, digest-pinned in the generator; the 666 binding ids only; 23 ids newer than ff7ede5 are
  listed in `absent_ids`). A weapon node (`weaponType` or `script ...weapon`) without the attribute is
  `unrestricted`, following how the server applies it. Canary was not needed: no unresolved id is missing from
  Crystal and absent from TibiaWiki except the one below.
- **UNKNOWN:** anything else, with a `reason`.
- D198 extension: a ranged-family binding without `ammotype` uses TibiaWiki `secondarytype`
  (`Crossbows` = crossbow, `Bows` = standard); the build fails if `ammotype` and `secondarytype` disagree
  (no case today).

Result (666 bindings): 37 crossbow / 158 knight / 470 standard / 1 unknown. No previously classified binding
changed class. Vocation basis over all 666: 322 PROVEN, 306 DERIVED, 38 UNKNOWN (37 of them are ranged or other
profiles whose class does not depend on the vocation).

The 424 previously `unknown` bindings resolve as:

| Deciding evidence | Label | Bindings | Class |
|---|---|---|---|
| TibiaWiki `vocrequired` `knights` | PROVEN | 115 | knight |
| TibiaWiki `vocrequired` `None` (Bright Sword, The Justice Seeker) | PROVEN | 2 | standard |
| TibiaWiki `secondarytype` (moonsilver 53225-53228, `vocrequired` paladins) | PROVEN | 4 | 2 crossbow, 2 standard |
| TibiaWiki `secondarytype` (replica bows/crossbows 26001-26067) | PROVEN | 18 | 9 crossbow, 9 standard |
| Crystal `items.xml` weapon without `vocation` (`unrestricted`) | DERIVED | 284 | standard |
| none: ink sword 51666 (no `vocrequired`, no Crystal weapon node) | UNKNOWN | 1 | unknown |

## Owner decisions applied

- D197: fist and every other melee weapon not covered by D200 is standard.
- D198: crossbow per binding: `ammotype=bolt` first, then TibiaWiki `secondarytype=Crossbows`.
- D199: perk enum mapping unchanged from ITEM-PROF-1 (`perk_mapping`).
- D200: Knight only when the resolved sword/axe/club vocation includes Knight (`knights`, `Knight;true, ...`);
  `unrestricted`, `None` or another vocation is standard.

## Notes for the control plane

- The DERIVED `unrestricted` rule (284 bindings) reads "Crystal weapon node without a vocation attribute" as
  no restriction; TibiaWiki has no `vocrequired` for all 284 either. This is the staged option (a) of the
  ITEM-PROF-1 open question, not option (b) (wiki absence alone).
- CI: `item-authoring-schema.yml` does not trigger on `imports/crystalserver/facts/**` or run
  `test_item_weapon_proficiency.py`/`--check`; a later change to `items-stats.json` would not be caught
  there. Reported on #162 for control-plane CI wiring; not changed here.

## Validation

- `test_item_weapon_proficiency.py` 13/13; `item_weapon_proficiency.py --check` up to date (also with the
  `claude/item-sem-2a-fixup` `items-stats.json`); `build_formal_schema.py` no diff; `verify_formal_schema.py`
  246/246; ruff 0.16.1 check and format clean; `item_key_references.py` no new errors; governance validator
  and `tools/agents/tests` pass; `git diff --check` clean. No Rust loader reads the changed files.
- Review: none requested by this worker (content tooling and import evidence; no runtime, protocol or contract
  change); review packet in the PR body.
