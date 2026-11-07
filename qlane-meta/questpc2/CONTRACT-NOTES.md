# questpc2 implementation decisions

These are implementation boundaries and explicit assumptions, not whole-quest
qualification. Continue owned missing mechanisms; no external engine deferral.

- Physical Instance corpse MINT uses the existing DUR-03 owner and the native
  sealed death scope. Assignment keys retain namespace tag 3; no Instance uses
  its origin Channel as custody or mutation authority. Existing Channel cause,
  placement and audit bytes remain compatible.
- Primal corpse39530 is already canonically materializable with capacity30.
  Preserve its existing `primal_corpse_items.py` admission and decay definitions.
  The earlier RESUME reference to absent native profiles concerns the separate
  spell-native provider file; it does not mean Primal needs re-admission.
- Spite33867 declares48 slots and Bakragore44012 declares30. Source:
  `imports/canary/items-xml/items.xml:63232` and `:76397`, Canary
  `04b83b512114bfd888000d6e1433ed8ecaec7c5b`, SHA256
  `1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2`.
  Evidence class: OTS_HYPOTHESIS_ONLY. Retain the exact capacities and decay
  chains; do not claim current Global verification from this donor.
- Declared container capacity and actual committed-entry budgets are distinct.
  Retain the existing30-entry corpse quota and16-entry generated-plan quota.
  A positive admitted container can declare more slots; the durable count and
  plan checks still refuse an oversized actual write. Do not clamp source data
  or increase runtime budgets merely to admit an empty reward-boss corpse.
- Missing corpse Item semantics use the existing Primal corpse policy:
  nonstackable, zero weight, not pickupable, and the already captured
  WorldObject movement flag. Preserve unknown unrelated semantic groups.
  This is a conservative Game custody rule, not a new claim about donor C++
  defaults. Fluid source and WorldObject collision remain their existing owners.
- Reward bosses keep one loot route: private chests. The existing producer
  clears the physical corpse loot table (`kill_reward.rs`, personal reward
  capture); native physical corpse admission must not duplicate that value.
- Corpse value retirement retains the existing DUR-03 materialization/loot
  window owner. This lane does not invent a competing P23 timer. Item metadata
  preserves source decay facts for the existing lifecycle owners.
- Content room qualification still requires the full native creature, map and
  callback closure. Lift the obsolete physical-corpse refusal only with native
  dispatch proof; a generic root fixture does not qualify a quest or source room.

- Shared canonical-shard fingerprints may be rebound only when the retained
  source/profile/refusal snapshot and every selected shop definition match an
  actual Git base. An unrelated corpse record in the same shard is not a new
  shop qualification. Changed selected facts still require full source capture.
- Filesystem recovery preserves source/PG and latest ready compiler outputs.
  Obsolete task incremental states may be archived and verified in /tmp before
  reclaiming their originals; never broadly clean or disable incremental builds.

- Reward-boss physical corpse admission is a separate positive proof from
  private reward-value admission. An explicit native creature/loot binding,
  source reward-boss flag and admitted positive-capacity corpse are required.
  An empty corpse row never admits missing loot Items, an ordinary creature,
  or an empty fabricated personal chest. Full reward rows retain their strict
  source table closure and the existing private contribution/settlement owner.
- Canonical Item changes rebind the119 navigation taxonomy evidence hashes;
  all12,252 targets and family profiles remain unchanged. Use the full tree
  generator, then its native registration phase; never hand-edit those hashes.

- Annihilator source is retained losslessly in
  tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz,
  blobs/62e92e8acd676bad545a501c8431c983dcd4edfa:1-84; source SHA256
  02913ff4a52944f154e0d4148b6d0baf16c9999373fd699bf76a7dd577ed7fa2.
  The inventory pins this exact blob to Canary04b83b512114bfd888000d6e1433ed8ecaec7c5b
  and Crystal00ce02a57ca5a12e48f32a3476e37471167e4c3f/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d.
  Four native players of level>=100 occupy x33225..33222,y31671,z13;
  destinations x33222..33219,y31659,z13. Six Angry Demon positions are
  (33219,31657),(33221,31657),(33223,31659),(33224,31659),
  (33220,31661),(33222,31661), all z13. Occupancy checks players in
  center(33221,31659,13)+/-4 cells. The selector is uid30025/item2772.
- settings.daily=true means one-way lever2772->2773 until map reset;
  do not substitute a24-hour character timestamp or enable daily=false cleanup.
  Source lines63-78 explicitly keep that alternative branch inactive.
- Group world callbacks preflight all participants, native destinations,
  source creature policies and spawn capacity before consuming the lever or
  moving anyone. If any spawn cannot be admitted, refuse the whole attempt.
  This conservative atomic rule replaces the donor's sequential unchecked
  createMonster failure case; it never grants a cheaper partial encounter.
  A failed attempt must leave the lever, participant positions and actors intact.

- Lane8d57515a1 already publishes effects.encounter and
  commit_quest_encounter_entry. Reuse these; the local parallel group prototype
  is retained only in a Git stash, not product code. The pinned group tool is
  read-only and verifies the actual shared callback.
- Annihilator door source: corpus blobs/1170e128e15ee05582db092987beea7a6935726a:4-18,
  SHA25603c6833cf9c9e4c335e2d33716b1180159696eb45cf1772d9e09e6036ef50ca1,
  Canary04b83b512114bfd888000d6e1433ed8ecaec7c5b,
  data-otservbr-global/scripts/quests/the_annihilator/door.lua. Read the real
  shared Reward10102 claim: value1 seals against the player; closed5113
  teleports onto the target and opens5114, closing checks occupancy.
  The source defines no escape timer or all-six-kill reward predicate.

- Corrected door-close interpretation: a present creature is displaced, not
  automatically refused. Canary04b83b512114bfd888000d6e1433ed8ecaec7c5b
  data/libs/functions/creature.lua:174-210, retained
  blob2ab1104989a5d1f50669dbb1e2d2aeef2f7c8dec, SHA256
  a34f5bad153897359cd1f2499b6cf189d32c61e3d7ae18dd35a67eeedfd357c7:
  query E,S,N,W with IGNOREBLOCKCREATURE, refuse if all fail, then teleport.
  Game keeps native collision/PZ/owner fences, and refuses multiple occupants
  rather than closing a solid tile over the remaining creatures.
- Source data/scripts/movements/closing_door.lua:58-90, retained
  blob1f977deb40366e3db24fbb3276fe84728778e268, SHA256
  e81594eae44ae08a8ef0175fef4480359362c94c84ba2a3628debfe698b5fd78,
  preserves generic quest-door step-out close after the player leaves.
  Opening requires a sealed proof of the future open state of exactly the
  selected door; bypassing the closed-door predicate alone cannot bypass the
  actual movement-cell collision index. Other solids/doors stay blocking.

- Continuous-route readback: existing content/world/placements/region-z13-x129-y123.b3
  SHA256718d9825424eb18b97b9f593601e27eb918f7b112f42452b8604db641ecd3939
  has closed door5113/action10102 at33216,31671,z13; lever2772/unique30025
  at33226,31671,z13; chests2472 atx33227,33229,33231,33233/y31656/z13
  with no unique attrs. The scripted door is in the entry corridor. The
  isolated door PG fixture is a mechanic proof, not a source-room placement
  claim. Implement source startup chest-selector binding and the continuous
  native route next, using retained source startup tables; no invented kill flag.

- Restored startup selectors6085-6088 with existing quest_chest_bindings.py,
  using exact retained Crystal201189f chest.lua:1235-1263 SHA256
  7c3ac1280196923e0bf5ca6f2ea932c19065bb9ac22d1559150a184f3cf06df4.
  The existing canonical region now carries those4unique attrs. A decoded
  before/after comparison proves all geometry/items/flags/depths/other attrs
  unchanged; no executable/object/archive/reference copy is added.
  The same real captured selectors now pass native PG USE, cold replay, one
  shared claim/MINT and alternate-choice refusal (PG18.6,43.95s).

- Continuous Annihilator fixture compiles source::project_regions(129,123) with
  the real registry and every captured floor. No placement/flag/geometry is
  replaced by synthetic walkable terrain. Runtime map boot follows production
  apps/game-server/src/node/serve.rs:2293-2338: indexed static Items without a
  native profile retain conservative owner defaults rather than becoming an
  ItemReference error. Item key sets agree exactly (34,111 definitions).
  Four genuine Fresh authorities share the source approach start33215,31671,z13,
  as ordinary admissions share a configured start, and independently walk to
  distinct source staging cells. No relocation supplies encounter entry.
  Progression uses the admitted D88 policy/XP writer; level/build facts and monk
  values are read through existing Character owners. The source bundle combat
  view supplies its actual frame binding. Entry evidence alone remains partial;
  native combat, earned chest, escape and sealed re-entry are still required.

- Real source combat preconditions: content/creatures/definitions/spell-native-profiles.json:1, SHA25606b0bee295bf0d09d5b3c87827d66cc276771bc79a3c0a05cf8cfebf4c1282a5,
  admitted Angry Demon health8200, armor40, physical resistance30%; pinned
  donor data-otservbr-global/monster/quests/annihilator/angry_demon.lua,
  Canary04b83b512114bfd888000d6e1433ed8ecaec7c5b,
  SHA25616c4ceb8eecd1064f0b97492a6ea538a30a502b2869000312e3677e8689a9b28.
  The existing official level-bonus curve in spell_data/formula.rs:17 gives
  325 at level2000; source native progression evidence ends at2000. An
  arbitrary64-think unarmed budget is insufficient, not a quest timer.
  Keep full health/AI/real-kill assertions, tick both owners at the actual
  CREATURE_THINK_INTERVAL_MILLIS, and use normal first-entry equipment plus
  physically minted/carried native health potions and actual item USE.
  No quest progress/reward, fake actor stats, invulnerability or forced kill.

- Equipped melee refusal was source-owned incomplete hand metadata:3425 is
  admitted taxonomy Shields but native weapon_type was UNKNOWN. Exact donor
  imports/canary/items-xml/items.xml:10447-10455, SHA256
  1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2,
  Canary04b83b512114bfd888000d6e1433ed8ecaec7c5b explicitly states shield.
  Generic hand_weapon_kinds.py completes only missing kinds where taxonomy,
  exact existing identity binding and XML agree; known defense34 wins over
  donor26 and all other facts remain identical. SourceResolver still fails
  closed for unknown hand policies. This enables real equipped lethal damage;
  the next actual refusal is canonical demon-corpse admission, not a kill flag.

- Demon corpse admission uses the existing generic encounter-corpse authoring
  owner, not a special reward bypass. Pinned Canary04b83b512114bfd888000d6e1433ed8ecaec7c5b
  imports/canary/items-xml/items.xml:12452-12465,16320-16324, SHA256
  1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2,
  defines5995(capacity15,10s)->4097(capacity24,300s)->4098(capacity24,300s)
  ->4099(no container,60s)->0. Existing captured WorldObjects independently
  identify each as a non-player corpse and own movability. Canonical Item
  admission and loot-table facts must agree before real lethal rewards commit.

- Actual PG90.53s next refusal: eleven Angry Demon loot Items lack canonical
  custody despite known physical facts. Source XML SHA256
  1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2;
  exact item:start-line witnesses 3060:7008, 3063:7020, 3284:8525, 3306:8837, 3364:9687, 3366:9713, 3414:10280, 5954:16097, 7368:19267, 7382:19393, 7393:19599.
  Reuse pinned client2dfa943b...dae7e2 and existing source binding importer
  for stack/container; retain all canonical physical/combat/charge facts.
  No loot probability, corpse capacity or reward budget is changed.

- Derived Prey model effect: admitting the real Demon loot closure makes the
  ordinary Demon reward table settleable too. Existing qualified_catalogue
  in crates/gameplay/dispatch/examples/prey_time_sim.rs:490-511 excludes
  admitted tables with more than7 entries, so its available/unsettleable
  tier4 counts both drop154->153. This is the unchanged model's bounded
  catalogue policy, not a change to quest loot odds or budgets. Keep the
  repository simulation's actual outputs; do not hand-pin prior counts.

- Actual PG180006 ran102.96s after Item closure: the first native lethal
  corpse/loot drain committed and its real eastward movement returned Moved.
  The next failure was a fixture lookup for the second actor at its old tile.
  Native AI may move source actors. Retain both exact actor identities and
  their full-health snapshots at source entry before combat, then require
  native lethal/reward drains for those same actors and exact cleared-tile
  movement. Do not disable AI, replace actors or substitute progress flags.

- Full Annihilator route begins at the actual exit/approach33213,31671,z13.
  Existing native DoorBook already owns expertise pair5111/5112, per-placement
  level action interpretation and step-off close (map/door_registry.rs:278-290;
  map/facts.rs:221-267). Its accepted USE contract toggles; real native WALK
  crosses the opening. Do not invent another door or bypass this owner.
  Source imports/canary/items-xml/items.xml:14552-14559, SHA256
  1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2,
  declares Door class and levelDoor base1000. Existing canonical identities
  have no semantic body and no native profiles. Reuse pinned client/source
  importer and exact bindings for ground-only profiles; retain every known
  canonical fact, allocate no identity, and keep source geometry/action ids.
  Fresh qengine2 a0100244c and questpc4 5ea9e52be retain the generic predicate
  but provide no5111/5112 admission to reuse.

- Confirmed source startup gate (no assumption): retained corpus
  data-global/startup/tables/door_level.lua:173-178, SHA256
  d8868f675aae7c541ddc3e76b3648b6a886e0ae31a4a614747d17e1784d234fc,
  declares1100 at33214,31671,13. Raw captured B3 has no action. Independent
  level-access recapture preserves the already finished chest source capture.
  Source map diff adds only that action; all geometry/other items unchanged.
- Actual PG100.04s passes gates, entry, native kills/drains and reward doorway.
  Source statues2113 block26,58 and34,58; display2012 occupies30,57. Route
  around these captured obstacles with real WALK; never remove collision.

- 2026-11-07 full Annihilator PG108.32s confirms actual source route through
  physical armor3388, replay/shared-choice protection, exit33213,31671,13 and
  claimed-door refusal. Imported canonical assessment holds and historical
  timed-escape language are provenance, not instructions to invent mechanics.
  Reuse existing generic level predicates and source startup action1100.
  Desert merge retains single-dictionary and list-shaped source table groups.

- Elvenbane continuation: fresh qengine2@97cb9db79 and questpc4@6d7cd8cc0
  provide no Elvenbane route/objective implementation. questpc6@53d5648b8
  explicitly SKIPPED_OTHER_LANE to questpc2. Reuse existing native XP/death
  receipts and fenced Quest writer; objective bindings must reject command
  causes, foreign principal/creature/scope and unearned death occurrences.
- Exact native tower identities found without guessed coordinates:
  content/world/spawns/spawns-02000-03999.json:1, SHA256
  00f3f314eb78e7eff2dd1ec1b199b74a329d46aac6703c8258b22737f262b04a,
  /records/1440/declaration Dragon32590,31645,z5;
  /records/26 MinotaurMage32592,31643,z4; /records/28 OrcShaman32590,31647,z4.
  content/world/spawns/spawns-00000-01999.json:1, SHA256
  624bdaf1c5b8ebbcf7e98c2410ffbf8c8a25a81fc851e3952d48c5573d962d66,
  /records/1175/declaration ElfArcanist32590,31643,z3 and two
  DemonSkeletons32592,31644,z3 and32589,31646,z3. These match the accepted
  authored recipe's mandatory Dragon plus five defenders exactly. Keep its
  personal completion/reward choice; ordinary shared loot is not that recipe.
- Elvenbane authored implementation choice: the kill-credit footprint is the
  bounded tower rectangle x32580..32600/y31635..31655 on each exact native
  defender floor. This boundary is an Oteryn choice, not an invented donor
  script. Carrier positions use positive floors (5/4/3); callback/map storage
  uses negative floors. Personal completion is Character-owned progress and
  rewards through the native tower; no NPC bypass supplies terminal loot.
- Elvenbane Spellbook resolution: choose normal equipped spellbook3059 from
  pinned items.xml:6988-7003 (SHA2561cf2992c...022f2) and its existing exact
  native binding. Same-named3101 lacks the normal weaponType/script declaration
  at7484-7489. This resolves an authored approximation; historical ambiguous
  import provenance remains unchanged. All seven chosen reward identities
  already have native item profiles; no new guessed identity is needed.

- 2026-11-07 source-map native objective kernel PG RED79.95s, PostgreSQL180006,
  /tmp/questpc2-native-objective-atomic-link-retry-pg.log. Source map collision
  and prior-route fixture setup passed; actual source Dragon death was refused
  corpse_item_inadmissible before queue capture. Existing native profile points
  to corpse5973. Pinned items.xml:12095-12108,16191-16195, SHA256
  1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2,
  gives5973(12slots,10s)->4025(10slots,300s)->4026(10slots,300s)->4027(60s,end).
  Reuse generic encounter_corpse_items.py and exact independent WorldObject
  Item pointers (objects-00500-00999 and01500-01999). Generator's3tests pass.
  This is source custody admission work, not whole Elvenbane qualification.

- Follow-up actual PG RED73.92s/180006 still refused corpse: derived native
  loot-tables.json carried materializable=false/capacity=null for5973 despite
  canonical admission. Regenerate with build_full_spell_artifact.py
  --refresh-loot-tables; never hand-edit eligibility. Native Dragon's accepted
  22-entry loot table additionally selected ten unqualified custody Items:
  3061,3071,3275,3322,3416,5877,5920,7430,11457,44158. Existing known physical
  and combat/charge facts remain unchanged; source flags qualify only custody.
- Trophy44158 has no pinned XML row, but exact binding in
  imports/crystalserver/bindings/items.json:1 and pinned client object44158
  establish the portable Dragon Trophy. Client asset SHA256
  2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2,
  content/assets/files/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat:
  name dragon trophy, take=true, cumulative=false, container=false. Existing
  canonical physical weight500/movable/pickupable are preserved. Generic
  importer already accepts absent XML; require portable nonstack/noncontainer
  flags so no absent XML capacity or stack limit is invented. Its3tests pass.
  Donor data-otservbr-global/monster/dragons/dragon.lua:90-112, SHA256
  44b7a3aa02085be56a94af2d837219d8828de3ddf8dad96aa11b5e1e8d6216c8,
  names the ordinary Dragon loot; preserve existing accepted canonical loot
  probabilities/identities (including trophy) rather than rewriting its source.

Native objective atomic proof, 2026-11-07: PostgreSQL180006 RED74.71s
reached native queue but captured0 objectives. The isolated Annihilator-based
fixture had never admitted QuestSession; refresh(insert=false) deliberately
updates existing sessions only (dispatch/.../quest_admission.rs:24-26,76-77).
Correct fixture admission uses insert=true and asserts coherent phase2 BEFORE
death. GREEN74.95s, marker154ms native subcase: real source Dragon death,
one captured objective, phase2->3, XP/Quest receipt xmin equality, replaystable.
This isolates engine atomicity; pre-route phase is an explicit fixture
precondition, damage uses the native owner seam, and no full Elvenbane route
or client combat/reward qualification is inferred. Authority4.05s and
complete earned Annihilator105.20s PostgreSQL regressions remain GREEN.
Shared persistence retains the existing ordinary Quest SQL and caller gates;
native write joins XP transaction rather than making a later separate write.

Elvenbane playable exploration continuation (authored Oteryn selectors, not donor quest scripts):
Committed source B3 decoded using tools/content-schema/world-authoring/world_region_codec.py; no geometry/attributes changed. Exact coordinate witnesses use binary-region line1 plus tile coordinates.
- content/world/placements/region-z03-x127-y123.b3:1 SHA256 521b9242b03f5f7f5af3aa79c536ce6e31735134de9bc07a2bc593a2125836f9
- content/world/placements/region-z05-x127-y123.b3:1 SHA256 70a99c547f3f3f4db33b72db5439221197dec182fe9ea1cff0b1e76ee522908c
- content/world/placements/region-z06-x127-y123.b3:1 SHA256 8a55c8b9400e0511cf2e2925982ffd00741964714cb4e88c9e8663007e59a15f
- content/world/placements/region-z07-x127-y123.b3:1 SHA256 d3eab921dcbe55c2708d5eee3dd8285e50becbca661a4c2da08e40c5b2dcea61
- content/world/placements/region-z08-x127-y123.b3:1 SHA256 34d7229b0638876931954aced076b0dd62e600bf073ec0bd0f02ed47f9f5549a
Chosen basement approach starts32599,31652,z7 and walks west onto ground437 at32598,31652,z7. Below, original north-stairs1947 at32598,31652,z8 resolve landing32598,31653,z8 via native floor rules (world/src/map/floor.rs:295-317); positive PG traversal is pending. Bind basement exploration to that landing.
Poison2121 covers central source floor6, including32589,31644,z6. Bind a distinct poison-crossing flag there; only then allow the actual stairs1947 arrival32588,31642,z5 to advance phase1->2. This is the chosen recipe route, not a donor scripted gate. Do not synthesize prior progress in the continuous fixture.
Original upper bedroom has two chest2472 placements at32590,31647,z3 and32591,31647,z3 (top-level placement index1). Bind once-only personal terminal bundle to a source chest after Dragon and all five defender counters; no NPC bypass. Production implementation and PG proof still pending.

Exploration PG continuation: RED73.42s/server180006 proved literal hole
landing32598,31653,z8 but no phase1. Three code-built authored callbacks now
use existing fenced STEP/Quest owner; phase0 tracks are installed before
world callback lowering, avoiding duplicate tracks/default-1 drift. Global
callback bound512 retained, reservation derived from built-in count.
Subsequent RED77.18s and diagnostic RED75.44s earned phase1 then disconnected
on the second fixture connection: command.rs diagnostic194 identifies
retained-NPC/Bank reconciliation. npc_command.rs:1114-1134 explicitly keeps
work unresolved when command.item_fence is absent; fixture had left it None.
Correct admission supplies current_quest_fence().item_fence() before input.
Temporary test-only tracing removed; no Bank or protocol guard changed.
Full basement/poison/Dragon-floor GREEN still pending this corrected fixture.

Source ladder1948 admission: XML3918-3920 explicitly type=ladder (same pinned
XML1cf2992c...); generic importer lacked LADDER in its existing native enum
mapping. Source profiles omitted1948, so production default-solid policy
correctly blocked the original stairs landing. Source capture tests
RED7.276s ->GREEN6.734s; exact one native profile addition, all existing
profiles preserved. Current peer heads listed RESUME have no1948 profile.
No map flag, stairs direction, field condition or default-unknown policy
was relaxed. Full source exploration PG remains pending.

Native ladder USE source: retained donor corpus manifest binds Canary04b83
data/scripts/actions/items/ladder_up.lua:5-17 SHA256
730bc7664484e953b6efc3c8faeba9936d355c5cdde91d1f3e0686f714f4df13
(blob5ec5f7c5b67e2983e660a3154908f0cef2e1cee1). It selects native Ladder
IDs and calls Position:moveUpstairs(), then PZ-checks selected destination.
data/libs/functions/position.lua:22-49 SHA256
d933c6df3c0d0e0e974048bf0a58063f14d7f872adca2d5bb0250d6cdc261920
prefers south upstairs then N/E/W/SW/SE/NW/NE; duplicated west is immaterial
in one locked turn. No source copy is added. New adapter uses native class
LADDER, static non-pickup/immovable unbound map entry, exact map revision,
current control/death/idle, movement proof collision/occupancy/PZ and existing
stair cooldown. Quest callback has priority; scripted/bound ladders do not
fall through to generic ascent. Source geometry and unknown defaults stay.
Explicit real source ladder USE PG RED73.99s/server180006 returned
NothingToUse vs requiredCommitted; stale-revision refusal already passed.
New adapter check22.72s passes; integration GREEN pending. Source fixture
now selects existing371 diagonal capability, uses existing profile-aware
Desert STEP helper (visibility only widened), keeps real protocol movement.

Final source traversal admission closure: XML:3517-3519 archway1626/1627;
XML:3544 buttress1634/1635; XML:3918-3920 ladder1948; XML:43330 grass21935;
pinned XML1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2.
Same pinned client2dfa943b... and exact bindings1d19709c... independently prove
static/non-pickup/walkable flags. Four tests RED9.544s ->GREEN9.318s, final
registration write/check43 pass, every old native profile retained. Native
world pins refreshed/checked to808f55a3... via repository compiler. Latest
PG76.98s/180006 passes basement/native ladder/stale/full-position replay then
requires existing diagonal receipt runtime flag OTERYN_STEP_WAIT_RECEIPTS=1.
Fixture keeps that receipt assertion; rerun with same flag as Desert driver.
No whole quest or exploration GREEN claimed before this final rerun.

## Exact-origin goals and native XP transaction (r2 continuation)

At base5a26f9a0e, `grizzly_kills.rs::settle` called native-goal settlement
before the ordinary XP future. These were separate transactions, rather than
an atomic descendant. The focused native objective PG case activated the
source Dragon exact-origin cohort alongside its isolated prior-route phase:
PostgreSQL180006 RED75.68s (`/tmp/questpc2-exact-goal-atomic-red-pg.log`),
receipt query returnedNone instead ofSome(true) for same XP transaction.
The original XP/objective assertions passed before that new assertion.

Reuse the published sealed native-death XP transaction helper for exact-origin
goals; bind immutable spawn origin as well as death/Character/revision, require
that transaction's principal death audit and current session fence, evaluate
bounded catalogue phase/counter/prerequisite guards under row locks. Preserve
existing five-objective receipt binding when no spawn-origin proof is present.
Native-goal receipts now use that helper's stable death+Character occurrence;
no production migration or whole-quest qualification is claimed in AUTHORING.
Generic arbitrary Command/CreatureDeath UUIDs must not create these counters.
Fresh qengine2@5a6de28c1 had no native-goal transaction helper to reuse. Existing
incoming six exact-origin goals and ordinary corpse/loot closure are retained.

The first composition reached PG RED82.90s: `game_character_quest_receipt_guard`
rejected an intermediate receipt after two Quest writes advanced the same
Character twice. Migration0056 also uniquely constrains both original and
committed revision. Preserve these guards. One native death must compose its
same-quest eligible native goal/objective effects into one bounded revision
receipt, binding the ordered closed component plan. No extra receipt per
component is required. The added counter proof now asserts actual native
counter1 and same-XP xmin; every original objective receipt/phase/replay
assertion is retained. Run this refined case RED before the combined fix.

## Native captured eligibility that caps before settlement

The isolated retained-death PG case freezes a real native Dragon capture, then
explicitly installs earlier prior credit before XP settlement. It does not
qualify wire combat or whole-quest traversal. PostgreSQL180006 RED74.83s:
second XP settlement returnedAuthorityRejected because the first XP commit had
no Quest receipt after all component guards refused. The fresh-death audit
correctly refused a new grant on replay; keep that guard.

Seal this terminal no-grant result in the original XP transaction with one
bounded read-only current-phase effect and the original component binding.
Do not rewrite the already-earned counter or re-grant it. The receipt then
replays before fresh-grant checks, preserving current phase and prior credit.
The dedicated stale case asserts counter1 remains in its earlier transaction;
the unchanged normal case still requires freshly earned counter1 in XP's
transaction, plus the original receipt/phase/replay assertions in both cases.

## Earned actual basement landing enables the defender cohort

Actual exploration PG180006 RED75.36s showed own phase1 earned at source hole
landing32598,31653,z8 but native defender phase absent, expected0. The incoming
isolated cohort activation at32598,31641,z8 remains compatible. First whole
route basement callback (ownphase0->1) now activates the same cohort phase0 in
that one atomic quest write, retaining every per-origin counter. The cohort's
intermediate return flag may already be-1/0/1; first whole-route visit enables
it without resetting any credit. Own phase0 gates this once, and completed
whole-quest state cannot re-enter that first-visit transition. These progress
flags are authored approximation, not invented donor storage facts; physical
landing and source recipe citations/hashes remain those above.

Route fixture early returns for isolated native atomicity and exploration now
close their owned PostgreSQL database after success or a returned error, using
the same teardown as the full route. Assertions and panic behavior unchanged.
Read-only inspection found35idle bp_annihilator_route test databases;33older
than10minutes were dropped withoutFORCE and without touching active/recent DBs
(`/tmp/questpc2-idle-route-database-cleanup.json`), recovering about1.4GiB.

The widerange cohort activation proposal above was NOT accepted: catalogue
validate_closed correctly rejects any reopening or noncanonical activation
(PG75.39s). It has been reverted. Continue with one canonical once-only
activation callback matching both evidenced physical basement positions;
preserve all closed activation/counter validation and original source tests.

Canonical activation implementation: bounded selector.position_aliases (at
most7 alternates plus required primary) for StepIn/StepOut only. USE, native
fixed-position rules, duplicates, primary duplication, invalid floors and
unpositioned aliases fail closed. Empty aliases serialize exactly as before.
Canonical elvenbane-basement-entry remains the sole -1->0 activation key,
matching original32598,31641,z8 and actual landing32598,31653,z8. It also earns
own first-basementphase1 in that same write. Remove the separate own basement
callback; keep all native-run/counter closure validation unchanged. Existing
cohort entry therefore cannot become active without earning own basementphase1.
Fresh QENGINE2@5a6de28c1 and PC3/PC4 had no selector-alias implementation to reuse.
Physical landing witness: content/world/placements/region-z08-x127-y123.b3:1,
SHA25634d7229b0638876931954aced076b0dd62e600bf073ec0bd0f02ed47f9f5549a;
original source floor rules world/src/map/floor.rs:295-317. Source geometry and
per-origin credit remain unchanged. Generator qengine_access.py write/check
and scoped native43write+43check passed. Alias6cases and closed goal2tests pass.
Current PG activation proof, clippy and fresh pins remain pending WIP.


2026-11-07: source wire combat continuation

Production schedules callback quest-copy recovery with a60second backoff
(composed_admission/quest.rs:55; mod.rs:854). Before this fix only Monk
callbacks published their committed delta immediately. Native Dragon lethal
capture before that reload therefore observed pre-basement/pre-Dragon quest
eligibility. Actual PG180006 RED89.39s: normal source activation, full earned
exploration and chase-driven damage, but native Dragon counter absent/ownphase2.
Publish every committed callback delta and refresh both retained kill-capture
caches under the already-held runtime owner; preserve revision-gap fail-closed
and the existing durable reload. No test progress/HP/damage injection.

Reward source correction: CURRENT committed donor corpus, not merely the
cached manifest, contains crystalserver9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d
at data-global/startup/tables/chest.lua:2551-2580,
SHA2560ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca.
The four claims6253/6254/6255/6256 include two explicit source containers2853:
6255 carries237,3147,3059;6256 carries3028x2,3031x100. Seven payload kinds
are not seven physical roots. Reuse existing bound native-container MINT,
once receipts and exact source claims. Earlier seven-root atomic wording was
an authoring assumption and is superseded by these four source claims. The
Father eleven-track atomic rule is unchanged. Existing generated container
projections cite a separate historical201189 revision; do not fabricate that
revision in the current corpus or rewrite unrelated lanes' source facts.

Final callback publication proof: PG1800062cases189.21s includes earned source
Dragon wire lethal/nativecounter1/ownphase3 plus existing real Monk shrine
regression. Earlier RED89.39s ->GREEN88.91s. Check19.24s, clippy58.11s,
fmt/duplicate migrations PASS. Quest whole qualification is still pending.
Inspected existing callback execute.rs on QENGINE2@5a6de28c1, PC3@5e72a3f0b,
PC4@db95b72d3; all retain Monk-only immediate publication, no generic reuse.


2026-11-07: actual upper traversal and missing carpet profiles

PG180006 all-six RED88.70s requires6sourcecredits, onlyDragon1 earned.
Normal ladder32593,31643,z5 USE and stairs32588,31643,z4 then reach actual
upper floors; corrected next PG137.73s stalls on floor3. Diagnostic PG134.10s
retains bound target, player32589,31642,z3, SkeletonEast32592,31644,z3,
chase=None throughout; native Forge scales its actual HP, no fake health.
Source room's red carpet2572-2580 lacks native profiles. Reuse exact pinned
source_gate_item_profiles importer to add that closed static family.
Canary imports/canary/items-xml/items.xml:5314-5316,
SHA2561cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2,
identifies red carpet floor decorations. Client objects2572-2580 at
content/assets/files/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat:1
prove immovable, non-pickup Ground items, movement/projectile flags false;
exact bindings imports/crystalserver/bindings/items.json:1 SHA256
1d19709c28a649d8a9f32bf53e5bf43ad847d7901d6c6254833f456154d2de5a.
Existing importer validates those complete hashes and canonical conflicts.
Original placement witness content/world/placements/region-z03-x127-y123.b3:1 SHA256521b9242b03f5f7f5af3aa79c536ce6e31735134de9bc07a2bc593a2125836f9,
carpet around32589..32593,31643..31646,z3. No source geometry/walls modified.
QENGINE2@5a6de28c1, PC3@5e72a3f0b, PC4@db95b72d3 have zero exact native
profiles for2572-2580. Reuse importer, no missing-engine deferral.
Source5cases RED12.202s missing2572 ->GREEN12.977s with allnine exact flags;
existing profiles preservation/conflict and original ladder tests unchanged.
Native43write/check and world pins/full-source PG proof remain in progress.

Final carpet/upper route proof: actual PG180006 GREEN94.83s, source route
18508ms. Native6 exact-origin credits and own counter caps1/1/1/2 asserted
after real wire lethal combat and nativeXP drains. Both Skeletons can move
across source carpets; no geometry or damage injection. Native43write/check,
check18.84s, final clippy40.71s, fmt/duplicates and isolated world pin-refresh/
check PASS. World digest 56275a3ba06db4611a016821d1c0194c261e320ef983fcb215631911b56bc306. Whole quest
remains pending four source chest claims, exit/journal and replay/capacity.


2026-11-07: source startup chest UID declarations restored, not binder relaxed.
Full source PG180006 RED95.95s firstbp1 NothingToUse vsCommitted. Existing
quest_chest_bindings tool adds lane-specific cohorts; extended exact pair
validation for committed crystalserver9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
chest.lua:2551-2580 SHA0ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca,
startup/others/functions.lua:97-120 SHA897c4ea5572c6adef1d2b300fd75c141910d608a093ca15c3d3bf65fcf56826c
(loadLuaMapUnique stamps UID). Door-level source pin unchangedd8868f... .
Keep original201189 captures and native container projection as-is; validate
current four source rewards/containers against canonical definitions. Only
four UID metadata fields6253..6256 restored by repo tool; exact geometry/
allotherattributes comparison /tmp/questpc2-elvenbane-uid-preservation.json.
Source14tests RED1.480s ->GREEN1.582s includes source-byte corruption refusals;
apply/check35startupattributes PASS. Repo pinrefresh/check PASS.
Next actual PG100.40s proves four physical source claims, replay/once, four
roots and five exact nested children/custody sharing root receipt xmin; native
four-floor descent passes. Still RED on proposed south-east exit path, so
whole quest not qualified. Existing canonical return marker32597,31642,z7
is the documented next exit endpoint to exercise, preserving source geometry.
