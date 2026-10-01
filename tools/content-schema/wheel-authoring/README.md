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
python client_icons.py --check
node verify_planner.cjs /path/to/pinned/TibiaPal
```

Open `wheel-comparison.html` for a standalone offline comparison by vocation and
search. It displays the candidate's structured data offline. Optional icon previews prefer original CDN sheets. Only pixel-equal cells may
fall back to pinned reference sheets; other blocked images show their IDs.
The viewer includes perk and gem icons without redistributing source image bytes.
`build` without `--check` writes `samples/wheel-candidate.json`. Validation never
writes files. Rebuilding needs only the committed source captures, without the
original JavaScript/WASM module or network access.
The default `validate` and `build --check` qualify the candidate's exact file hash,
coverage, corrections and evidence digests. Custom `--file` authoring validates
semantics only; supply `--evidence` to qualify that exact file. The CLI labels the
difference. Non-finite numbers are rejected when reading and validating JSON. The reader
rejects duplicate object keys, including nested and Unicode-equivalent keys,
rather than choosing one conflicting value.

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
- `client_icons.py` and `samples/client-icon-manifest.json`: reproducible complete
  reference crop bindings, without redistributing assets.
- `samples/source-icon-reference.json`: five inspected, hash-pinned sheets and
  the older-client layout comparison.
- `samples/reference-selection.json`: target date, primary-source precedence and
  explicit reconciliation of the remaining source differences.
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
renders an invalid-code message. Seven requested Fandom pages, official news 8833/8944, the official character
manual and the five original CDN sheets were subsequently read with real Chrome
through Remote Desktop/CDP after normal HTTP/extraction failed. The browser was
used only for public internet research, then closed. The audit preserves source
revisions, content hashes, access methods and unresolved disagreements in
`samples/browser-source-audit.json`; `wiki_verified` now describes that bounded
seven-page coverage. It does not mean all values match current Global.


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
Guiding Presence and Sanctuary. Runic Mastery uses base magic level for the
single rune effect, as explicitly stated by Fandom r1206174. Official 8833 binds Battle Healing's shield
multiplier to 3, Focus Mastery to a 2-second focus-spell group reduction, and
Guiding Presence to 100% shared mantra. Official 8944 adds the raw 33% party-bonus
increase. That raw percent has unit `source_percent`: its native arithmetic,
rounding and self-copy semantics remain the owning Spell Q5 decision. Area references and Divine Dazzle's +4-second duration use the
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
The fresh official manual says grading does not reduce cooldowns *further*;
repository manual notes were corrected to preserve that distinction. The browser
audit records the old wording and correction; the candidate's fixed reduction is correct.

Progression includes Global eligibility, the level-minus-50 point formula,
temple removal, five promotion scrolls, the Monk quest bonus and Grade IV points.
The current Fandom reference additionally lists up to 50 Hunting Task Shop points;
that unadmitted reference is recorded without introducing a runtime point grant.
The 69-point Grade IV maximum is corroborated by all five candidate vocation
catalogues: 46 eligible basic types plus 23 supreme types, each worth one point.
The reference values do not activate those features in Game: WHEEL-0's Premium
activation and extra-point dependencies remain explicit.

Atelier data includes clockwise domains (green/red/purple/blue), reveal/switch
fees, fragments, grade costs, initial eight gems and revealed/unrevealed yields.
Official manual yields override the conflicting OTS yields. Fees remain labelled
`OTS_HYPOTHESIS_ONLY` candidate values, as in WHEEL-GEM-0. Operation policy records all-action Wheel eligibility, reveal inventory scope,
first eligible initial-gem grant, current-revision vessel placement anywhere,
vocation-catalogue grading, tradeability, locking, last-domain
and in-vessel refusals, initial-gem lifetime, placement constraints, grade limits,
crusher charges and existing vendor prices. The 250 revealed-gem limit remains
`PARITY_PENDING` under the owning decision. Crystal loot probabilities are retained
as `OTS_HYPOTHESIS_ONLY`, with its exact independent-trial count and category
precedence; this reference does not allocate Forge/fiendish integration.
This tool does not perform
economy transactions, revelation RNG, grade writes, vessel writes or loot changes.

Icons bind the current planner's category/index to an explicit crop manifest:
205 distinct crops and 520 candidate JSON-pointer bindings, across five sheets.
Each sheet records its immutable reference URL, dimensions, byte count and SHA-256.
All five original and reference strips were visually inspected and hashed. A
comparison through the same Chrome Canvas RGBA pipeline confirms 189 of the 205
used cells equal; nine Conviction, one Revelation and six Supreme cells differ.
The manifest marks those 16 cells as ineligible for fallback, avoiding a wrong
icon when the original CDN is blocked. Original cells and fallback cells have
independently checked bounds. The older client's slot layout differs in 13
Conviction assignments and is not used for current bindings. Browser tests cover
original, fallback, offline and wrong-dimension responses across all five vocations.
The manifest and snapshot-selection files are digest-bound by exact-file validation,
including a custom candidate's supplied serialized bytes. No PNG or proprietary
client implementation is included, and public availability does not grant runtime
redistribution rights. Qualify locally supplied source bytes with
`python client_icons.py --check --assets-dir /path/to/reference-sheets` and/or
`--originals-dir /path/to/original-sheets`; each directory uses `dedication.png`,
`conviction.png`, `revelation.png`, `basic_mod.png` and `supreme_mod.png`.


`samples/reference-selection.json` fixes the spell/augment authoring snapshot at
2026-09-27 and binds the existing project spell input by SHA-256. Released official
news precedes captured Fandom, then Canary hypotheses; BR wiki is separately dated
secondary evidence. Mystic Repulse II retains +40% from the Sept 14 primary
capture rather than treating Sept 2 BR +15% as a newer observation. This completes
the authoring selection; current live Global parity remains an external claim.

## Revisions and admission

An initial revision accepts no previous candidate. Successors require the previous candidate;
`value_only` rejects changes to topology, perk identities, effect kinds/units and
other nonnumeric structure. `wheel_reset` describes the Wheel's revision policy,
not permission to erase paid gems: any Gem contract change additionally requires `release.gem_revision`, with kind
`declared_compatible` or `staged_migration`, an owned JSON reference under
`samples/gem-revisions/` and its SHA-256. The referenced
`OTERYN_WHEEL_GEM_REVISION_REFERENCE/v1` record binds both revision IDs and Gem
contract hashes, the same kind, `runtime_admitted: false`, and
`runtime_validation: PENDING_GEM_R_ADMISSION`. A compatible declaration must leave
stored mod/quality/grade interpretation equal. A staged declaration requires a
nonempty `native_migration_reference`; this checks authoring provenance only.
GEM-R must still qualify the actual WHEEL-GEM-0 §5.3 mapping/migration and DUR-02
execution before admission. A Wheel reset never waives this requirement.
Source identities, complete effect shapes, areas, targets, placement and icon
indices must agree with the committed input captures. Structural source updates
must update those captures; numeric effect tuning remains possible. Count units require integers; probabilities, thresholds and reductions are
bounded at 100, while fractional seconds, larger damage bonuses and signed basic
penalties remain valid. Operation
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
not newly establish exact fees, the 250-gem cap,
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

The browser continuation corroborates mitigation 0.075% in official 8833 and both
cooldown selections in original official 8944. It confirms Fandom Conviction
1206174 selects Mystic Repulse II +40%, while BR 443774 says +15% and the planner
+60%. Fandom overview 1151969 lists 225 revealed gems and Supreme Grade III
12,500,000 gold; accepted project values remain 250 and 12,000,000. These conflicts
are explicit evidence for owners, not silent revisions of accepted decisions.
The Fandom Battle Instinct paragraph incorrectly repeats Battle Healing prose;
it is recorded as a source anomaly rather than selected over native conditions.
The manual's fixed repository path, actual file digest and reviewed-result status,
plus the browser audit digest and complete seven-page observations, are now
validated; changing the manual also triggers authoring CI.

Wiki coverage qualifies only canonical HTTPS URLs on the exact Fandom authority,
with the recorded Gem Atelier redirect explicitly allowed. Each requested page
needs a matching observed page, successful Chrome/CDP read, positive content
length, Fandom revision and SHA-256; foreign-host lookalikes and empty captures
are refused. Browser audit schema and false admission/parity flags are enforced.
