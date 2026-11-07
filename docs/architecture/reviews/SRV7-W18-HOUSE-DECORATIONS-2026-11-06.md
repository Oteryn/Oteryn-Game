# SRV7 W18 — native House decoration population

Scope: current-world House decorations. Crystal reference revision
`201189f871d23597b82caf88fd611454a2ddefb8`; Canary cross-check revision
`47dfd51f45280a59a1d3e50ba7edd573d7234446`. Donors are compatibility hypotheses
(`OTS_HYPOTHESIS_ONLY`); the retained Wiki observations below are independent
`TIBIAWIKI_STRUCTURED` evidence. No donor Lua, sprites or appearance banks are
redistributed by this change.

## Source rules and production owners

| Rule | Evidence | Native production path |
|---|---|---|
| Wrap catalogue, including directional variants | Crystal `data/items/items.xml`, `wrapableto` attributes; oven6355 at17630; complete1774 source ranges (XML overrides plus official wrap/unwrap defaults) retained by `furniture_item_profiles.py` | `house_wrap::execute` → `wrap_house_furniture` → current SQL `game_house_furniture_wrap_pair` and existing immutable wrap receipt |
| Construction kits | Crystal `data-crystal/scripts/actions/other/construction_kits.lua:1-58,63-92` (56); `data-global/scripts/actions/other/kits.lua:1-14,18-33` (12 present/pinata/dragon kits) | Existing explicit House placement command; real carried source kit is accepted only for its exact populated source/result pair |
| Ordinary USE | Crystal `data/scripts/actions/items/decay_to.lua:6-482,487-501`; remaining native `transformonuse` XML attributes | `use_house_event_item` → `consume_transform_in_house`; source/current-generation policies and current named-House custody are re-proved |
| Rotate, independently of USE | Crystal `src/game/game.cpp:4521-4573`: current item/type, unique-id refusal, owner/invited House access, reach and `rotateTo` transition | Capability384, command669, opaque observed map target → `rotate_world_item` → dedicated rotate planner and same House receipt owner |
| Stable light state | Crystal `decay_to.lua:53-54`, table-lamp2934/2935; `items.xml:6375-6378` | Definition change remains on the same instance and is published in the existing native map view |
| Timed light/animation | Crystal `items.xml:6273` (lit candelabrum), `decay_to.lua:487-501` calls `decay()` after transformation | Existing `game_item_timed_states`/write records, source action receipt, and the existing generation/session-fenced House tick |
| Container rotation | Crystal item XML and appearance-bank container facts, captured source capacities | Preserve the existing container instance and all child custody; target capacity must contain all current children at admission and commit |

The generated capture contains662 deterministic USE pairs,29 closed side-action rules,753 admitted rotation pairs (741 official-qualified donor pairs plus12 current Wiki easel states),68 construction kit pairs,31 folded-carpet placements and2250 native profiles. The source acquisition census contains905 wrapped offers across30 NPC identities:904 from29 normal retailers and one from the seasonal Messenger of Santa. Actual world placement is qualified separately below. Bed,
mailbox and dummy rotations stay with their owners; a floor trapdoor and the
existing podium state action are not treated as generic decoration transforms.
Of these decorations,124 have both USE and ROTATE. A separate native Rotate
command is necessary: ordinary USE of a bookcase must still reach its container
owner, and USE of a lamp must not silently select its directional rotation.
The existing four-chair MapBase USE rotation compatibility path is retained.

Current post-main integration numbers: capability388
`WORLD_ROTATE_V1`; command670 `ROTATE_WORLD_ITEM`; mechanic ordinal646; offer
order643. Migration0992 only. No new transport message is required: the native
rotation command uses the existing bounded opaque-target and UseResult codecs.

## Preserved native interface and authority

The native House placement interface is the already accepted one in
`docs/contracts/protocol-oteryn/v1/house-furniture-wrap-v1.md`, migration0595,
migration0721 and `gameplay_transport/house_wrap.rs`. It places a carried kit
atomically at the selected House tile. This translates the donor's floor-first
construction → generic-kit → unwrap UI sequence into the existing native House
placement command. SRV7 does not add remote placement authority or change its
House access, ACL, occupancy, inventory capacity, generation/session and source
instance checks. This existing interface translation is retained by the owner
for SRV7; no new floor-first model is introduced.

A generic wrapper obtains its exact unwrap target from the latest immutable
wrap receipt, ahead of the original shop target. An item rotated after purchase
therefore wraps and unwraps its actual current definition, including every
directional variant. A direct constructor cannot name another kit's target.
Both live endpoints must belong to the current native Item generation.

Timed decoration actions retain the exact before timer in the immutable action
or wrap receipt. Wrapping pauses the remaining budget; placement/unwrapping
starts or resumes it. Expiry uses the existing timed Item rows, with an immutable
House/session/node-context receipt because timed write rows do not contain that
House authority. Item identity, mint lineage, location and reclaim provenance
stay unchanged for a transformation. A burn retires the same instance and
removes its exact former custody. Complete cosmetic cycles of clocks/tesseracts
are skipped after a House was unoccupied; the original elapsed phase remains
source-authored, rather than resetting the clock on entry. There is no additional
clock or scheduling table.

## Recorded source conflict: festive table30231

Crystal XML says30231 wraps to35027. Both pinned donor appearance banks lack35027;
no such native object is fabricated. Sister states30229 and30230 wrap to23398.
The retained structured Wiki fact in
`imports/tibiawiki/facts/items-stats.json` identifies all three states as the
same **Festive Table**, page96628, revision1020057, content SHA-256
`686a45779cd467f4f85791f2eb3c6bb2f356d222896d1328fe570139877323a3`.
The owner accepted normalization30231 →23398 using that independent shared
identity and the two admitted sister-state wrappers. The immutable wrap receipt
still retains30231 as its exact unwrap target. This is not an alias or an
invented35027 appearance.

Other XML-only appearance ids12699,34277 and37883 are not materialized. Capture
records their absence; an old donor XML entry alone does not admit world content.

## Current-world nonadmission: legacy bed modification kits

Legacy `data/scripts/actions/items/bed_modification_kits.lua:1-7,10-49` names
831/832/833/834/17972. These ids have no current native Item policy or reachable
NPC/dialogue/quest acquisition. The retained1253-page BR NPC trade census
(`imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json`, `pages`)
and the independent Tibiopedia census list current beds or separate head/foot
sections for Edvard, not legacy modification kits. The BR Edvard observation is
page11920/revision353458; Tibiopedia's `NPC: Edvard` lists the current footboard/
headboard offers at12134 onward in its captured facts.

The owner retained these legacy kits outside current-world admission. No new
persistent paired-MapBase overlay is invented for inaccessible legacy items.
Existing bed sleep/occupancy remains with its owner; generic decoration mutation
explicitly excludes bed parts. Direct live Wiki API checks for
`Bed_Modification_Kit` are **wiki-pending**: Fandom returned402 and BR returned403;
neither protection was retried or bypassed.

## Validation

Focused runtime and native play checks are pending the final integrated tree.
The initial attempted chair wrap regression was corrected: the donor only
registers chair2360 as a construction result, not as a wrapable source. The
source-valid roundtrip case uses bookcase6109 ↔6115. PostgreSQL baseline admission
rejects that pair; the populated admission accepts it. The separate real framed
caller test validates the final behavior, source identity, custody and replay.

## Additional source qualification and integration evidence

The source capture admits official wrap/unwrap flags with the donor default23398
(Crystal `src/items/items.cpp:237-242`) before exact XML overrides. A definition
absent from both pinned donor XML catalogues is not fabricated from appearance
flags alone. Legacy bed modification items831/832/833/834/17972 have neither
current native admission nor a current acquisition source; current Edvard offers
the complete bed furniture. These obsolete kit rows do not authorize a new
paired MapBase bed mutation model. Live wiki checks refused by the provider are
recorded as wiki-pending.

The generated `furniture-rules.json` records the accepted Dracoyle rotation
correction: charged9034↔9039 and inactive9035↔9040. XML rotation alternated charge
families, contradicting their descriptions, seven-day decay targets and retained
independent Wiki pages85866/revision1115889 (charged) and30395/revision1148769
(inactive). Rotation must preserve charging and its original absolute deadline.
Gem677 enchanting remains the sole admitted charging action.

The shipped-world B3 census is retained in
`SRV7-W18-WORLD-DECORATION-CENSUS-2026-11-06.json`:1208 regions,19373519 tiles and
24983331 item placements,41.945s. All eight legacy mirror source definitions have
zero placements with a nonzero native house ID, and no kit/shop/loot admission.
Positive House-tile controls independently confirm that the source House field
is populated in current shipped maps. Global non-House mirrors remain with the
parallel world-novelty owner rather than being described as player House items.

The folded-carpet path preserves the existing accepted placement constraint:
full pinned base-cell walkability, blocked-door and live-occupant preflight, and
one owned furnishing per House cell. The donor overlapping-carpet removal arm
cannot be reached through that admitted placement owner. No source trap3481/3482
behavior is imported from the parallel W08 task.

Portable furniture death-loot admission is reproducible through the canonical
`build_full_spell_artifact.py --refresh-loot-tables` hook
`srv7_furniture_loot.py`, using only exact source-qualified Item bindings with
known pickupability and existing CharacterInventory admission. Source capture
does not rewrite canonical loot outputs. The qualified refresh SHA-256 before
final native materialization was
`a999e2ef5c2bf5fc3500111f07debe958d8cef3f369cf1b689828ba5b276f42d`.

Minimal presentation foundation is DUPLICATE-ON-BRANCH from
`claude/happy-davinci-ijz3wy-srvpkg4-sv8` at
`813a2d4b1a121e96f807ad2248f77415f37af389`, renumbered source capability350→385
and state domain104→105. Only the strict magic-effect codec, session/native
domain admission and actual effect publication are required by these House
side actions. Unrelated novelty behavior and S17 casting are excluded.
Rotate384/669/mechanic642/offer639 were allocated before PR3013 independently
claimed the same values. The post-main owning next-free audit renumbered only
SRV7 Rotate to capability388/command670/mechanic646/offer643; main mechanics
status384/669 remains unchanged.

Validation remains in progress; this receipt is not a declaration that pending
real PG/native capture regressions have passed. Valid constructor semantic RED
was73.880s (NotPossible versus Committed); explicit House Rotate real caller
GREEN was77.534s. Setup failures such as NativeInputDigestMismatch, capability
floor mismatch and malformed fixture commands are not counted as semantic RED.

Official non-retail Rotate exclusions:18 XML pairs lack `flags.rotate` in the
pinned official appearance bank. The complete source and world census is
`SRV7-W18-NONRETAIL-ROTATE-CENSUS-2026-11-06.json`; all have zero current House
placements, no acquisition NPC sales/construction/loot. Gnomission buys three
legacy trophies from players; those are not acquisition offers. Profiles remain
for their other owners. The native affordance and server rule both refuse these
XML-only rotations rather than inventing official flags.

Fish-tank semantic RED59.504s and harp semantic RED60.012s reached the actual
House source owner and refused NotPossible before the committed cue hook was
restored. The Edvard72.593s failure was a full-project fixture ordering error,
not a semantic NPC RED. The later genuine HI/TRADE/BUY guard RED94.770s justified
its production hook; exact wrapper/cost/origin/replay then passed114.039s. That
fixture used a synthetic room; authentic merchant placement proof is separate.

Current retail artist easels use six symmetric two-state rotations, preserving the bare/canvas and portrait/quality identities. Both pinned XML donors omit their targets; each endpoint has the official Rotate affordance. The independent 2026-11-03 Wiki SQLite snapshot at `tibia-sh/tibiawiki-data@31748fd6abc1a18378ecade240c805a7aab6bf20` supplies six exact two-ID article families. Full immutable source receipt: `SRV7-W18-ARTIST-EASEL-SOURCE-2026-11-06.json`. The source-rule regression failed before population (`14687` had no target), then passed after capture admitted all twelve endpoints; exact filter: `python3 tools/content-migration/test_furniture_easel.py -v`. The repo capture/check reports 753 retail Rotate transitions and 2250 item profiles.

The actual shipped B3 source has two bare easels at `(33572,31867,12)` and `(33573,31867,12)` outside houses. The focused real-source compiler test passed in 33.61s and emitted `/tmp/srv7-easel-real.bundle`. The production-caller fixture uses real MapBase pickup and preserves that minted UUID through the actual House handoff before Rotate. Its explicit fixture staging between source regions is not evidence for a travel route.

Actual House fish-tank and harp framed USE tests passed in 60.242s and 59.913s, respectively (`srv7_house_fish_tank_committed_cue_and_replay`, `srv7_house_harp_committed_cue_and_replay`). Their committed publisher captures include exact domain105 bytes, actual HouseMap snapshot, independent viewer position/House/generation, and replay suppression. Native rendering verification uses these captured world bytes rather than centering a synthetic camera on the effect. Remaining timer/cost lifecycle and native play checks stay pending until their real caller tests pass.


## Actual normal-retailer placement closure

The original50 static placements (including17 Summer additions) and21 timed placements contained none of the29 normal furniture retailers. The seasonal runtime already places Messenger of Santa; therefore the pre-change world reached only one of the905 source furniture-offer rows. Neither WORLD-SRC5e60c197 nor QuestPC6c09e036 supplied the missing normal-shop placements. Source metadata and the synthetic Edvard trade test were not proof of a live-world merchant.

The owning placement converter now appends29 original retailer positions after the unchanged base/Summer shards, yielding79 placements/78 NPC identities. Crystal201189f8 `data-global/world/world-npc.xml` supplies27 home cells; Canary47dfd51f `data-otservbr-global/world/otservbr-npc.xml:115` supplies John(Bounac). Nicholas is absent from both donor placement XMLs: the current2026-11-03 SQLite snapshot31748fd6, NPC article68008 (active, timestamp2025-07-26), supplies33640/31898/5. The older retained BR coordinate is recorded as a source discrepancy rather than silently substituted. Dorbin's OT-only13944/14036 placement is excluded; the current retail32783/31236 home is retained. Exact per-row source hashes/lines and Wiki coordinates are versioned in `tools/content-migration/furniture_npc_placements.json`.

A complete read-only shipped-map audit found original walkable ground/House0 and a public talk cell within the production range3 for every retailer. Zaidal is a counter merchant with no adjacent open tile, but twelve valid talk cells within3; no stair, doorway or adjacency is invented. The production compiler regression first failed0-versus29 in0.17s. After source placement population it realized27 and held exactly Fayla/Hofech as BlockSolid (5.02s): their original ornate/red carpet fixtures17396/2579 lacked native nonblocking facts. The existing full-source furniture capture now qualifies only those two additional physical records, Ground-only and nonmaterializable, from Crystal XML5336–5338/35502–35504 plus pinned official flags. Actual active materialization and the subsequent compiler/runtime checks remain required.


## Integrated source and caller verification through cohort31

The final source-only physical supplement contains ten exact nonblocking,
nonpickupable Ground records: the two retailer carpets above, seven original
cosmetic-NPC center fixtures and Gnomilly's floor15846. All2250 captured records
remain source-qualified. The cosmetic population appends56 normal retail NPC
poses after the79 existing placements; separate acquisition support adds
Gnomilly and Norbert, giving137 placements/136 identities. Original shards are
byte-preserved. Developer alternate poses, Testserver Assistant and Dawnport are
excluded. Population does not waive existing quest or instance entry gates.
The owning full-family compiler qualified all56 in4.985s and both support NPCs
in4.383s; authentic source-region export passed46.185s. The29 furniture retailers
qualified in4.584s. The real-world Edvard native HI/TRADE/BUY/replay caller passed
109.036s, independently of the earlier synthetic trade fixture. Actual server
NPC/trade/inventory frames also drive the native client presentation test.

Golden display population adds the source's sixteen USE transitions, guarded by
actual Golden Outfit storage3 rather than the currently worn outfit. The source
registers item IDs, with no action-ID requirement. Current native House tests
passed64.247s (full outfit, definition change and effect171) and61.282s (locked,
unchanged item, source feedback and POFF3), after genuine source-population RED.
Royal acquisition remains a separate boundary: King137–139 uses unWrap31510;
Queen84 and Emperor89 use legacy actionId36345. Normalizing all three rewards to
King's working31510 equivalent is recorded explicitly. The native Character
Inbox translation follows the existing DAILY_REWARD0 R1 decision; the source's
20-current-entry free-slot condition is independently enforced. The genuine
500-million-gold native King purchase reached the missing-kit baseline in
109.770s. Its paid-source/Inbox child composition passed the genuine native base caller
in141.647s. The coupled1b test also passed all three purchases, bank0, storage3,
one kit and actual native Inbox retrieval before a test-only wrong House command
id stopped the continuation; House installation and USE remain pending.

Focused current caller passes include actual Artist MapBase pickup→same UUID→
House Rotate70.833s; plain House MOVE9 Place/Take/replay72.974s; table lamp83.478s;
static Farm Torch BASE USE/CAS/replay105.069s; original charged Dracoyle absolute
clock/House Rotate/tick63.170s; historical completed-clock exclusion93.728s;
legacy existing-clock House placement59.412s; two House gem transformations
62.674s/58.941s; genuine charged-loot corpse pickup and original deadline119.276s;
and carried enchantment→Rotate→expiry→inactive Rotate57.940s. Controlled overdue
clock fixtures are identified as clock fixtures, not elapsed real-world days.
The closed all80 portable receipt comparison passed53.76s, including strict
public/private metadata rejection. Its off-state3046 correction changes only
the exact retained ambiguous legacy temporal shape to the pinned inactive
source policy; active3047 and paused remaining budgets retain their owners.

The high-level130 lethal House gift now proves the actual source effect,
current-town temple and original dropped-backpack/direct-or-nested child rows.
Authorized observer admission remains under diagnosis before its native
OPEN/pickup/deadline continuation can be claimed. Source-policy validation and
all final integration checks remain pending; this report does not mark these
unfinished acquisition/custody callers DONE.
