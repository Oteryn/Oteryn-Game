# Charm authoring schema candidate v1

The static Charm catalogue: the 25 Bestiary Charms (14 major, 11 minor) with their category, kind, per-stage
cost and value, and a typed effect. Per the full-game tree contract
([`OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md`](../../../docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md),
"Bestiary / Bosstiary / Charms"), this is only the content layer:

- Unlock, assignment and proc rules belong to `rulesets/progression/charms/`.
- A character's Charm Points, Minor Charm Echoes, unlocks and assignments are durable Character state.
- Per-creature charm points belong to the Bestiary facts in `content/creatures/bestiary/`.

None of these is modelled here. The `key` values (`oteryn:charm.<name>`) are minted at content population, which is `content` below;
`capture`, `build` and `validate` still write only under this directory.

| File | Purpose |
|---|---|
| `charm.schema.json` | One catalogue (`OTERYN_CHARM_AUTHORING_CATALOGUE/v1`), JSON Schema 2020-12, closed shapes. The effect is a `oneOf` over typed shapes: `attack_proc_damage`, `attack_proc_resource_damage`, `kill_area_damage`, `reflect_damage_taken` (each damage shape says whether it ignores resistances and whether armor reduces it), the timed effects (paralyse, haste, prevent flee) and the parameterless effects (dodge, leech, critical and so on). |
| `charm_authoring.py` | `capture` writes the source facts. `build` derives the catalogue and the comparison report from them, and `build --check` diffs an in-memory build against the committed samples. `validate` runs the schema and semantic checks. |
| `charm_authoring.py content` | Writes (`--check` verifies) `content/charms/index.json` (`OTERYN_FAMILY_INDEX/v1`) and the shard `charms-00000-00024.json` (one `definition` per Charm, identity `{key, revision: definition-r1}`), and registers `Charm` in `content/project.json` (`migrated_families`), `content/manifest.json` (`families`, `managed_files`) and `content/content.lock.json` (`family_counts`). |
| `test_charm_authoring.py` | No-network tests: infobox parsing and its rejections, the pinned Canary digest, the committed build, and one negative case per validator rule. |
| `samples/charm-sources-2026-09-29.json` | The captured source facts (details below). |
| `samples/charms-candidate.json` | The candidate catalogue, which validates. |
| `samples/charm-source-comparison.json` | Where TibiaWiki and Canary disagree. |

## Sources

- **TibiaWiki (tibia.fandom.com), `Derived`, primary.** The 25 pages of `Category:Charms` (`Infobox Charm`).
  - Stored per page: page id, revision id, timestamp, SHA-256 of the raw wikitext, and the infobox name, type, cost,
    the per-stage percent triple and the `implemented` version. No wiki prose is stored.
  - Parameters that do not vary by stage (5% of the creature's maximum health, the damage caps of 2× and 6× level, the
    8% cap, resistance and armor behaviour, the 10 s and 30 s durations, 15%, 2.5%) are written in `CHARMS` in the
    code. Capture rejects the page unless its text contains the phrases listed next to each entry; those phrases are
    stored as `confirmed_phrases`.
  - Currency (major charms cost Charm Points, minor charms cost Minor Charm Echoes) is from the wiki pages
    `Major Charms` and `Minor Charms`.
- **Canary `47dfd51f`, `OtsHypothesisOnly`, cross-check.** `data/scripts/systems/bestiary_charms.lua`, pinned by
  SHA-256 and read with a strict pattern parser.
  - Supplies `kind` (offensive, defensive or passive), which the wiki does not categorise, and `charm_id` (list
    position minus one). Whether that id is the client protocol id is not verified.

## Rules (`validate`)

- The schema passes.
- Keys, names and Canary ids are unique, and each key follows its name.
- The currency matches the category (major: `charm_points`; minor: `minor_charm_echoes`).
- `stage_value` matches the effect type: an effect that triggers carries `trigger_chance_percent`, an always-on
  bonus carries `effect_percent`.
- There are exactly the stages 1, 2 and 3, and both cost and value strictly increase across them.
- A trigger chance is at most 100%.

## Content population

`tools/content-migration/world_project_v2_to_tree.py` registers the committed Charm family (no legacy source) when it
regenerates project, manifest and lock, so `content --check` and the generator agree byte for byte.
Nothing loads `content/charms/` at runtime; `runtime_source` stays `legacy_until_separately_qualified`.

## Wiki and Canary comparison (2026-09-29)

All 25 charms agree on name, category, stage costs and stage values. The catalogue follows the wiki, including its notes:

- **Overpower and Overflux:** shown as physical, but the damage ignores the creature's resistances (`ignores_resistances`). This
  matches Canary's neutral damage.
- **Parry:** shown as physical, ignores resistances, and is reduced by the creature's armor. Canary's Lua table says `PHYSICAL`,
  but its C++ handler (`iobestiary.cpp`) deals `COMBAT_NEUTRALDAMAGE` blocked by armor, which matches the wiki.
- **Carnage:** capped at 6× the character's level and reduced by armor; Canary and Crystal use the same cap.

2 points stay open for the rules layer:

- **Bless:** Canary also carries `percent = 10`, but its death-loss code uses only the 6/9/12% stage values, like the wiki.
- **Carnage:** the wiki calls it Physical Damage and says nothing about resistances. Canary deals it as neutral.
  The catalogue keeps the wiki (`ignores_resistances: false`) until verified in the live game.

Crystal (`00ce02a`) has the same charm table and handlers as Canary.

```sh
pip install -r requirements.txt
python charm_authoring.py build --check
python charm_authoring.py validate samples/charms-candidate.json
python charm_authoring.py content --check
python test_charm_authoring.py

# evidence refresh (network + Canary checkout at 47dfd51f; local only)
python charm_authoring.py capture --canary <canary checkout>
python charm_authoring.py build
```
