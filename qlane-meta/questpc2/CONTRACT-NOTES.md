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
