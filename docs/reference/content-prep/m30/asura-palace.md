# M30b Asura Palace: native entry and chamber evidence

The native palace entry observes recipe s1 at the canonical
Asura Palace Area point `(32953,32685,z7)`, whose base-map tile is exactly one
ground placement, source appearance 923, ordinal 0. The existing chosen EXPLORE
profile supplies native STEP admission, two one-shot counters and the live
Premium predicate. It grants no kill credit, reward or completion.

Regenerate with `python3 tools/content-schema/quest-authoring/engenc_asura_entry.py`.
Its `--check` verifies the generated binding against both the canonical recipe
and decoded WorldPlacement source; it refuses a moved or changed native anchor.

## What the placed doors actually establish

Decoded `content/world/placements/region-z02-x128-y127.b3`, with the palette in
`content/world/placements/index.json`, contains these three sealed-door candidates:

| Position (source positive floor) | Ordinal | Appearance | Geometry |
| --- | --- | --- | --- |
| 32947,32679,2 | 1 | 1674 | East boundary of the western enclosed room |
| 32959,32679,2 | 1 | 1674 | East boundary of the central enclosed room |
| 32962,32674,2 | 1 | 1676 | South boundary of a one-tile teleport alcove |

The alcove's `(32962,32673,2)` placement 775 teleports to `(32814,32754,9)`.
It cannot hold the two distinct defender placements required by M30b.
The destination is the existing Asura hunting-area spawn field; its return
teleport `(32812,32754,9)` targets `(32953,32670,7)`. It is not a third enclosed
two-defender chamber in the floor-2 palace geometry.
`tools/content-schema/world-authoring/samples/quest-door-coverage.json` lists all
three as unbound: no donor QuestDoorAction or QuestDoorUnique at those positions.
These are exact placed doors, but they do not establish three admitted chambers.

The lower room bounded by x32957–32962/y32673–32678/z4 contains the two static
Destroyer points `(32960,32674,4)` and `(32959,32677,4)`. Its eastern boundary
has two distinct closed doors, `(32963,32674,4)` and `(32963,32677,4)`, both
appearance 5736, ordinal 1. The pitfall above is `(32959,32675,3)`, appearance
21973, ordinal 1. The map alone does not identify whether either or both exits
are the quest-owned door, nor a source admission of the existing static actors.

## Donor source gaps and authored resolution

The Flaming Orchid's BR encyclopedia Document
(`content/documents/documents-01000-01499.json`,
`oteryn:document/monster-encyclopedia/the_flaming_orchid`) says two Greater Fire
Elementals unlock its boss room. The held Flaming Orchid spawn at
`(32955,32679,2)` is inside the central enclosure. This resolves the s3 boss-door
candidate to `(32959,32679,2)`, but does not locate or admit the elemental pair.

The checked donor evidence does not associate recipe s2/s3/s4 with an entry trigger,
two exact defender placements and the door(s) their deaths unlock. The finite
canonical spawn shards contain neither Greater Fire Elemental nor Greater Energy
Elemental placements. Their names occur in the chosen recipe and reference facts,
not in a native chamber roster or executable source interaction. The source-data gap is the s2/s3/s4 **entry → admitted defender pair → unlocked door** association:
s2's selection between its two exits, s3's elemental pair and entry, and s4's
elemental pair, entry and target door. Both elemental pairs' coordinates are
absent from the canonical spawn population. The palace location, floor-2 doors
and s3 boss-room candidate have been resolved from actual repository data.

The canonical recipe explicitly chooses sequential kill gates rather than
inferred cracks, door placements or storage IDs. The pinned source-facts record
for Asura in `samples/unbound-source-facts/authored-facts.json` (under
`tools/content-schema/quest-authoring/`) marks trigger/placement identity and
progress semantics UNKNOWN. Its exact spoiler-body receipt points to the
non-repository path `/tmp/quest-completion-round3/wiki-inventory/raw-captures/`
`tibia_fandom-75336-1138350.wikitext`; that body is not retained in this checkout.

The historical base-map decision in
`docs/agents/tasks/archive/OTV2-20260928-world-base-map.md`, Excluded scope,
explicitly excludes `asura_resp.otbm` as an unreferenced local-coordinate draft
overlapping the off-by-default Movement Trainer. Its source-tree filename is not
an admitted global chamber placement. No draft geometry, counterfeit source
action/storage ID or third room in the teleport alcove is introduced here.

M30a's named Child of Destiny spear/rune bindings and native hit pipeline already
exist. The mounted `engenc_combat_credit` PostgreSQL cases cover successful hit
settlement, exact source checks, retained queued outcomes, fencing and replay.


## Crystal 201189f8 follow-up

The complete, non-truncated Git tree and all 3,209 Lua files under
`data-global/scripts`, `data-crystal/scripts` and `data/scripts` were checked at
Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
Neither `greater fire elemental` nor `greater energy elemental` occurs in these
scripts, case-insensitively. The complete tree also contains no monster definition
for either name. This is a bounded search of that exact donor revision, not a
claim that no other source can supply the missing associations.

The relevant executable door source is
[actions_doors.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_secret_library_quest/the_lament_asuras/actions_doors.lua),
SHA256 `d14bc25d03361937d29e4f3345bcc88604a1b17bd0b0224bedf9c6b539a99950`.
It registers action 4911 for two exact positions:

- `(32959,32679,2)` requires Secret Library Asuras Questline storage >= 1.
- `(32962,32674,2)` requires Flaming Orchid storage >= 1 and level >= 250.

Neither handler spawns or admits a defender pair, counts their deaths, or selects
the lower Destroyer room's two exits. These are Secret Library access predicates;
copying them would not implement the chosen Asura Palace s2/s3/s4 recipe.
The sibling `movements_elemental_portals.lua` describes timed boss encounters
for Diamond Blossom, Lily of Night and Blazing Rose at different underground
coordinates. It does not define the two Greater Elemental pairs required here.
Thus Crystal confirms exact existing access-door positions but does not supply
the missing s2 exit selection, s3 elemental entry/pair, or s4 entry/pair/door.

## Owner-authored chamber identities (2026-11-09)

The standing owner rule now explicitly authorizes filling missing content with
Tibia-faithful authored choices. The MediaWiki API request for page
`Asura Palace Quest`, with an honest Oteryn research User-Agent, returned HTTP
402; no current revid was received. The repository's pinned spoiler receipt is
page 75336, revid 1138350 (the absent local body described above). Neither this
receipt nor the failed request is presented as newly retrieved page content.

`samples/engenc/asura-chambers.json` under `tools/content-schema/quest-authoring`
is the machine-readable authoring input for the remaining three chambers.
All otherwise unresolved associations are **authored per owner rule**:

- s2 retains the two canonical Destroyer positions and selects the southern exit
  `(32963,32677,4)`. The northern exit is not a quest-controlled passage.
- s3 uses the eastern/central floor-2 chamber, with two new Greater Fire Elemental
  placements at `(32958,32677,2)` and `(32958,32680,2)`; its existing east door is
  `(32959,32679,2)`.
- s4 uses the western floor-2 chamber, with two new Greater Energy Elemental
  placements at `(32944,32677,2)` and `(32945,32680,2)`; its existing east door is
  `(32947,32679,2)`. This adapts the recipe's unlocated “lower chamber” to an
  existing enclosure rather than inventing another floor or using the teleport
  alcove as a two-defender room.

The input also fixes each entry tile, landing and room bounds. Its unit check
verifies distinct defender positions, enclosure bounds, existing native tiles,
and the exact door appearance at ordinal 1. It does not claim these authoring
inputs are runtime bindings or PostgreSQL-qualified routes yet. Remaining work
is encounter callback generation, missing elemental native policy admission,
participant-scoped death/door wiring and real-caller PostgreSQL qualification.

The chamber generator now emits three exact native map-USE encounter callbacks,
each with two defenders and a distinct entered/count/opened track, followed by
its counter and personal door callback. The southern Destroyer door opens to
5737; the floor-2 doors open to 1675. Entry requires the preceding chamber's
opened track (s2 requires s1), and the door requires entered=1 and count=2.
No chamber transition completes the quest. Native encounter minimum level is 1,
the engine's positive minimum, so it excludes no playable character.

These are owner-authored callbacks, not counterfeit Crystal receipts. A closed
admission matcher accepts only the exact generated Asura callbacks, including
their native selectors, storage and rosters. Existing donor callback admission
is unchanged. The shared effect validator permits personal encounter data only
for these exact callbacks; completing their dispatch transaction remains a
separate runtime step. Unit checks cover one-shot doors, sequential chambers,
unchanged admission epochs and rejection of rebound native selectors. These
checks do not replace real-caller PostgreSQL qualification.

The mounted server PostgreSQL entry case now also executes the authored s2
admission through `commit_character_world_callback_with_items`, refusing the
s2 door and s3 entry beforehand, then checking s2 replay, entered=1, exactly two
total receipts including s1, and no quest completion. This qualifies the durable
server-writer route. It does not qualify physical encounter publication or
ordinary-combat defender deaths; those still require the dispatch scenario.


Personal encounter dispatch now preflights staging, room occupancy, teleport
proofs and defender policies before the durable entry write. It applies the
committed quest copy and refreshes native counter capture while retaining the
Channel turn, so the source roster captures the admission's committed revision.
The current pending callback can resume only with matching actor, session,
movement snapshot, callback, object identity/revision and original command cause.
A coherent copy already at the receipt revision is preserved on replay.

The focused pending-fence regression and ten existing capture tests cover this
slice. Both real framed Child of Destiny hit-credit PostgreSQL callers passed
again after the shared snapshot refactor. Full physical Asura encounter/death/door
qualification remains pending, along with the two elemental native policies.

`engenc_elemental_profiles.py` in `tools/content-migration/` implements the
owner-authored Greater Elemental policy transformation. Pending a retrievable
Tibia revision, **authored per owner rule**: use the admitted Massive Fire and
Massive Energy Elemental combat profiles as the respective bounded baselines,
retaining their health, speed, attacks, armor and resistances. The variants use
new Greater identities and display names, zero XP, no bestiary progression,
no prey/forge eligibility and no inherited soul-core or encyclopedia reference.
The original provider records remain byte-for-byte equivalent as JSON values.
The factory is registered by the provider integration described below. No donor
script is represented as implementing a Greater Elemental identity.

Provider integration now adds the two variants after donor attack-closure binding
and before native manifest and loot pins are generated. Their exact authored
baselines are Fire: 1800 HP, armor 57, speed 119; Energy: 1100 HP, armor 35,
speed 215. Both variants have no corpse reference and an explicit null-loot row;
no canonical donor identity or external monster ID is invented. The retained
Ability and Presentation references remain the admitted Massive Elemental ones.

MediaWiki API requests for page titles `Greater Fire Elemental` and
`Greater Energy Elemental`, using the same honest research User-Agent, returned
HTTP 402 on 2026-11-09. No revid was received for either page. Their unresolved
values therefore use the documented **authored per owner rule** fallback.
The existing Prey generator was rerun with `--simulate` (15 cases, 777 eligible
creatures): model results and slot values are unchanged, except their input
hashes. Both authored variants remain outside Prey eligibility.


Native activation qualification now passes with the corrected attack packet:
its Creature declaration and Creature profile bind the authored identity, while
its admitted Ability, Behavior and Presentation references remain unchanged.
The activation regression was observed failing on the original copied Creature
binding and passing after this correction. Explicit null-loot rows and no corpse
references keep the variants outside loot/corpse rewards. Registration freshness,
123 repository content tests, fmt and dispatch all-target clippy passed. World
pin regeneration preserved digest
`7b680a6007b27dff037ef35cfdb4bd97deec63b32e47c88cc597a603cd28a41c`.
The remaining M30b qualification is the full three-chamber dispatch path through
admission, ordinary-combat deaths and personal door opening.

The three native chamber admissions now pass PostgreSQL qualification through
literal framed USE and `serve_admitted`, with the production authored selectors,
actual B3 crops and native creature policies. Each route proves teleportation,
two defenders, one durable entry receipt and replay without another spawn.
Tests: `engenc_postgres_native_destroyer_chamber_admission`,
`engenc_postgres_native_fire_chamber_admission`, and
`engenc_postgres_native_energy_chamber_admission`.

The Destroyer crop exposed two obstructed previous home anchors (ball/chains
5471 and clutter 2696). **Authored per owner rule**: use adjacent plain floor
410 at (32959,32674,4) and (32958,32677,4). Stage USE at (32958,32675,3),
beside the crack at (32959,32675,3), whose item 21973 has floor-change down.
The native map objects are preserved. The generator now supports a separate
staging tile without changing the native target selector.

The dispatch qualification also exposed a production pending-command leak:
successful encounter execution returned before clearing its pending callback,
so the committed admission produced no wire result. The encounter branch now
clears that record on committed execution, matching the ordinary callback path.
The Destroyer literal USE test was observed failing before this correction and
passing afterward. Ordinary defender deaths and participant-scoped door opening
through all three real callers remain to be qualified.

The native admission PostgreSQL cases additionally exercise early door USE with
both defenders still alive. A bounded route search uses the retained native
floor and avoids the admitted defender positions; literal STEP commands move
the participant beside the exact native door before literal USE. The checks
require refusal, no door receipt, an unchanged object revision and both live
defenders. These are negative door qualifications; successful personal opening
after ordinary combat deaths remains separate work.

Combat/door qualification extends each native dispatch case after its early
refusal. The admitted player negotiates combat capabilities, walks to a reachable
cardinal neighbor of each exact admitted defender, sends framed ATTACK_TARGET,
and drains ordinary auto-attacks and production kill settlement. Durable count
one must still refuse the door; count two must commit exactly one door receipt
and change the native object revision. A level-10000 player fixture keeps the
combat loop bounded; native defender health, armor, resistances and policies
are unchanged. No death or quest receipt is manufactured by fixture setup.

All three expanded native dispatch PostgreSQL cases passed on 2026-11-09
(3 passed, 0 failed, 115.71 s): Destroyer, Greater Fire Elemental and Greater
Energy Elemental admission, ordinary combat credit and corresponding personal
door opening. Both elemental and Destroyer cases require count one to refuse
and count two to commit. Dispatch all-target clippy, workspace fmt and explicit
formatting of the mounted test file also passed. This supersedes the earlier
notes identifying ordinary deaths and successful opening as pending.
