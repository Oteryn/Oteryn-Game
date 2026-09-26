# Creature Bestiary notes authoring candidate v1

- Status: CANDIDATE; example only, no production Creature or client activation.
- Scope: an optional player-facing narrative field on the static Bestiary overlay for a Creature.

## Field separation

| Oteryn authoring field | Meaning | Donor equivalent |
| --- | --- | --- |
| `presentation.look_description` | Short look/name text, e.g. `a cyclops` | Canary/Crystal `monster.description`, registered as `nameDescription` |
| `bestiary.locations` | Where the Creature is found | Canary/Crystal `monster.Bestiary.Locations` |
| `bestiary.notes` | Authored narrative text for the Bestiary UI | No separate Canary/Crystal monster field or Bestiary packet field |

`bestiary.notes` is optional text. It cannot drive behavior, loot, combat, unlock
rules, or identity. Its absence means that no note has been authored; it does not
mean the note is an empty string. The Oteryn client must have its own explicit
projection before any note can appear in game. This proposal does not reinterpret
the existing Bestiary wire packet or change the Reference Creature runtime.

In pinned Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446` and Crystal
`ff7ede593c69d4c658b382c97443e8155926924a`,
`ProtocolGame::parseBestiarysendMonsterData` writes only one narrative string
after the element data: `mtype->info.bestiaryLocations` (packet `0xD7`). The
Cyclops source definition has `description = "a cyclops"` and a separate
`Bestiary.Locations` value. The code does not supply the longer note shown on
TibiaWiki Fandom, which includes a passage attributed to Tibia Library.

`docs/architecture/examples/cyclops_bestiary_notes_candidate.json` illustrates
the proposed overlay. Its narrative is an original short paraphrase of the
documented facts, not a copy of long-form Wiki/Tibia prose. The proposed
`creature:cyclops` key is illustrative; actual identity binding is a separate
admission step. Source evidence and revisions remain in import/evidence data,
not a `source` field in the Creature definition.

This candidate extends the intended static Bestiary authoring surface in
`OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md`; it does not change the
existing `READY_UNPOPULATED` directory state, game runtime, or `content/world`.
