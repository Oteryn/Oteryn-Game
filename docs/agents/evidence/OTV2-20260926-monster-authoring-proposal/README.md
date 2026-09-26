# Monster authoring schema — proposal v2

Retained on `codex/monster-authoring-schema-20260926`, outside `main`.
This is a static authoring proposal with executable validation, not an accepted
WorldProject/v2 replacement, implemented Game importer or runtime activation.

## Files

| File | Purpose |
|---|---|
| [monster.schema.json](monster.schema.json) | Creature, Behavior, Presentation and base Loot types. |
| [monster-target-template.proposal.json](monster-target-template.proposal.json) | Empty field template; placeholders are deliberately invalid as ready data. |
| [monster-dependencies.schema.json](monster-dependencies.schema.json) | Ability, Effect, Formula, Document, canonical Item capability projections and nested Loot. |
| [monster-dependencies-template.proposal.json](monster-dependencies-template.proposal.json) | Empty direct-dependency template. |
| [monster-import-readiness.schema.json](monster-import-readiness.schema.json) | Git-source field/disposition ledger, not a complete-source admission proof. |
| [validate_monster.py](validate_monster.py) | Exact-decimal structural and cross-field/reference validation. |
| [verify_formal_schema.py](verify_formal_schema.py) | 104 focused synthetic verification cases, including repair counterexamples. |
| [build_formal_schema.py](build_formal_schema.py) | Reproduces the three schemas and two templates. |
| [verify_source_coverage.py](verify_source_coverage.py), [field-census.json](field-census.json) | Reproduce mapping of 242 inventoried standard registrar/spell paths. |
| [normalize_monster_fields.py](normalize_monster_fields.py) | Bounded geometry and HP-helper classification for already decoded source values. No Lua execution. |
| [REPAIR_NOTES.md](REPAIR_NOTES.md) | Source evidence, corrected semantics and advisory review disposition. |

The existing REVIEW_NOTES.md and REVIEW_SUPPLEMENT_2026-09-26.md are preserved
historical advisory reviews. REPAIR_NOTES.md describes this successor's response.

## Units and corrected semantics

- Chances are JSON numbers **0–100 percent**, including fractions, in exact
  0.0001 percentage-point steps. 0.19 means 0.19%, not 19%. The supported validator
  reads Decimal; generic binary-float `multipleOf` checking can reject valid values.
  Native ppm conversion is an integration detail: multiply percent by 10000.
- Durations/intervals use positive integer milliseconds. A damage-schedule condition
  has no authored fixed duration. A fixed-duration condition requires duration_ms.
- DoT stores a nominal **total damage range**, tick interval, automatic/fixed first
  amount, decreasing profile and delayed first tick. It does not store an independent
  range for each tick or promise that schedule sum equals nominal budget.
- Ability needs_direction is explicit and independent of final area. Positive radius
  replaces source length geometry without erasing length-derived direction. For area
  casts, center precedence is target, facing-adjacent position, caster.
- presentation_only requires a visual binding and contains no fictitious damage.
- ignore_period_underground bypasses day/night restriction underground. It is not
  a general underground permission. Concrete placement and timers belong to Spawn.
- Movement pass_through is distinct from can_walk/pushable/push flags; resolve its
  value explicitly from native authoring/source rules rather than inferring it.
- summonable and convinceable share mana_cost; require it when either applies and
  forbid it when both are false. Familiar has a separate profile cost.
- Bestiary stars and locations retain source detail. Locations is original editorial
  text; taxonomy holds normalized classification. Original monster Notes belong in
  the referenced Document; source-language lore, voices and inspection are unchanged.
- min_count may be zero, max_count remains positive and min<=max. A resolved zero
  draw produces no Item. target_distance_tiles may be zero. Native adapter support
  for both must be proved; silently replacing zero with one changes source meaning.
- skip_later_same_item_after_success is explicit, with no implicit default. Process
  entries in authored order. Positive-quantity success marks the exact Item
  (family/key/revision) only when the flag is true. Later entries are then suppressed
  within the invocation; zero draws do not mark, and nested/separate calls have their
  own scope. This is not global item uniqueness. Foreign IDs must resolve consistently
  to canonical references. Conditional reward/loot modifiers retain their owners.
- Ordinary max/initial HP remain positive with initial<=max. health=maxHealth=0
  helpers are blocked pending owned entity/Interaction/Encounter resolution or an
  explicitly approved omission, not repaired by invented HP.
- Weight is a capability projection in integer centioz (0.01 oz); corpse/container/
  decay properties remain canonical Item-owned. The compact Item projection is not
  the full native Item schema; exact canonical ItemRef admission is separately required.

## Reproduce the checks

Use Python 3 with the versions in requirements.txt in an already prepared environment.
From this directory:

```powershell
python build_formal_schema.py
python verify_formal_schema.py
python verify_source_coverage.py
python validate_monster.py synthetic-valid-monster.json synthetic-valid-dependencies.json --catalog synthetic-catalog.json
```

The verification suite writes only local generated reports and synthetic fixtures.
Check generator output against the retained five JSON files; successful regeneration
alone does not prove equality. A publication receipt outside the final commit binds
byte equality and checks to its exact frozen remote SHA.

The validator checks declared exact references/assets, branch requirements, numeric
ordering, duplicate definitions, container capability, local cycles and supplied
manifest rows. Catalog identities/assets are declarations, not proof of canonical
runtime availability. CLI reports source_coverage_proven=false and
runtime_qualified=false. A resolved supplied manifest does not prove source census
completeness, script behavior, rights, runtime execution or Global Reference parity.

## Provenance and scope

The ledger currently accepts real Git repository revisions and file/line locators.
Wiki/Fandom/BR/Tibiopedia data stay in the collection dossier until an immutable Git
archive preserves original URL, page revision when available, capture timestamp,
digest and original content, or an accepted native non-Git provenance route is used.
Refer to the archive's actual commit and text lines; never fabricate a Git hash for a
wiki. A screenshot fragment cannot certify complete original prose and remains blocked.

Original static corpus: Canary 47dfd51f45280a59a1d3e50ba7edd573d7234446 and Crystal
ac447fef0935e6df52dc6b6376ae4c1534ecd73f. Repeat audit checked current Crystal
be61cdd3f12d197490cc50be1020f56cb1ff26bb: only two unrelated quest/party paths changed,
so the immutable monster census remains reusable. Native pass_through was rechecked
on Oteryn main de7c5499a2e6ad4ec533ea895754bdd82bda292e.

Schema IDs are proposal:2. The breaking field/lifetime changes require explicit
reauthoring of v1 source-resolved candidates. The 242 mappings cover inventoried
standard paths; arbitrary Lua scripts and runtime state are not species fields.
