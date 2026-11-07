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
