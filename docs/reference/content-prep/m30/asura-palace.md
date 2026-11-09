# M30b Asura Palace: native entry and chamber evidence

The shipped binding covers recipe s1 only. It observes entry at the canonical
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

## Exact remaining data gap

The Flaming Orchid's BR encyclopedia Document
(`content/documents/documents-01000-01499.json`,
`oteryn:document/monster-encyclopedia/the_flaming_orchid`) says two Greater Fire
Elementals unlock its boss room. The held Flaming Orchid spawn at
`(32955,32679,2)` is inside the central enclosure. This resolves the s3 boss-door
candidate to `(32959,32679,2)`, but does not locate or admit the elemental pair.

No checked-in admitted binding associates recipe s2/s3/s4 with an entry trigger,
two exact defender placements and the door(s) their deaths unlock. The finite
canonical spawn shards contain neither Greater Fire Elemental nor Greater Energy
Elemental placements. Their names occur in the chosen recipe and reference facts,
not in a native chamber roster or executable source interaction. Thus the missing
datum is the s2/s3/s4 **entry → admitted defender pair → unlocked door** association:
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
