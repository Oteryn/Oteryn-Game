# Wheel of Destiny authoring schema

A closed JSON Schema 2020-12 and a populated, reproducible reference candidate for
W-R and GEM-R. It covers all five vocations, 180 slots, the 20 vocation/domain
Revelation assignments, 46 populated basic mods and 94 supreme mods. The candidate
is deliberately separate from runtime admission (`runtime_admitted: false`).

## Build and verify

With Python 3 and `requirements.txt` installed, run from this directory:

```sh
python build_schema.py
python wheel_authoring.py build --check
python wheel_authoring.py validate
python -m unittest discover -s . -q
python build_report.py
```

Open `wheel-comparison.html` for a standalone offline comparison by vocation and
search. It displays the candidate's structured data; it requires no web service.
`build` without `--check` writes `samples/wheel-candidate.json`. Validation never
writes files. Rebuilding needs only the committed source captures, without the
original JavaScript/WASM module or network access.

## Data and evidence

- `wheel.schema.json`: closed shapes, effect kinds and units.
- `samples/wheel-candidate.json`: the full populated candidate.
- `samples/source-wheel-reference.json`: observations from TibiaPal's planner API.
- `samples/source-graph.json`: the pinned Canary topology and mitigation constant.
- `samples/source-parameters.json`: typed stage values, gem policies and explicit
  corrections, using the pinned sources and the existing project decisions.
- `samples/planner-allocation-snapshots.json`: 180 legal allocations.
- `samples/planner-graph-comparison.json`: 1,080 observed unlock states with no
  differences during the tested fill sequences. This is bounded evidence, not
  an exhaustive proof of every possible allocation.

Each build records SHA-256 digests of its three input captures. Source commits:
TibiaPal `61ffa3e0502879ccec44e59ead859e92b6d88531`, Canary
`99902524e052f37574194466c2949c576e4ab269`, Crystal `summer-update`
`00ce02a57ca5a12e48f32a3476e37471167e4c3f`. Crystal's inspected Wheel/Gem modules
match its main branch at `96d13eff5a1afef17b11b9abc7d574020381cc53`.

No upstream C++ implementation or proprietary image files are redistributed.
The live TibiaPal site and requested Fandom pages could not be verified because
network access to those destinations was denied. Official facts already captured
in `docs/reference/tibia-manual/characters.md` and the project's spell evidence
are used with their existing provenance; this task does not claim a new wiki fetch.

## Wheel semantics

State slots 1–36 map to Canary enum positions and TibiaPal Q… tile IDs. Each domain
contains nine slices with a capacity of 1,000 points. Unlock prerequisites are
directed references to any full neighbouring slice, including cross-domain
neighbours. The allocation checker requires a path through full slices from an
inner slice and rejects isolated cycles, over-budget, below-minimum and
above-capacity allocations. Live level loss and strict decreases belong to W-1.

Dedication encodes health, mana and capacity as flat values and mitigation as an
increase to base mitigation. The exact mitigation increment is **0.075 percentage
points per promotion point**, pinned to Canary `io_wheel.cpp:19` and corroborated
by Crystal. Dividing a rounded planner output does not recover the exact constant.
WHEEL-0's older dedication-resistance wording remains a source conflict requiring
reconciliation before admission. Active planner slots contain no legacy elemental
Conviction resistance IDs; resistance trade-offs occur in the gem catalogue.

Conviction includes source identities, categories, full-slice values and typed
augment stages. Area references and Divine Dazzle's +4-second duration use the
existing spell evidence. These names are reference bindings, not newly admitted
Spell or WorldQuery identities. Crystal's declared Shield Slam damage-reduction
field has an upstream TODO; a declared number does not establish working combat.

Revelation has three stages at 250/500/1,000 domain points, typed numeric effects,
conditions retained in descriptions and the shared +4/+9/+20 damage/healing
bonuses. Avatar cooldowns are converted from minutes to seconds; Gift of Life
cooldowns count battle-sign time. Lord of Destruction's stage-2 death critical
bonus uses the project's corroborated **22.5%** value while preserving the
planner's **25.5%** description as an explicitly recorded conflict.

## Gem semantics

Every vocation binds its family and three existing item keys. Lesser/regular/
greater gems hold 1/2/2 basic mods and 0/0/1 supreme mods. Per-vocation slot lists,
the distinct-basic-mod-ID compatibility rule and all four grades are included.
Empty basic IDs 32, 42 and 43 are excluded. Signed basic penalties are retained.

Each domain binds three resonance slots. Activation order is basic 1, basic 2,
supreme. Quality matches at resonance 1/2/3 give +1/+1/+2 damage and healing.
Effective grades follow the preceding mods in the same gem: take the minimum of
self and preceding present mod grades. Grades belong to a character's mod type;
a type at Grade IV adds one promotion point. Cooldown supreme mods retain their
base cooldown reduction across grades and add Momentum chance at higher grades.

Atelier data includes clockwise domains (green/red/purple/blue), reveal/switch
fees, fragments, grade costs, initial eight gems and revealed/unrevealed yields.
Official manual yields override the conflicting OTS yields. Fees remain labelled
`OTS_HYPOTHESIS_ONLY` candidate values, as in WHEEL-GEM-0. This tool does not perform
economy transactions, revelation RNG, grade writes, vessel writes or loot changes.

Icons include the sprite URL, horizontal-square-cell layout and source index.
The index identifies a reference cell; asset availability, appearance and the
client asset crosswalk require client verification before UI delivery.

## Revisions and admission

An initial revision has no predecessor. Successors require the previous candidate;
`value_only` rejects changes to topology, perk identities, effect kinds/units and
other nonnumeric structure. `wheel_reset` describes the Wheel's revision policy,
not permission to erase paid gems: GEM-R admission separately requires the
WHEEL-GEM-0 §5.3 compatible mapping or staged migration.

The authoring package creates no runtime ruleset, protocol capability, Character
writer or native-key admission. Runtime owners must bind the effects and reference
areas, resolve the recorded parity conflicts, verify client icons and qualify
ruleset revision/migration behaviour before admitting it to a world.
