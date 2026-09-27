# Oteryn WorldProject/v2 creature admission v1

- Date: 2026-09-27
- Status: CANDIDATE / admission route for the Canary monster authoring bundles; implementation follows in the slices of §7
- Task: `OTV2-20260927-creature-admission-design`
- Programme: KAN-16 / #162
- Companions: `OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md` (authoring format, §5 mapping), `OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md` (v2 source profile), `OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md` (content tree), `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` (encounters)

## 1. Decision

The owner chose to admit the monsters that are already fully resolved into the protected
WorldProject/v2 project. WorldProject/v2 is extended first with the fields the authoring
format carries, so that the monsters are admitted complete, not in part. The owner's wording
was "druga droga", on 2026-09-27; the alternative of admitting only today's v2 fields was
declined.

WorldProject/v2 (`content/world/**`) is the only content the server reads. The content tree
(`content/creatures/**`, `content/abilities/**`, `content/loot/`) is regenerated from it, as it
already is for Items. The monster authoring bundles stay the working format and are never read by
the server.

The v2 rule stands: the **executable** subset is the Reference record graph linked by
`link_reference_playable`. Everything else is **declarative, candidate-only** authoring profile
data that no runtime path interprets until a later owned slice selects its semantics.

## 2. Scope of the first admission (wave A)

The 1,656 Canary `47dfd51f` monster files break down as follows:

| Group | Count | Status |
|---|---:|---|
| Fully resolved monsters (`population_census.py`, one digest each in its bundle index) | 1,490 | resolved |
| Covered by an Encounter manifest | 101 | their events live in an Encounter, and there is no Encounter runtime yet |
| Referencing an Item missing from the Oteryn Item registry | 59 | 68 Canary item ids newer than the Crystal registry |
| With a loot entry whose minimum count is 0 | 1 | Duke Krule: the Reference Loot entry requires a count of at least 1 |
| **Admitted in wave A** | **1,329** | |

The staging tool (§6) computes these groups; its counts are the authority. The 59 monsters wait
until the Item domain registers the 68 items. Duke Krule waits for an owned decision on zero-count
loot entries. The 101 monsters wait for an
Encounter runtime slice, because admitting them without their encounter rules would change the
fight: Kesar would not be immortal, and Urmahlullu's forms would not follow one another.

## 3. Identities

Keys are Oteryn production keys (`ProductionKey`), revision `definition-r1`. The provisional
`canary:` keys of the bundles are provenance only.

| Family | Key |
|---|---|
| Creature | `oteryn:creature.<slug>` |
| Presentation | `oteryn:presentation.creature.<slug>` |
| Behavior | `oteryn:behavior.creature.<slug>` |
| Loot | `oteryn:loot.creature.<slug>` |
| Ability (monster attack or defense) | `oteryn:ability.creature.<slug>.<attack-N or defense-N>` |
| Ability (registered spell shared by several monsters) | `oteryn:ability.spell.<slug>` |
| Effect / Formula | the Ability key with `ability` replaced by `effect` / `formula` and the bundle suffix kept |

`<slug>` is the bundle slug: the Canary type name, lowercase, with non-alphanumerics folded to
`_`. No admitted key carries a non-production marker (`test`, `fixture`, `synthetic`,
`evidence`). A shared spell is admitted once; bundles that reference it must agree
byte-for-byte on its Ability, Effect and Formula, otherwise the writer fails.

**Item references.** A bundle Item reference `canary:item/<id>` resolves through the protected
Item identity map (`export_reference_item_identity_map`, allocation digest `ee9219cc…`) from
the Crystal source item id to the Oteryn Item key. Canary and Crystal share the client item id
space: of the 2,655 referenced ids present in both, 17 differ only by name variant ("remains
of" or "dead") or by a missing Canary `items.xml` entry. Items stay identity records; no Item
semantics are changed.

**Provenance.** Admission adds a source `oteryn:source.canary`: revision
`47dfd51f45280a59a1d3e50ba7edd573d7234446`, evidence `OtsHypothesisOnly`. Each creature gets a
source identity binding with namespace `canary/monster-file` whose external id is the path of its
Canary monster file below `data-otservbr-global/monster`, disposition `EXACT`. Wiki values adopted under D15 are recorded row by row in the import
manifests. The line-level import manifests stay regenerable evidence outside the
repository. The writer records their digests in the import batch.

## 4. Executable Reference records

Each admitted monster adds records the current linker accepts unchanged:

- `Creature { client_projection: ClientSafe, presentation, behavior, loot? }`;
- `Generic` Presentation (`ClientSafe`) and Generic Behavior (`ServerOnly`);
- `Loot { algorithm: IndependentBernoulliPpm, entries }`: `probability_ppm = percent × 10,000`,
  exact under D1, and `min_count`/`max_count` as authored;
- `Ability { effects }` (`ServerOnly`);
- `Effect { effect_family, formula }` for the damage and heal effects;
- `Formula { identity }` (`ServerOnly`).

The Reference profile knows only the `Damage` and `Heal` effect families. The other effect
operations of wave A have no executable semantics yet, so they are not Reference records.
They remain typed inline effects of their Ability's authoring profile (§5):

| Operation | Count |
|---|---:|
| condition | 1,718 |
| appearance transform | 107 |
| presentation only | 63 |
| create item | 57 |
| summon | 10 |
| remove condition | 6 |
| remove items | 1 |

The Ability record lists its executable effects in authored order. The profile keeps the full
authored order.

## 5. Declarative authoring profiles

`ProjectV2AuthoringProfileData` gains typed profiles that mirror the authoring format. They are
validated structurally in Rust (ranges, sorted and unique sets, exact references), while the
Python authoring validator stays the semantic source.

| Profile | Carries |
|---|---|
| Creature (extended) | the existing fields, plus defense, critical chance, flags, summoning, system and spawn eligibility, name forms, inspection, corpse Item and death residue, bestiary class, taxonomy, stars and locations, reflection and healing-from-damage |
| Behavior (new) | movement, targeting and target change; attack and defense schedules (Ability, `interval_ms`, `chance_percent`, magnitude, range); voices; summons |
| Presentation (new) | appearance, palette, attachment and visual-effect asset binding tokens; light; audio (asset binding tokens stay unbound until an asset slice admits them) |
| Ability (extended) | kind, range, target and direction needs, area geometry, chain, variants, path requirement, and inline non-executable effects |
| Effect (new) | damage type, mitigation, affected side and presentation of damage and heal effects |
| Formula (new) | kind (`range`, `speed_modifier`, `caster_magnitude`, `melee_attack_skill`) with its parameters |
| Loot (new) | `skip_later_same_item_after_success`, true on 16 entries, which the Reference Loot entry lacks |

A profile never overrides a Reference record field. When both carry a value, the writer emits
equal values or fails. Health admits `max_health` as `health`; every wave A monster has
`initial_health == max_health`.

## 6. Writer and regeneration

`tools/content-migration/creature_admission_stage.py` reads the census bundles and checks each one
against its digest in the bundle index. It also reads the protected Item identity map
(`export_reference_item_identity_map`) and writes one canonical staged file. That file holds the
Reference records, the authoring profiles and the source identity bindings, in the exact serde
shapes of the game server. It also lists every deferred monster with its reason. The
materializer (`materialize_content_world_project_v2`) pins the staged file by SHA-256, adds it
to the project with the `oteryn:source.canary` import batch, and writes the canonical v2
documents with their manifest and Content Lock. The content tree is then regenerated from v2. Rust admission (`ProjectV2Draft` load, validation and
`link_reference_playable`) is the acceptance check, together with focused tests over one pilot
monster of each profile shape.

## 7. Slices

1. This decision.
2. Rust: the §5 profiles, `canonicalize`, `validate_v2_authoring_profile` and focused positive
   and negative tests. No content change.
3. Writer and pilot: about 20 monsters, including a shared spell, inline condition effects and
   a skipped loot entry, through admission and linking.
4. Wave A in bulk (1,329) and regeneration of the content tree.
5. Later: the 59 Item-blocked monsters after the Item domain registers the 68 items; Duke
   Krule after a zero-count loot decision; the 101
   encounter monsters with an Encounter runtime slice.

Each slice runs the repository gates. A slice that changes `content/world/**` also gets one
independent exact-head review before the Merge Queue (standing authorization in
`OWNER_FUNDED_AI_POLICY.md`).

## 8. Boundaries

Admission does not add spawns, placements or any runtime behaviour. It does not bind client
appearance assets or licensing, and it does not change Item semantics. It does not admit
Encounters or Lua scripts. Everything declarative stays candidate-only under the v2 rule.
