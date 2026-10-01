# Wheel of Destiny authoring schema

A closed JSON Schema 2020-12 and a populated, reproducible reference candidate for
W-R and GEM-R. It covers all five vocations, 180 slots, the 20 vocation/domain
Revelation assignments, 46 populated basic mods and 94 supreme mods. The candidate
is deliberately separate from runtime admission (`runtime_admitted: false`).

## Build and verify

With Python 3 and `requirements.txt` installed, run from this directory:

```sh
python build_schema.py --check
python wheel_authoring.py build --check
python wheel_authoring.py validate
python -m unittest discover -s . -q
python build_report.py --check
python verify_item_assets.py --check
node verify_planner.cjs /path/to/pinned/TibiaPal
```

Open `wheel-comparison.html` for a standalone offline comparison by vocation and
search. It displays the candidate's structured data offline. Optional icon previews load
directly from the original Tibia CDN; blocked or missing images show their IDs.
The viewer includes perk and gem icons without redistributing source image bytes.
`build` without `--check` writes `samples/wheel-candidate.json`. Validation never
writes files. Rebuilding needs only the committed source captures, without the
original JavaScript/WASM module or network access.
The default `validate` and `build --check` qualify the candidate's exact file hash,
coverage, corrections and evidence digests. Custom `--file` authoring validates
semantics only; supply `--evidence` to qualify that exact file. The CLI labels the
difference. Non-finite numbers are rejected when reading and validating JSON.

## Data and evidence

- `wheel.schema.json`: closed shapes, effect kinds and units.
- `samples/wheel-candidate.json`: the full populated candidate.
- `samples/source-wheel-reference.json`: observations from TibiaPal's planner API.
- `samples/source-graph.json`: the pinned Canary topology and mitigation constant.
- `samples/source-parameters.json`: typed stage values, gem policies and explicit
  corrections, using the pinned sources and the existing project decisions.
- `verify_planner.cjs`: replays all source observations against the pinned
  upstream WASM and string library; it checks every slot/perk, complete mod
  catalogues, vocation availability lists and five fully populated wheels.
- `verification.schema.json`: the closed shape of the verification record.
- `samples/verification-evidence.json`: coverage, source classifications, observed
  icon proof and exact remaining live-verification blocks.
- `samples/live-source-audit.json`: live Tavily source revisions/content hashes, HTTP
  observations, bounded official facts and per-conflict outcomes. No original
  manual, upstream module or sprite bytes are redistributed.
- `samples/item-asset-reference.json`: 18 Gem item appearances, sprite IDs and
  atlas files from the existing digest-verified 15.30 assets, reproduced by
  `verify_item_assets.py`. It does not assign icons to Wheel perks or admit assets.
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
On 2026-10-01, Tavily extracted the live TibiaPal page and official character
manual. The live-served planner module, string catalogue and renderer are
byte-identical to the pinned TibiaPal inputs, whose complete replay passes. This
qualifies `live_website_verified` for planner content only; it does not establish
interactive browser behavior or current Global parity. The supplied planner code
renders an invalid-code message. Seven requested Fandom pages failed extraction;
`wiki_verified` remains false. BR wiki pages are separately revision-pinned
secondary observations. The original official 8944 article failed extraction and
returned HTTP 403; its existing project capture remains the primary evidence.

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

Conviction includes source identities, categories, full-slice values, typed
augment stages and all nine unique perk definitions. Unique conditions, affected
skills/spells and numbers are encoded separately, including Battle Instinct,
Positional Tactics, Runic/Focus/Ballistic Mastery, Healing Link, Battle Healing,
Guiding Presence and Sanctuary. Area references and Divine Dazzle's +4-second duration use the
existing spell evidence. These names are reference bindings, not newly admitted
Spell or WorldQuery identities. Crystal's declared Shield Slam damage-reduction
field has an upstream TODO; a declared number does not establish working combat.

Revelation has three stages at 250/500/1,000 domain points, typed numeric effects,
explicit behavior rules and reference areas, with conditions also retained in descriptions and the shared +4/+9/+20 damage/healing
bonuses. Avatar cooldowns are converted from minutes to seconds; Gift of Life
cooldowns count battle-sign time. The official 8944 cooldown changes select -4 s for Mystic Repulse I and Thousand
Fist Blows II instead of the planner's -6 s. Great Fire Wave I preserves both
effects in the existing project spell evidence: critical extra damage +15% and
critical chance +10%. Mystic Repulse II selects project damage +40% while retaining
planner +60% and BR wiki +15%. Flurry I selects its enlarged affected area and
retains the planner's range +1 as an unselected hypothesis; no independent cast
range bonus is established. The Special Spells secondary cooldown remains a separately labelled
Canary-only hypothesis. All selections are recorded with their source.
Lord of Destruction's stage-2 death critical
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

Progression includes Global eligibility, the level-minus-50 point formula,
temple removal, five promotion scrolls, the Monk quest bonus and Grade IV points.
The reference values do not activate those features in Game: WHEEL-0's Premium
activation and extra-point dependencies remain explicit.

Atelier data includes clockwise domains (green/red/purple/blue), reveal/switch
fees, fragments, grade costs, initial eight gems and revealed/unrevealed yields.
Official manual yields override the conflicting OTS yields. Fees remain labelled
`OTS_HYPOTHESIS_ONLY` candidate values, as in WHEEL-GEM-0. Operation policy records reveal eligibility, tradeability, locking, last-domain
and in-vessel refusals, initial-gem lifetime, placement constraints, grade limits,
crusher charges and existing vendor prices. The 250 revealed-gem limit remains
`PARITY_PENDING` under the owning decision. Crystal loot probabilities are retained
as `OTS_HYPOTHESIS_ONLY`, with its exact independent-trial count and category
precedence; this reference does not allocate Forge/fiendish integration.
This tool does not perform
economy transactions, revelation RNG, grade writes, vessel writes or loot changes.

Icons include the sprite URL, horizontal-square-cell layout and source index.
Renderer source confirms horizontal-square cells for all five sprite categories.
The bundled Revelation sheet was visually inspected: 544×34 pixels, 16 cells,
SHA-256 recorded in `icon_evidence`, and every Revelation ID fits those cells.
The other four sheets are absent from the upstream bundle. Fresh requests to all
five original CDN URLs returned HTTP 403; the cause is not established. The
missing four categories remain visually unverified. No client asset crosswalk is
implemented by reference tooling.

## Revisions and admission

An initial revision accepts no previous candidate. Successors require the previous candidate;
`value_only` rejects changes to topology, perk identities, effect kinds/units and
other nonnumeric structure. `wheel_reset` describes the Wheel's revision policy,
not permission to erase paid gems: GEM-R admission separately requires the
WHEEL-GEM-0 §5.3 compatible mapping or staged migration.
Source identities, complete effect shapes, areas, targets, placement and icon
indices must agree with the committed input captures. Structural source updates
must update those captures; numeric effect tuning remains possible. Operation
invariants follow WHEEL-GEM-0, Battle Instinct thresholds/caps must be feasible,
and loot probabilities require a positive denominator and bounded chances.

The Wheel Authoring Schema workflow checks deterministic schemas, candidate,
evidence, item assets, comparison HTML and regressions on relevant pull requests.
Its source replay against upstream WASM remains an explicitly separate local
check; CI does not fetch upstream source or sprites.

The authoring package creates no runtime ruleset, protocol capability, Character
writer or native-key admission. Runtime owners must bind the effects and reference
areas, resolve the recorded parity conflicts, verify client icons and qualify
ruleset revision/migration behaviour before admitting it to a world.

## Live continuation assessment (2026-10-01)

The official manual freshly confirms the listed eligibility, operation/refusal,
fragment-yield and grade-chain facts in `samples/live-source-audit.json`. It does
not newly establish exact fees, the 250-gem cap, the 0.075 mitigation increment,
initial gem count, grade costs or quest/scroll point counts. Existing
OTS/PARITY_PENDING classifications for those values remain.

The BR wiki corroborates both selected -4 s cooldowns. Lord of Destruction II
gets additional derived corroboration: the wiki's combined 52.5% minus Master of
Decay's base 30% gives the selected 22.5 percentage-point increment. This is
secondary-source evidence, not a new official confirmation.

Mystic Repulse II has three observations: planner +60%, existing project target
+40%, and BR wiki r443774 +15%. The Great Fire Wave omission was corrected using
both existing project effect rows, also observed in BR wiki r423338. Flurry's
cast-range interpretation remains unselected. The source audit preserves all
observations and their revision dates. Neither a
secondary page nor successful access selects a new Global target version.
The corrected candidate retains `live_global_parity_confirmed: false` and
`runtime_admitted: false`. Independent content review and
protected integration still belong to the active programme control plane.
