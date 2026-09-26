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
| D1 | Chances stay JSON numbers in **0–100 percent** (`0.19` = 0.19%) with at most 4 decimal places. The structural schema carries only the range; the 4-decimal rule is enforced by `validate_monster.py`. Native ppm = `percent × 10000` and must be an exact integer. | Keeps the owner-selected readable format. Removing `multipleOf: 0.0001` lets any binary-float JSON Schema validator (editors, CI) accept valid values such as `0.29` and `4.93`, which it previously rejected. A Rust importer reading `f64` must compute `round(value × 10000)` and reject the value when it differs from the unrounded product by more than `1e-6`. |
| D2 | Ratios (`resistances`, `mitigation_percent`, reflection/healing, speed multipliers) must be in **lowest terms**; zero is `0/1`. | Matches `validate_v2_ratio` in WorldProject/v2, so a valid authoring ratio cannot be rejected on admission. |
| D3 | Durations and intervals are **integer milliseconds** everywhere. | One unit across the schema. WorldProject/v2 `ProjectV2FamiliarProfile.duration_seconds` is converted on admission (see §5); a value not divisible by 1000 is a mapping gap, not a reason to change the authoring unit. |
| D4 | Health keeps **`max_health` and `initial_health`** with `1 <= initial_health <= max_health`. | Both are distinct source facts. WorldProject/v2 currently has one `health`; see §5. Zero-HP helper entities remain blocked as `unresolved_semantics` rather than admitted with invented HP. |

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
- Loot entries are processed in authored order. With `skip_later_same_item_after_success`,
  a positive-quantity drop suppresses later entries for the exact same Item within that table
  invocation. `min_count` may be 0; `max_count >= 1`; `min_count <= max_count`.
- `$defs.item` is a capability projection used to verify corpse/container/decay facts; it is
  not Item authority. `ItemRef` must resolve to the canonical Item catalogue before admission.
- The disposition ledger accepts Git revisions only. Wiki captures need an immutable Git
  archive or an accepted non-Git provenance route; a Git hash is never invented for a wiki page.

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
| `bestiary.class`, `taxonomy`, `stars`, `locations` | — | GAP |
| `bosstiary.*` | `ProjectV2BosstiaryProfile` | MATCH |
| `summoning.familiar.duration_ms` | `familiar.duration_seconds` | MATCH via `/1000` (D3) |
| `summoning.familiar.vocation`, `summon_ability`, `mana_cost`, `owner_speed_bonus` | `ProjectV2FamiliarProfile` | MATCH |
| `loot.entries[].probability_percent` | `LootEntryDocument.probability_ppm` (`content/project.rs`); v2 has the `Loot` family but no Loot profile | MATCH via D1 |
| flags, targeting, voices, summons, presentation, spawn/system eligibility, reflection/healing | — | GAP |

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
python verify_formal_schema.py     # 117 focused positive/negative cases
python verify_source_coverage.py   # 242 inventoried Canary/Crystal registrar/spell paths accounted for
python validate_monster.py <monster.json> <dependencies.json> [--catalog C] [--manifest M]
```

`build_formal_schema.py` is the source of the schemas; edit it, not the generated JSON.
Validator success means authoring structure and declared-reference closure only; it reports
`runtime_qualified=false` and `source_coverage_proven=false`.
