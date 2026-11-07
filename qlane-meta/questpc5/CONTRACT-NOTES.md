# Active L5 contract decisions

- Keep investigations within ten minutes. Use pinned donor facts first; where a fact is absent, record the conservative native rule here and continue with a PostgreSQL test.
- Static startup IDs are separate from World occurrence/change state. Do not modify worldchg/wvar/wcontrib/wevents mechanics.
- Restore explicit startup selectors during World compilation, preserving source B3 bytes. Crystal `data-global/startup/tables/corpse.lua:86-94` explicitly says the OTBM stopped storing IDs. The served map contains the exact source appearances and positions, with missing attributes. This is missing startup materialization, not a new quest rule.
- Pinned source: crystalserver `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`; skeleton UID20003, appearance4285 at32211,31273,10 (`corpse.lua:87-89`); forcefields3059/3060/3061, appearance1949 at32157,31126,floors9/10/11 (`teleport.lua:387-399`). Source blob hashes and exact lines are generated into `content/world/quest-startup-bindings.json` from the retained corpus.
- Reuse QENGINE's explicit startup-table projection, with compiler-owned output instead of B3 rewriting. Reject ambiguous objects, houses, conflicting attributes and unbound provenance. Preserve every unrelated attribute and use the same projection in compilation and equivalence checks. Compiled sector bytes bind the selectors into the existing World digest; regenerate pins with repository tooling.
- No behavior assumption is needed for these source objects. Missing private/source bytes will not become a stopping condition; the owner authorizes documented conservative rules for later entries.

## Literal written source leaves

The closed `native_source_leaf_v1` format derives one Item at count1 and its exact normalized text from a pinned literal Lua table entry, plus exact physical selectors from the startup projection. Its claim receipt owns once eligibility; the existing item-only claim transaction records the existing progress obligation. The shared monotone flag lowering teaches the new dedicated instruction track. The source's storage assignment remains an independent required hoist instruction, alongside the construction plans.

The source leaf is independent of the historical chest pilot. That migration processes its original pilot formats; the new producer validates this format against the retained donor bytes. Current-source definitions and historical dispositions remain intact. This is a reusable literal written-reward format, with closed fields, text digest/carrier checks, native Item admission and no charge-state inference.

## Hoist direction switch and complete route, 2026-11-07

The source movement program reads the remote lever at32156,31125,10: appearance2772 selects floor11, otherwise floor9 (`data-global/scripts/quests/formogar_mine_hoist/movements_hoist.lua:18-23`, Crystal201189f871d23597b82caf88fd611454a2ddefb8, blob5fd560ef6dbdf27bcb58f0f14447750fe90dc062). The reusable reversible USE pair is source-defined by `data/scripts/actions/items/decay_to.lua:33-34`:2772→2773 and2773→2772. Verified exact pinned URL https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/scripts/actions/items/decay_to.lua (SHA256563402b5c26dadadf753422e76384a962c7f415e5038ead293dc897fef95f27f). No new gameplay assumption is needed for the switch.

The retained physical World cell initially contains5770,5772,2773, all top-level and without script attributes. The lever is ordinal2 and initially selects floor9. A focused physical-map regression binds these facts. Position/appearance callbacks must therefore accept either member of their declared reversible USE pair, including an initially reversed source object. The matcher remains constrained to the declared position and clicked placement; a ground click cannot match the lever. Source UID callbacks retain their existing immutable attribute checks.

The composed hoist fixture includes both actual source rewards and all three projected source forcefields. It abbreviates navigation between the distant objects through the existing movement-proof owner; it never seeds instruction flags. Plan acquisition and cold replay retain all original exact custody, readable text, once, stale-fence and atomic-refusal assertions. The continued player command stream then earns the separate skeleton instruction through real USE, checks refusal with only the plans, traverses each hoist, switches the real direction lever twice, and verifies travel/switch operations do not change quest tracks, receipts or Character revision. The previously recorded conservative construction-plan bridge remains explicit and keeps both independent instructions mandatory.

Fixture correction proved by the complete-route run: bind cells to the activated content generation, then retain the live cold owner with its actual command-ingress journal. Keeping only AdmittedSession and rebuilding the owner correctly fails callback-resume recovery; the fixture now preserves both. No production scope, controller, recovery or command-counter check was relaxed. Complete route PG65.01s, including actual instruction/travel program3.564828244s.

## Present source exchange, 2026-11-07

Crystal `data-global/npc/seymour.lua:100-107` at9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d (SHA256d7593e3c151aca33ba89d5b856cab39d57e286af806e145042b18071effb3632) asks about a box, then consumes one2856 and grants one3374 on yes. The existing chosen Present recipe explicitly selects this exchange and a once completion; the native final flag enforces that chosen scope. Native offer keywords accept both source `box` and the chosen recipe's `present box` phrase. Seymour's already admitted source placement is32104,32189,z7, backed by `content/world/npc-placements/placements-00000-00033.json` and its retained Canary/Crystal provenance. The test abbreviates only travel to that exact NPC cell; it must earn the Present through the actual6113 claim, consume the nested instance atomically, retain both original water-subtype1 children and the torch, and prove replay/once behavior. Existing shared item exchange and reward MINT owners are reused.

## Next Power Ring reconciliation decision, 2026-11-07

The chosen Power Ring recipe names Power Ring and Bronze Amulet. The retained Wiki object sources resolve inactive Power Ring to3050 (page2184/revid1205616, wikitextSHA2563d73b74371a94193b8a4fdff3ec37bb79cb0dbf0254d3d0a603599d87f9639f0) and Bronze Amulet to3056/200charges (page2466/revid1194240, wikitextSHA2565fa91126008b798f1c9e3950dcd01757133a7aa0d0d23a4ae6baf3c4f9bb71c9), in `imports/tibiawiki/quest-reward-items/fandom-snapshot-r1.json`. Both pages explicitly name Power Ring Quest as an acquisition source. URLs: https://tibia.fandom.com/index.php?curid=2184&oldid=1205616 and https://tibia.fandom.com/index.php?curid=2466&oldid=1194240. A current Fandom request returned HTTP402; retained pinned bytes supply the facts without a blocker.

Decision for the next native authoring slice: select canonical3050x1 at source chest6151 instead of the donor's3031x1 reward, preserving that discrepancy in source evidence and using an explicit bounded selected-reward projection; retain3056x1 with its exact full-default200charges at6152. Inactive3050 equips to3087; the existing admitted timed-item packet gives the active form1800000ms and an unequip transform back to3050 (`docs/agents/evidence/OTV2-20261003-timed-item-facts-v1.json`). No new timed mechanic or invented item is needed. This is a recorded implementation decision, not qualification evidence; the next slice must run PG RED/GREEN on the actual selected chest rewards and once behavior.

Present qualification PG98.32s (marker4.345191309); actual native claim/exchange/replay program37.446318748s. Original source water/torch instance facts survive the nested box burn. The source NPC loader needed a finite measured scan-budget update232→243 for the committed startup file across11 locator lookups. The boundary test still rejects242 and loads243. This changes no per-directory, JSON, document, reference or import limits.

## Power Ring native selection and exact defaults, 2026-11-07

Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d `data-global/startup/tables/chest.lua:1755-1767` (SHA2560ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca) binds6151 at32599,31776,z9 and6152 at32601,31776,z9. Line1759 is literally `reward = { { 3031, 1 } }, -- power ring`; line1766 is `reward = { { 3056, 200 } }, -- bronze amulet`. Thus the native selection repairs a documented donor ID/comment mismatch using the pinned Wiki's inactive Power Ring3050; it preserves3031x1 in `native_selection.original_reward` and never runs an additional parallel grant. The Bronze Amulet remains the source exact3056 full-default200-charge interpretation.

The reusable `native_selected_leaf_v1` accepts only a singleton plain once leaf. Repo tooling requires the exact original reward and Crystal selector, a materializable selected Item with compatible stack facts, the exact hashed Wiki page/revision/item ID and an explicit matching once reward in the owning canonical quest recipe. It records the original amount/hash, source identity/revision/position and Wiki snapshot digest. Original donor imports and other formats are untouched. The runtime's closed bounded decoder checks witness/selector/reward equality and original amount hash, then delegates to the existing ordinary claim parser, native Item admission and once MINT owner. Unknown fields, stateful/container projections and mismatched selectors/IDs fail closed. The committed-data validator independently rebuilds this projection; the historical migration still sees the same original pilot identity.

The chosen two-source quest is tested with one Character: actual6151 claim/cold replay, actual6152 USE and refusal on reuse. It keeps the original Ring instance, proves both receipt keys occur once, proves the exact200-charge Amulet default and absence of fabricated partial timer/charge rows, and checks that no gold is granted. The admitted active Ring3087 supplies the existing1800000ms equipped-time budget with stop-while-unequipped; inactive3050 is minted and the existing equip/unequip owner remains authoritative.

## Thais committed occupancy projection, 2026-11-07

Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d `data-global/scripts/quests/thais_lighthouse/movement-stair.lua:6-16,24-33` (SHA256 f12470dbceaf81ce90577ac5e55c7452a64916f7738cff9905e63d6a00718eb3) accepts any creature, without an isPlayer guard. UID25010 opens the source stair429 to7767 at32225,32282,z9 and transforms the plate to430; departure restores429/431. URL: https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/thais_lighthouse/movement-stair.lua .

Conservative native rule: pure paired pressure projections follow committed living presence in the current pinned scene. Reserved destinations, dead creatures, retired actors and other frames do not hold a plate. Ordinary positioned NPCs do count. Repeated observations are idempotent; scene observation and command ingress reconcile through existing runtime/lane locks and the existing atomic map-effect owner. No additional durable flag or quest receipt represents occupancy. Pair admission is closed to an exact position/appearance selector, one StepIn and one matching StepOut, transformations and replacement passages; all unrelated effects/gates are rejected.

The two source treasure selectors6014/6015 are derived by the existing startup producer from chest.lua; source placement data is not rewritten. Whole Thais qualification additionally requires the composed actual lever, native descent and both once claims; this mechanic slice alone does not qualify the quest.

Fixture qualification correction: the generic World boot ground is `terrain:grass`, which has no source item ID and intentionally yields unknown spell-tile facts. Native creature relocation correctly refuses such a destination. The occupancy fixture now names ground356, corroborated at physical Thais neighbor32224,32267,z9 by `content/world/placements` and its canonical palette. It keeps the original plate object/UID and every original player descent/StepOut assertion. This corrects test input qualification; movement admission is not weakened or bypassed.


## Whole Thais route qualification, 2026-11-07

The chosen canonical recipe has no level/prerequisite requirement and selects Dark Shield3421x1 and Battle Hammer3305x1 once. Source chest.lua:685-700 at Crystal9f5 (SHA2560ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca) binds the actual6014/6015 placements. Source action-lever.lua:5-46 at the same revision (SHA25670d0d169b8a3fde2e5f121c0c2a8646b77803abbfc3defa87086cba1f1500ac5) supplies30004 ladder351/7767 and30005 paired1949 portals. URL: https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/thais_lighthouse/action-lever.lua . The native treasure claims are unconditional; preserve their original cold/cohort assertions, compose the puzzle before the second acquisition and add no fabricated storage gate. Navigation is abbreviated through the existing fenced relocation owner; every puzzle interaction, descent, portal traverse and grant is a literal native command.


## Grouped pressure and source-owned remote objects, 2026-11-07

Crystal9f5 `data-global/scripts/quests/demon_helmet/movement-walls.lua:15-38,44-64` (SHA256b93439a8c9210be489b490b2983e4310622c671a49a6339ad869479ef502857b) requires two players at33190/33191,31629,z13; creatures cannot substitute. It removes1295 at33210/33211/33212,31630,z13. Lines53-54 eject wall occupants to33211,31631,z13 before restoration. Native forced world ejection retains current actor/frame/revision and native landing qualification, allows the literal shared destination (source stacking), and prequalifies every moved occupant under the existing lane/runtime owners. It does not weaken normal client-command teleport occupancy checks. The physical selected World has ground10145 at both plate cells; identity transformations preserve that source appearance.

Crystal9f5 `data-global/scripts/quests/devil_helmet/movements_sewer.lua:5-19,27-40` (SHA25671542a6acba0cd100b6f2d86339473e86781c777fa10aa339a62ea02d54e1e84) requires a player and creates/removes435 at32482,32170,z14. Startup tile.lua:2233 binds25030 to431 at32468,32119,z14. The existing source producer/compiler restores that exact UID. The generic bounded movement-item projection owns only its own created instances, preserves identities across unchanged observations, and removes them on committed absence, relocation or retirement; no extra durable truth is stored. Sources: https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/demon_helmet/movement-walls.lua and https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/devil_helmet/movements_sewer.lua .


## Exact helmet key access, 2026-11-07

Reused pc6 commit8aacc18bb's pinned key-door producer, source binding matcher and full native wire/replay/relog fixture. The complete3610 source group has two doors, so producer completeness checks selected group identity rather than assuming exactly one door per key. Exact new bindings are6010/5097 at33211,31634,z13;3610/1628 at32462,32151,z15 and3610/1631 at32462,32153,z14;3667/5106 at32592,32102,z14. Existing3940/3980 bindings survive. Capture witnesses and physical map hashes are in tools/content-schema/world-authoring/samples/quest-key-door-bindings.json, sourced from Crystal9f5 door_key.lua and storages.lua. The World consumer is restricted to this key supplement; compiler-owned quest startup selectors retain their owner. Only unambiguous exact locked items on unhoused tiles without conflicting actions receive the source action.

The pinned Canary999 item XML and appearance bytes were fetched from https://raw.githubusercontent.com/opentibiabr/canary/99902524e052f37574194466c2949c576e4ab269/data/items/items.xml and https://raw.githubusercontent.com/opentibiabr/canary/99902524e052f37574194466c2949c576e4ab269/data/items/appearances.dat ; both SHA256 hashes match canary-ground-proof.json exactly. Existing profile producers/migrations admit only1631/5097/5106 and regenerate Content. The previous five records remain identical. Input binaries stay in /tmp, outside the checkout.

Expertise source access (2026-11-07): Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d data-global/startup/tables/door_level.lua:173 binds1100 at33211,31638,z13; line27 binds1030 at32479,32174,z15. SHA256 d8868f675aae7c541ddc3e76b3648b6a886e0ae31a4a614747d17e1784d234fc. URL https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/startup/tables/door_level.lua . Exact physical appearances5102/5111 and open partners5103/5112 are imported through the existing pinned native profile tooling. Existing engine level comparison and departure closure are reused. PG fixture uses native CharacterCastFacts level, the same admission source as production; no progression SQL/guard changes.

Demon source acquisitions2026-11-07: Crystal9f5 chest.lua:799-817 binds6029/6030/6031 to physical2472 at33313,31574/31575/31576,z15 and exact3554/3387/3420 rewards. chest.lua:1161-1166 binds6076 physical2474 at33063,31624,z15,2853bag with2972/3114/3034x2/3049/3115 and keyAction6010. SHA2560ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca. URL https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/startup/tables/chest.lua . Existing claim definitions and durable MINT owner are unchanged. This source registration reuses the prerequisite bag without taking ownership of the Parchment Room quest/reset mechanic.

Demon encounter closure2026-11-07: content/world/spawns/spawns-52000-53999.json:1 SHA256e6dc3a9a16e769338d8852150031f4d3f93499a11e7bc8505a9db27cc7c8b07a declares exact four Demons33319,31573;33323,31573;33320,31577;33324,31576,z15. content/creatures/definitions/creatures-00000-00499.json:1 SHA25682a320ded4b2e08091b3eda9c145a03c5f40db52247692714250e2d66c417eef supplies8200HP, armor44,6000XP and corpse5995. Neither creature facts nor loot probabilities/quantities are changed. Canonical source admission closes the four-stage corpse5995→4097→4098→4099 and ten disabled loot leaves using each exact Item's own pinned native importer facts; no other Item is used as a template. Native policies already cover12of14; only4097/4099 are added. Canonical source flags/capacities/decay derive from Canary99902524e052f37574194466c2949c576e4ab269 data/items/items.xml and appearances.dat, hashes in quest-encounter-item-profiles.json. URLs https://github.com/opentibiabr/canary/blob/99902524e052f37574194466c2949c576e4ab269/data/items/items.xml and https://github.com/opentibiabr/canary/blob/99902524e052f37574194466c2949c576e4ab269/data/items/appearances.dat .

The combat fixture abbreviates navigation with the existing fenced source-step proof and a flat test arena at the exact source cells. It preserves native creature HP/defence/loot and uses a high-level fixture attacker, as the existing Radiant qualification does, to bound real auto-attacks. It never creates kill events or XP receipts directly. Runtime rewards consume the generated loot section; canonical admission therefore requires repo loot-section refresh and its dependent Prey simulation/pins, not a runtime bypass. Derived canonical-shard/definition descriptors are refreshed only after the guarded repo tool proves all shard changes equal this exact14-Item source projection; donor source hashes and captured native records stay identical.

Pinned Canary999 XML source lines16310-16314 provide5995initial15slots/10s→4097;12442-12456 provide4097/4098with24slots and300s each, then4099terminal60s. File SHA256bf773d2c8f1d2767b31a20749fd5278ae48f2ed5b05757a678e3788e2a70bf85. Existing Premium source travel checks guard Edron geography; no extra quest-specific Premium storage is invented on its source doors/chests. Native activation keeps its ordinary player headroom: the combat fixture declares10slots for four Demons, its already-admitted player and five spare slots. Production limits are unchanged.

Navigation provenance correction: all119 owner-reviewed navigation citations include SHA256 of the complete canonical definition map. Fourteen newly admitted facts therefore change those119digests even though none of their family assignments or owner decisions changes. The guarded canonical-input refresh tool computes both base and new complete maps and changes only matching old digests. The first broader taxonomy diagnosis was incomplete because it counted row identities rather than this shared citation; no unrelated taxonomy is reauthored.

Devil acquisitions: Crystal9f5 chest.lua:492-505 binds5016/5017 to4240at32576,32216,z15 and387at32589,32100,z14, iron keys2969 bound3667/3610. Lines1659-1678 bind6139→3029x4 at32459,32144,z15,6140→3356x1 at32465,32148,z15,6141→3269x1 at32466,32148,z15. Physical6140/6141 appearances are2480;6139is2472. SHA2560ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca, https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/startup/tables/chest.lua . Source typos devilhemet1/2/3 remain unchanged.

Known-fact repair: Source imports fill only Unknown materialization groups (physical/stack/container/temporal) in canonical definitions. Complete Known/NA groups retain their original owner, including partial Wiki equipment groups and imbuement slots3366=2,3414=1,7382=3. Canonical stack maximum holds remain Unknown; native profiles keep their separately qualified100ceiling. Existing native Known/NA always wins; accepted legacy facts fill only native Unknown. All unrelated native policies and original native attributes/bindings stay identical. Immutable legacy definitions supply the original facts; the exact earlier source-only projection is the sole recognized repair shape, not a blanket conflict bypass. Main tree passes; only static Imbuement current-Item evidence fingerprints remain to refresh.

Projected dropdown USE: Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d data/scripts/actions/items/ladder_up.lua (SHA256730bc7664484e953b6efc3c8faeba9936d355c5cdde91d1f3e0686f714f4df13), https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data/scripts/actions/items/ladder_up.lua . Item435 is explicitly registered and takes the non-ladder branch: same x/y and source z+1. PZ-locked players cannot enter a protection-zone landing. The reusable map utility resolves the current opaque map identity and uses existing actor-control, USE lane/rate/replay, native reach, collision/occupancy and sealed movement commit owners. The Devil plate creates435at32482,32170,z14; USE lands32482,32170,z15 while a second player remains on25030. No quest flag, reward, carried Item or new persistence is introduced.

Static source fields: the Devil key3610/UID5017 source tile is387small-hole ground plus2123fire field at32589,32100,z14. Existing qualified field profile content/test-packs/spells/r25/item_profiles.json:1397 at f3f6a43fb retains MAGIC_FIELD, immovable/non-pickupable facts and Canary99902524e052f37574194466c2949c576e4ab269 field recipe7x10fire/10000ms (items.xml SHA256bf773d2c8f1d2767b31a20749fd5278ae48f2ed5b05757a678e3788e2a70bf85). Destroy Field3148 uses the existing pinned remove_field native plan and its exact listed field references. Static removal is a map placement, never an invented ItemInstance: preserve native rune cost/LOS/PZ/replay, bind one map_fields descriptor to the complete spell intent, commit the existing terrain zero-tombstone CAS in that same transaction, and require a same-xact exact map receipt. Existing physical Item-line/outbox guards remain intact. Tombstone lifetime follows the existing Channel/reset-epoch map owner. Native standing-field contact must read composed live placements after recovery so a removed2123 cannot remain a fire source.


Newhaven required reward Item closure (2026-11-07): immutable Crystal source
https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml
SHA256 8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509
selects Guardian Ring52732 (weight90, ring), Scarf3572 (weight200, necklace),
and Book of Wisdom51977 (weight100, writable). Exact served client2dfa943b
confirms their names, portability and absence of container flags. Only these
three canonical admissions are promoted; accepted Known equipment and carrying
facts are retained recursively. Ordinary nondecaying item rule applies because
the pinned XML contains no decay/charge attributes for these identities. This
item slice alone does not qualify either complete Newhaven quest recipe.

Selected Newhaven recipe rule: canonical quests-00200-00299.json records/78
stages s1..s6 explicitly mark CHOSEN_OTERYN_APPROXIMATION and select ten Trolls,
Guardian Ring52732, Scarf3572 and1600XP. Crystal201189f Gustavo source
data-global/npc/gustavo_the_guard.lua:74-100 instead requests twenty Muglex
goblins and awards3052. That superseded branch is not transplanted. Reuse the
existing generic personal native kill mechanism from questpc6 commit390e024503a7c02b9e15d67e680cec0ac711fd5b
for the selected Troll goal; keep immutable donor source evidence unchanged.
Item source exact ranges: scarf11307-11314, wisdom book87353-87358, guardian
ring88295-88302 in the pinned XML cited above.


Shared Newhaven NPC reward composition (2026-11-07): generic optional
minimum_level is checked from the locked progression row inside the existing
quest transaction; experience lowers to the existing quest XP obligation;
completes lowers to the existing terminal quest flag. Default fields are omitted
from serialization, preserving all previous durable callback plans exactly.
The chosen tutor role binds to existing canonical Gustavo, The Guard. Four
source-derived callbacks provide welcome, tour confirmation, Guardian milestone
and Book hand-in. The selected recipe's1600XP is split800/800 across the ring
and scarf milestones. The component PG deliberately seeds ready quest predicates
to isolate this mechanism; it does not qualify source exploration or native kills.
Gustavo's literal greetings are extracted from the already committed full source
in batches/quests4/newhaven.json and its exact content SHA, not a synthetic reply.
The whole selected Legion recipe has requirements.prerequisites=[] and requires
both conversations. The old status text requiring full completion of both
separate source missions was stronger than that chosen recipe and is superseded.


Newhaven native goal reuses questpc6 commit390e024503a7c02b9e15d67e680cec0ac711fd5b,
with only this lane's selected Newhaven origins installed. Ten authored ordinary
Troll spawns use existing canonical Troll50HP/20XP and existing map cells. The
producer proves current-client ground flags, no protection/house/zone flags,
spacing from the existing population and an empty walkable adjacent approach.
Ordinary90-second respawn is the explicit conservative training rule; personal
quest eligibility still starts once and caps at ten. No source population or
map tile is replaced. ActualPGRED4.95s proved the absent death transition;
expanded actual deaths then reproduced corpse_item_inadmissible92.10s. Close
5960→3987→3991→3992→0 with source capacities7/10/10/none and durations
10/300/300/60seconds, plus held possible Troll loot3277/3336/3355/9689.
All prior37 encounter capture rows are checked unchanged. Wrong-species
refusal uses the already-qualified native Dragon closure, retaining the exact
negative property without requiring unrelated Rat corpse admission.
The selected tour flags are deliberately seeded only in this bounded native
goal component proof; whole Newhaven still requires actual exploration bindings.

Troll Item source: opentibiabr/canary@99902524e052f37574194466c2949c576e4ab269
data/items/items.xml SHA256 bf773d2c8f1d2767b31a20749fd5278ae48f2ed5b05757a678e3788e2a70bf85,
lines11894,11920,11926,16108 select3987/3991/3992/5960. Immutable URL:
https://raw.githubusercontent.com/opentibiabr/canary/99902524e052f37574194466c2949c576e4ab269/data/items/items.xml
The same existing pinned importer admits these eight Item facts; loot quantities,
probabilities, the charged-life-ring3054 exact200 charges and all previous
37 encounter capture records stay unchanged.


Newhaven actual exploration choices: training-school approach32570,32506→
32570,32507 (south), adjacent to source Viola32571,32507; supply approach
32568,32489→32567,32489 (west), adjacent to source Flavius32566,32489.
Crystal201189f world-npc.xml:2161-2163,2184-2185 SHA
db07b021856008bce61efe816e8a8eea00a01cb9750f1be5e750093758d1c029
anchors the NPC locations. The callbacks themselves are explicitly selected
Oteryn exploration objectives, not a claim that this XML contains Lua callbacks.
The committed B3 map and exact official2dfa client verify clear walkable POI
and approach cells. Their protection-zone flags are retained in the route
fixture; no world tile is changed. Stage0 and minimumlevel2 gate both once-only
visit flags. The complete route fixture starts level2 through the normal XP
owner and seeds no quest flags, counters, reward items or completion. It must
recover its actual Book through the existing source chest USE before hand-in.
