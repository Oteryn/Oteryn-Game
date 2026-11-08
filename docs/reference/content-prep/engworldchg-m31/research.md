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
