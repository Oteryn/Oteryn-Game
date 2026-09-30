# Oteryn Monster Authoring Schema v1

- Date: 2026-09-26
- Status: CANDIDATE / authoring schema with executable validation; no runtime or WorldProject storage supersession by this document
- Task: `OTV2-20260926-monster-authoring-schema-v1`
- Programme: KAN-16 / #504
- Admission main: `3b578bc5b35e40fbff70fbe5758079fdbed6a3c0`
- Machine artifacts: `tools/content-schema/monster-authoring/`
- Origin: monster authoring proposal v2 at
  [`3630e92e9fcee07274411e81d3193e603be1bc24`](https://github.com/Oteryn/Oteryn-Game/tree/3630e92e9fcee07274411e81d3193e603be1bc24/docs/agents/evidence/OTV2-20260926-monster-authoring-proposal)
  on `codex/monster-authoring-schema-20260926`, including its advisory reviews and `REPAIR_NOTES.md`

## 1. Outcome

Oteryn has one candidate authoring format for monsters and their direct dependencies.
It is the working format for describing monsters sourced from Canary/Crystal and for
preparing their later admission. It is a companion to
`OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md` (`content/creatures/**`, `content/loot/`),
in the same way `OTERYN_ITEM_AUTHORING_MASTER_SCHEMA_V1.md` is the Item companion.

The server does not read this format. Executable adoption happens by extending
WorldProject/v2 (`apps/game-server/src/content/project/v2.rs`) in later implementation
slices, using §5 as the mapping and the validator cases as the specification.

## 2. Shape

```text
monster bundle (monster.schema.json)
├── creature       identity, names, inspection text, stats, resistances, immunities, flags,
│                  summoning, bestiary, bosstiary, corpse/residue/soul-core Items,
│                  encyclopedia Document, reflection/healing, system and spawn eligibility
├── behavior       movement, targeting, attacks[], defenses[], voices, summons,
│                  periodic audio, faction/preferences, event bindings -> Interaction
├── presentation   appearance/palette/attachment/effect asset bindings, light, audio
└── loot           IndependentBernoulli table (optional)

direct dependencies (monster-dependencies.schema.json)
└── abilities[], effects[], formulas[], documents[], items[] (capability projection), loot_tables[]

import disposition ledger (monster-import-readiness.schema.json)
└── Git source revisions + per-field source file/line dispositions
```

Every cross-definition link is an exact `{family, key, revision}` reference using the
WorldProject/v2 family vocabulary. Original source-language text (inspection, voices,
Document content) is kept unchanged; nothing is translated or invented.

## 3. Numeric decisions

These decisions close the open questions raised in the proposal reviews.

| # | Decision | Rationale |
|---|---|---|
| D1 | Chances stay JSON numbers in **0–100 percent** (`0.19` = 0.19%) with at most 4 decimal places. The structural schema carries only the range; the 4-decimal rule is enforced by `validate_monster.py`. Native ppm = `percent × 10000` and must be an exact integer. | Keeps the owner-selected readable format. Removing `multipleOf: 0.0001` lets any binary-float JSON Schema validator (editors, CI) accept valid values such as `0.29` and `4.93`, which it previously rejected. Admission must preserve the original JSON numeric token and validate it with exact decimal/integer arithmetic before any binary-float conversion. For its exact ratio `n/d`, require `(n × 10000) % d == 0` and emit the integer quotient. No rounding tolerance is permitted, including in a future Rust importer. |
| D2 | Ratios (`resistances`, `mitigation_percent`, reflection/healing, speed multipliers) must be in **lowest terms**; zero is `0/1`. | Matches `validate_v2_ratio` in WorldProject/v2, so a valid authoring ratio cannot be rejected on admission. |
| D3 | Durations and intervals are **integer milliseconds** everywhere. | One unit across the schema. WorldProject/v2 `ProjectV2FamiliarProfile.duration_seconds` is converted on admission (see §5); a value not divisible by 1000 is a mapping gap, not a reason to change the authoring unit. |
| D4 | Health keeps **`max_health` and `initial_health`** with `1 <= initial_health <= max_health`. | Both are distinct source facts. WorldProject/v2 currently has one `health`; see §5. Zero-HP helper entities remain blocked as `unresolved_semantics` rather than admitted with invented HP. |

Trailing zeros and exponent notation are accepted when the exact value lies on the 1 ppm
grid (for example `0.290000` and `2.9e-1` both yield 2900 ppm). Extra nonzero digits
such as `0.29000000001` or `0.29000000000000000000000000000001` are rejected, not rounded.
The Python validator uses `Decimal.as_integer_ratio()` followed by integer `divmod`,
so acceptance does not depend on the active Decimal precision. Data read as `f64` alone
cannot establish the precision of the original source token.

### Import decisions (owner, 2026-09-26)

Recorded after the Canary test batches (`tools/content-schema/monster-authoring/samples/`).

| # | Decision | Source basis |
|---|---|---|
| D5 | Imported monsters with no source counterpart set `behavior.movement.pass_through=false`. | Canary/Crystal have no pass-through monster flag. |
| D6 | Creature event scripts that only feed quest/task progress (e.g. `RationalRequestRatDeath`) belong to Quest/Interaction and are recorded as `approved_omission` in the monster manifest. | The inspected events are task counters, not monster behaviour. |
| D7 | `creature.death_residue` is `{item, fluid_type}`. Races `venom`/`blood`/`ink`/`chocolate`/`candy` map to splash Item 2886 with fluid `slime`/`blood`/`ink`/`chocolate`/`candy`; `undead`/`fire`/`energy` leave none, and the field is omitted. Summons drop no corpse or residue. | `creature.cpp` `Creature::dropCorpse`, identical in Canary `47dfd51f` and Crystal `be61cdd3`/`ac447fef`. One splash Item differs only by fluid, so an `ItemRef` alone could not express it; this replaces the proposal's `death_residue_item`. |
| D8 | `bosstiary` stores `prowess/expertise/mastery_points` per stage instead of one `boss_points`. | Canary `io_bosstiary.hpp` `levelInfos` awards points per reached stage (bane 5/15/30, archfoe 10/30/60, nemesis 10/30/60). Found by the second Canary batch. |
| D9 | A boss is a monster with the optional `bosstiary`, `reward_boss` and `reward_encounter`; arena, phases, timers, cooldowns, reward chest and combat-changing boss scripts belong to the referenced Encounter. | Canary/Crystal register bosses as ordinary monster types; the boss logic lives in quest scripts (e.g. Forgotten Knowledge `HealthForgotten`, boss-kill cooldowns). |
| D10 | The importer resolves a monster spell name as Canary does: registered rune spell, then registered instant spell (case-insensitive), then built-in kind. The resolved script is recorded in the manifest. | `Monsters::deserializeSpell` calls `Spells::getSpellByName` before building a built-in kind; §8. |
| D11 | Every monster spell is an `Ability` (+ `Effect`s, `Formula`) under `content/abilities/**`. A player spell or rune used by a monster is the one shared Ability; a monster-only script gets its own Ability keyed by the source spell name. The monster attack entry keeps interval, chance, range and its damage magnitude, which overrides the Ability formula. | `Combat::getCombatDamage` uses `Monster::getCombatValues` whenever the monster entry has a non-zero min/max; §8. |
| D12 | Spell schema extensions are added in census order, each with a batch monster that needs it: area matrix with explicit centre and directional rotation; constant-tick DoT (count, interval, per-tick amount); attribute-modifier condition (skill/stat, percent or absolute); `Ability.variants` with a uniform pick; chain targeting (further-creature count, range, backtracking; player-spell extensions in §8.6, S23). The damage distribution is a world combat rule, not a per-formula field. | Census frequencies in §8. Canary draws all monster damage with `normal_random`; whether Tibia Global does the same is unproven, so the rule is an OTS hypothesis until checked. |
| D13 | A spell with custom logic becomes an `Ability` with a `native_behavior` key and its data parameters. Native behaviours are shared and parameterized by pattern (e.g. one path-chain behaviour with an element parameter), not one per source script. The content compiler rejects a key without an implementation. No Lua is admitted; an implementation is written only when a playable monster needs it, and until then the manifest row stays `unresolved_semantics`. | 44 custom-logic scripts cluster into recurring patterns (path chains, summon-N, cast-then-remove-self); §8. |
| D14 | A spell reference that has no effect in Canary is recorded as `approved_omission`; when the reference-date wiki shows that attack, it is authored as an ordinary Ability from the wiki instead. | `energy beam` returns false for a non-player caster (4 monsters). |
| D15 | Where the reference-date (the programme target date, D33) wiki differs from Canary, the wiki value replaces it. So far this is applied to mitigation, `pushable`, loot items missing in Canary and loot probabilities. Loot rate rule: use the highest-version `Loot Statistics` block at the cut (the largest-sample source; other sites such as Tibiopedia are cross-checks only); estimate = drops / kills rounded half-even to 1 ppm, with a 95% Wilson interval recorded. At 10 or more drops the estimate replaces the Canary probability; below 10 the Canary probability is kept and marked low confidence (an item missing in Canary is still added, marked low confidence). An item the infobox lists but the statistics block does not show keeps its Canary probability (probably added after that version). An ambiguous item name is resolved by the item page `itemid`. | The wiki tracks Tibia Global more closely than OTS sources. Batch 1 comparison: `samples/canary-47dfd51f/wiki-2026-07-28.json`. |
| D16 | A familiar is split three ways: the familiar creature (one per vocation) stays a monster with `is_familiar`; the summon parameters (vocation, level, mana, cooldown, duration) belong to the player summon `Ability`; the familiar looks are a character cosmetic catalogue with per-character unlocks and selection. The monster keeps `presentation.appearance.selection=owner_familiar_look` with the vocation default look as `asset_binding`. Moving the summon parameters out of the creature and the look catalogue are later admission work. | TibiaWiki: familiars of one vocation differ only by name and look, chosen in "Customize Character". Canary: `data/libs/systems/familiar.lua` `FAMILIAR_ID` default looks, set by `creaturescripts/familiar/on_login.lua`; per-character choice from `data/XML/familiars.xml`. |
| D17 | Any Item can be a monster corpse; the validator no longer requires the Item capability `is_corpse`. | Canary drops whatever Item id the monster names (45 monster files use ashes, fish, remains and similar items without the corpse flag). |
| D18 | Registered spell scripts with custom logic are expressed through the 19 shared, parameterized native behaviour patterns of §8.4; boss-specific logic (`boss_form_swap`, `boss_escape_utility`, `map_or_quest_specific`) belongs with the Encounter definitions (D9), not with monster behaviours. Wiki ability scenes stay review evidence (§9.2) and are not adopted for now. | Owner acceptance of the §8.4 grouping (93 blocking scripts, model-assisted with evidence lines). |
| D19 | Plain combats get schema fields instead of behaviours: damage `mitigated_by` (§8.5) and the `remove_condition` operation for `COMBAT_PARAM_DISPEL`; the converter binds `setParameter` keys and values as the engine reads them (an undefined constant is 0). | `combat_functions.cpp luaCombatSetParameter`, `lua_functions_loader.hpp getNumber`, `combat.cpp setParam`/`CombatDispelFunc`, `monsters.cpp deserializeSpell`; the Crystal Server `monsters.cpp`, `combat.cpp` and `blockHit` paths were checked and match. |
| D20 | Encounter mechanics (boss events, `mType` callbacks, summons on fixed map positions) get their own Encounter definition format; a design is drafted for owner acceptance before any implementation. | 168 monsters are blocked by events alone (§9). |
| D21 | A damage-over-time whose tick amounts grow geometrically is authored once as `tick_profile: geometric` (base range, factor, tick counts), not as one variant per base value. | `ghastly_dragon_curse.lua`: 393 combats, base 40-170, each tick x1.2, 5-7 ticks. |
| D22 | Wiki loot whose item page lists several item ids for one name (for example `giant shimmering pearl`, 281 green and 282 brown) is added as one entry per id, each with an equal share of the estimated probability. | The Loot Statistics page counts the name, not the id. |
| D23 | Wiki loot without a Loot Statistics page keeps the Canary loot list: the item is not added, because no probability is known. | Extends the D15 loot rate rule (fewer than 10 drops keeps Canary). |
| D24 | A creature without a visible appearance (Canary `lookType` 0, no `lookTypeEx`, not a familiar) is authored with `appearance.selection: invisible`. | Wild magic traps; the wiki lists them as Traps. |
| D25 | A combat whose damage type is undefined in the source (`COMBAT_UNDEFINEDDAMAGE` from a missing or wrong constant) is a source data error; the reference-date wiki ability decides the element. More generally, wherever the source is uncertain, the reference-date wiki decides (extends D15). | Owner rule in this session. |
| D32 | Loot counts follow the reference-date wiki too. When a Canary entry adopts the wiki probability (at least 10 drops), the count observed in Loot Statistics replaces the Canary min-max if it falls outside it; an observed range inside the Canary range is kept, since it is a sample. A Canary `minCount` of 0 (Canary draws the count from 0..max and drops nothing on 0) becomes count 1..max: the wiki estimate already counts only kills with a drop, and without wiki statistics the probability is scaled by max/(max+1), which is exactly what the engine does. Low-confidence entries and items split over several Canary entries keep the Canary counts. | Owner, 2026-09-27 ("używaj wiki do potwierdzania loota dla potworów i bossów"); extends D15. |
| D33 | The reference date is the programme target date, which the owner moved from 2026-07-28 to 2026-09-27 ("na dzień dzisiejszy"); the owner may move it again. Every wiki value is read at that date and keeps its page revision. The population comparison, the batch comparison and the ability scenes are re-read at 2026-09-27. | Owner decision 2026-09-27, `docs/agents/programs/OTERYN_TARGET_DATE_20260927_DECISION.md`. |
| D43 | TibiaWiki BR fills health and experience where the Fandom page at the target date is missing or gives no certain value (empty, `?`, `~` or unparsed). The owner's source order puts BR after Fandom, and BR is used for tables and cross-checks. The BR value must be a plain number. BR element modifiers are not used, because "100%" there is often the template default; BR speed and loot are not used either. `wiki_br_capture.py` reads the pages on a hosted runner (the build container gets a Cloudflare bot check). `wiki_br_fill.py` selects the fills into `samples/wiki-br-fill-2026-09-27.json`, and the converter applies them after the D15 values. | Owner answer 2026-09-27 ("Tylko HP i doświadczenie"), after BR gave Dark Knight 7,900 health where Fandom has "?" and Canary 1,800. |
| D44 | A monster Tibia has at the target date and Canary lacks is authored from the wiki as close to Tibia as the sources allow. `wiki_authored.py` writes it as a Canary-format file from pinned facts (Fandom first, then TibiaWiki BR, per the source order) and converts it with the normal converter; each manifest row cites the wiki revision line of its value. A value no wiki page gives is taken from the closest Canary template monster, and its row is marked `NEEDS VERIFICATION (D44)`; an ability whose damage no source gives is left out and recorded as an omission to verify. The creature is bound to its Fandom page id (`mediawiki/page_id`) under its own `oteryn:source.tibiawiki` import batch. First case: Dark Merudri (Grave Danger, summoned for a Monk in the Count Vlarkorth fight); template Dark Knight. | Owner answers 2026-09-28 ("uzupełnić to co masz, żeby było maksymalnie zbliżone do Tibii, ale zaznacz, że to wymaga kiedyś weryfikacji"); Canary and CrystalServer, all active branches, have no Dark Merudri. |
| D45 | An ability may point to an encounter (`encounter`) instead of listing effects or variants. A monster spell whose Canary script summons creatures, and keeps counters, flags or timers for it, is converted this way: the monster keeps the ability with its range and targeting, and the encounter's `ability_cast` rule does the summon (OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1 D45). | Owner answer 2026-09-28 ("Tak, przez encountery"). |
| D47 | Source order for the reference-date state (owner answer 2026-09-28): a Tibia.com news item dated on or before the target date, then the Tibia.com creature library, then the wiki at the target date, then Canary and CrystalServer as hypotheses. The library has no history, so a capture counts as the target-date state only while no news item between the target date and the capture changes the creature. `official_library.py` records the library health and experience from TibiaData captures; the converter adopts them over Canary and the wiki as an `official_capture` source. A percentage from a news item is applied directly only where the library has no entry, and its rounding is marked uncertain. A change without numbers is recorded as known but not quantified. Library descriptions stay out of the repository; the entry keeps the library URL. | Guessing values for changes without numbers. |

### Game version 15.30 monsters from CrystalServer

Rule from the game version 15.30 decision
(`docs/agents/programs/OTERYN_GAME_VERSION_1530_AND_OTS_BRANCHES_DECISION_20260928.md` §1–2); it is not a new
owner decision. The Summer Update 2026 monsters are in no Canary branch, so `crystal_batch.py` takes them from
`zimbadev/crystalserver`, branch `summer-update`, pinned at commit `00ce02a57ca5a12e48f32a3476e37471167e4c3f`
(read 2026-09-28). It selects the files under `data-global/monster/summer_update_2026` whose monster name no Canary
file creates: 49 files, 8 of them bosses.

- The files are in the Canary format. The normal converter reads them with its Canary `47dfd51f` engine rules, and
  resolves loot names through the 15.30 `items.xml` and `appearances.dat` of the same Crystal commit.
- Each manifest names the Crystal repository and commit as source 0 and the Crystal file as the source file of its
  rows.
- The source order of D47 applies unchanged. The reference-date wiki values
  (`samples/wiki-population-crystal-00ce02a5-2026-09-27.json`, D15) and the library values
  (`samples/official-library-crystal-00ce02a5-2026-09-28.json`, from the same TibiaData captures) are adopted over
  the Crystal values.
- `population_census.py` adds the result to the census as its own group, `crystal_*` counts. The admission binds
  each admitted creature to its Crystal file under the import batch `g4-creature-crystal-1530-r1`, whose
  `oteryn:source.crystalserver` source revision is `crystalserver-creature-1530:<commit>`. `crystal_batch.py` stops
  unless both checkouts are at their pinned commits with clean read paths, and lists the monster files from those
  commits.

## 4. Carried semantics

Retained from the proposal without change (details in the origin `README.md`/`REPAIR_NOTES.md`):

- DoT stores a nominal total damage range, tick interval, automatic/fixed first tick,
  decreasing profile and delayed first tick; not per-tick ranges.
- `lifetime=damage_schedule` forbids an authored `duration_ms`; `fixed_duration` requires it.
- `Effect.operation=presentation_only` requires a visual binding and carries no gameplay payload.
- `Ability.needs_direction` is independent of the final area; area centre precedence is
  target, facing-adjacent position, caster.
- `ignore_period_underground` bypasses the day/night period underground; it is not a general
  underground permission.
- `summonable`/`convinceable` share one `mana_cost`; Familiar has its own profile cost.
- `bestiary.notes` is optional Oteryn-authored narrative. It is separate from
  source Bestiary `locations`, the short inspection description and library
  `encyclopedia.description_document`. Do not manufacture Notes from absent
  Canary/Crystal data or bulk-copy third-party wiki prose.
- Loot entries are processed in authored order. With `skip_later_same_item_after_success`,
  a positive-quantity drop suppresses later entries for the exact same Item within that table
  invocation. `min_count` may be 0; `max_count >= 1`; `min_count <= max_count`.
- `$defs.item` is a capability projection used to verify corpse/container/decay facts; it is
  not Item authority. `ItemRef` must resolve to the canonical Item catalogue before admission.
- The disposition ledger accepts Git commits and MediaWiki page revisions. A MediaWiki source is
  pinned by page id, immutable revision id and the SHA-256 of that revision's wikitext
  (D15); `source_file` is the page title and `source_line` the wikitext line. A Git hash is never
  invented for a wiki page. Wiki values are player observations and are recorded as such.

## 5. Mapping to WorldProject/v2

`ProjectV2CreatureAuthoring` at admission main is the only creature structure the server
parses today. Status per area:

| Authoring field | WorldProject/v2 | Status |
|---|---|---|
| `stats.max_health` / `initial_health` | `health: Option<u64>` | GAP: v2 has one value; admit `max_health` as `health` only when equal to `initial_health` until v2 gains a split. |
| `stats.experience`, `speed`, `armor` | same names | MATCH |
| `stats.defense`, `critical_chance_percent` | — | GAP |
| `stats.mitigation_percent` | `mitigation: ProjectV2ExactRatio` | MATCH (D2) |
| `resistances[].reduction_percent` | `resistances[].percent` | MATCH after rename (D2); v2 also requires sorted/unique entries |
| `immunities.damage_types` + `conditions` | `immunities: Vec<String>` | MATCH after flattening |
| `behavior.movement.pushable` / `push_items` + `push_creatures` / `pass_through` | `pushable` / `pushes_objects` / `pass_through` | PARTIAL: v2 has one push flag |
| `behavior.attacks[]`, `defenses[]` (`ability` refs) | `abilities: Vec<ProjectV2DefinitionRef>` | PARTIAL: v2 has refs only, no interval/chance schedule |
| `bestiary.difficulty`, `occurrence`, `kill_thresholds`, `charm_points` | `ProjectV2BestiaryProfile` | MATCH |
| `bestiary.class`, `taxonomy`, `stars`, `locations`, `notes` | — | GAP |
| `bosstiary.*` | `ProjectV2BosstiaryProfile` (single `boss_points`) | PARTIAL: v2 has one points value, D8 has three |
| `summoning.familiar.duration_ms` | `familiar.duration_seconds` | MATCH via `/1000` (D3) |
| `summoning.familiar.vocation`, `summon_ability`, `mana_cost`, `owner_speed_bonus` | `ProjectV2FamiliarProfile` | MATCH |
| `loot.entries[].probability_percent` | `LootEntryDocument.probability_ppm` (`content/project.rs`); v2 has the `Loot` family but no Loot profile | PARTIAL: D1 defines the exact scale conversion; v2 has no Loot profile to store this entry. |
| flags, targeting, voices, summons, presentation, spawn/system eligibility, reflection/healing, `death_residue` | — | GAP |

GAP rows are the input for the executable adoption slices; they are added to v2 only when a
real playable monster needs them.

## 6. Boundaries

This candidate does not change WorldProject/v2 storage, the compiler, runtime, protocol or
persistence; does not populate `content/creatures/**`; does not admit Canary/Crystal Lua
scripts, which remain `script` entries requiring an explicit native behaviour resolution; and
does not establish Tibia Global parity or asset/licensing rights.

## 7. Validation

From `tools/content-schema/monster-authoring/` with `requirements.txt` installed:

```text
python build_formal_schema.py      # regenerates the 3 schemas and 2 empty templates byte-identically
python verify_formal_schema.py     # 229 focused positive/negative cases
python verify_source_coverage.py   # 242 inventoried Canary/Crystal registrar/spell paths accounted for
python validate_monster.py <monster.json> <dependencies.json> [--catalog C] [--manifest M]
```

`build_formal_schema.py` is the source of the schemas; edit it, not the generated JSON.
Validator success means authoring structure and declared-reference closure only; it reports
`runtime_qualified=false` and `source_coverage_proven=false`.

Manifest destinations use the JSON string form of RFC 6901 pointers. Array tokens must be
`0` or an ASCII digit sequence starting with `1`–`9`; signed, zero-padded, whitespace and
Unicode-digit indices are rejected. `-` cannot resolve an existing element. Object member
names remain unrestricted by the array-index rule. Only `~0` and `~1` are valid escapes,
and scalar values cannot be traversed. The empty root pointer is supported by the resolver
but is not a valid manifest destination (the manifest requires a nonempty string).

## 8. Registered monster spells

Source: `tools/content-schema/monster-authoring/samples/spell-census-canary-47dfd51f.json`,
produced by `spell_census.py` over all 1,656 Canary `47dfd51f` monster files. Six files cannot
be read without quest configuration (five Soul War bosses and one helper file). Only `attacks`
and `defenses` entries are counted.

### 8.1 Resolution in Canary

`Monsters::deserializeSpell` first calls `Spells::getSpellByName(name)`: rune spells, then
instant spells, case-insensitive. It builds a built-in kind (`combat`, `melee`, `speed`, ...)
only when no spell is registered under that name, so a registered name shadows a built-in kind
(`fear` and `soulwars fear` are scripts). The script's `onCastSpell` runs with the monster as
caster. When the monster entry has a non-zero min/max, `Combat::getCombatDamage` takes the
damage from it (`Monster::getCombatValues`) instead of the script formula. The value is drawn
by `normal_random` (normal distribution, mean 0.5, sd 0.25, redrawn outside [0, 1], scaled onto
[min, max]) for built-in kinds as well.

### 8.2 Census

| Tier | Meaning | Spells | Monster references |
|---|---|---:|---:|
| P1 | player spell or rune reused by a monster; one Combat execution | 15 | 37 |
| P2 | `onCastSpell` only executes one Combat | 166 | 281 |
| P3 | `onCastSpell` executes one Combat picked by `math.random` | 48 | 94 |
| P4 | custom logic: summons, target search, loops, timers, per-target Lua callbacks | 44 | 87 |
| NOOP | player-only script that returns false for a monster caster | 1 | 4 |
| MISSING | no registered spell and no built-in kind | 0 | 0 |

P1–P3 (229 of 274 spells, 412 of 503 references) are declarative. Their most frequent
primitives by spell count: area matrix 191 (130 of 200 area uses are script-local matrices,
70 name one of 15 library constants), condition on hit 94, projectile effect 50, constant-tick
DoT (`addDamage`) 37, attribute condition 30, paralyze formula 24, chain value 8. The tiers
come from a pattern match on each `onCastSpell` body; converting a specific monster still
requires reading its scripts, as for creature events.

### 8.3 Implemented extensions and conversion

The schema now carries the D11/D12 pieces: `area.matrix` (rows of `.`/`x`/`c`/`C`, authored
facing north with an optional north-west `diagonal`, rotated as `AreaCombat::getArea` does), a
`fixed` DoT profile with explicit `fixed_ticks` and `first_tick` `immediate`/`after_interval`
(`Condition:addDamage` and `CONDITION_PARAM_DELAYED`), `attribute_modifiers`
(`percent_of_base`/`add`), `Ability.variants` (uniform pick), `Ability.chain`, the Formula kind
`caster_magnitude` and the schedule fields `magnitude` and `range_tiles`. Spell decision S17
(`OTERYN_SPELL_AUTHORING_SCHEMA_V1.md` §5) adds the condition fields `light` (`level`, `color`), `regeneration`
(`health_gain`/`health_interval_ms`, `mana_gain`/`mana_interval_ms`) and `buff_spell`.

`spell_scripts.py` evaluates a registered spell in a stubbed sandbox, calls its `onCastSpell` with
a stub caster and records which Combat ran; every value of a small `math.random` range is tried,
so a P3 pick becomes explicit variants. `canary_batch.py` turns P1-P3 spells into one shared
Ability per spell name (`canary:ability/spell/<name>`), with the monster's `minDamage/maxDamage`
as the schedule `magnitude`. Variants that convert to the same Ability collapse into one. A
parameter or condition type that is not a Canary engine constant is treated as nil, as Lua does.
P4 and NOOP spells stay unresolved (D13) or are omitted (D14).

Canary facts found while converting: `CONDITION_PARAM_SKILL_DEFENSEPERCENT` is not an engine
constant, so the "skill reducer" spells that use it (war golem and others) change no skill in
Canary; legacy TFS paralyze formulas with negative factors are read by Canary's
`ConditionSpeed` as the new speed and clamp to speed 40. Both are converted as Canary behaves and
noted in the manifest, pending the reference-date wiki comparison.

Batch 2 impact (before the conversion): `war_golem` needed the first four D12 extensions (`war golem electrify` is P2
with a constant-tick energy condition; `war golem skill reducer` is P3 with attribute
conditions). `knight_familiar` needs D11 (`sudden death rune` resolves to the rune, not the
conjuring spell; `ice strike` is P1); its per-player familiar look is a separate gap.

### 8.4 Native behaviour patterns for custom spell logic (D13 proposal)

`samples/p4-behaviour-patterns-canary-47dfd51f.json` groups the 93 registered spell scripts that
still block a converted monster into 19 shared, parameterized behaviours, each spell with its
parameters and evidence lines (model-assisted read; four findings re-read by hand). By blocked
monster references the largest are `conditional_summon` (23 spells), `remove_magic_walls` (2 spells,
20 references), `heal_allies_in_area` (14), `path_trail_missile` (3 spells, 17 references: a drawn
effect trail plus one single-target hit, not a real chain) and `plain_combat_unsupported_schema`
(9 spells: plain combats blocked only by `BLOCKARMOR`, `DISPEL`, `COMBAT_LIFEDRAINDAMAGE` or a
custom area constant, which need schema fields rather than a behaviour; `BLOCKARMOR` is now covered
by `mitigated_by` and `DISPEL` by `remove_condition` (D19, §8.5); the two `COMBAT_LIFEDRAINDAMAGE`
waves and `COMBAT_PHYSICALDAMAGEDAMAGE` are undefined constants, which the engine reads as 0,
physical damage). Boss-specific logic
(`boss_form_swap`, `boss_escape_utility`, `map_or_quest_specific`) belongs with the Encounter
definitions of D9. Canary defects found on the way: `gorerilla small ring` uses the undefined
`COMBAT_PHYSICALDAMAGEDAMAGE`, `metal gargoyle curse` has a one-step loop, `icicle heal` deals 100
damage, and `gaz'haragoth summon` calls `setSummon` with an undefined value. The grouping is a
proposal for owner review; no behaviour key is created by it. After D18 the converter tags each
blocking manifest row with its pattern and `population_census.py` groups the blockers by pattern;
the rows stay unresolved until a pattern has an accepted parameter contract and runtime owner.

### 8.5 Plain combat fields (D19): `mitigated_by`, `remove_condition`, engine parameter binding

A damage Effect may carry `mitigated_by: ["armor"]`, `["shield"]` or both: the target defences that
reduce it. Absent means neither, which is the engine default for spells. The field records only
whether a defence applies; the reduction formula is a world combat rule (like D12(6)). Canary
`47dfd51f` and Crystal Server `37d0d54` behave identically for monsters:
`monsters.cpp deserializeSpell` sets `COMBAT_PARAM_BLOCKARMOR` and `BLOCKSHIELD` on every monster
melee and `BLOCKARMOR` on an inline physical `combat`; a registered spell sets them itself (28
Canary scripts, e.g. `berserk.lua`, `explosion.lua`). `Combat::setParam` maps them to
`blockedByArmor`/`blockedByShield` and `Creature::blockHit` subtracts a random share of the target's
defense and armor (Crystal additionally lowers the armor by a player's weapon proficiency, which
monsters do not have). The converter now emits the field from these three rules; before it, monster
melee and physical attacks lost the armor and shield reduction silently.

`COMBAT_PARAM_DISPEL` becomes a `remove_condition` Effect after the combat's damage or heal
(`CombatDispelFunc` runs after the health change or on a combat without damage), for example
`ultimate healing` (heal, then remove paralysis) and `djinn cancel invisibility`. `setParameter`
keys and values are bound as the engine reads them: `luaCombatSetParameter` reads both as numbers,
so an undefined global (nil) is 0 and a constant of another enum selects by its number. This turns
`COMBAT_LIFEDRAINDAMAGE` (undefined) into `COMBAT_PHYSICALDAMAGE` (0), and `COMBAT_PARAM_SHOOT_EFFECT`
(undefined) in `targetfirering` into a `COMBAT_PARAM_TYPE` call whose value `CONST_ANI_FIRE` (4) is
`COMBAT_UNDEFINEDDAMAGE`; the converter had read that spell as fire damage and now leaves it
unresolved with the other undefined-damage entries.

### 8.6 Data forms for three behaviour patterns (D18)

Three D18 patterns are plain data, not native code:

- `summon_creature` (`conditional_summon`): creatures, `count_mode` `fill_to_limit` or `fixed`,
  `count`, `only_below_summons`, `owned` (the caster becomes master) and `max_offset_tiles`.
- `remove_items` (`remove_magic_walls`): items in priority order and a `selection` rule over the
  Ability area.
- `affects` on a damage/heal Effect (`heal_allies_in_area`): `masterless_monsters`,
  `non_player_side`, `player_side` or `named_creatures`, plus `top_creature_only`,
  `excludes_caster_name` and `includes_caster` (`combat.cpp CombatFunc` passes the caster only to a
  non-aggressive combat).

The converter does not read these parameters from the model-assisted grouping. `spell_probes.py` runs
each script against stub worlds (casters with 0-15 summons, players, player and monster summons,
masterless and named monsters, tiles with and without the listed items, low and high random rolls)
and records what it does; the parameters are derived only when the recorded behaviour matches the
data form exactly. Everything else stays unresolved with the reason: summons on fixed map
coordinates or without a limit (`plagirath`, `razzagorn`, `tenebris`: Encounter work under D18), a
value rolled once while the script loads (`minotaur cult prophet mass healing`), engine item
constants the stubs do not model (`destroy magic walls`) and scripts that need more world API.
This resolves 21 more monsters.

`path_trail_missile` (`singlecloudchain`, `singledeathchain`, `singleicechain`) is one exact
template: `Position:getPathTo(target, 0, 0, true, clearSight, 8)` must find a path or the cast
fails, the trail effect is sent on every path tile, then one combat hits the target. It is authored
as a single-target Ability with `path_requirement` (search distance, clear sight) and the
presentation `path_asset_binding`; the "chain" in the names hits only the target.

The players-only chain picker (`poison chain`, `podzillaphyschain`) is one exact template too. A
`CALLBACK_PARAM_CHAINPICKER` whose body keeps a target only when it is a player outside a protection
zone is authored as `Ability.chain.target_filter: players`. A monster caster already cannot hit a
player in a protection zone (`combat.cpp` `canDoCombat`), so the only effect of the filter is that the
chain skips other creatures, such as player summons. Any other picker body stays unresolved. This
resolves Quara Looter, Rootthing Bug Tracker, Mould Phantom and Rotten Golem.

`Ability.chain` (D12) gained optional fields for player spells (spell rule S23, owner 2026-09-28; behaviour and
sources in `OTERYN_SPELL_CHAIN_BEHAVIOUR_CANDIDATE_V1.md` §3-§5):
- `shape`: `sequential` (default; each jump starts at the last creature hit) or `fork` (every further creature is
  chosen within `range_tiles` of the first creature);
- `initial_range_tiles`: search radius for the first creature of a cast without a target; absent means `range_tiles`;
- `damage_step_percent`: integer; creature at step `i` gets `(100 + i * damage_step_percent)%` of the rolled value,
  at least 0; absent means 0;
- `target_filter` gained `ranged_monsters` (monsters that are not summons, not reward bosses and prefer a target
  distance above 1), next to `players`.

`max_targets` counts the further creatures after the first, so one cast hits at most `1 + max_targets` creatures.
This is what the stored Canary value already means (Canary hits the returned value plus one), so no monster data
changes; the D12 wording "up to `max_targets` creatures" was one short. No monster uses the new fields, and the
creature admission does not accept them yet.

Fork import rule: a Canary fork chain value (`pickForkTargets`, a chain value callback that returns the fork flag)
is the total number of creatures, the first included, so the importer stores it minus 1. `max_targets` always
counts the further creatures, for both shapes (Forked Glacier: Canary 7, `max_targets` 6). Equal distances go to
the lowest creature id in both shapes (chain §7 Q1 proposal, until the owner's in-game test).

Step rounding: each creature hit draws its own value (Canary `doCombat` per chain target), an integer after the
truncated formula bounds. Its step value is `value * max(0, 100 + i * damage_step_percent) / 100` in exact integer
arithmetic, rounded half away from zero (Canary `std::round`), before any mitigation. It equals Canary's double
computation for every step Chained Penance reaches (−5%, steps 0-5 with a Wheel +1); Canary's doubles differ only
on some exact halves beyond, which are floating-point artefacts. Example: 101 at step 1 of −5% gives 95.95, so 96.

`area_damage_named_target` is probed like `heal_allies_in_area`. A `CALLBACK_PARAM_TARGETTILE` callback that, on each
tile of the ability area, takes a fixed or rolled amount of health from the top creature through `Creature:addHealth`
is authored as an extra `damage` effect of damage type `untyped`:
- the damage bypasses every resistance, mitigation and element, as `addHealth` does in Canary;
- it affects `players` (players only, not their summons) or `named_creatures` (the monsters the script names), with
  `top_creature_only`;
- there is one effect per target group and amount, next to the ability's own combat.

A callback that heals, changes health more than once, hits unnamed monsters, or hits depending on a roll stays
unresolved. This resolves:
- Freed Soul;
- Ravenous Lava Lurker;
- The Corruptor of Souls;
- The Remorseless Corruptor;
- The Source of Corruption.

For example, `remorseless wave` takes 0-600 health from players and 700-1,500 from a stolen soul.

### 8.7 Owner decisions D21-D25 in the converter

- D21: the 393 random variants of `ghastly dragon curse` collapse into one `geometric` damage over
  time (base 40-170, factor 6/5, 5-7 ticks, 4 s) only after every variant is checked against the
  model (ticks computed by repeated double multiplication and truncated as `Condition:addDamage`
  does) and every (base, tick count) pair occurs once. `metal gargoyle curse` calls
  `math.random(2.32, 2.32)`, which LuaJIT returns as 2.32, so it is one fixed 24-tick schedule.
- D22/D23/D25 loot: an item page listing several ids splits the estimate evenly; a disambiguation
  page resolves through the variant whose `droppedby` names the monster, otherwise all variant ids
  share it; an item page id that differs from the Canary name match decides; a row reached through
  a disambiguation page is not added twice; no statistics or zero drops keep the Canary list.
- D24: `lookType` 0 without `lookTypeEx` (the wild magic traps) is `appearance.selection:
  invisible`, which forbids an asset.
- D25 damage: an inline combat with an undefined type takes the element of the one wiki ability it
  matches best by effect, missile, shape and maximum (score at least 3, no tie); the wiki abilities
  of such monsters are recorded in the population wiki file. `grimeleech` attack 3, `angry sugar
  fairy` and the shared `targetfirering` script stay unresolved because no single wiki ability
  matches. An inline `effect`/`strength` entry without any visual has no effect and is omitted (D14).

## 9. Import readiness of the Canary population

`population_census.py` converts every Canary `47dfd51f` monster file in memory, applies the D15
wiki values of §9.1 and records the result in `samples/population-canary-47dfd51f.json`: of 1,656
files, 1,548 convert, validate and resolve every manifest row (1,103 before registered spells were
converted, 1,315 before wiki adoption, 1,298 before D19, 1,308 before the probed D18 patterns, 1,329
before the two rules below, 1,345 before `path_requirement`, 1,350 before D21-D25, 1,377 before the first Encounter, 1,383 before the Dream Courts, Forgotten Knowledge and Ferumbras
Ascension encounters, 1,389 before the Cults of Tibia, Wrath of the Emperor, Rathleton and other
encounters, 1,427 before the fourth encounter slice, 1,442 before the callback rule below, 1,453 before the D29 encounter slice, 1,463 before the sixth encounter slice, 1,478 before the seventh, 1,486 before the eighth, 1,490 before the ninth, 1,498 before the tenth, 1,505 before the eleventh, 1,511 before the twelfth, 1,517 before the thirteenth, 1,523 before the fourteenth, 1,533 before the fifteenth, 1,539 before the players-only chain picker below, 1,543 before the tile damage below);
102 are blocked; 6 do not convert (five Soul War bosses
need quest configuration at load and one file is a helper library, not a monster). No bundle fails
structure validation.

Inline `mType.on*` callbacks: 47 monster files define 115. A callback whose body is empty
(`function(...) end`) has no effect and is an approved omission. The reward-boss template
`onAppear` (`if monster:getType():isRewardBoss() then monster:setReward(true) end`) repeats the
reward registration that `data/scripts/lib/register_monster_type.lua` already attaches to
`onSpawn` for every `rewardBoss` type; reward tracking belongs to the reward domain (D27), so it is
an approved omission too. Every other callback stays unresolved.

Converter rules transcribed from the engine for this result (all recorded in `sources.json`
`rules` and in the manifest rows): a chance above 100 behaves as 100; `changeTarget.interval=0`
disables timed target changes while a nonzero chance still retargets onto a blocking opponent
(`change_target.interval_ms` may be 0); a zero-total damage condition never starts; a condition
total drawn from a range may be 0 (`total_damage_range.minimum` may be 0); more than 100% element
reduction equals 100%; `runHealth` above `maxHealth` equals `maxHealth`; summon counts are capped
by `maxSummons`; an undefined Lua constant is nil; a numeric or wrong-enum `effect`/`shootEffect`
selects that numeric id; items with `duration=0` do not decay; `lookAddons`/`lookMount` map to
attachment bindings; a missing description is the monster name. `RegisterPrimalPackBeast(monster)`
after registration (11 monsters) registers a separate derived type "<name> (Primal)" and leaves the
monster unchanged, so it is an approved omission (the derived Primal types are not generated yet). A
Bestiary without a valid race (5 monsters) takes its taxonomy from its own class when the
reference-date wiki `bestiaryclass` agrees (D15).

Creature events come from `samples/events-canary-47dfd51f.json`: 193 events named by monster
files, each classified by a model-assisted read of its registering script with evidence lines
(five classifications re-read by hand). Only high-confidence `quest`, `encounter_bookkeeping` and
`no_effect` events are omitted (D6, D9); `encounter_mechanic` (125) and `monster_behavior` (2)
stay unresolved.

Remaining blockers by affected monsters: encounter-mechanic events (215; 168 monsters are blocked by
events alone), inline `mType` callbacks (up to 28 per callback kind), custom spell logic grouped by
D18 pattern (largest: `conditional_summon` on fixed map positions 14,
`delayed_telegraphed_nuke` 8, `escalating_dot_curse` 6, `area_damage_named_target` 6), wiki loot that
names no single Canary item (14 list `giant shimmering pearl`, which is two items, 281 green and 282
brown), wiki loot without a Loot Statistics page (5) and the invisible wild magic traps without a look
type (4). These need Encounter definitions (D9), native behaviour work, an item decision or a
presentation decision; none is solved by relaxing validation.

Population bundles are not committed (about 67 MB). `population_census.py --bundles DIR` writes
the four files of each fully resolved monster under `DIR`, and
`samples/population-bundles-canary-47dfd51f.json` records one SHA-256 per bundle (name, byte
length and content of the four files in order) together with the SHA-256 of the pinned wiki
reference, so a regenerated population is checked with `git diff --exit-code` on the index.

### 9.1 Population wiki comparison

`samples/wiki-population-2026-09-27.json` (`wiki_compare.py --population`) compares the plain
Canary conversion of every convertible monster with its TibiaWiki (Fandom) page at the
2026-09-27 target date (D33): 1,569 monsters compared and 81 without a page under the Canary name. Over the
infobox facts: 24,162 MATCH, 2,491 DIFF, 5,819 uncertain on the wiki (`?` or `~`, mostly
`100%?` element modifiers), 63 unparsed and 7,829 unknown. The most frequent differences are
mitigation (748 monsters), the loot item list (649), flee health (122), experience (110),
paralysis immunity (86), `pushobjects` (65), health (65) and element modifiers (about 30-80 each).
A Canary damage immunity counts as a 100% element modifier. Over 14,894 Canary loot entries: 7,004
inside the 95% interval of the wiki estimate, 5,182 outside it, 2,212 not observed in the
highest-version statistics block, 490 split over several Canary entries and 6 with invalid wiki
counts. Against the 2026-07-28 cut, 932 of the 3,621 cached pages have a newer revision at 2026-09-27. In
wave A this changes 599 Loot records, mostly probabilities, because the Loot Statistics pages keep growing.
It also changes 14 Creature profiles, each following a page revision from 2026-08-16 to 2026-09-24:
- the experience of the five apparitions;
- mitigation or element modifiers of Lion Knight, Minotaur Idol, Tremor Worm, Betrayed Wraith, Elephant,
  Nighthunter, Retching Horror and Timira the Many-Headed;
- a condition immunity of Latrivan.

The 2026-07-28 comparisons (`wiki-population-2026-07-28.json`, `wiki-scenes-2026-07-28.json` and the batch's
`wiki-2026-07-28.json`) stay as the record of the earlier admission.

The converter adopts only DIFF rows (D15): health, experience, armor, mitigation, element
modifiers, `pushable`, `pushobjects`, `senseinvis`, paralysis immunity, `illusionable`, flee health,
Bestiary difficulty/occurrence and the summon/convince costs (`--` clears the flag, a number sets it and
the one mana cost; two different costs cannot be expressed and stay unadopted); speed (the wiki lists
observed speed, Canary the engine value) and the Bestiary class are not adopted. Every adopted value keeps the superseded
Canary row as an `approved_omission` and adds a MediaWiki-sourced row. Wiki loot missing in Canary
is added only when its name resolves to one item: by name, by the item page `itemid`, or by
dropping the equipped state of an `items.xml` `transformEquipTo` pair. Over all converted
monsters 748 mitigations and 12,722 loot rows (probabilities, counts and added items) are adopted; 1,188 of
the fully resolved monsters carry at least one adopted value. Under D43, TibiaWiki BR gives the health of 65
and the experience of 37 fully resolved monsters whose Fandom value is missing or uncertain; 124 monsters have a BR
fill in `wiki-br-fill-2026-09-27.json`; the other fills equal Canary or belong to monsters that are not fully resolved.
Under D47, `official-library-2026-09-28.json` gives the Tibia.com library health and experience of 647 Canary
monsters; among the admitted creatures only Sabretooth differs (experience 11,931 becomes 12,830), and the four Darklight
Core creatures, still deferred for their encounter, take the 2026-08-25 balance (-7% experience) that the wiki misses.
Under D32, 971 loot count bounds (a minimum or a maximum) of 334 fully resolved monsters take the
observed wiki range, and Duke Krule's twelve `minCount` 0 entries become count 1..max.

Under D44, `wiki_authored.py` adds Dark Merudri, which Tibia has at the target date and Canary lacks. Its health, experience, flags, immunities and corpse (item 50311) come from Fandom; its three attacks, element modifiers and ignored fields come from TibiaWiki BR. Its look (the male Monk outfit 1824 in the Dark Knight colours), speed, armor, defense, attack intervals, chances and effects follow the Dark Knight template and are marked NEEDS VERIFICATION. The BR earth wave without damage is left out. The census reports it as `wiki_authored` next to the 1,548 converted monsters.

### 9.2 Wiki ability scenes

`samples/wiki-scenes-2026-09-27.json` (`wiki_scenes.py`) compares the ability scenes of the
creature pages at the cut with the plain Canary conversion; nothing is adopted. A scene names a
shape of `Module:SceneBuilder/data` (2 caster, 3 target, 1 hit tile) and effect/missile pages whose
`effectid`/`missileid` is the client id. Canary areas are rebuilt with the engine's `AreaCombat`
rules (radius table, length/spread cone, matrix; a directional area is anchored on the caster, a
targeted one on its target, a combat without area always hits its target, and a missing effect shows
the damage type's default hit effect from `Game::combatGetTypeInfo`). Hit sets are compared relative
to the anchor under the four rotations.

Over 1,031 monsters with an ability list: 1,990 wiki abilities have no scene; of those with a scene,
1,065 match a Canary Ability and 411 do not. Among matched abilities the shape is equal in 749 and
differs in 297 (mostly beam/wave lengths and single-target versus area), the effect id is equal in
665 (64 of them through the default hit effect; physical damage on a player shows blood) and
differs in 278, and the missile id is equal in 293 and differs in 160. The scenes are drawings, so
they are review evidence for a later owner decision, not an automatic D15 adoption (D18).

## 10. Soul War group: monster-side extensions (SW-1, SW-2)

- Status: **ACCEPTED** 2026-09-30 (owner answers in §13.6 of the encounter format). Nothing here is implemented yet.
- Companion: §13 of `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` holds the encounter-side mechanics of the same group
  (SW-3..5), the owner questions (Q1-Q6) and the decision test for all five extensions.
- Scope: the two registered spells that block Soul War hunting monsters, `soulwars fear` and `destroy magic walls`.
- Evidence: Canary `47dfd51f` (paths below relative to that checkout); TibiaWiki at the 2026-09-27 cut (D15, D33).
  The census numbers were re-derived by converting every monster file in memory with the `population_census.py` rules
  and listing each monster's open manifest rows.

| Spell | Census monsters | Extension |
|---|---|---|
| `soulwars fear` (D18 pattern `fear`) | 4: Bony Sea Devil, Turbulent Elemental, Goshnar's Spite, Hazardous Phantom | SW-1: `Ability.windup` |
| `destroy magic walls` (pattern `remove_magic_walls`) | 5: Bony Sea Devil, Brachiodemon, Cloak of Terror, Many Faces, The Monster | SW-2: none; the existing `remove_items` `top_item_first_tile` plus two converter fixes |

The plain `fear` spell (`data-otservbr-global/scripts/spells/monster/fear.lua:1-22`, one combat with a 3 s
`CONDITION_FEARED`) already converts: Fungosaurus, Shrieking Cry-Stal and Doctor Marrow carry a `feared` condition
Effect. So the condition itself exists; only the windup is missing.

### 10.1 `soulwars fear`: SW-1, `Ability.windup`

1. **Mechanic.** The caster shows a smoke effect, and 2 s later its target is feared for 3 s.
2. **Canary behaviour** (`data-otservbr-global/scripts/spells/monster/soulwars_fear.lua`):
   - Lines 1-6: one combat with the blue ghost effect and a `CONDITION_FEARED` of 3,000 ms.
   - Lines 18-22: the cast sends `CONST_ME_GHOST_SMOKE` on the caster and schedules the combat 2,000 ms later with
     the cast's target variant.
   - Lines 10-16: the delayed combat runs only if the caster still exists. It hits the target taken at the cast. Range
     and sight are checked only at the cast (`blockWalls`, `needTarget`, lines 26-27).
   - The engine's fear rules apply as for every `feared` condition: a player immune to fear or already feared is not
     feared again, and only (party members + 5) / 5 members of a party can be feared at once
     (`src/creatures/combat/combat.cpp:1003-1029`, called at 1074).
   - Users: `bony_sea_devil.lua:113` and `turbulent_elemental.lua:108` (interval 2 s, chance 1), `goshnars_spite.lua:105`
     (chance 10), `undeads/hazardous_phantom.lua:95` (chance 2); Goshnar's Megalomania green and purple also cast it but
     do not convert (quest configuration at load, §9).
   - The reference-date wiki confirms the two effects and the delay: `Feared` (revision 1149698) says 2-3 s, and
     `Soul War Quest/Spoiler` (revision 1134584, Ebb and Flow section) says 2 s. `Feared` adds: "If the creature
     changes targets between these two effects, the new target will be feared." That differs from Canary (Q3 of
     §13.6).
3. **Extension SW-1.** An Ability may carry `windup: {delay_ms, caster_asset_binding}`.
   - At the cast the usual checks run and the caster binding is shown on the caster's tile. After `delay_ms` the
     Ability's effects run if the caster still exists. Which creature they hit follows the owner answer to Q3: the
     target taken at the cast (Canary) or the caster's target at that moment (wiki); with no such target nothing
     happens.
   - Allowed only on a single-target Ability (`needs_target: true`, no `area`, `variants`, `chain` or `encounter`).
     That is all `soulwars fear` needs; a windup over an area is not decided here.
   - The converter matches the script as one exact template, like `path_trail_missile` (§8.6): a caster
     `sendMagicEffect`, then `addEvent(f, N, creature:getId(), var)` where `f` runs one Combat only if the caster
     exists. The Combat converts as a P2 spell. Any other body stays unresolved.
   - Fear movement (runtime obligation, from Gudii 3/6 and the wiki): a feared player never moves onto a field or
     into a teleport; this belongs to the world rule for every `feared` condition, not to the Ability.
   - Declined: a native `fear` behaviour (D13). The delay is the only non-data part, and it is plain data.
4. **Authored JSON** (the shared Ability `canary:ability/spell/soulwars_fear`):
   ```json
   {"identity": {"key": "canary:ability/spell/soulwars_fear", "revision": "canary-47dfd51f"},
    "kind": "spell", "range_tiles": 0, "needs_target": true, "needs_direction": false,
    "windup": {"delay_ms": 2000, "caster_asset_binding": "canary.appearance:effect/ghost_smoke"},
    "effects": [{"family": "Effect", "key": "canary:ability/spell/soulwars_fear/effect-condition-1", "revision": "canary-47dfd51f"}]}
   ```
   The Effect is the one `fear` already produces: `condition` `feared`, `fixed_duration`, 3,000 ms, impact
   `canary.appearance:effect/blue_ghost`.
5. **Validation and tests.**
   - Schema (`build_formal_schema.py`): optional `windup` with `delay_ms` (integer, at least 1) and
     `caster_asset_binding`. Semantic (`validate_monster.py`): `windup` only with `needs_target: true` and without
     `area`, `variants`, `chain` and `encounter`.
   - `verify_formal_schema.py`: one positive case and negative cases for a zero delay and for `windup` with `area`.
   - `canary_batch.py`: the template match, and a negative check that a changed body stays unresolved.
   - Rust: the Ability profile mirrors `windup`, with a focused positive and negative admission test.
6. **Out of scope.** The fear runtime and the party rule (a world combat rule for every `feared` condition); which
   monsters cast fear at all (Q4 of §13.6).

### 10.2 `destroy magic walls`: SW-2, no schema extension

1. **Mechanic.** The caster removes one magic wall or wild growth within two tiles.
2. **Canary behaviour** (`data-otservbr-global/scripts/spells/monster/destroy_magic_walls.lua`):
   - Lines 1-6: the ids are the engine constants `ITEM_MAGICWALL_SAFE`, `ITEM_MAGICWALL`, `ITEM_WILDGROWTH_SAFE` and
     `ITEM_WILDGROWTH`: 10181, 2128, 10182 and 2130 (`src/utils/utils_definitions.hpp:596-602`).
   - Lines 11-24: the 5x5 square around the caster is scanned column by column, west to east, each column north to
     south. On the first tile whose top visible thing is one of the ids, that item is removed and the cast ends
     (line 20).
   - `Tile::getTopVisibleThing` returns a visible creature first, then the items (`src/items/tile.cpp:438-458`). A
     creature cannot stand on these walls, so in practice the tile holds the wall.
   - Line 19 sends `CONST_ME_POFF` on the caster's tile, not on the removed wall. The `spell:cooldown` of line 31 binds
     player casters only; a monster casts it by its own entry (interval 1 s, chance 30 for all four Soul War users;
     `the_monster.lua:91`, chance 50).
   - The wiki lists the wall breaker for Brachiodemon, Branchy Crawler, Cloak of Terror and Many Faces (Many Faces in
     its behaviour text). Canary gives it to Bony Sea Devil, not to Branchy Crawler (Q4).
3. **Extension.** None. D18 already has the data form: `remove_items` with `selection: top_item_first_tile`, and the
   Rust profile has `TopItemFirstTile` (`apps/game-server/src/content/project/v2/creature.rs`). No admitted Ability
   uses that selection yet. Two converter gaps keep the spell unresolved today (§8.6: "engine item constants the stubs
   do not model"):
   - the probe world gets the four `ITEM_*` constants with their engine values;
   - `probe_remove_items` gets a second branch for `top_item_first_tile`: the stub's top visible item is set on one
     tile at a time, and the branch requires one removal on exactly that tile, then no further tile read. Only then is
     the Effect emitted.
   - The schema text of `top_item_first_tile` says "the first tile" without an order. This design fixes the order as
     Canary's scan: columns west to east, each column north to south, over the rows of the Ability area. It is a
     clarification for a value no content uses yet.
4. **Authored JSON** (the Effect of the shared Ability `canary:ability/spell/destroy_magic_walls`):
   ```json
   {"identity": {"key": "canary:ability/spell/destroy_magic_walls/effect-remove", "revision": "canary-47dfd51f"},
    "operation": "remove_items",
    "removed_items": {"items": [{"family": "Item", "key": "canary:item/10181", "revision": "canary-47dfd51f"},
                                {"family": "Item", "key": "canary:item/2128", "revision": "canary-47dfd51f"},
                                {"family": "Item", "key": "canary:item/10182", "revision": "canary-47dfd51f"},
                                {"family": "Item", "key": "canary:item/2130", "revision": "canary-47dfd51f"}],
                      "selection": "top_item_first_tile"},
    "presentation": {"impact_asset_binding": "canary.appearance:effect/poff"}}
   ```
   The Ability has `area.matrix.north` of five rows `xxxxx` with the centre `C`, `range_tiles` 0 and no target. By the
   existing convention (`anomaly break`) the impact binding shows on the removed wall's tile; Canary shows it on the
   caster's. That is presentation only, and Q4 covers it.
5. **Validation and tests.** No schema change. `canary_batch.py` gains the constants and the probe branch, with a
   negative probe (a script that removes more than one item) that stays unresolved. The schema description of
   `top_item_first_tile` gains the scan order. The census must show the five monsters losing this row and no other
   row changing.
6. **Out of scope.** `anomaly break`, which already converts as `first_listed_per_tile`; the magic wall and wild
   growth items themselves (Item domain).

### 10.3 Creatures without a Canary file: owner answers (2026-09-30)

- `samples/wiki-only-candidates-2026-09-30.json` lists the 64 client 15.30 creatures (`imports/cipsoft-staticdata`)
  with no Canary file and no admission. It is committed as evidence.
- 50 of them have a CrystalServer file at `00ce02a5` outside `summer_update_2026`. **They come through a widened
  `crystal_batch.py`, with the reference-date wiki applied over the Crystal values as for every Canary monster
  (answer c).** The remaining 14 are authored from the wiki (D44), which needs `wiki_authored.py` to take a table of
  creatures instead of one.
- **Ordinary monsters first (answer a).** The 23 quest, event and raid creatures wait for their encounters.
