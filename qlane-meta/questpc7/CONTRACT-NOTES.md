# questpc7 contract decisions

- Scope is the owner's25 ACTIVE ledger entries, with entries already QUALIFIED
  on another branch skipped. Fresh qengine2 head9d3c3869f03a523601d5567cdb352d40854f3517
  qualifies Festive Outfits (RESUME.md:44-55, PG18.6 case89.658220585s).
  It is recorded as external qualification, not as locally reimplemented work.
- Reuse the reward engine's NPC value owner first. Preserve this branch's
  existing world callback signatures and read-only hoist semantics while
  integrating NPC exchanges; world effects are a separate subsequent slice.
- Explicit fluid and charge operands remain owned by the existing exact-match
  owners. New ordinary static reward constructors specify no fluid/charge
  instance override; they never infer a charge grant from an unrelated item.
- Read-only prerequisites belong in `requires`, retain their original quest
  ownership, and are checked under the same root and sorted track locks.
- No donor fact has been invented in this slice. Investigation is capped at
  ten minutes per fact; any later conservative authored rule must be recorded
  here before its focused PostgreSQL proof and publication.

- Wire-level outfit integration uses qengine2's fresh-character fixture with a
  creation-bound sex and the appearance packet rebound to the fixture content
  revision. Historical sex-less characters are not valid appearance-capability
  test actors. Keep the normal capability25, catalogue delivery and selection
  assertions; do not bypass the appearance owner or disable the capability.

- Palimuth is now globally qualified by the reused reward catalogue. Its older
  unavailable-greeting expectation becomes exact source greeting/farewell/
  walkaway assertions; the unrequested Spectulus refusal remains unchanged.

- Golden uses the pinned selected recipe's100000/50000/50000 gold-unit prices
  (`content/quests/definitions/quests-00100-00199.json:1#/records/52/definition`,
  SHA256afe08860377d1027a1698e4dca459f617d34f39a5ddbf987ae5af25a73be5e50).
  King Tibianus' donor accepts carried money plus Bank balance at
  `data-global/npc/king_tibianus.lua:131-189`, Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
  SHA256e6509e41311669a22f9c7ac6c1787fcf5b63120d06d802a1ad268e0adac9d9bd.
  Preserve that existing settlement policy with the chosen Oteryn prices;
  do not substitute the donor's500m/250m/250m economy. Each character's
  commission is once-only; earned appearance facts retain account ownership.

- Payment support reuses qengine2 commit33b640335 (head e06058c74),
  including the closed fee cause, exact price/bank policy, same-transaction
  source bridge and immutable receipts. Migration1099 preserves all previous
  carried conservation, custody and revision predicates; no new wallet is added.
  Golden's selected final confirmation writes completion atomically through
  the reusable callback `completes` flag.
- This checkout's hand-maintained fee-tag golden inventory additionally needs
  tag18 and its own codec fixture. Added both, retaining every pre-existing
  golden byte and exact inventory assertion. The imported qengine commit
  did not include this checkout-specific fixture update.

- Friends and Traders follows the chosen canonical linear programme at
  content/quests/definitions/quests-00100-00199.json:1#/records/47/definition.
  It retains all ten named priests in the exact selected order and folds
  collect stages into exact carried-item handins; no quest-state seeds are used.
  The recipe does not replace Gamon's price: retained Crystal9f5a72c
  data-global/npc/gamon.lua:64-68 (SHA256466de8702c51fc9c550f3f9f75e0301713eeec3a81bb8974fc848f10df5ead83)
  charges5000 via carried+Bank before giving Item5941. The same exact fee owner
  is reused. Sweaty Cyclops source:109-137 (SHAbc694bf7d62f6a0daae1eda9f6542c6bd79fb749805fe58c23275189ce1e5d0a)
  consumes3 Item3560 and unlocks3381->5887; Marina:105-116
  (SHA8574dd5721f258bef8d649657ed428537e3e351e540a990fe72bcc21c89b8682)
  consumes50 Item5902 and unlocks10 Item5879->1 Item5886.
  Source citations for every prayer are embedded with exact file lines and SHAs.
- Repeatable unlocked services use a separate bounded monotone receipt counter,
  preserving the finite programme track and its completed revision. Maximum1e12
  is the documented engine bound; every exchange must produce one fenced Add(1)
  receipt and cannot advance past its bound. Existing QuestEffectKind::Add owns
  the arithmetic and overflow checks. No alternative item/fee writer is added.

- The first counted-service PG attempt proved counter1 but the second legitimate
  exchange hit the shared MINT owner's once-only claim key. Reuse the ordered
  NPC exchange pattern: counted callbacks receive a deterministic claim identity
  scoped to callback/session/command; ordinary callbacks keep their existing
  once-only key. Migration1100 retains the exact native MINT proof and binds the
  command-scoped key plus an exact same-transaction bounded Add(1) quest receipt.
  No global claim policy, prior receipt, input predicate or constraint is removed.

- Unavailable-stage prayer and repeat-blessing words can receive ordinary NPC
  chat acknowledgement (NpcIntentDisposition::Ok) without a quest action.
  The PG contract checks zero new quest receipts, unchanged programme progress
  and unchanged stake quantities; reply category alone is not a grant proof.
  The fixture allocates14 actor slots for13 selected NPCs plus its player,
  using the existing bounded constructor; production carrier checks stay intact.

- Eligibility reuses the native transaction-scoped Premium latch (the existing
  quest premium trait), not a client/session boolean. Level is locked native
  progression; completed-parent facts are read-only under the Character fence.
  Make Believe canonical is `oteryn:quest.authored.make_believe_quest`,
  content/quests/definitions/quests-00300-00351.json:1#/records/15/definition.
- Illuminated source lookup was timeboxed. Retained Summer00ce02a57ca5a12e48f32a3476e37471167e4c3f:
  data-global/npc/wayland_smythers.lua:112-114 SHA9a6b7b1ff8c351bc44275c053a6992093e69984f6ae77a519dcdc9040be7b851
  confirms2500/1250/1250; its script and XML contain no Illuminated grant IDs.
  https://www.tibiawiki.com.br/Illuminated_Warrior_Outfits?action=raw and
  https://www.tibiawiki.com.br/Illuminated_Warrior_Outfits_Quest?action=raw
  confirm Premium, Make Believe and counts, with no look ID.
  https://www.tibiawiki.com.br/Scraps_of_a_Radiant_Attire?action=raw confirms
  weight0.30oz/nonstackable; no numeric Item ID is published there. Current
  https://raw.githubusercontent.com/zimbadev/crystalserver/main/data/XML/outfits.xml
  also has no Illuminated Warrior row. Do not spend further time repeating this
  lookup: author distinct Oteryn compatibility identities using existing admitted
  appearance references, preserving known weight/stack behavior and exact totals;
  add a bounded escrow if totals exceed the existing carried-tree bound. This is
  an explicit native adaptation, not a claim of an unknown original numeric ID.

- Exact canonical parent source: content/quests/definitions/quests-00300-00351.json:1#/records/15/definition SHA2564a24de17cc2af26d6122fea2502b95974c8b5db55682e3540da813c0f28ebe6a. Parent completion is a read-only prerequisite, and its fenced fixture precondition does not qualify that other-lane quest.

## World wardrobe adapter — 2026-11-07

Reuse qengine2's fenced world-outfit transaction pattern (origin/claude/happy-davinci-ijz3wy-qengine2 at412fba411), without replacing this lane's read-only hoist/remote selector or asymmetric passage rules. Only the outfit participant is admitted here; unrelated portal/encounter effects are not implicitly enabled. The retained shared wardrobe producer emits an explicit top-level outfit_reward. The regression uses the retained Warmaster wardrobe and a fenced fixture of its independently owned prior mission; it does not qualify or claim that other quest.

Formal Dress fact budget is closed: Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d data-global/scripts/quests/25_years_of _tibia_quest/lord_retro_death.lua:3-10 (sha256019afff6aac0e7ad9bd6e53edfb60ecf79094ab5a8393594238ac37c5ae78545) grants base looks1460/1461 from Lord Retro; usable_outfit_items.lua:51-52 (sha25669b5b133b329a76628dcd678f13d86bc3e5bd97902dc79998340c886e860af37) grants plume addon1 and button addon2. The selected quest instead explicitly calls for an exhibition display. Its display is an authored Oteryn adaptation of that selected recipe, not a claimed donor display location. Preserve existing consumptive item owners and their exact source grants.

### Concrete selected Formal Dress exhibition binding

For the next native route slice, host the explicitly on-demand personal exhibition beside existing King Tibianus, rather than fabricate a donor display or add binary map data. Use the admitted existing object appearance2110 at(32315,32170,-6); its adjacent exhibition entry is(32314,32170,-6), ground464. These exact tiles are in content/world/placements/region-z06-x126-y125.b3 (sha256ade86a8e432522df4d30b1d8f56e8c6536a35c4e0664d426f4e895a2f80ddbfe). The object's known projection/collision is content/world/objects/objects-00500-00999.json:1#/records/131 (sha2562ebd31bd6ba0bb6cd3f1561258b61893954891e9b907ef918444ddbb4206c1fb). King already has a native placement at(32311,32172,-6) in content/world/npc-placements/placements-00000-00048.json:1. This is an explicitly authored exhibition role for an existing object/NPC, not a claim that the donor's palace object originally awarded Formal Dress.

The selected quantities remain one button and one plume. Planned personal acquisition callbacks use the existing fenced NPC MINT owner, with the plume acquisition requiring the actually earned button addon; final completion requires the full semantic outfit entitlement and grants the selected existing Sharp Dressed achievement. Its canonical fact is content/achievements/achievements-00000-00499.json:1#/records/429, keyoteryn:achievement/sharp_dressed, two points, staticdata532. Reuse the existing consumptive actions and source addon mapping. Enforce the selected level20/Current Premium requirement using the native owners. This records the implementation choice only; the Formal Dress ledger stays ACTIVE until its complete PG route is green.

### Formal Dress catalogue promotion evidence

The retained Crystal9f data/XML/outfits.xml:134,266 (sha25602a4f73b60237946109da1ec6ccb34257b246d9f3f746b5935caeacf2e9a0757) marks Formal Dress1461female/1460male as Premium, locked and store. The same retained Lord Retro death callback cited above explicitly grants both looks. Author an explicit selected quest exception to the catalogue's store filter for this pair only, supported by that actual non-store grant. Both official appearance records are present in imports/cipsoft-appearances/outfits/outfits-01050-01199.json (record SHA2561460=625c4e9912bd20f05e51a5008511015f272759fbb3d52279faa58f3e337a648c;1461=00a202670daa1c8f914e9814ae9c6cb304d0aecbe822e427b03ffc5bdf4962cb). No unknown visual ID or implicit fallback is needed. Existing item actions are retained but require that explicit native catalogue promotion before this outfit is usable.

### Native consumptive bindings and contacts

Use the existing materializable31737/31738/37604/37605/50060/50061 policies unchanged. A redundant requalification attempt raised a physical canonical conflict; inspection showed all six already have native physical admission, so no alternate weight or stack rule is introduced. Explicit look-pair mappings use the existing item-cosmetic-actions.json grant declarations and immutable provenancecrystal-item-cosmetics-201189f-v1, independently of the9f quest base provenance. No grant-key spelling inference is used.

The Fiend recipe explicitly names The Forgemaster; attach the selected supplementary conversation to existing oteryn:npc.the_forgemaster (retained9f data-global/npc/the_forgemaster.lua, sha2564d3c8a44257795fcb949cdd1bb30a0f1ba0cad8b88fad050fba97852946db76e). The Hand recipe leaves its inquisitor contact authored; choose existing Henricus (data-global/npc/henricus.lua, sha2561b98530e7f7ae4bef44e9d55373254e6590a160c7c8f5d8d9dd41b578bf7977d). These are selected Oteryn conversations, not claims that those Lua scripts originally award these outfits. Parent completions are read-only admission facts; subject milestones are never fixture-seeded.

### Formal Dress world authority

The chosen exhibition uses admitted palace ground464 at(32314,32170,-6) and
display2110 at(32315,32170,-6). Both callbacks require current level20 and
Current Premium in the fenced writer, independently of the prior King admission.
The selected exhibition is an Oteryn adaptation of the canonical explore/use
recipe, not a claim that a donor placed this display here. Existing native USE
consumes Brass Button37604 for addon bit2, then Decorative Plume37605 for bit1;
completion requires semantic ownership of the complete pair. The world fixture
models the known entry464 as walkable ground with speed150, rather than using
the common harness's default wall semantics for passage objects.

### Native bounded encounter credit (next shared mechanism)

Reuse questpc8 7eeee8fb152806de66af062831f94c2183c915ee death_objectives,
grizzly_kills capture/settlement and guardian admission. Preserve the original
frozen principal/party/corpse queue; duplicate lethal notifications and replay
retain one credit. No quest kill progress is written by the PG test fixture.
Hidden Threats selected explore/kill5/use/complete recipe is authoritative.
The chosen chamber entry(33079,32015,-13), ground10113, and empty guardian
tile(33080,32015,-13) are existing placements in
content/world/placements/region-z13-x129-y125.b3 SHA256
cd6d870e934961d3e6dd89e5b4b9ec6922c4d63b79203678bb876370859c83a1.
The retained Canary04b83b512 movement-chests_check.lua:18-22 identifies the
nearby Corym chamber positions; SHA256
0e00845e4ce1d3cf40dba3139210f86bc96dcb0f8f3242345a4a01aa3badc9b4.
This is an explicit Oteryn selected-entry adaptation of that chamber, not the
donor's StepOut storage callback. Count any of Charlatan/Skirmisher/Vanguard
inside x33077..33081,y32014..32017,floor-13 toward the shared five-kill quota.
The additional native guardian uses one existing empty ground tile,60s respawn,
normal occupancy/pinned-corpse checks and no source map binary edits.
Keep Hidden Threats ACTIVE until its valuables and complete route are qualified;
this first slice qualifies the reusable native death/guardian mechanism only.

### Hunting for Tokens exact selected route

The retained Crystal9f data-global/npc/jorge.lua SHA256
127ff40ceb3fd0612663cbd952176840977e1ba9b9f22534ba52b2d059963db0
lines61,115,125-128 explicitly binds Abacus19151 and exactly20 Silver Raid
Tokens19083. The same bytes are available at the public raw URL recorded in
TOKENS-PG-PROOF.json; generation reads only the committed corpus. Preserve the
canonical once-only selected programme while leaving unrelated souvenir offers
outside its qualification. Jorge's retained Canary/Crystal source candidates
agree at(32204,32292,-6). Add that placement through existing tooling; qualify
both physical items through the canonical accepted-first importer. Inputs in
the PG fixture are carried resources, not a claim about raid/event acquisition.

### Complete hoist programme and native selector bindings

The selected recipe requires exploration,one floor plan,one instruction,use of
the control,and hoist access. Existing native callbacks require two knowledge
flags; no physical plan/instruction Item identity is supplied. Conservatively
model acquired knowledge through a chest and the hoist fittings, not invented
Item IDs. Source-native chest2472 is at(32154,31116,-9),fittings5772 at
(32156,31125,-9),and the lower lever is2773 at(32156,31125,-10). The final
control projects2773->2772 and completes the selected route. These acquisition
locations are authored Oteryn adaptations,not claimed donor clue coordinates.

All three admitted platform tiles(32157,31125,-9/-10/-11) contain drawbridge
5770 and lack donorUID3059/3060/3061. Generic exact-selector bindings require
the expected original UID selector before substituting the explicit native
position/appearance. Guards,read-only storage,transport and asymmetric remote
lever behavior remain original. No B3 binary is changed. The producer records
all region SHA256s; authoring verifies real placed appearances and absent UIDs.
The legacy skeleton/chest namespace is not inferred ownership; PG asserts the
loaded typed tracks belong to the hoist. The Ice Islands completion stays a
read-only prerequisite; only that foreign parent is a fenced test fixture.

## Hidden Threats valuables and shared world MINT (2026-11-07)

The canonical selected four-stage route is explore, five Corym deaths, chest use,
completion. Bind the existing chest2469 at(33080,32014,-13); the existing native
room objective supplies actual lethal credit, with no subject state fixtures.
The reward is two Gold Nuggets and one each Small Ruby, Small Emerald and Small
Amethyst, all directly in the main backpack, atomically with quest completion.
Conservative identity assumption: standard Gold Nugget3040 is selected. Retained
`imports/canary/items-xml/items.xml:6833` defines3040 (file sha2561cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2) and :51931 defines27488 with
the same name/weight; targeted retained Hidden Threats forge action does not
resolve the duplicate. The bounded external wiki lookup
https://www.tibiawiki.com.br/Hidden_Threats_Quest?action=raw returned403. This is
an explicit authored choice, not a claim that the donor uniquely identifies3040.
Reuse questpc8 backpack output owner from805a061415355a68a924b3b32d02023fe608fc97,
with local migration numbers1101/1102. Preserve the fee/counter proof additions,
exact charges and Father11-track write. New layout is immutable in the MINT
binding/reservation; legacy false omits the new binding tag and retains replay.

The original key-fragment01 source claim shares the physical chest. Retained PG
caught unconditional overlay routing intercepting it. The valuables overlay now
activates only at selected programme1; the original claim remains reachable
outside that phase. Activation exactly repeats an authoritative write guard;
runtime quest copies select routing only, never authorize the reward.

## Hydra Egg personal native pickup (2026-11-07)

Keep ordinary map-source ownership and reset epochs; no UID quest chest and no
quest reward MINT. Entry uses existing empty1019 at(33003,32665,-3); one native
Hydra guardian uses existing empty1019 at(33005,32665,-3), with personal quota1.
The selected nest egg is the existing4839 at(33004,32664,-3), ordinal2;1063 on
that same tile is an explicitly authored search acknowledgement marker. Its
visual identity is not claimed as a uniquely resolved donor "Hydra's Nest" ID.
Source `content/world/placements/region-z03-x128-y127.b3:1#(33004,32664,-3)/2`,
sha2564143a9190fa20ae867b2cc015dcc6e974fdfbd786c6eb91f750cb5d832ba9072.
No world placement binary is edited. The entry is a selected chamber staging
binding, not a claim of a recovered donor movement script.

The current accepted physical profile already admits4839 as StackCapable100,
materializable to carried inventory/Ground. Preserve those accepted facts;
the attempted redundant qualifier was removed. XML `imports/canary/items-xml/
items.xml:14105-14108`,sha2561cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2,
provides the retained name/20oz weight; absent XML stack attribute does not
override the accepted canonical/client stack facts.

Reusable `owned_items` is read-only current carried quantity. Optional
`map_source` additionally requires personally completed native MapBase pickup:
existing same-command map MINT + item-loop pickup(operation5) + exact source
line/character/world/definition/placement. This survives normal stack merging;
it does not demand that the new materialized instance remain live after merge.
A stranded MINT, scripted Ground relocation, unrelated carried egg, or another
Character's pickup does not qualify. Successful pickup history is monotone
across channels/reset epochs in the same World; acknowledgement still requires
fresh Premium, current carried quantity and the selected Hydra kill quota.

Conservative legacy materialization rule: raw B3 Item count0 is the implicit
single object already rendered as1 by map facts. Only materialized Item entries
normalize it to absent/default1 during export; Terrain counts and positive Item
counts stay exact. The shared sector codec preserves explicit zero unchanged,
and runtime typed-zero pickup eligibility remains strict. Compiler equivalence
uses the same explicit adapter. No charge, subtype, timer or use effect inferred.

## Fire-Fighter selected Gnomprona training (2026-11-07)

Reuse existing native39544/39545 physical policies and201189 item actions:
Firefighting Axe grants addon1; Portable Flame grants addon2; both require the
base and CurrentPremium, and the source grants FriendlyFire when both are owned.
Existing fire_fighter outfit look pair is female1569/male1568. Add explicit
semantic compatibility mappings, not duplicate item policies or a cosmetic engine.

Gnomadness takes an explicit Oteryn quartermaster role alongside its existing
source Gnomprona identity. Retained Crystal9f `data-global/npc/gnomadness.lua:69`
uses hazard.gnomprona-gardens; :102-105 greet Gnomprona. SHA256
 e3411f8704927cc665340418f3b65fa40fdafa6d607364fde90827516b9f7279.
The placement candidate is(33527,32865,z14),north,look493 in
`tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json:1`.
Preserve original hazard words and services; their world-mechanic ownership is
another lane's scope. Gnomad was reviewed and rejected because its retained
script concerns Deathstrike's Warzone rather than Gnomprona.

Choose existing clear, unflagged17465 ground at entry(33528,32864,-14) and
training guardian(33530,32864,-14), roomx33527..33532/y32863..32866. Source
`content/world/placements/region-z14-x130-y128.b3:1`,sha256
 d20aef8024cef1a9ef0156fcfc98a99e2f660a1310f3f3ab2815f64a3c9cd66a.
No map binary/geometry change. Fire Elemental is an explicitly chosen existing
fiery training species, not an inferred donor encounter ID. One bounded native
slot respawns60s until the personal quota10 is met; no invented party credit.

All selected grants require level250 and CurrentPremium; there is no parent
quest. Report after10 native kills grants the base; axe supply requires that
base; flame supply requires the actually earned axe addon; final acknowledgement
requires both addons. The real consumptive item owner grants FriendlyFire.

Fire-Fighter disposition: the preceding local prototype choices are superseded
by external qengine2 qualification6738c1da7. Duplicate product code discarded
before commit under owner's skip rule. Its actual PG results remain an audit
of the discarded prototype, not a claim that this branch ships those choices.


## Hero of Rathleton selected native programme

Canonical chosen recipe is definitions/quests-00100-00199.json:1#/records/58,
seven stages, min0/current Premium, no prerequisite. The original global
machines, delayed source callbacks and FastWay unknown remain unchanged.
Use existing workshop machinery20476 at(33567,31950,-14) as chosen plot
evidence, ground20536 at(33564,31951,-14) as the voice investigation,
ground20545 at(33568,31951,-14) as earned dungeon investigation access,
and existing1855 at(33571,31952,-14) as chosen reward container.
These are explicit finite-programme approximations, not invented donor IDs.
Source region-z14-x131-y124.b3:1; retain its digest in generated callbacks.
Native species Glooth Golem and Professor Maxxen already have admitted
policies; Jaccus Maxxen's boss binding uses Professor Maxxen's canonical
identity. Ten constructs unlock the boss via the existing stage-track owner;
no new party/global-machine semantics or direct kill writer is introduced.
Frozen lethal principal receives credit once; no inferred party entitlement.
Final atomic direct-backpack plan is blueprint21897x1, four individual
Glooth Bags21203 and platinum3035x5. Completion grants this investigation
access milestone, not the unresolved original global transport/FastWay route.

Hero first expanded PG run50.474679451s reached all native kills but final
USE ended without a result because blueprint21897 lacked native physical
admission. Extend the existing qengine_reward_item_profiles qualifier's exact
cohort; no custom blueprint USE is inferred. Native admission source is
imports/canary/items-xml/items.xml:43242, sha256
1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2;
plain quest item, weight100. Independent Crystal201189 evidence is
items.xml:43440, sha2568a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509.
Glooth Bag21203 is an existing stackable ingredient, not an inventory
container; the selected four units retain that native policy unchanged.
The programme's required kills and reward intents do not certify optional
ordinary corpse loot; original corpse-admission refusal remains unchanged.

Hero validation exposed an exact published callback-count assertion10→14,
accounting for the four added Hero callbacks. Update that exact count only;
all malformed-requirement/read-only refusal assertions remain unchanged.

Hero review correction: final completion also writes the already typed, same-
quest native door entitlement oteryn:quest-progress/hero_rathleton/access_door
from[-1,1] to1 in the reward transaction. Existing owner mapping is
content/dialogues/quest-rules/quest-batch3-b-hero_of_rathleton.json:1;
Crystal201189 strange_pipe.lua:62/storage declaration:195. Preserve already
earned1 and all original gate/transport callbacks; no guessed numeric alias.
PG now asserts the real entitlement absent initially and1 after completion.

## Ferumbras selected seals and reusable world reward bundles

Whole selected recipe: ten Demonic Essences, ordered Plagirath/Zamulosh/
Mazoran/Razzagorn/Ragiaz/Tarbaz/Shulgrax/MortalShell, five Silver Tokens,
Rift Warrior and Hat Hunter. Actual PG18.6 RED41.469268453s proves missing
entry. Generic world bundle unit RED unknown achievements field, then GREEN;
all22 world-callback tests GREEN with every old mixed-field refusal retained.
Explicit reward_bundle opt-in allows simultaneous item/outfit only on a
once-only completing USE. Achievements are unique canonical keys, max8,
stateful once-only USE; use existing fenced account grant_missing inside the
same root transaction. Generic context-less callers remain refused.

Donor Crystal9f actions_reward.lua:15-23 uses4073 and gives Silver22516x5,
Hat Hunter and full outfit bits3. Existing Native Rift Warrior semantic mapping
wins over conflicting donor look IDs; no duplicate Source-item look aliases.
Physical retained4073 chest is(33388,31409,-14), region-z14-x130-y122.b3:1.
Darashia entry(33275,32390,-8)409; ritual flame1949 at(33275,32388,-8),
exact10 essence offering at(33275,32389,-8), region-z08-x129-y126.b3:1.
Selected god-slaying preparation is a durable ritual-readiness milestone;
no unsupported physical Godbreaker item or seven-part crafting is claimed.
This explicitly conservative finite recipe does not certify the donor raid's
other phases, parties, global levers/cooldowns or ordinary corpse loot.

Source boss anchors come from the eight retained actions scripts (exact hashes
in generated callbacks/objectives). Use nearest admitted clear floor cells by
those anchors, rather than forcing actors onto retained fields/decorations.
Mortal Shell uses the final Ferumbras arena anchor; no source phase-transform
mechanic is inferred. Original map bytes, no-logout/protection flags and source
raid callbacks remain unchanged. Earned personal passages connect the actual
arenas. Each passage requires its own native kill, advances a bounded passage
track and initializes only the next objective. Final passage8 therefore proves
all eight seals; no increase to the eight-effect callback bound is needed.
Silver22516 is admitted by the existing exact source physical qualifier.

Ferumbras expanded PG44.343084621s / diagnostic43.456666898s exposed a
fixture error before the tenth essence: input_item minted Ground using
placeholder content-1/vec[1], then moved previous input with the real content/
frame binding. ground_cell_limit.rs:304-306 correctly refuses mixed bindings
in one cell. Fix fixture MINT Ground metadata to the same actual channel pin
and frame used by its loop drop. Keep the production compatibility guard and
all short-offering assertions unchanged. The rerun adds exact native passage
position assertions and uses actual account appearance unlock facts.

The next actual PG44.125205511s reaches the repaired fixture but correctly
refuses two separate Ground stacks9+1: the existing ritual consumer requires
one physical stack with the exact offering amount. Keep that contract.
The story now performs fenced pickup9, normal carried merge with the tenth
essence, and fenced drop10 before ritual USE. Short9 refusal/non-consumption
assertions remain; add one-live-stack and actual pickup/drop proof. No Ground
consumer guard or SQL proof is weakened, and no new migration is needed.

Combined reward diagnostic PG50.768375617s reports InvalidStoredState after
all native stages and refusals pass. Exact cause: achievement source event IDs
are bounded to64 bytes (gameplay/progression/src/account_achievement.rs:6,68),
but the new world hook supplied raw serialized occurrence data. Hash that
stable occurrence to32 bytes with existing SHA-256; retain the64-byte bound,
existing NPC event bytes and all transaction/rollback guards unchanged.


## Grave Danger twelve native regions and combat route

Selected recipe:12 grave visits, five Lich Knight encounters, Zelos, Cobra
investigation, Scarlett, base Hand of the Inquisition and Inquisition's Hand.
Minimum250/currentPremium, no parent. Real PG18.6 RED40.799342316s proves
missing complete native journey. No quest-state seeds in the story.

Seven exact grave positions are upstream references verified against retained
map tiles: https://raw.githubusercontent.com/opentibiabr/canary/666a60f6fef803b043cbaf1c7b5fab219da9bfa9/data-otservbr-global/scripts/quests/grave_danger_quest/actions_grave_sanctify.lua :1-38, sha256 5d9d35aaabacfdd4242860446852e76474b4281194afb2352a119f57f3e024a5.
The other five use explicit selected native graves in the named regions:
Edron(33222,31670,-7)1980; Ghostlands(32190,31816,-8)31116;
Cormaya(33262,31983,-8)31109; Darashia(33244,32471,-11)1982;
old Thais temple(32347,32169,-12)31109. These five are conservative Oteryn
bindings, not claimed as exact donor haunted-grave coordinates. Native
placement paths/digests are generated per callback; region positions and
existing physical objects were checked in repo. Brazil wiki returned403;
no protection bypass attempted or further exact-coordinate lookup required.

A personal spectral trail connects the twelve actual sites and real boss
arenas. All actor spawn/arrival/exit cells are admitted clear native floors.
Five Lich Knight encounters require Count Vlarkorth, Duke Krule, Earl Osam,
Lord Azaram and BOTH Sirs; one Sir cannot stand in for the other. King Zelos
is unavailable before the fifth encounter. Earn real same-quest Zelos-door
entitlement. Original Cobra access STEP stays intact; new programme USE
inspection requires its earned flag before Scarlett. Old activation remains
USE-only: an initial invalid STEP authoring run7.966ms was rejected before
PG and is NOT counted as PostgreSQL evidence. Final outfit/achievement and
completion reuse the published fenced world value owner. Base outfit only;
separate Hand outfit addon programme remains independent. Source raid party/
cooldowns, global mechanics and optional corpse loot are not certified here.

Arena anchor data-global/scripts/quests/grave_danger/actions_count_vlarkorth_lever.lua:1-22, Crystal9f, sha256 922080f0439540f839a84919936b708f70befe57c6916dee3d1617745764ece7.
Arena anchor data-global/scripts/quests/grave_danger/actions_duke_krule_lever.lua:1-22, Crystal9f, sha256 88f0501371c85b690db03686c7e2da8efcded8b5cbbfc4851f4b882a052c24d4.
Arena anchor data-global/scripts/quests/grave_danger/actions_earl_osam_lever.lua:1-22, Crystal9f, sha256 70e52dcada604e3444abd4ed46ec11043a4c270f41e5799abcb56a2b4d731f03.
Arena anchor data-global/scripts/quests/grave_danger/actions_king_zelos_lever.lua:1-22, Crystal9f, sha256 f6e3b16fdf34ff1542ce1957b4ae7536f4875fb0ad92d2d610837f97bfa54bd6.
Arena anchor data-global/scripts/quests/grave_danger/actions_lord_azaram_lever.lua:1-22, Crystal9f, sha256 61767895e2331b8c03f0e031846af8f4dc761f8554c120ba13787440f2b56649.
Arena anchor data-global/scripts/quests/grave_danger/actions_sir_baeloc_sir_nictros_lever.lua:1-22, Crystal9f, sha256 74d75d91cefa47e03d6f93f79fe3283e0229924f0482ef5dd108c356465a9f14.
Arena anchor data-global/scripts/quests/grave_danger/cobra_bastion/actions_scarlett.lua:1-22, Crystal9f, sha256 7cf1f42e77e0f20423eae7cc8d898200cd7f739e6c4fa0bc57cad08398f9ac2c.

## Grimvale native contacts — 2026-11-07

Reuse canonical NPC declarations oteryn:npc.rhargu and oteryn:npc.domizian;
no new identity or donor Lua is fabricated. Existing approximate visual mapping
is retained without a canonical appearance-fidelity claim. Peer70a0d92af's
apply_authored_placements is reused, extended to pinned completion242 recipe
records as well as authored68 recipes. Admission still requires the canonical
NPC identity, an explicit scoped key, exact recipe SHA256, named talk target,
chosen-approximation basis, bounded cells, directions and no donor replacement.

Chosen Rhargu(33138,32353,-5) and Domizian(33140,32353,-5) stand outside the
Ancient Feud entrance. These are selected contact cells, not original NPC
coordinates. Physical region-z05-x129-y126.b3 SHA256
60db4d14173f0c4e584d81d39240aeb45959152fa1f401098dfed1294b303062
has clear unowned ground4559 at both cells; native compiler collision admission
is tested separately. Retained entrance source:
data-global/scripts/quests/grimvale/actions_ancient_feud_entrances.lua:1-8
at Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d, SHA256
3570f68f69848c0176703c72cb35a6316c5986bdf1ce4214272fd659b608a1ef.
The frozen recipe names both talk targets; contact placement alone cannot
advance or qualify the subject quest. Full selected programme remains ACTIVE.

PG RED41.1128287s reaches PostgreSQL18.6 and refuses absent Rhargu placement.
The same test will require both published contacts, distinct real runtime
actors, and unchanged absent Grimvale programme state after native admission.

## Whole selected Grimvale programme — 2026-11-07

The frozen13-stage personal recipe remains the contract. No global full-moon
calendar, world bloom, original party/raid lifecycle, source Feroxa form chain
or optional boss loot is claimed by this finite selected programme. Native
Feroxa is a real bounded death objective at the retained room(33390,31540,-11);
the four other named leaders retain their source arena centres.

The static Grimvale surface region has no22063/22064 plants. The conservative
personal harvesting rule selects five clear native botanical survey plots:
(33329,31690,-7),(33331,31687,-7),(33335,31685,-7),
(33338,31682,-7),(33340,31680,-7). Each grants one actual native22082 purple
nightshade blossom through a different once-only receipt; all five are required
in carried custody when preparing the trap. These are authored collection
plots, not a claim of original source plant placement or global bloom timing.
Source XML confirms22082 weight200 and official client flags mark it cumulative.
Native22074 is a closed silvered trap; its source description confirms silver
coating. Preparation grants that physical trap; deployment consumes exactly
one carried22074 through existing item_on_target on a native arena ground.
The separate timed liquid-silver vial22058 is not used or misrepresented.
Retained trap source: actions_closed_silvered_trap.lua:1-19, Crystal9f5a72c6,
SHA256cba8b2e3c3843fbb2f9226682faaf36c26c209fa8f95c312fe89042466c08e94.

Repository area anchors bind the later selected investigation to real regions:
content/world/areas/islands/islands-00000-00059.json:1#/records/18 (Grimvale),
#/records/41 (Oskayaat,33068,32917,-7);
content/world/areas/hunting-places/hunting-places-00000-00444.json:1#/records/203
(Hyaena Lairs,33212,32359,-8) and#/records/311
(Oskayaat Undercity,32999,32933,-8). Nearby clear native ground is chosen for
the cursed site, mosaics, moon-symbol survey and ritual. These are selected
inspection locations, not asserted original source clue coordinates. Every
callback appearance is checked against the actual native B3 cell.

Ten afflicted kills admit Werewolf/Werebear/Wereboar to one bounded shared
counter; all four distinct leaders require their own death. Twenty Werehyaena
kills precede Domizian. Rhargu and Domizian use their existing canonical NPCs,
chosen contact placements and new personal phase advances. Missing individual
Lua greetings fall back to exact retained common npc_handler defaults, citing
that library rather than inventing NPC source bytes.

Ritual completion atomically gives27565 Foxtail Amulet, Over the Moon, Drama
in Darama and typed investigation access. Two receipt-bearing native transports preserve the earned flag and
check earned access and fresh Premium for actual exit and reentry. No foreign
quest is completed or seeded. Physical item facts come from pinned source XML
47dfd51f and official client appearances; native profiles are repo-generated.

Earned travel reuses the existing preserve-state quest receipt owner so fresh
requirements run inside the normal fenced transaction. The legacy read-only
transport contract still refuses requirements; that guard is not relaxed.

The first hunt PG failure was fixture authority, not absent Werewolf source.
The active source profile already exists. The new case now invokes reusable
prepare_native_hunter, matching previous Grave/Ferumbras test bootstrap: install
the activated policy table and bind the actual admitted attacker lease. No
creature policy, guardian constraint or subject quest state is fabricated.

## Heart shared mechanisms — 2026-11-07

The128 static-description bound is distinct from32 live guardians. Cooling
metadata no longer exhausts the lifetime quota after32 completed objectives;
live actor admission remains bounded independently. All existing spatial,
source, quota, occupancy and corpse-pinning predicates remain.

The owner explicitly requires missing shared quest-engine features to be
implemented. Yana's retained lines185–187 name Powerful Strike, Void and
Vampirism and contain the missing defeat check. The selected Heart completion
now serves as the native learning entitlement for shrine application of those
three tier3 families. This closes that portion of the former Global-profile
quest-unlock omission; no arbitrary unrelated quest or scalar storage alias
can substitute. Native completion is immutable and already fenced, so no
duplicate entitlement table or new SQL migration is introduced. The fresh
coherent quest snapshot must have the same Character revision as the durable
apply fence. Basic/intricate, other unconfigured families and ordinary filled
scroll/etcher use retain their existing policies. Whole native PG proof of
refusal before learning and actual paid powerful apply is required before
qualification; parser units alone do not qualify the feature or quest.

## Heart fixed expedition bindings — 2026-11-07

The selected Destruction Destabilizer is the retained Anomaly control lever
8911 at32245,31244,-14 (actions_anomaly_lever.lua:38, SHA256
b552a05e52be2c70c107ea0a1034710c081f3ac03ebfff98a8f957478abbc831).
It transforms to8912, with route admission and the first boss counter in one
fenced USE. This is an explicit selected role for an existing native device,
not an invented original item ID. The retained Farmine/Zao breach arrival
33026,31367,-8 (movements_teleport.lua:10) supplies the selected Zao regional
entry; the exact original Zao Steppe portal name is not asserted. All six
source arenas retain literal centres, with adjacent clear native passages.
The recipe explicitly replaces party levers, timing and daily locks with a
fixed once-only sequence; original boss form/phase fidelity and optional
backpack decorations are outside that selected programme.

Yana's retained agreeing source cell32200,32304,-6 is published without
re-authoring her existing identity. Her final report atomically completes
the programme, grants five actual22721 Gold Tokens and Ender of the End.
Native immutable completion is consumed by the powerful-imbuing adapter,
not just a displayed flag. The whole PG case must demonstrate no powerful
apply with sword/materials/fee before learning and a real250000-gold,
20-hour Powerful Strike after learning, including actual material burns.

Two ordinary source imbuing shrines in the PG map are independent service
fixtures, not new production World placements. Generic native service inputs
(sword, normal coins, three recipe materials) use fenced MINT and transfer;
no Heart reward, tool, stage or completion is inserted by the fixture.

## Forgotten Knowledge: finite selected route and native learning

The selected eleven-stage recipe is the scope, not the donor's original party,
lockout, form or cross-quest progression. Ivalisse's source names her father
Silus (data-global/npc/ivalisse.lua:81, SHA256
455bf9ca148de6aec55a2801f9234bb492b161bac253afd3703cb8c609f11bda).
Her chosen dialogue supplies the fire-route instructions; it does not complete
The New Frontier or assert its original storage29 prerequisite. The source
recipe itself has no parent quest. Source Albinius identifies Heavy Old Tome
23986 (data-global/npc/albinius.lua:27,76, SHA256
1a476ee079849acee80dd99fb107ae5fd674c2d3c3343beed8002840d09cea18).
A personal once-only temple recovery interaction produces one real tome;
Ivalisse consumes that exact item in one fenced hand-in. It is not an NPC
shop purchase or a claim that the selected archive site originally spawned it.

The selected egg USE milestone binds an existing physical dragon egg4839 at
33003,32662,-3. This is a documented chosen adaptation, not the original Frozen
Horror combat mechanic. Source actions_frozen_horror.lua:10 instead creates a
Dragon Egg creature at32269,31084,-14 (SHA256
53f1005d33185fe0993cf7324ce6ae3a83618ff3de25edf75ba63fe03d000fe7).
The original isolated Soul spawn33359,31182,-12 has one clear tile and no
arrival tile in the retained map. The selected terminal Soul therefore fights
in the source Dragon King arena33357,31182,-10. The source action declares
that base arena (actions_dragonking_zyrtarch.lua:2-4, SHA256
388437b82e425285e6df26c92e1f747b388f3028370d9dd619ae8de4630c53a1).
Native terminal identities follow creaturescripts_bosses_kill.lua:1-16,
SHA256a253bd0bd92d9acba483a5a181998f59a09d48aef91c50b20de2128661599907.
Last Lore Keeper uses a clear source-arena cell; original astral phases are
not claimed by the finite selected single-kill objective.

Completing all selected milestones licenses the source Forgotten Knowledge
powerful families implemented by native recipes. This conservative rule waits
for whole completion instead of granting original per-boss unlocks early.
The selected egg milestone substitutes for the omitted original ice-boss path;
it never claims that Melting Frozen Horror was defeated. Source XML
(data/XML/imbuements.xml:37-415, SHA256
7e4ae3be28e0b3e740e856e8cec2c8c9ff01e7f9ab4b2b8d3b970b51ca1df8fd)
provides exact family/storage mappings. Featherweight has storage0; Vibrancy's
46317 is unrelated and is not attributed to this quest. Native elemental-damage
recipes are absent and are not fabricated. Heart remains an independent
alternative only for its three source-named powerful families. Basic/intricate
recipes and other established access rules retain their existing treatment.

The selected father stage also requires actual A Strange Chalice dialogue: chalice, daughter, story, disguised, thirsty, learned. Existing canonical identity and retained candidate33419,31125,-9 are reused. Source data-global/npc/a_strange_chalice.lua:60-113 SHA2567eb9bf22d46533d4584fc67a1fa60e49cda9395e05f2e1dc4b50c250c18243ea identifies the father and provides zzubaran; Ivalisse alone does not finish that stage.
