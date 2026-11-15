# Global parity evidence tables

One table per gameplay area, comparing the live Global game with our content and code. Evidence only: a table grants no authority and changes no behaviour. Fixes land in separate commits that cite the row.

## Format

Each file has a header (area, date, source legend, confidence legend) and rows:

`| item | dimension | Global value | source (TW rev / measurement) | measured? | ours | ours file | status |`

Source legend:

- TW: TibiaWiki (tibia.fandom.com MediaWiki API), with the page revision id.
- Measurement: an owner-supplied observation on Global, with date and character setup.
- OTS donor (Crystal, Canary): hypothesis only, never Global evidence.

Confidence (the `measured?` column):

- MEASURED: observed on Global.
- WIKI: stated by TW at the given revision.
- REFERENCE: only an OTS donor or an inference.

Status: MATCH, MISMATCH, FIXED (was a mismatch, corrected in the cited commit), UNKNOWN (no Global source found), GAP (we lack the mechanic or its consumer).

When the Crystal re-audit (`docs/architecture/reviews/OTERYN_GAME_CRYSTAL_REAUDIT_2026-10-28.md`) marks a mechanic DONE with evidence, a row cites the audit row id instead of re-deriving it.

## Areas

- `healing.md`: healing spells, potions, monk mechanics, leech, regeneration. Machine rows: `healing.json`.
## Machine rows (`<area>.json`)

Next to each evidence table, one JSON file per area (`healing.json` first; damage, defence, progression and
regeneration/conditions follow). Test-only data: a lib test loads the file, runs each row
through the real owner and compares. Nothing in the server reads these files.

### Schema `OTERYN_PARITY_AREA/v1`

```json
{ "schema": "OTERYN_PARITY_AREA/v1", "area": "healing", "description": "...", "rows": [ Row ] }
```

A row:

| field | meaning |
|---|---|
| `id` | unique, stable |
| `case` | key the area adapter maps to an owner call (healing: `spell.cast`, `potion.use`) |
| `inputs` | adapter-specific scenario (vocation, level, ML, item, modifiers) |
| `status` | `Evidence`, `KnownGap` or `Pending` |
| `expected` | Global-measured facts (integers). Required for Evidence and KnownGap, absent for Pending |
| `current` | KnownGap only: the value the owner produces today, asserted exactly |
| `tolerance_percent` | optional, Evidence only: allowed deviation where the source samples the extremes (default 0, exact) |
| `source` | where the value comes from |
| `notes` | why, and what to do when it changes |

Status rules (`parity_rows.rs`, shared loader):

- `Evidence`: every `expected` fact equals the owner's (within `tolerance_percent`).
- `KnownGap`: Global evidence exists but content diverges. The owner must produce `current`
  exactly and must NOT match `expected`, so the row turns red when Content fixes the value; flip
  it to `Evidence` then. Never invent `expected`.
- `Pending`: no evidence. The owner path runs and is sanity-checked (`min <= max`, `mana > 0`
  unless a free rune cast); no value is asserted.

A summary test per area counts the rows by status and fails if the total drops below its
`MIN_ROWS` floor; raise the floor with the data.

### Adding an area

Add `<area>.json` and one adapter: a function that maps `row.case` and `row.inputs` to the real
owner and returns the observed facts (`BTreeMap<&str, i64>`), then calls `row.judge(&observed)`.
The healing adapters are `crates/gameplay/spell/src/spell/healing_parity_matrix_tests.rs`
(spell cast, magnitude owner) and
`crates/gameplay/dispatch/src/gameplay_transport/item_use_healing_parity_tests.rs` (potions,
because the item-use gate is private to dispatch). Both mount the one loader,
`crates/gameplay/spell/src/spell/parity_rows.rs`.

### Healing facts

`spell.cast`: `castable`, `mana`, `cooldown_ms`, `group_cooldown_ms`, `min`, `max` (final heal,
after the magnitude owner when the row sets `final`), `base_min`, `base_max` (formula bounds),
`harmony_spender`, `free_cast`. Inputs: `spell`, `vocation`, `level`, `ml`, `item_ml`,
`harmony`, `stance` (`sustain`), `final`, `wheel_flat_healing`, `item_healing_percent`,
`item_base_healing_permyriad`, `spell_name`.

`potion.use`: `admitted`, `health_min/max`, `mana_min/max`, `usable` (level and vocation
restriction). Inputs: `item`, `vocation`, `level`.

Not driven from data yet (rows stay `Pending`): Wheel spell augments and gems (the owners are
not reachable as a row input; `wheel_flat_healing` is the value they feed the magnitude owner),
the Wheel point-to-flat conversion, and the Mass Spirit Mend heal itself (native_combat plan).
