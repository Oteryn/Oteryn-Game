# M23 Last Creep Standing: chosen personal disguise trial

This implements the accepted `CHOSEN_OTERYN_APPROXIMATION` recipe's fixed order:
Lost Soul, Pharaoh, Carniphila, Quara Hydromancer, Blightwalker, Mutated Rat,
Zombie, Nightstalker. It uses existing quest, carried-item, condition, World B3
and STEP transport owners. No Global coordinate or display-encoding parity is claimed.

The accepted recipe is in `samples/authored68/recipes.json`, and its one-count
canonical progress keys are recorded in
`samples/server-completion/chosen-progress/records.json` under
`oteryn:quest.authored.last_creep_standing`. The native body writes those exact
`s2` through `s9` tracks once each. No parallel phase counter is introduced.
Eight-form completion is derived from `s9 == 1`; later `s10` through `s12` and
all quest reward intents remain untouched.

## Exact identities and chosen visuals

Pinned XML source:
https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml

| Form | Exact source dummy Item ID |
| --- | --- |
| Lost Soul | 12052 |
| Pharaoh | 12047 |
| Carniphila | 12053 |
| Quara Hydromancer | 12048 |
| Blightwalker | 12056 |
| Mutated Rat | 12049 |
| Zombie | 12050 |
| Nightstalker | 12051 |

Shapechanger is source Item12251, a retained portable nonstackable 2oz quest
item. Dummies retain their source solid, immovable, nonpickupable physical flags;
they are not made portable. The exact capture inputs and extracted facts are in
`source-identities.json` and `physical-flags.json`.

**Explicit Oteryn visual choice:** every form uses the exact corresponding
source dummy as an ephemeral permanent ItemEx look. “Permanent” means no online
timeout, not persistence through logout. The Global Shapechanger's exact display
encoding was not recovered. Seven creature look bindings exist independently,
but the generic Pharaoh look was not proven; these were not silently substituted.
The static dummy sprites do not share source sprite IDs with creature outfits.
No assets are redistributed here.

## Interaction and owner boundaries

M01a owns Casper TALK and the legitimate Shapechanger reward. M23 does not
manufacture that talk event, grant another Shapechanger, create a private claim
track, or replace the NPC owner. The integration contract is exact:

- `oteryn:quest-progress/authored/last_creep_standing/s1 == 1` must already have
  been committed by the actual talk owner.
- The character must retain a real Item12251 in live carried custody.
- USE the chosen challenge marker near Casper. Existing reusable
  `item_on_target` admission independently reads and locks the carried tool and
  preserves it. This is ordinary marker USE, not a selected-tool USEWITH frame.
- The marker transports to the central white tile without advancing any stage.
  The same marker supports reentry with the same prerequisites.
- USE the retained Shapechanger at the exact central tile. The current form is
  selected from `this stage == 0` plus `previous stage == 1`, using existing
  read-only `TrackRequirement` predicates. The item and quest progress remain unchanged.
- STEP onto the matching portal while that exact item-look and condition key are
  current. The existing quest receipt commits the canonical stage once and then
  the existing teleport owner moves to the shared safe room. A wrong form,
  wrong stage or stale authority cannot advance progress.
- STEP onto the safe-room return portal to reveal the selected base appearance
  and return to the central tile. An exit portal also restores base appearance
  and returns outside. Other condition families remain intact.

The M23 PG fixture may seed the completed M01a prerequisite with the existing
quest writer and obtain the tool through the existing MINT fixture. That proves
the M23 body, not the unowned NPC conversation or subsequent quest rewards.

## Explicitly authored geometry

Casper's documentary BR reference is `(33261,32363,7)` (revision401880), retained
in the canonical NPC record. It is not a native NPC placement claim. The chosen
challenge marker is at `(33261,32365,7)`, on a verified existing plain ground231,
non-House tile. It adds one bare marker without replacing source content. The
exit lands on existing plain ground `(33264,32366,7)`.

The arena occupies an otherwise absent, bounded B3 sector inside the current
World envelope: source floor15, region80/80, sector640/640. Its white floor409 and
portal appearance1949 are source-qualified definitions. The arena is explicitly
chosen Oteryn geometry, not a fabricated donor-source map claim.

- Central tile: `(20494,20492,15)`.
- Matching portals: `(20483 + 3*i,20483,15)` for fixed-order `i=0..7`.
- Each exact dummy marker stands immediately north of its matching portal.
- Safe landing: `(20485,20506,15)`; return portal `(20485,20504,15)`.
- Exit portal: `(20508,20493,15)`.

All runtime descriptor floors use native negative values. New tiles are chosen
protection-zone tiles. There are484 new tiles and503 new placed entries including
the outside marker. No existing tile is replaced, no other region is changed,
and source floor/walkability/collision facts are not weakened.

Portal1949 is a **bare visual**: no raw teleport destination attribute is added.
Exact native selectors bind position, ordinal1 and appearance with all legacy
selector fields absent. Existing `quest_callback_native_object` rejects nondefault
attributes and dropped teleports. Existing quest STEP callbacks own all transport;
there is no second teleport mechanism, native auto-teleport bypass or new Transition schema.

## Source evidence and limits

- Pinned Wiki base:
  https://tibia.fandom.com/index.php?curid=50862&oldid=1184997
- Pinned Wiki spoiler:
  https://tibia.fandom.com/index.php?curid=50863&oldid=1050074
  Retained source-facts record line13 names all eight dummies and the Shapechanger.
- Public in-game instruction book (article51222):
  https://raw.githubusercontent.com/raulmaiz/tibia_dungeons/4a83d4bc92f58fb6f846fbb12f88a32f1b1984cc/game/data/book.json
  It describes white central floor, form-matching teleporter, safe room and central return.
  The accepted personal model replaces Global monthly/four-hour/ten-minute timing.
- Documentary Casper location:
  https://www.tibiawiki.com.br/wiki/Casper (retained revision401880).
- Existing authored-geometry precedents:
  `quest/fire_feathered_native.rs` and `quest/shared_world_progress.rs`.

The exact pinned Crystal map was scanned:19,492,788 tiles,78.125s, zero placements
of the eight dummy IDs. That map cannot prove this game's portal geometry.
Blocked Wiki APIs/HTML and public search challenges were not bypassed.

At draft preparation, source/codec/idempotence/collision tests executed; actual
native compilation and framed PG qualification remain pending. Do not report
this draft as gameplay complete until those checks execute on the integrated head.
