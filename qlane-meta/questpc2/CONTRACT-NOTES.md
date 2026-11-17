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

Canonical source return at32597,31642,z7 is real-route reachable: PG104.46s
passes all Native6/four-source claims/replays/atomic containers/descent/return,
then ownphase3!=4. Implement generic bounded claim-guarded callback completion,
using family/key once identity (0012 later revisions do not reopen claims).
Read same-character game_reward_claims under current fence/physical transaction,
after original receipt replay and before offering consumption. One terminal
8track write also sets completed journal state. No newSQLmigration. Pure
completion unit validates distinct<=8keys, terminal storage and exact current
Elvenbane source SHA; qengine_access/Native43write+check passed. FullPG remains
pending with zero/partial claims refusal and allfour positive finish.

Completion firstPG109.45s proves actual zero-claim return refuses closing
eitherphase. Driver re-ascent landed88,42,z3 then flatpath chose down-stair
88,43; correct native movement descended4, so assertion failed. Use ordinary
East step to89,42,z3 before flatsearch, matching previouslyobserved source
room approach. No geometry/guard/health/damage changes. Threecrateclippy149s
and driverclippy21.76s PASS; full partial/all-claim finish proof stillpending.

## Main #3020 composed receipt fixture configuration

The diagonal route assertions require actual pacing receipts, whose production
rollout switch defaults off (connection/authority.rs:429). Their fixture now
explicitly selects receipts through a cfg(test) composed-owner override. This
leaves the process default and negotiated capability requirements unchanged and
retains the nonzero, three-source-beat assertions for every diagonal.

## Elvenbane qualification and retained data-only recipes

Published runtime23852002b803e2d01021518c146b503d91f4c625 passed the full
Elvenbane source route, four physical claims, nested custody, earned exit and
actual wire journal in150.647s against PostgreSQL180006. The all39focused run
passed390.313s. Authored recipe compiler at
tools/content-schema/quest-authoring/authored_quest_authoring.py:86-99 always
emits data-only native holds/runtime_enabled=false. Those are frozen SOURCE
receipts, not production runtime gates; preserve them. The actual production
loader separately extends the native catalogue and callback/journal owners.
Whole chosen Elvenbane qualification follows the exercised runtime, without
promoting unrelated data-only recipes or deleting their tests/holds.

## Endgame template engine request and branch reuse check

Fetched qengine2,questpc3,questpc4,questpc5,questpc7,questpc8 on2026-11-07.
Their activity_template.rs is identical to main/current (SHA256 prefix
24a16c0dfca5). Lines362-370 reject unknown/source hooks beyond BossLeverOnDeath;
lines402 onward refuse summons/events. Rotten Blood candidates retain
callbacks_known=false and source closure blockers. Requested shared qualification
for registered source boss hooks/summons/reward closures fromqengine2. Do not
remove guards or turn these bosses into vanilla creatures. Existing progression,
physical corpse and private-reward owners remain the reusable implementations.

## Empty source reward-boss corpse capacity repair

Source imports/canary/items-xml/items.xml:63232-63236 declares Goshnar's Spite
i33867 with48 slots (containersize at63235). Donor04b83b512114bfd888000d6e1433ed8ecaec7c5b;
file SHA2561cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2,
repository import commit1b2e040732a7b0da269579380a4d3fa8a93d17aa. No source
capacity or Item definition is changed. The main-integration strict30 capacity
repair regressed this existing empty physical corpse: PG RED64.847s expected
one committed native source corpse, observed0; Rotten Blood and Primal passed.

The24-line new creature_reward_empty_corpse.rs helper reuses existing row
construction with no loot table, an exact native binding required by its caller,
and the reward-boss flag checked locally. It permits positive u16 source slot
counts only for this always-empty physical row. Ordinary/private row admission
retains1..=30, and actual contents/loot-plan quotas are unchanged. Existing PG
assertions still require empty physical contents, exact source custody/replay,
no fabricated Spite/Bakragore personal chest, and Primal's earned private census.
No tests are edited. This is a small repair of the existing physical owner, not
a new generic encounter feature; all six fetched lane branches lack this source
physical fallback and offer no replacement implementation to reuse.

## Soul War source Item admission subset

The seven source reward-boss loot tables contain44 distinct Item keys. Existing
source importers can complete43 canonical admissions; potion49271 is separately
frozen by the condition-potion SOURCE witness and remains canonicalfalse/native
true. It needs the requested shared native-loot bridge. The scope produces31new
canonical admissions, including green gem3038 (previous native custody alone did
not make its canonical record admitted). Existing13 canonical records, including
the protected potion, retain their full records. All44 qualified native physical
records remain byte-identical to publishedc8b42aba3. Existing engine policy at
native_goal_loot_items.py preserves admitted native physics: source values229,
101 and1500 for33921/33952/34109 must not be replaced by documentary230/110/150.
The new regression tests the actual fact owners and freezes both record sets; no
existing test or source guard is relaxed. Source inputs remain the exact pinned
Canary XML/client/Crystal identity bindings of the reused importer.

Shared source-shard receipts rebind only after matching their prior digest to
committedHEAD and proving selected shop/potion records unchanged. This avoids
a stale historical rebind base while preserving both original equality checks.
The source generation model was replayed:15cases/776creatures. Unchanged World
geometry was reused; registered native/authoring stages completed.


## PREPUSH-2 main NPC/ground-write integration

Decision: retain packet ownership until the coordinator transfers named quests;
compare current other-lane branches before starting another quest. qready1 and
qready2 have new commits; qbind1/2 and qdata1-8 still point to main 1ae39c7f.
QENGINE2 c7b6297fc still admits only BossLeverOnDeath in activity_template.rs;
the existing Soul War/Rotten Blood engine requests therefore remain applicable.

Main f641d8d6c contains an E0061 compile failure: consume_npc_inputs calls
apply_input with three arguments after MAINRED1107B added SpellLanePermit and
CurrentCharacterGameplayFence. NPC hand-ins already receive their own
NpcItemPermit, constructed in quest_state/npc_program/items.rs:41 after the
NPC owner performs current equipment admission (npc_program.rs:313).
Do not invent a spell permit for that owner. The small new
quest_world_callback_npc_input.rs wrapper requires the real NPC permit and
refuses ground through the carried-only location query. The existing spell
wrapper retains its exact channel check. Both call the unchanged SQL mutation
body. The regression covers all four admitted carried location kinds and the
ground refusal; the real NPC XP hand-in remains the integration proof target.
No migrations, quantities, receipt assertions or production guards are weakened.


## Decisions from the PREPUSH-2 unit run

Five source corpse aliases are newly available relative to main: Apprentice
Sheng uses admitted Monk corpse5981; Bazir and Infernatil use Demon corpse4097;
Dharalion uses Elf corpse6011; Bakragore uses Rotten Blood corpse44012. These
are consequences of the existing questpc2 source corpse admissions, not new
quests or new private reward entitlements. The source reward-boss census is
387 total:52 missing bodies,324 bodies still inadmissible,10 named independently
checked corpse closures, and the existing Primal empty-row case. All five new
aliases still have inadmissible private loot. Preserve every old private-loot
assertion, add those five explicit refusal expectations, and keep the complete
census exact. Scratch derivation: /tmp/questpc2-unit-source-corpse-census.json.

Spite's source corpse33867 has48 slots; Bakragore corpse44012 has30. The ordinary
private-row API retains its existing30-slot bound, so the exact source refusal
for Spite is CorpseItemInadmissible before private-loot lowering; Bakragore's is
LootItemInadmissible. Keep both exact fail-closed reasons and empty physical
corpse assertions. Do not relax the30-slot private bound or admit unknown loot
just to make tests pass. Independent private admission remains requested from
qengine2. The previous new test incorrectly expected the same error ordering
for both source bodies.

Nine durability PG setup cases hit SQLSTATE53100 while completed build caches
filled the lane filesystem. Retiring only completed lane incremental caches
recovered8GiB; this is an infrastructure retry, not a mechanism blocker. Collect
all failures from the full selected run before applying the repairs together,
then rerun the complete same selected set. Own PG compilation was restarted.


## Six Rotten Blood source Item facts, without runtime admission

The retained Crystal summer XML at revision
00ce02a57ca5a12e48f32a3476e37471167e4c3f, data/items/items.xml,
SHA25613a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece,
declares43501/43502/43503/43504 at lines75234/75237/75240/75243 and
43899/43900 at76732/76736. These are the four boss essences, cursed wood and
darklight geode. None declares weight: the retained Canary constructor declares
int32_t weight=0 at src/items/items.hpp:282, revision
04b83b512114bfd888000d6e1433ed8ecaec7c5b, SHA256
b8c12f45122eeb575ad548719c5fb178a96460d8a41ad882d0d88b2e66d797e6.
The existing build_source_item_profiles.profile loader applies that same zero
default; the fact packet labels it loader_default_zero rather than claiming an
explicit XML weight. Existing client/identity/Canary XML pins remain mandatory.
The existing identity factory accepts these six exact binding rows. Stack and
portability come from its pinned client and loader policies, not descriptions.

The49-line helper replays the retained archive, exact XML and existing identity
factory. It captures normalized core facts only. Native admission and use
 effects remain false; descriptions about consuming essences authorize no new
use/charge mechanism. Runtime admission needs its own real caller proof in a
subsequent slice. A tampered archive fails closed. The new source test was RED
for the missing producer, then RED for missing generated facts (2.973s), then
GREEN2cases2.854s. No runtime content, pins or production Rust changes here.

The conservative four-crate rerun from the previous batch completed:
1326/1326 selected tests PASS1024.412s, including all9 repaired infrastructure
retries. Existing filters/ignore declarations are unchanged; no excluded case
is counted as passing. Publish the final receipt with this source-fact batch.


Main advanced to47db34f09 (#3032) before publication. Reused main's
apply_input_rows and removed the duplicate mutate_input layer. The NPC wrapper
still requires NpcItemPermit and refuses ground; its negative test remains.
Main's SQL and spell channel check are unchanged. One merge conflict was
resolved immediately, retaining the lane party fence helper. The source fact
batch therefore also changes durability and must run its units plus fresh real
NPC hand-in and endgame PG cases before publication (or report pending tests
under the owner cadence exception).

The clone fetch refspec tracks only main. Explicit own-branch destination
refspecs now refresh its remote-tracking ref too; always use them before push,
then merge actual remote movement. Never rely on the stale historical lane ref.


## Runtime admission of the six ready Rotten Blood Items (in progress)

Real PG RED: rotten_source_items_materialize_into_exact_character_custody ran
35.586s against PostgreSQL180006 and refused43501 with DefinitionNotFound.
The test uses the production native definition lookup, mint, transfer and
carried-custody query. The other two selected PG cases passed. Source regression
also refused all six canonical definitions before the admission change.

Existing canonical physical facts remain authoritative: essences43501-43504
retain weight150, cursed wood43899 and geode43900 retain450. The two already
admitted native43899/43900 records retain their own qualified zero weights.
The four new native essence profiles receive canonical150, not a retune of
accepted facts. All34105 nonselected canonical records and3687 nonselected native
records have frozen complete-record hashes in the regression. No use effects
are admitted by this Item custody slice.

A small authoring hook supplies the six fully source-verified profiles to the
existing native custody merger, which retains binding, exact charges/time and
physical fact-owner checks. Default callers still use the original pinned
importer. New profile sets must match all six selected Item identities exactly.
The generic encounter importer still refuses arbitrary alternate XML scopes;
its44158-only guard is unchanged. No runtime quest-engine feature is added.

Post-main3032 validation finished:698 selected durability units PASS314.521s,
NPC hand-in and Rotten corpse PG cases PASS2/2 in67.798s. No pending failures
remain from that merge; publish the final receipt with the next green slice.


- Remaining Summer memberships (43961/43962/43964/43966/43967/43968)
  preserve the already admitted EXACT binding rows at00ce02a57ca5a12e48f32a3476e37471167e4c3f.
  Their retained XML evidence is data/items/items.xml:76781,76785,76793,76801,
  76805,76809; SHA25613a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece.
  The identity factory accepts independent201189f/8a4a526c evidence only and
  rejects all six with "Selected Item identity requires independently qualified source revision".
  Decision: publish normalized membership facts for qengine2; do not relabel
  Summer bytes, revise identities, relax the guard or admit runtime/use effects.
- Items43946/43947/44048 have named, takeable client appearances but no row in
  the retained server XMLs or accepted binding table. Client usability alone
  does not prove server registration, portable weight or use effects. Preserve
  refusal; do not infer scroll promotion or horseshoe effects from display names.
- This source-only batch adds no Rust or SQL and changes no runtime content.
  Existing PG180006 proof3/3PASS133.133s belongs to the prior six admitted Items;
  it is not evidence of admission for the six new membership candidates.


- Collector3030 merge78294c081: main already admits portable slain-skeleton
  Items3990/4103/4104. Keep their complete native records and both carried/Ground
  destinations. The lane's conservative no-pickup policy still applies to the
  other17selected corpse/decay Items. Canonical physical/container fields for
  these three now come from the same pinned existing source importer as main,
  rather than overwriting accepted source facts with a conservative default.
  Canary04b83b512114bfd888000d6e1433ed8ecaec7c5b:data/items/items.xml:11924-11929,
  12485-12494, SHA1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2.
  Client flags use existing pinned appearances2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2;
  Item4103capacity8 is the accepted loader/client default from
  build_source_item_profiles.py:73-78, not an invented XML attribute.
  Existing movement and weight must still agree; missing admitted portable
  profiles and conflicting known facts refuse before publishing any output.
- Merge keeps both runtime guard sets: earned-reward and shared-world gates;
  pending-party admission plus added world objects; lane quest persistence
  plus main shared credit/benevola/hive guards; exact NPC exchanges plus main
  Yeo/fluid/full-charge behavior. Reuse main's typed input_authority permit,
  with the lane carried-only NPC wrapper and negative ground test retained.
- Strict outside-six Item fingerprints must be regenerated after accepted
  upstream record changes, using questpc2_rotten_fingerprint_rebase.py during
  the exact active main merge. It first reproduces the old committed proof.
  Keep both full-record digest assertions; never hand-edit the fingerprint pins.
- Owner freeze decision: publish this merge once, then freeze questpc2 for the
  collector. No new features on this branch; subsequent features belong on
  claude/happy-davinci-ijz3wy-questpc2b. Remaining quests are not qualified by
  this integration work or by source-only membership evidence.

- Migration composition after collector3030: own1240-1242 run before main1333.
  Preserve main's exact opposite-side and RHS typed-definition substrings for
  its Yeo extension. Side3 permits only item BURN/MINT kinds and maps only the
  exact expected input identity/ordinal on the LHS; wrong input keys becomeNULL
  and fail IS DISTINCT FROM. Plain-input, empty-box and20hcooldown predicates
  stay intact. Main1333 is unchanged. Full chain through1349 passed onPG180006
  in3.001s; Yeo/Al Dee/Uzgod/Seymour guards all retained. All12initial PG failures
  shared this setup cause; do not count them as executed mechanic proof.

- Continuation2026-11-08: owner says resume while keeping questpc2 frozen for
  collector. Decision: allocate claude/happy-davinci-ijz3wy-questpc2b from
  frozen0a6d6e208; continue original packet and publish receipts/fixes there.
  The13focused runtime cases run on the unchanged frozen production tree.
  Earlier pending commands failed compilation ENOSPC, so they are not passing
  runtime evidence. Retire only inactive lane artifacts, then rerun the exact
  relevant dispatch cases without compiling unrelated package test binaries.

- Pits of Inferno source review: movements_fire_tile.lua:41-48 atCanary04b83,
  SHAdc674efaebf58f68cf579cad612095e2b7b4af9ea9caec463a3941e3ce41d9d9,
  applies300fire to the matching vocation and escalating300..7200fire/physical
  damage otherwise. Do not replace this with visual cues or silently bypass
  it. Shared effect requested fromqengine2; no whole-quest qualification and
  no new implementation started from merely inspecting the source.

- Hero of Rathleton continuation: reuse main's active four Strange Pipe NPC
  programs unchanged. The retained Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d
  data-global/npc/strange_pipe{,1,2,3}.lua:59-62 sets exactly its own access
  track on greeting. SHA256 respectively68dd38d00e9e4b2155f998e41f2b51cc7096086a6a36c385fa528bfe3adb572b,
  7b30d3a0c2d3bb76285fc3039d561947895f8f81b4fc952ac60ca9210378f5dc,
  e58745d7bf70da2779e2e7b4651db4c204cc5318d5051380c2671cd080874027,
  f91fc304eeee77003f8db299ed7ba05493da256c943930bf7f3c88981ff0a2d2.
  Add real command/receipt coverage for existing behavior; do not invent a red
  defect or claim a new implementation from a passing baseline. Access grants
  alone do not qualify the quest. Timed waves, room occupancy, cleanup and
  boss-death portal requested fromqengine2; source movements_professor_maxxen.lua
  :3-30,56-82 SHA6005aec45eeed29581e0066a8597df22f7957a3a112c1052ed507a14dbb51abf.
  Counts after starting this owned slice:3qualified,3inprogress,23notstarted,
  6excluded. No named ownership transfer received.

- Hero PG RED onPostgreSQL180006:8receipts instead of4after repeated hi,
  duration66.353s, log/tmp/questpc2b-hero-pipes-baseline.log. All four access
  values and NPC source replies passed first; failure is redundant durable
  commits from planned Set1->1 writes. qengine2dd35529b6 has the same path.
  Small local helper catalogue_track_mutation.rs (8lines) detects whether any
  planned track value differs. Keep the entire original write list when any
  value differs: Father acceptance must remain one atomic11-track write.
  No inventory/reward semantics or source rule changes. Re-run Hero plus the
  existing realPG Father acceptance and XP hand-in checks; retain the RED
  assertion unchanged. This is a real caller fix, not merely baseline receipt.

- Mainb3864f14a public projections merged into continuation only. Preserve
  public statistics queue2 and own objective cache3; fix three stale positional
  accesses together. Main migrations1350/1351/1352/1354/1355/1356 unchanged;
  lane1240-1244 unchanged, no renumbering or duplicate needed.
- The broad unit baseline observed exactly two real assertion failures before
  cadence interruption: cap371offer_gate metadata and profile count3706versus
  actual3768. Exact62added keys, no removed main keys; enumerate existing
  source cohorts in new24-line helper, assert presence/zero-speed for each,
  retain raw-source speed and exact unknown-speed checks. This admits no Item.
- Broad collection remains partial:723old-tree results,2035notexecuted. The
  verified name partition covers2758planned cases but is NOT completed proof.
  Two worker interruptions are not additional assertion failures. Full newmain
  suites go toCI; explicitly use PREPUSH-2step3exception for compile checkpoint.
  Main merge must not wait for broad suites. Seven focused regressions remain
  mandatory follow-up evidence; never claim PG or full unit success prematurely.
- ENOSPC retries retire inactive own target caches only. Disable incremental
  caches for this validation; compiler checks and test scope are unchanged.

- Book2824integration: countfix exposed an additional original assertion,
  expected qualifiedspeed0versusNone. Existing main also hasNone. Pinned
  NativeGoalLoot source already yields0, but existing-record merge omitted it;
  SRX1correctly preserves physical admission and therefore doesnot repair it.
  Bound repair to2824only, fillunknownspeed from verified source0, retain
  knownvalues/otherattributes. CanaryXML5813-5817SHA1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2.
  SourceproducerRED8.235s;7producerGREEN and8fingerprintnegative-bound checks.
  Strictoutside-six proof remainsactive. Repository48-line refresh tool first
  reproduces HEADproof, permits only existing2824speedNone->0, requiressource0
  and canonicalidentity. No hand-edited fingerprint/pins or weakened assertion.
  Final7/7Rust215.213458s including5realPG180006; Python16PASS33.306s.

- OldrakNPCslice decision: reuse generic rules for source102-124and151-153,
  Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/npc/oldrak.lua
  SHAdc124275de082d9f39bc2f802b31f823646548153fe0e248417dc54f7b0eba40.
  Coalesce synchronousProgress1then2intoone final-value atomic write. Preserve
  sourcebranch order:Progress2/Done<1elseif is unreachable behindDone<1.
  Done1finishdialogue is a distinct NPC step, not earnedwaveproof. Existing
  canonicalwriter may establish that explicit testprecondition only.
  NativeDoneabsent0anddonor-1both satisfy theonlyinitialDone<1guard. Noaxe
  grant,fee,item919orwavequalification inthisslice. Requestbigfeaturesfrom
  qengine2; narrowlypinnedNPCsource admission is a smalllocaldatahelper.

- PREPUSHmain83acbdac3attempt:44conflictsacrosssourceexecution,persistence,
  worldcaptureandderivedfiles. Keepbothimplementations; no blindtheirs/ours.
  Ownerauthorizesabortwhenunsafeinside15min. LocalOldrak5251checkpointfirst,
  thenabortmerge; publishcompilingstateafterchecksandcontinueintegration.
  Sourcecustody39unitfailures sharedoneextra per-fileguard; sameexactOldrak
  custodychecksnowcoverbothgates. Fullrerun289/290:soleexpectedactivation
  closureupdatedfornewdemon_oakprogramme, allotherlegacy/hashassertionskept.

### Main integration 2026-11-08: stationary profile composition

Main 6f61ac15d introduces floor projection profiles for 1948 and 1791. The
retained source-gate packet identifies 1948 as LADDER from its pinned Canary
XML; main's Crystal packet has no item-type classification for the same physical
record. Preserve the positive LADDER fact and main's nonmaterializable projection
admission. For 1791 both packets match in every field except materializable;
preserve main's false admission. The new composition helper requires exact
record equality after only these named differences, rejecting other collisions.
Source custody remains in source-gate-item-profiles.json and
floor-assembly-item-profiles.json, including their XML hashes and line evidence.

The merge driver's additive Python resolution duplicated `native_charge_item`,
`claim_items` and `CHARGED_QUEST_CLAIMS`. The later Desert-only declaration
silently shadowed main's five exact claims. Keep one shared implementation and
the union map. Canonical3083 is already materializable in BOTH merge parents;
3082 is materializable in the lane parent. Main's tests that assumed3083 held
were therefore stale even on main. Preserve all negative mutations using an
explicit held copy, assert the real catalogue is unchanged, and exercise all
five exact claims. Focused source-reward7cases now pass13.184s.

Pinned source coverage retains283 supported-unqualified claims after composition:
Draconia becomes supported and Triangle Tower gains main's dispatch witness.
Neither transition certifies a complete quest. Coverage was regenerated using
Git blob7b9f823542f9b8d5f99fb6960abe070324c89b8e (Crystal SHA2567c3ac128...)
and retained Canary blob13efcb4d1294ec1ff1ee5e5da18e2eeb3c904c02
(SHA2562b130d41...). The coverage generator verifies the full pinned hashes.

WorldProject base regeneration was bounded to applicable outputs: all11 accepted
WorldProject documents have identical Git blob IDs in both merge parents. Stop
the unnecessary historical predecessor re-admission before any repository base
write, and run world_project_v2_to_tree.py against those accepted inputs. Native,
registry and pin producers still run. Dispatch test build is ordered AFTER final
pin refresh because client placement-index hashes are compile-time inputs.

Exact classification evidence for the composition: imports/canary/items-xml/items.xml:3918-3920,
SHA2561cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2,
explicitly declares1948 type=ladder. Main's retained floor packet pins
Crystal201189f data/items/items.xml:3935 (1948) and:3873 (1791),
SHA2568a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509.
The helper preserves the positive classification and conservative false
materializable admission; every remaining field must match exactly.

### Oldrak paid axe follow-on: reuse audit 2026-11-08

Pinned Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d
`data-global/npc/oldrak.lua:126-149`, SHA256
dc124275de082d9f39bc2f802b31f823646548153fe0e248417dc54f7b0eba40:
Progress2 offers an axe blessing; success consumes3274x1 and1000 gold across
inventory+bank, then gives919x1. The source also emits yellow energy at the NPC.
Canary XML:3150-3153, SHA256
1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2,
has no decay/duration on919; the NPC's warning is not evidence of a timed Item.
Canonical919 is materializable but lacks native admission. Canonical3274 is
held/stack-unknown, while an existing native profile admits its carried intake.

Main's paid RewardCallback can consume a fee and items without a storage write.
It cannot safely be appended beside the current source Oldrak program: source
catalogue topics and quest_dialogue topics are distinct. A proposed axe offer
would retain quest_dialogue topic2 across a source `mission` offer (catalogue
topic1); the next `yes` is selected by reward_callbacks BEFORE catalogue, so it
would buy an axe instead of accepting the mission. Routing evidence:
`npc_command.rs:788,834`, `npc_command/quest_dialogue.rs:27-69`,
`npc_command/catalogue.rs:91-108`. This is a rejected composition, not a claim
that the current (non-selling) Oldrak program has this bug.

The generic source item owner excludes TakeMoney, and storage-free item plans
are refused at `durability/quest_state/npc_program.rs:278-279`. Latest qengine2
aa85b2500 was checked; no compatible source-money/topic bridge was found.
Request sent to QENGINE2 for repeatable atomic source item+bank-money exchanges
under one NPC topic. Do not invent a shadow progress counter or patch a second
confirmation owner into this lane. Continue another supported packet entry.

Halvar was inspected for the owned Barbarian Arena packet entry. Exact source
Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d
`data-global/npc/halvar.lua:74-114`, SHA256
10ac5f7b7e7c58e6406112bc8a3eaae9c0704dbcc25473fb567fc31aa99f51eb,
requires a bank-capable fee, PitDoor and per-tier questlog writes, plus topic
reset on every non-yes fallback. Source NPC money ownership remains closed;
request forwarded to QENGINE2. No Halvar gameplay implementation was started.
Sven's Barbarian Test is a different, unowned quest; no Sven work was authored.

Next supported slice: Henricus recruitment and first mission in The Inquisition.
Exact Crystal9f5a72c source `data-global/npc/henricus.lua:65-102,204-225,286-290`,
SHA2561b98530e7f7ae4bef44e9d55373254e6590a160c7c8f5d8d9dd41b578bf7977d.
These paths need only existing storage/topic writes. The unconditional blessing
price query at:63 contributes no state effects to these selected branches.
New PG regression RED on180006 after65.812208608s: missing source recruitment
offer: []. No direct quest-state SQL seed, fabricated guard credits or mission
completion. Implement the missing source program next.

Henricus's unconditional price query is read-only for Player state:
Crystal9f5a72c `data/libs/systems/blessing.lua:231-244`, SHA256
e4cc6975aab546c73e2fa8ba59ee42427e030ce39dad0b913800bc1b54569b4a,
counts missing blessings and computes/returns price from level. Its Lua filter
assignments do not mutate Character state. Storage-only selected branches do
not need a new money/blessing owner merely to compute an unused price.

The first Henricus programme follows source branch order (inquisitor, join,
blessing/flask refusal, mission/report, confirmation and decline). All five
interrogation guard flags must equal1 before its report-success branch writes
Questline3 and Mission01=7; the positive report path is authored but NOT yet
PG-qualified because guard acquisition is not implemented in this slice.
The current PG case earns recruitment and first mission through NPC talk,
then proves an unearned report refuses. Whole-quest counts stay unchanged.


## Gerimor assignment slice (2026-11-08, questpc2b)

Decision: preserve the source's volatile selected cult by compiling Lua
`storage[playerId]` plus topic 3 into distinct private conversation topics
300-306. They carry no durable progress; only a live `yes` writes the selected
mission to 1 and, when still unset, the shared questline to 1 in one existing
storage-owner transaction. Topic 2 uses exact, case-insensitive whole-message
cult selection. It ignores `yes` and `no`, as the table lookup precedes those
branches. Greeting/farewell resets must discard the selected confirmation.

Custody: Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
`data-global/npc/gerimor.lua:255-343,355-425,449-452`, SHA256
bf0fba86d07bb8a70d6242c2cc748fbb137cc7b583b47f0886530b6b5c584b2f.
The producer verifies retained corpus bytes against this exact hash before
extracting text. Source table keys and per-cult goals remain distinct. No
boss progress, experience, item rewards, or final access are granted by this
initial-assignment slice. Reports at their goal are deliberately outside this
slice's executable branch set; do not certify the whole quest from these tests.
The retained Minotaurs mission domain ends at 5 although this source's report
increments 5 to 6 (lines 266,397-399); that later report requires an explicit,
source-backed authoring correction, not a hidden test seed or relaxed bound.

Ownership checked against qbind1-2/qdata1-8 (all 1ae39c7f), qready1
(a588ad232), qready1b (39a47f16e) and qready2 (a9f723152); no Cults assignment
program exists on this lane (the real-dispatch RED test confirms it); no transfer
was received. The qready1 ref disappeared during its bounded refresh, so its
unavailable head is not claimed as inspected implementation evidence. QENGINE2 was refreshed to
614d00506, whose tested authored-binding capability will be reused separately.


## Next supported work / engine boundaries (2026-11-08 after checkpoint50cf70e7f)

Bounded follow-on reads made no new quest implementation and do not change the
3whole-qualified/6active/20not-started owned count. QENGINE2 refreshed to
614d00506, with tested authored kill and native recipe bindings. Do not merge its
123-file feature bundle speculatively before an owned quest can consume it.

Asura Palace's accepted authored recipe explicitly requires premium and uses
fixed sequential kill gates. The authored native MapUse/StepIn owner lacks a
fenced account predicate (RewardCallback premium applies to that separate owner,
not earlier world admission). Requested that missing predicate rather than
silently permitting a free account. Source choice remains the accepted recipe,
not inferred door coordinates or fabricated encounter storage IDs.

Arkulius Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
data-global/npc/arkulius.lua:98-169, SHA256
3d9f5879d653c522d80b7548e547acd150d43fa1c301feb43ec7a0788af36985,
requires level80 and distinct sorcerer/druid/paladin/knight offers. The current
storage owner and QENGINE2 614d00506 both refuse Level/Vocation guards at
evaluate.rs:327; the plain item owner also refuses these guards. Requested the
fenced source-plan capability; did not bypass class or level requirements.

Stricken Soul, same revision, data-global/npc/stricken_soul.lua:63-94, SHA256
506efb1fc93172d5a9fef91360686e1cb080d6830897d44e5cb8dfd86ea8b39c,
requires the current player's name after two yes confirmations and interpolates
it in two of eight replies before questline1. Native rule input has no such
value/keyword. Requested the source dialogue capability; no hardcoded player
name, substring wildcard or premature progress credit was authored.


## Albinius temple contribution source contract (2026-11-08)

Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
data-global/npc/albinius.lua:105-137, SHA256
1a476ee079849acee80dd99fb107ae5fd674c2d3c3343beed8002840d09cea18.
Five exact heavyoldtomesi23986 are removed before Tomes1. First success retains
topic1; a subsequentyes reports alreadydonated and resets0. Missing five leaves
the confirmation open. No ends interaction. Independentifs mean templeyes offers
and contributes in one message. Tomes info sets topic7 and cancels that pending
contribution. No realm, boss, rune, portal or shop qualification is claimed.

Decision: use the existing plain-item/storage owner, exact item guards and
continue_matching source order. Unsupported mixed buy/astral-shaper-rune/five
portal phrases are excluded from this partial program, so no contribution is
earned by silently omitting known later shop/rune/portal effects. Source fixtures
use existing fenced mint/transfer owners to carry individual books; no quest
progress is SQL-seeded. The first fixture attempt duplicated an already-present
backpack and is NOT a mechanic RED proof. Coordinate conversion uses explicit
checked u16/u16/i8 conversion from movement's i32/i32/i16 types.


## Gnomus initial ordnance admission (2026-11-08)

Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
data-global/npc/gnomus.lua:119-176 SHA256
04ce96e16bbfd9f6fa6b114b377643b1bb71c3df33f05c2b1e1d0d5e40bdc5a1.
Initial offer requires Ordnance<1 and sets topic22. The following yes is an
elseif: combined ordnanceyes only offers, unlike Albinius's independentifs.
Acceptance conditionally sets shared questline1, sets Ordnance1, resets both
GnomesCount/CrawlersCount0 and sets topic1, all through the existing storage
owner's ordinary limit. A pending mission says come back; repeatyes cannot
restart it. Literal cooldown predicates remain >0 versus <=0, not an invented
UTC comparison. Other task/status/conversion phrases are excluded from this
partial program so their effects are not silently omitted from combined input.

The PG fixture earns acceptance through ordinary NPC talk with no quest SQL
seed. Initial counter defaults are0, so that case does not prove resetting an
earned nonzero escort count; the data test independently requires both source
reset effects. No escort events, points, rewards, or warzone completion are
qualified. No new generic engine or transaction limit was introduced.

## Follow-on source checks (2026-11-08)

Gnomus measurements needs nine written tracks, charting ten (including conditional
questline). Existing set_track counts even unchanged writes against eight; do not
split acceptance or extend the Father-specific exception. Requested QENGINE2.
Task reports also combine item reward, added points and now+72000 cooldown; the
plain-item planner excludes SetNowPlus. Requested atomic support, no split writes.
QENGINE2 refreshed64b3597c1 contains validation/main integration, not these APIs.

Mr Brandon retained crystal9f5a72c data-global/npc/mr_brandon.lua:1-62
SHA0ff104958d5027d2f0299e903ada4e3de9385105e45c5cac7ba6802d9a6b3259
is configuration only with no ring quest dialogue. Do not invent donor testimony
for Twenty Miles Beneath the Sea from this file. No quest work started there.

Next supported work: Albinius holy/energy/earth/death offerings at retained
data-global/npc/albinius.lua:194-280 (same pinned SHA as initial contribution).
All four fifty-count offering types already materialize as stack-capable Items.
Ice remains excluded because its Formorgar prerequisite belongs to another quest.
Source portal no retains the topic; combined keyword+yes and multiple portal
keywords execute independent if blocks. Preserve these rather than invent cancel
semantics. RED PostgreSQL cases added before runtime program extension.

Partial-program confirmation policy: unimplemented Gnomus task keywords clear
its local confirmation topic without replies, progress, inventory or reward.
This is an explicit Oteryn admission boundary, not a claimed literal donor
setTopic(0). Otherwise measurements after an ordnance offer could leave topic22
and allow the next yes to start the wrong task. Native PG regression is authored
before applying this runtime change. Ordinary unrelated words are unchanged.

Albinius combined four-portal confirmation immediately after tome donation must
also retain the donor's earlier already-donated reply (topic1 survives success):
nine replies total, not eight. The source independent if ordering explains this;
no test expectation is derived from an invented cancel/reset transition.

## Night-retro full authoring validation (2026-11-08)

Fresh mainf5b584ef1 is CI-only and merged cleanly. The new mandatory full checks
found one frozen SOURCE discrepancy: live shared Item admission makes Draconia1
native_contents available, but SOURCE compilation had dropped its historical
claim_item_semantics_missing witness. Extend only claim_catalogue's private
historical hold view; do not change Draconia gameplay or canonical reward data.
The existing amendment tool proves every frozen SOURCE core unchanged, then
refreshes compiler pins. Its accepted prior compiler SHA2de10695b57d2d95cbf012df84b9a8086539e2d43866b53c18f3e1e75ba5c694
is exactly the previously pinned compiler; the older f8ed snapshot stays accepted.
No manual proof SHA edits, new approved source pairs or changed immutable inputs.

Variant test keeps the two stack normalizations and exact previous37 held count,
while explicitly accounting for all three native projections (Ornamented Shield,
Triangle Tower, Draconia1). Draconia source UID6023 has exactlyi3284x1+i3297x1;
new assertions bind that payload. Current native-lowering error total34 reflects
that existing projection, not deleted evidence. The completion matrix still
asserts Draconia SOURCE holds and NOT_ASSESSED wholequest playability.

Coverage33qualified/20notplayable/283supported keeps the full336inventory and
283denominator. Five already-qualified selected keys (Nightmare Teddy, Purple
Tome1-3, Panpipe key4055) are additionally asserted individually with source
reward matching. No status was promoted by changing the test. Full unit runner
collected all failures: quest_tree_authoring (five errors and one failure),
quest_claim_qualification and quest_completion_matrix; checks also found the
related old variant total. Fix these together, regenerate the documented complete
quest dependency pipeline, rerun both exact CI authoring commands and content
validation before push. Own6realPG remain GREEN; no wholequest count increase.

Post609 validation: final dispatch nonPG1613/1613PASS723.980s with content frozen;
no remaining failures. No new quest code after that push. Coordinator10:20 asks
for receipt publication before further quest work.
Read-only source check: Within the Tides canonical source data identifies Marapur
and Timira, while data-global/npc/tarun.lua:65-96 (9f5a72c, SHA8e032f7015dd3ee2135ba9552a588a3f26083327e23fb7a1619423486c482fde)
uses TheLostBrotherQuest storage. Do not substitute that unowned quest's progress.
Timira source actions_boss_timira_fight.lua:174-241,299-364 SHA49604399bcd3bd22e62b62babc48b6ad72d61dc0f8ec889ee218ba7654b1ed68
needs item-driven encounter stages, transformations and delayed reset/evacuation;
request recorded, no local shared engine written. Maeryn data-global/npc/maeryn.lua:134-173 source43b066a640d3b3d01a73cec683c0c0ddf7fce54e8e8491b0ddd80dc9dec954f8
has a supported helmet-choice storage slice; leave it for a later batch. No
Within the Tides or Grimvale gameplay was authored or newly qualified here.
