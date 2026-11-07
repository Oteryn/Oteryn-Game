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
