# M31 world-change source facts (2026-11-08)

Scope: Hive Outpost, Insectoid Invasion, Jungle Camp, Kingsday, Lumberjack and
Horse Station. This is reference research for the selected Oteryn recipes,
not a claim that an imported title or coordinate admits a runtime effect.
The checked package head was `5c655223ff1a68203e81b1d15b9f7839db46bd11`.

## Binding rules already decided by the owner

`docs/architecture/reviews/OTERYN_GAME_EVENTS_WORLD1_DECISION_2026-10-15.md`
W1–W8 fixes one World decision per reset, channel-consistent named variants,
native layers and source multiplicity, and current-epoch raid prerequisites.
W15 fixes Kingsday at **24 permille**, Lumberjack at **90 permille**, uniform
variant weights where variants are selected, and no minimum gap. Its v1
duration is until the next reset. These accepted values take precedence over
an OT implementation's startup frequency. The document itself records that
some real events may persist across saves; the day-level observations do not
determine an hourly or multi-save algorithm.

## Sources and access

- `TIBIAWIKI_STRUCTURED`: retained, revision-pinned facts in
  `tools/content-schema/quest-authoring/samples/wiki-source-all373/source-specs-373.json`.
  Each named record has `source_fact_supplements`, exact revision hashes,
  line/span witnesses, and `historical_acquisition_verified` provenance. Its
  original raw bodies were `REFERENCE_ONLY_SCRATCH`, not committed runtime input.
- `OTS_HYPOTHESIS_ONLY`: executable Canary and Crystal scripts cited below.
  Public GitHub tree APIs were read at Canary
  `47dfd51f45280a59a1d3e50ba7edd573d7234446` and Crystal
  `201189f871d23597b82caf88fd611454a2ddefb8`; the retained corpus also provides
  exact verified bodies at Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`
  and Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`.
- Existing native Area/NPC/Spawn declarations are useful binding candidates.
  They remain distinct from a proof of a trigger, conditional membership,
  house safety or a currently active world variant.
- Normal HTTPS access to the Fandom MediaWiki API returned HTTP402; the
  TibiaWiki BR API returned HTTP403. Those routes were stopped. A normal
  Tibiopedia Chip request returned its `/setup` page, not the NPC article;
  no gate was bypassed. GitHub API/code search and raw-source access worked.

## Per-event facts

| Event | Source-backed facts | Concrete existing identities | Remaining distinctions for native authoring |
|---|---|---|---|
| Hive Outpost | Pinned wiki spoiler line6 says to fight Hive Born on Vandura, west of Liberty Bay. The base identifies the optional bosses The Manhunter, The Mean Masher and The Hungerer; it does not state a quantitative reward or their selection chance. | `oteryn:area.hunting_place.hive_outpost`, anchor `(32163,32838,7)`. Native hunting data names Crawler, Hive Overseer, Insectoid Worker, Kollos, Spidris, Spidris Elite, Spitter, Swarmer and Waspoid, plus the three optional bosses. | A selected recipe requiring ordinary insect kills need not invent an optional-boss chance or grant a unique boss reward. A day chance or native cohort chosen by the implementation must be labelled a chosen Oteryn adaptation, not a recovered Tibia probability. |
| Insectoid Invasion | Base line11 names Spider, Bug, Wolf and Insectoid Scout. Spoiler line5 references Greenshore, Thais and the Rotworm hole. The retained source does not supply an executable shared restoration threshold. | Exactly two native Insectoid Scout points occur in the checked spawn tree: `(32208,32058,7)` and `(32209,32060,7)`, both with90,000ms respawn. See precise spawn identities below. Greenshore Area source295 is centred at `(32255,32070,7)`. | The authored recipe's ten personal kills are an existing chosen objective, not evidence that the real world's global restoration threshold is ten. Source membership and the selected personal count should stay separate. |
| Jungle Camp | Base lines14/16 identifies Hunter/Poacher/Arthom the Hunter versus Dworc Fleshhunter/Venomsniper/Voodoomaster/Oodok Witchmaster. Spoiler line6 explicitly names the alternative Hunter Camp and Dworc Camp, a one-day interval, and conditional rare bosses. | Native hunting-place declarations `oteryn:area.hunting_place.hunter_camp` and `.dworc_camp` share anchor `(32900,32792,7)` in Tiquanda. Thus two geographically unrelated invented camps are unnecessary. | The anchor is not a proven rectangular spawn boundary. Rare-boss selection is not quantified. The chosen recipe's five Hunter/Poacher and five Dworc kills can use the ordinary groups; no rare boss is required by those steps. |
| Kingsday | W15 accepts24permille. Pinned spoiler line witnesses identify Sunday free prize exhibition, greeting the King for Loyal Subject, and cursing the King causing jail until the next arena raid with logout unavailable in jail. The raid-level note8 is not a global quest minimum. | King Tibianus's source placement is `(32311,32172,6)`. Existing achievement `oteryn:achievement/loyal_subject`, one point, secret, non-Premium. | The current raid catalogue has no Kingsday/arena raid entry. Do not mistake unrelated Thais raids for that encounter or apply level8 to the greeting recipe. The chosen greeting objective does not require the optional curse/jail branch. |
| Lumberjack | W15 accepts90permille. Pinned spoiler states order `talk_Chip` then `tell_Queen_Eloise`, awarding Whistle-Blower. Wood can be sold for50gold, but quantity is unspecified and the sale is not a guaranteed quest reward. | Chip's existing NPC wiki position is `(32418,31702,7)`; Queen Eloise's source placement is `(32314,31754,7)`. Existing achievement `oteryn:achievement/whistle_blower`, one point, secret, non-Premium. | The recipe is the report sequence. Do not add an automatic wood/gold payout or infer the felled-tree item count from a named title. A conditional Chip placement should use the admitted NPC identity and actual site. |
| Horse Station | Executable Crystal startup source gives exact stable rectangles/counts. Its stable step handler removes an ordinary `Horse` at appearance31159. While RAID_STORAGE120226 is a future timestamp, a1/30 draw clears that storage and respawns both stables. Wiki structured facts say wild-horse checks every3hours starting at save, with source count range0..3; regular horses cannot be tamed with Sugar Oat; automatic reset minimum4days. | Palomino rectangle `(32438,32231,7)`–`(32442,32238,7)`: Horse3, Grey Horse1, Brown Horse2. Appaloosa `(32846,32114,7)`–`(32850,32120,7)`: Horse1, Grey Horse1, Brown Horse1. Three concrete Wild Horse raid points are available below. | The donor's `waveEvent` is declared but never assigned, so its `stopEvent` cannot establish a working native cancellation lifecycle. A native controller must own its deadline/cancellation; copying that variable is not a fix. The wiki's four-day minimum and v1 until-reset policy differ in scope and must be documented rather than silently conflated. |

### Exact retained wiki URLs

| Event | Base | Spoiler |
|---|---|---|
| Hive Outpost | <https://tibia.fandom.com/index.php?curid=76240&oldid=969491> | <https://tibia.fandom.com/index.php?curid=76241&oldid=887146> |
| Insectoid Invasion | <https://tibia.fandom.com/index.php?curid=54570&oldid=1106148> | <https://tibia.fandom.com/index.php?curid=55050&oldid=1160735> |
| Jungle Camp | <https://tibia.fandom.com/index.php?curid=76242&oldid=847067> | <https://tibia.fandom.com/index.php?curid=76243&oldid=887148> |
| Kingsday | <https://tibia.fandom.com/index.php?curid=76244&oldid=845148> | <https://tibia.fandom.com/index.php?curid=76245&oldid=994341> |
| Lumberjack | <https://tibia.fandom.com/index.php?curid=76246&oldid=845163> | <https://tibia.fandom.com/index.php?curid=76247&oldid=893906> |
| Horse Station | <https://tibia.fandom.com/index.php?curid=54701&oldid=844780> | <https://tibia.fandom.com/index.php?curid=54702&oldid=502074> |

### Executable Horse Station source

- Startup rectangles/counts and suppression while RAID_STORAGE120226 is future:
  <https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/globalevents/spawn/horse_station_world_change.lua>.
  Git blob `6927277b292b1d41da6c27523c2bf7a32f64d68d`, SHA256
  `631b28ccc0120d2f8d9d74c1f9540b40989065b6785d7d4331f495389c92e07b`.
- Stable appearance31159, one-in-thirty reset, and the unassigned `waveEvent`:
  <https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/globalevents/others/horse_station.lua>.
  Git blob `0368a37794eefc18f2800c863d3dac4745489b8e`, SHA256
  `0d6615155766a3ccd21a5ba8519a687dcca1ff5314ed57d07600dbe021c863f0`.
- Death feedback is not an escape implementation:
  <https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data/scripts/creaturescripts/monster/horse_death.lua>
  only makes Horse/Wild Horse say “With its last strength the horse runs to safety.”
  SHA256 `4024b5cd156b24343409773aca7cc5d0e97c0ecd11b6c3053c8701595809ed49`.
- Concrete ordinary raid cohort:
  <https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/data-otservbr-global/raids/thais/wild_horses.xml>
  spawns one Wild Horse at each `(32462,32214,7)`, `(32445,32227,7)` and
  `(32442,32265,7)`, after2,000ms. SHA256
  `fd735e7251d80d7f8773c60715537ce725bb43399c0046d24874404ea41707ae`.
- Canary also has duplicate Lua raid registrations at the same revision:
  `scripts/raids/monsters/wild_horses.lua` and
  `scripts/raids/thais/wild_horses.lua` both define `thais.wild-horses` and
  seven groups of three with3h auto-advance, but differ in minimum players
  (3/0) and minimum gap (12h/23h). Do not register both or call either timing
  a proven Global schedule. Their two zones are
  `(32456,32193,7)`–`(32491,32261,7)` and
  `(32431,32240,7)`–`(32464,32280,7)`.

### NPC and scene binding evidence

Fresh normal HTTPS read:
<https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/world/otservbr-npc.xml>,
SHA256 `a02d95945d73c1f5c24f9b0446d3f63102d2bfa2d4c2d4a59bb40c1ea3fd2fe9`:
King Tibianus `(32311,32172,6)`, Queen Eloise `(32314,31754,7)`, both centre
offset0, source spawn interval60s. Chip is absent from that source XML;
his existing native declaration instead retains the wiki position above.

Native scene declarations checked:

- `content/world/areas/hunting-places/hunting-places-00000-00444.json` supplies
  the Hive Outpost, Hunter Camp and Dworc Camp keys/anchors/membership above.
- `content/world/spawns/spawns-10000-11999.json`:
  `oteryn:spawn.x32210_y32056_z7` owns the Insectoid Scout point
  `(32208,32058,7)`; `oteryn:spawn.x32210_y32063_z7` owns `(32209,32060,7)`.
  Both source respawn periods are90,000ms.
- `content/world/spawns/spawns-14000-15999.json`:
  `oteryn:spawn.x32928_y32754_z7` has Hunter points `(32930,32753,7)` and
  `(32926,32756,7)`,90,000ms. Their existence alone does not prove that this
  entire spawn group is the Jungle Camp variant; do not classify nearby
  unrelated Asura/underground sources by an oversized coordinate rectangle.
- Hive Outpost already has ordinary insect source points in
  `spawns-04000-05999.json`, for example
  `oteryn:spawn.x32161_y32847_z5` with Insectoid Workers
  `(32160,32848,5)` and `(32163,32849,5)`. Restrict membership by exact
  native source identities/creatures; the surrounding map contains unrelated
  Rotworm/Bonelord/Quara sources that should remain unchanged.

## Authoring boundary

Source-grounded native choices may implement the selected recipes, but must be
identified as such in their provenance. Existing recipes already choose the
personal objective quantities (Hive Outpost20 ordinary insects, Insectoid10
Scouts, Jungle5 Hunters/Poachers plus5 Dworcs). Those counts are not recovered
Global activation/restoration thresholds. Keep optional unique bosses and
unselected reward branches outside that bounded completion claim unless their
own multiplicity and eligibility policies are authored and proved. No synthetic
global survey terminal, name-only event admission, or arbitrary nearby spawn
reclassification is justified by this research.

## Selected personal journal source bindings

The existing definitions already limit these journeys to chosen personal recipes:

- Fury: “The rotating world gate becomes a fixed personal entrance with ten
  chosen Fury kills and a sealing interaction. No world-wide gate closure,
  original schedule or unrecorded achievement is claimed.”
- Grimvale: “Calendar and shared world-change states become an on-demand
  personal encounter. Ten Werewolves and the survey marker are chosen; no
  unsupported cure or boss loot is inferred.”
- Hive Outpost: “The personal outpost defence is always available and uses an
  authored twenty-kill target. Rare bosses are optional encounters, not
  guaranteed spawns or rewards.”

Therefore the personal journals do not justify enabling excluded global
random/raid/restoration branches. W5's accepted until-next-reset policy remains
the global owner policy; the multi-day wiki observation is not a replacement.

### B3 facts read with the repository codec

The following stacks were decoded using
`tools/content-schema/world-authoring/world_region_codec.py::decode_region`
and the palette in `content/world/placements/index.json`. Coordinates here use
source `z`; callback/movement native `floor` is `-z`. Every listed entry is
top-level (`depth=0`). These are concrete source objects or ground, not a new
global terminal. Assigning a personal marker meaning remains an explicit
chosen Oteryn binding.

| Selected purpose | Source position | Actual stack and selector facts | Source region SHA256 |
|---|---|---|---|
| Fixed personal Fury entrance beside the Thais gate | `(32265,32164,7)` | Raw base contains only `7066`, ordinal0, flags0, house0, no action/unique. There is **no dormant portal** at this point. Selecting this exact source-ground USE/STEP is a native adaptation; do not describe it as an existing dormant teleport. | `region-z07-x126-y125.b3`: `b7f4b09e880cc1637e2d056e058c37e8b2a77b614751333d4b89496a7fdfe037` |
| Actual daily Thais portal position | `(32264,32164,7)` | Raw base also contains only `7066`; the selected Fury layer adds the actual `6117` with action9710, plus archway777 and5064. | Same region as above; layer blob `a131dd8d0962a448e4e6a357d8eeb0d5fc951f78` |
| Personal Fury seal at the dungeon's real return archway | `(33290,31785,13)` | Stack `[10549,14113,776,5068,5066]`; archway776 is ordinal2, flags0, house0, no action/unique in the raw source. Candidate AID9715 is a **startup source assignment**, not present in these raw B3 bytes. A position+appearance776 selector is a real existing object; an AID9715-only selector needs an actual restoration hook. | `region-z13-x130-y124.b3`: `e69a2d5bb21781baa06dc24b8df6ea3a0b244c060adb584e85ab20b83104dc3a` |
| Chosen Grimvale curse survey stone | `(33311,31709,7)` | Stack `[4405,1866]`; stone1866 ordinal1, flags0, house0, no action/unique. It is an existing island source object, not an original named “curse survey marker.” Adjacent source ground includes `(33312,31709,7)` stack `[4409,311]`. | `region-z07-x130-y123.b3`: `694e5e9f3a6c3678dbb74f0925453b0f25f04100b2b773e808c96c0e24db29a9` |
| Chosen Hive Outpost survey stone | `(32159,32844,7)` | Stack `[12859,12908,3733]`; stone3733 ordinal2, flags0, house0, no action/unique. Nearby `(32158,32845,7)` has only ground12854. Bind the exact stone, rather than every similarly named hive structure. | `region-z07-x125-y128.b3`: `9665df96db1add63af38ed6e8a6e9e59f0dcbb67e06c0a5b7487a779896df2f7` |

The selected `fury_gates/thais` layer in
`content/encounters/world-changes/layers.json` preserves7066 at the proposed
personal point `(32265,32164,7)`: its stack there is `[7066,5064]`. The actual
daily portal is on the neighbouring `(32264,32164,7)`, with stack
`[7066,6117(action9710),777,5064]`. This lets a personal source-ground binding
remain independent of the ordinary daily STEP portal; its exact dispatch
exposure and callback sequence still require the production proof.

Official appearance flags were checked directly with the existing
`build_source_item_profiles.engine_items.load_appearance_objects` reader over
the pinned `appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat`:

- `1866`: `unmove=true`, `ignore_look=true`.
- `3733`: `unmove=true`, `hang=true`.
- `776`: `top=true`, `unmove=true`.
- `7066`: `bank=true`, `bank.waypoints=150`, `unmove=true`, `fullbank=true`.
- The parchment635 at `(33340,31679,7)` carries real “About
  Were-Transformation” text, but has `take=true`; it is **not** an immutable
  map landmark and should not be substituted for the selected survey marker.

The source Lua `movements_strangely_glowing_mark.lua` names22076/22077,
but neither was found in the checked Grimvale region files at `x130,y123`,
floors7–15. A source script item ID alone therefore does not prove an existing
native marker at a selected survey position. Its separate haste/healing
mechanic must not be fabricated as part of the chosen journal marker.

### Wild Horse / Sugar Oat and completion receipt boundary

Existing `content/cosmetics/taming.json` row17 declares:

- item `oteryn:item.tibia.i12802` (Sugar Oat);
- creature `oteryn:creature.wild_horse`;
- mount `oteryn:content.mount.war_horse`;
- grant `oteryn:taming.war_horse`;
- source OT chance15%, break enabled, failures “The horse runs away.” and
  “The horse ate the oats.”; success “You have tamed the horse.”;
- achievement `oteryn:achievement/lucky_horseshoe`;
- source `canary:data-otservbr-global/scripts/actions/mounts/mounts.lua:49`,
  <https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/data-otservbr-global/scripts/actions/mounts/mounts.lua>.

The separate Music Box rule uses item16244 at100% and is not Sugar Oat.
Ordinary Horse/Grey Horse/Brown Horse are not valid Sugar Oat subjects.

At the checked tree, dispatch
`crates/gameplay/dispatch/src/gameplay_transport/mount_taming.rs` builds
`MountTamingRequest`, then calls the fenced
`DurabilityRoot::commit_mount_taming`. In
`durability/item_use/mount_taming.rs::attempt`, historical command receipt
lookup occurs before a new attempt; the success branch (`outcome==0`) writes
achievement and appearance grant receipts/unlocks, then the item debit and
`game_mount_taming_attempts` receipt commit in the same transaction. The
request currently contains **no quest transition/journal participant**. M14's
generic participant was not present in this checked package tree. The
reusable boundary is this same fenced success transaction, preserving the
historical early return; a later cosmetic-ownership read is not an atomic
proof of this character's selected journal step. Existing ownership returns
`None` without an attempt, RNG, consumption or new grant and must not be
misclassified as a successful fresh taming event.

### Exact achievement identities

| Name | Existing canonical key | Additional source identity |
|---|---|---|
| Fire from the Earth | `oteryn:achievement/fire_from_the_earth` | static client15.30 ID256; wiki62474@845084; existing `world_event/changes/counters/fire_earth.rs` grants this exact key |
| Loyal Subject | `oteryn:achievement/loyal_subject` | wiki62553@845162 |
| Whistle-Blower | `oteryn:achievement/whistle_blower` | wiki62814@845295 |
| Lucky Horseshoe | `oteryn:achievement/lucky_horseshoe` | static client15.30 ID400; wiki75125@847079; existing Wild Horse taming rule |

## Native callback shard integration (2026-11-08 follow-up)

These are code-inspected integration facts, not additional source-game policy.
`crates/gameplay/quests/src/quest/world_callbacks.rs::load` explicitly parses
`EMBEDDED` plus its named `include_str!` shards; a standalone
`content/quests/world-callbacks/m31-*.json` is not discovered automatically. Add
its registration to this list. `extend` adds each callback's storage tracks and
`primary_transition`; the latter currently hardcodes `completes: false`.
`reward_callbacks.rs::DOCUMENTS` likewise explicitly includes only the base
shared-reward document and `npc-callbacks/l6-stories.json`; register an owned
M31 NPC shard there. Reward callbacks already carry a supported `completes`
boolean, used in their constructed transition.

`world_callbacks.rs::gated_binding` returns no binding when both world-admission
and opening-only are false. Otherwise it serializes the Callback to JSON,
removes `sources`, `success_message`, and `failure_message`, serializes the
remaining object and hashes it with SHA256. Adding a serialized default field
to Callback therefore changes existing bindings. An owned journal-policy
sidecar/decorator avoids that incidental change; it still must validate that
its callback keys exist and refer to the intended quest, and bind any new policy
semantics where appropriate. No existing generic completion decorator was
found in this inspected path. `loader.rs::load_embedded_quest_state` applies
world-callback and reward-callback extensions to the parsed base mission state.

`tools/content-schema/quest-authoring/quest_state_lowering.py::expected` lowers
SOURCE `source_data.progress`; it does not collect native callback shards.
Do not hand-edit its generated `content/missions/quest-state.json` to add native
callbacks. `quest_tree_authoring.py` has a specific Monk native annotation
reader and explicitly lists base `content/quests/world-callbacks.json` among
its authoring sources; it does not implicitly project a new M31 shard into
SOURCE evidence. `batch3_world.py::outputs` owns its named batch3 inputs and
merges them into the base callback document; an independently named M31 shard
is not part of that producer. `qengine_rewards.py` owns the generated shared
reward callback document. Preserve those unrelated producers.

`tools/content-migration/regenerate_content.py::refresh_quest_callback_proof`
refreshes the approved completion242 callback-inventory witness for the BASE
world-callback document, plus the receipt's approval-file hash. The guard in
`source_fix_guard.py::verified_receipt` verifies pinned descriptors and reviewed
SOURCE inputs; a new standalone shard does not automatically become an approved
SOURCE lowering proof. Use a producer/check for the new M31 native projection;
never hand-edit a proof SHA. `quest_callback_profiles.py::admit` reads the base
world callback document for carried-tool admission; a new shard with new
carried inputs would require explicit admission integration. Native Sugar Oat
already has its independent taming rule.

World pin collection is separate: `tools/world-bundle-compiler/src/main.rs`
`INPUT_PATHS` includes world placements, NPC definitions/dialogues/services,
creatures, layers/runtime and compiler sources; it does NOT include
`content/quests/`. `pin_refresh.rs::run` rebuilds every world pin/identity and
updates the preproduction manifest/client placement source pin. The content
regenerator runs source/tree producers and authoring tools before pin-refresh
LAST; pin input diagnostics read the staged tree. A callback-only shard is not
a reason to fake a map pin change; source NPC placement changes are.

### Orockle and Chip availability

`apps/game-server/src/node/serve/daily_world_changes.rs::apply` returns before
placement when `reset_epoch == 0`. Its dynamic HiveBorn entry places Orockle at
`33448,31309,7` only after daily admission and the NPC boot admission check.
Chip is not in that daily list. Neither NPC occurs in the 56 records in
`content/world/npc-placements/placements-00000-00055.json`. Thus existing NPC
identity declarations alone do not provide either actor at world epoch zero.

`tools/content-schema/npc-authoring/convert_placements.py` consumes
`samples/promotion-candidates-v1.json`, `npc_placement_scope.json`, and native
NPC definitions. Orockle has a candidate at `33448,31309,7` (north, interval60,
radius1), but is not in the placement scope. Chip is absent from candidates and
scope; its native definition retains wiki position `32418,31702,7`. The world
bundle compiler's `source.rs::add_npc_families` uses the indexed placement
family. `bundle_npcs.rs` reconstructs those static actors, independently of the
world epoch; dynamic placement does not supply a missing static catalogue row.
`crates/gameplay/content/src/content/npc_catalogue/service.rs::build` admits
NPC identities if referenced dialogues/services resolve; an empty services
list does not itself exclude an NPC, and dialogue absence generates replies.
Placement and identity admission are separate conditions.

## Additional exact source ground candidates

Decoded with the repository `world_region_codec.decode_region`, using the
placement palette's source appearance IDs. Every item below has depth0 and no
attributes. Ground walkability was checked against the pinned official
`appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat`
with `engine_items.load_appearance_objects`: each stated ground has `bank`,
`unmove`, and no `unpass` or `avoid`; listed overlay311 also has no such blocker.
These checks prove source stack/appearance facts, not an entire route from a
city or runtime absence of another actor. Positive source z becomes negative
native callback floor.

| Purpose | Exact source coordinate | Full source stack | Tile flags / house |
|---|---|---|---|
| Grimvale adjacent STEP |33312,31709,7|4409,311|0 / 0|
| Hive Outpost adjacent STEP |32159,32845,7|12851|0 / 0|
| Outpost alternative ground |32158,32845,7|12854|0 / 0|
| Chip adjacent northern STEP |32418,31701,7|4518|0 / 0|
| King's clear side tile |32309,32173,6|464|1 (protection) / 0|
| Surface Hunter/Dworc camp STEP |32900,32792,7|1019|0 / 0|
| Source Dworc adjacent STEP |32831,32866,10|351|0 / 0|
| Volcano encounter staging |32037,32588,7|103|0 / 0|

The King's tile is in
`region-z06-x126-y125.b3`, SHA256
`ade86a8e432522df4d30b1d8f56e8c6536a35c4e0664d426f4e895a2f80ddbfe`.
Actual SourceDworc spawn `oteryn:spawn.x32832_y32866_z10`, recorded in
`content/world/spawns/spawns-36000-37999.json`, places
`oteryn:creature.dworc_fleshhunter` at `32832,32866,10`, respawn90000ms.
Its adjacent ground is directly connected by a single cardinal step. The map
region `region-z10-x128-y128.b3` SHA256 is
`46b36c5f4933028839921c20029dc7f89b97562593a61b88b5cc60367c576ac9`.
This dungeon spawn is not evidence of a surface camp Dworc placement.

A source-backed 8-cell encounter rectangle inside the accepted volcano bounds
is x`32037..32040`, y`32589..32590`, z`7`. All eight source tiles contain ONLY
appearance103, flags0, house0, no attributes. Staging `32037,32588,7` is one
cardinal step north of the rectangle and also ONLY103; `32037,32587,7` is the
same ground. Their region is `region-z07-x125-y127.b3`, SHA256
`b3dfe5b9f55eeaee53c8e7ca2a24b6c4649dfcb8595e221a0019a2eed04063ed`.
Selecting this existing source scene for a bounded personal cohort is a native
adaptation choice, not a claim that the original shared volcano script used
this exact rectangle. Existing global-inactive kill masking still needs an
explicit personal encounter route to avoid silently counting excluded natives.

## Horse personal room, Hive landings, and Chip stock follow-up

The same direct B3 reader and official appearance-flag check establishes these
small exact selectors. Every listed tile has flags0, house0, depth0 items with
no attributes; every listed appearance is bank/unmove/fullbank with no unpass
or avoid flag.

| Role | Source coordinate | Entire stack |
|---|---|---|
| Horse staging outside chosen room |32462,32213,7|4519|
| Horse room monster cell |32462,32214,7|4522|
| Horse room player landing |32463,32214,7|4530|
| Hive minimum-stage2 inner landing |33599,31228,1|12854|
| Hive minimum-stage3 inner landing |33494,31197,1|13038|

The two Horse room cells form rectangle x32462..32463/y32214/z7, with staging
one step north of the monster cell. The source coordinate32462,32214 is already
the Wild Horse point in the retained Canary raid XML described above. Selecting
a single-monster personal encounter here is a native recipe adaptation; do not
copy the global raid timer. Source stacks do not prove runtime actor vacancy:
the Encounter's existing empty-room check must still run. Map region
`region-z07-x126-y125.b3` SHA256 is
`b7f4b09e880cc1637e2d056e058c37e8b2a77b614751333d4b89496a7fdfe037`.

`crates/gameplay/quests/src/quest/hive_access.rs` preserves the actual gate
crossing axis: appearances13290..13292 cross x; the opposite landing is x+1
when approaching from the west, x-1 when approaching from the east. Accordingly
stage2 gate33598,31228,z1 leads inward to33599,31228; stage3 gate33495,31197,z1
leads inward to33494,31197. The source `outer_route` witnesses are33577,31235
for the stage2 gate and33501,31197 for stage3; they identify the exterior side,
not a replacement teleport destination. Gate source locator:
https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data/scripts/actions/objects/hive_gates.lua
and retained wiki56902@930222:20. Landing B3 SHA256 witnesses:
`region-z01-x131-y121.b3` =
`e9dcd49d7ce26d8c71e04050256629e1596ffefea74145abab68e143c3a8b498`;
`region-z01-x130-y121.b3` =
`ab09aa7157ba7b409bcb3bcfe27ba3d46f1add7c7ea64423af95e8ed4052427e`.
Personal access after the selected50point exchange and10defenders is a separate
personal journal policy; these source coordinates do not authorize changing
shared thresholds or worldwide expiry.

### Chip trades: direction known, executable offer not established

In `content/npcs/definitions/npcs-00000-00499.json`, Chip has `services: []`.
His retained `oteryn:source.npc.bulk.wiki_trade_reference` is literally
`{"BuyFromPlayer":{"Wood":[null]},"SellToPlayer":{}}`. Thus the retained
reference describes Chip BUYING Wood from the player; it is not stock that
sells Wood to the player, and its price is null. Source public article locators
retained in that definition are https://tibiopedia.pl/npcs/Chip and
https://tibia.fandom.com/wiki/Chip. Previously attempted current public wiki
routes did not supply an article body, and inspected pinned Canary/Crystal
trees contained no Chip NPC script. A fresh GitHub code search for
`filename:chip.lua wood` found no relevant Tibia script. These inspected
references do not establish an executable quantity/price offer.

The materializable native Wood identity DOES exist:
`content/items/definitions/spell-native-profiles.json` row whose
`production_definition.production_key` is `oteryn:item.tibia.i5901`.
Its presentation name is wood; it is movable/pickupable, stackable max100,
weight500 in the native unit. This row's exact production binding is donor
server item5901, Crystal revision
`ff7ede593c69d4c658b382c97443e8155926924a`; the same mapping appears in
`imports/crystalserver/bindings/items.json`. The other historical items named
wood (e.g.i7916/i7921..) have unknown semantics and are not evidence for the
tradeable5901 product. Source item URL:
https://github.com/zimbadev/crystalserver/blob/ff7ede593c69d4c658b382c97443e8155926924a/data/items/items.xml

A completed access flag can prove the character unlocked the selected journal
reward permission. With the currently inspected catalogue it cannot honestly
prove that Chip has a working Wood shop offer: there is no native Chip service
or known-price offer to exercise. Do not convert the quest's unspecified50gold
text into an invented unit price or quantity; any explicit native offer choice
must be recorded as such, preserving the proven BuyFromPlayer direction.

## Final starter and ordinary-population verification

Direct B3 extraction establishes these full stacks (all depth0, attributes
absent, flags0/house0/zones empty):

| Starter | Source coordinate | Entire stack |
|---|---|---|
| Fury dungeon STEP |33290,31786,13|10549,14113,1866|
| Insectoid start STEP |32208,32059,7|4407,6480,1071|
| Horse station starter STEP |32438,32231,7|4522,5485,6218|

Official appearance objects confirm first IDs10549/4407/4522 are bank grounds;
none of the nine listed appearances has unpass/avoid. The remaining IDs are
immovable overlays, several with clip/ignore_look, rather than invented source
objects. Select the ground explicitly if the selected journey uses ground
STEP; retain the full stacks as evidence.

### Grimvale personal encounter scene and missing original cohort

A source-backed3x2 room excluding the existing survey stone is
x33310..33312/y31710..31711/z7. Staging33312,31709,z7 has stack4409,311 and
lies immediately north of landing33312,31710. The six full stacks are:

| y | x33310 | x33311 | x33312 |
|---|---|---|---|
|31710|4402,4577|4405,311|4397,315,6218|
|31711|4399,407,21935|4406,315,6217|4518,6218|

All six tiles have flags0/house0/zones empty/depth0/no attributes. Each first
appearance is bank ground; no appearance in these stacks has unpass/avoid in
the pinned official appearance object. Thus one player landing and five
Werewolf cells can fit the chosen bounded encounter, subject to runtime
vacancy and existing Encounter ownership checks. Two personal waves of five
are an explicit native selected-journey adaptation, not a source calendar
rule. Existing survey stone33311,31709 remains immediately outside the room.
Region witness remains `region-z07-x130-y123.b3` SHA256
`694e5e9f3a6c3678dbb74f0925453b0f25f04100b2b773e808c96c0e24db29a9`.

The native spawn tree contains NO Werewolf point in the checked Grimvale
rectangle33300..33360/31665..31730 on any floor. The closest retained Werewolf
point to the Grimvale area anchor is33438,31619,z10 (Edron-side), source
`oteryn:spawn.x33435_y31622_z10` in `spawns-30000-31999.json`,90s respawn.
Do not describe that as an existing Grimvale population. The daily Grimvale
producer in `crates/gameplay/world/src/map/daily.rs::grimvale_spawns` only adds
Wereboar/Werebadger on full-moon days, versus Bandit/Badger/BlueButterfly/
YellowButterfly otherwise, within33330..33350/31670..31690/z7. It does not
supply the selected recipe's ten Werewolves.

### Ordinary Fury and Outpost sources survive shared variant projection

The source spawn tree has29 Fury points in33260..33330/31760..31850, allfloors.
A concrete example in `spawns-48000-49999.json` is
`oteryn:spawn.x33315_y31798_z14`, Fury33316,31800,z14,60000ms.
For the Outpost rectangle32135..32195/32805..32870, allfloors, the source point
counts are: Waspoid37, Crawler21, InsectoidWorker20, Spitter16, Swarmer14,
Spidris7, Kollos4. There are no HiveOverseer/SpidrisElite points in this checked
rectangle, so don't claim all nine nominal area species are present. These
seven ordinary species still support the chosen ordinary-kill objective.
Examples in `spawns-04000-05999.json` include Crawler32156,32852,z5 and
Kollos32154,32862,z5; all seven listed source types use90000ms in the checked
sampled points.

The actual runtime masking caller is
`apps/game-server/src/node/raid_scheduler/world_changes.rs::project`:
`project_world_spawn_cohorts` finds a rule only by exact base creature plus
bounded home cell and leaves a point unchanged when none matches. Its source
`content/encounters/world-changes/credited-kills.json::spawn_rules` has no Fury
or Werewolf rules. Hive Kollos/Spidris/SpidrisElite rules are restricted to
33001..33700/30900..31400/z0..15; the southern Outpost rectangle is outside
those bounds. Therefore those checked Fury and Outpost source cohorts are
not suppressed by these shared phase rules. Normal map composition may still
reject a source point on obstructed/protection tiles, and native creature
admission remains required; this inspection does not replace a live dispatch
proof. `map/boot.rs::activation_spawns` retains ordinary source points subject
to composed-ground/door/protection checks and then appends daily Grimvale
sources; it does not replace all ordinary cohorts with the daily list.

## Hive Favour admission and western native defender cohort

`crates/gameplay/durability/src/durability/world_event/changes/counters/shared_progress.rs::credit_completed`
currently gates BOTH native task acceptance and completed task turn-in on
`load(world,epoch)` returning a day. Missing reset/day returns Refused. Task
acceptance additionally requires precisely the expected active-track0->1
command effect. Eligibility is stage3 for anytask, stage1 for tasks0..3,
stage2 for tasks4..5; stage0 refuses. Completed task turn-in independently
checks source action/cause, the real objective changes and stage eligibility
before crediting the shared World owner. Thus these existing tasks cannot
already earn personal Favour at reset0/inactive World through this path.
A selected personal programme needs an explicitly scoped exemption or separate
personal receipt policy that retains real task objective/turn-in validation,
without manufacturing shared global credits.

`quests/src/quest/shared_world_progress.rs` binds six actual tasks: antennae8,
digestion blobs7, chitin8 plus hatchlings8, pores7, Kollos9, monoliths5.
Only their actual completed task transitions add the Favour track.
`hive_access.rs::favour_reward` grants1 for tasks0..3,2 for tasks4..5;
`BUY` consumes50 Favour and records PURCHASED. Fifty Favour therefore cannot
be substituted with a direct marker grant. The retained task source locator
is https://tibia.fandom.com/index.php?curid=56916&oldid=1184992
(War Against the Hive), as identified by shared_world_progress's source header.

The source spawn catalogue contains62 Crawler points west of the eastern gate
inside x33470..33597/y31165..31250/z3..9. Counts by sourcefloor: z3=3,z4=8,
z5=13,z6=10,z7=17,z8=4,z9=7. There are NONE on z1 in this checked box, so the
z1 gate landings do not imply a z1 Crawler cohort. Concrete nearby records:

| Source shard | Source key | Crawler coordinate | Respawn |
|---|---|---|---|
|spawns-06000-07999.json|oteryn:spawn.x33585_y31228_z7|33586,31228,7|90000ms|
|spawns-06000-07999.json|oteryn:spawn.x33592_y31215_z7|33593,31215,7|90000ms|
|spawns-00000-01999.json|oteryn:spawn.x33594_y31219_z4|33595,31219,4|90000ms|
|spawns-00000-01999.json|oteryn:spawn.x33592_y31236_z3|33593,31236,3|90000ms|

There is no Crawler rule in credited-kills.json's spawn_rules. Accordingly the
source projection caller described above leaves these ordinary Crawler points
unchanged at global stage0. This is a source-backed hunting eligibility bound
on the western side, not a demonstrated continuous traversable path between
the two z1 gates: connectivity and floor transitions were not proved by this
spawn inventory inspection. Chip findings remain unchanged: checked native
identity5901 and buy-from-player direction are established; no executable
known-price native Chip offer was located in the inspected sources.

## Hive task callback guards and newly accessible Chip price reference

First-four task callbacks in `shared_world_progress.rs::callbacks` have
`requires_world_admission:false`, empty world_requirements, `native_rule:None`,
and no world_change effect. `world_callbacks.rs::world_change_requirement`
does not map these objective keys. Their actual requirements remain the active
personal task track, bounded objective count, exact position/appearance and
carried reusable tool (tools13974/13976/14052/14051 respectively). Hatchlings
add their separate exact marker/tool requirement. `npc_command/hive_shared.rs`
selects acceptance/turn-in using actual tracks and task counts; no calendar or
World stage check occurs in that NPC selector itself.

There IS another boot/projection prerequisite besides shared_progress:
`apps/game-server/src/node/raid_scheduler/world_changes.rs::project_map_parts`
returns without installing anything when `!day.world_counters`. It skips
unadmitted counters, then installs every Hive callback source with `active=true`
regardless of shared stage. Therefore stage0 with admitted world_counters has
the Hive objects, but reset0/default world_counters=false does not. A personal
programme must explicitly project the existing exact source cohort as well as
exempting its personal task receipt; do not fake client object names or bypass
source custody. `dispatch/.../quest_callbacks.rs::shared_added_source` verifies
server Added identity encoded position/appearance, exact expected generated
callback equality, nohouse and current movement custody. In
`map/facts/quest_callbacks.rs::create_world_change_callback_item`, those objects
are made nonpickupable and bound. Completing Hive USE leaves the shared task
object present; execute.rs removes only non-Hive shared objects. No additional
first-four terrain-tool/native-rule stage guard was found in these inspected
paths.

### Fresh ordinary public HTTP evidence supersedes earlier inaccessible pages

On2026-11-08 normal public requests (no access bypass) returned real articles:

* https://tibiopedia.pl/npcs/Chip — HTTP200,72627UTF8bytes,
  SHA256`1a642e3847d832d1689c5386c5f789aa2bfb224e883d175a7ef8cdf8fc6afc74`.
  Its actual HTML `table id="table_buys"` row links
  `https://tibiopedia.pl/items/Wood` and literally displays `Wood (50gp)`.
  Its sells tab says `Brak.` (none). Article description says Chip appears
  during Lumberjack Mini World Change and trades wood to rescue his abducted
  family; informing the queen leads to Whistle-Blower.
* https://tibiopedia.pl/items/Wood — HTTP200,
  SHA256`d6dadc9dc84c406bfbdeeb9428ec3d072fa9fa2f5d6cf04fba8a3f96d0361abe`.
  Under `Sprzedaż` the actual `item_trade` table columns are Miasto/NPC/
  `Cena [gp]`, with row `Carlin | Chip | 50` and `Tibia | Yasir | 5`.
  Under `Kupno`, it explicitly says the item cannot be bought from any NPC.

This is specific item-level shop-price evidence for Chip buying Wood at50gold,
stronger than an unspecified quest payout. Neither page states a special batch
quantity. A native one-item priced trade row can be explicitly selected from
this public price reference, with catalogue identity i5901 established above;
do not claim the reference contains an executable Lua offer or a source batch
quantity. Earlier catalogue assertions of unknown price describe the older
retained data, not this newly fetched article. Keep runtime admission and
transactional stock/trade implementation explicit; the native Chip services
array was still empty when inspected.

Bounded GitHub code searches across whole current public Canary/Crystal
repositories for `lumberjack` returned only quest/storage catalogue matches,
not executable Chip/world-change shop scripts. The pinned filtered tree path
scan also found no lumberjack/Chip script. No executable-source price evidence
was located. These negative searches are bounded observations, not proof no
implementation exists anywhere; the now-accessible public shop tables supply
the concrete price reference without inventing50gold per undefined quest event.
