# OTV2-20261007 reward-only Quest runtime audit

Status: retained read-only evidence and implementation blocker classification
Baseline: Oteryn/Oteryn-Game@66f0d4336e47df5d93bc85a53669ea1a047253c5

## Outcome

The 42 all-373 rows previously classified DEFINITION_READY_RUNTIME_UNKNOWN are a single
bounded class: every one is a canonical Quest with kind reward_only.

These quests do not need a synthetic QuestState track merely to make a chest reward executable.
Their runtime owner is the existing RewardClaim / chest path. The all-373 matrix therefore
classifies this class as REWARD_ONLY_RUNTIME_PENDING; this is a work-queue classification only
and does not claim playability.

## Canonical claim census

PROVEN on the baseline above:

- reward-only wiki-title rows: 42
- distinct canonical RewardClaims referenced by those quests: 59
- referenced RewardClaims found in canonical shards: 59 / 59
- RewardClaims with readiness ready: 59 / 59
- RewardClaims with one or more concrete placements: 59 / 59
- RewardClaim placements carrying a QuestState quest_transition: 0
- claims with canonical quest missing_data: 0 for this 42-quest class

The absence of quest_transition is not by itself a defect for reward_only: the one-time
RewardClaim is the reward mechanic. CHEST-QUEST-BIND-1 only adds a QuestState transition when an
exact source progress write can be lowered to a QuestState track.

## Known global chest-placement blockers do not belong to this class

Current plain RewardClaim shards contain exactly these ready/provisional appearance cases:

- oteryn:reward-claim.quest.u11_80.the_secret_library.small_islands.parchment uses appearance
  28828;
- oteryn:reward-claim.quest.u15_24.targuna.mana_potions_chest uses appearance 28827;
- the Targuna silver-amulet claim also uses 28827 but is not readiness ready.

None of those claims belongs to the 42 reward-only quests.

The one current readiness-ready placement that has no CrystalServer legacy unique id is
oteryn:reward-claim.quest.u8_6.wrath_of_the_emperor.chest_items; it also does not belong to the
42 reward-only quests.

The baseline Item shard does not contain oteryn:item.tibia.i28827 or
oteryn:item.tibia.i28828, so CHEST-APPEARANCE-ADMIT-1 is not treated as implemented by this
audit. This does not block the 59 claims above.

## Runtime boundary

PROVEN current code:

- apps/game-server/src/content/world_reward_claims.rs implements CHEST-PLACE-BIND-1 and can
  bind an embedded ready RewardClaim placement to a WorldBase entry by exact native cell,
  CrystalServer unique id and appearance, while requiring the reward Item to be admitted.
- apps/game-server/src/durability/reward_claim_mint.rs owns the one-time durable RewardClaim
  commit.
- the fixture/native-entry gameplay path injects with_entry_chest and can execute that bounded
  chest path.
- a configured World bundle in apps/game-server/src/node/serve.rs currently passes
  MAP-CUTOVER-1a checks and then stops with WorldBundleUnserved; the comment explicitly says
  serving the bundle needs MAP-CUTOVER-1b.
- current apps/game-server/src/content/activation.rs has no activate_world_bundle, and the
  node does not call bind_embedded_reward_claims for a served imported World.

Therefore the 42 reward-only quests are not PLAYABLE_VERIFIED. Their common blocker is the
already-designed imported-World serving composition (MAP-CUTOVER-1b / WORLD-CONTENT-SERVE-1),
not missing Quest definitions, QuestState lowering, or missing RewardClaim identities.

## Required next proof

After the imported World serving owner is allocated and implemented, run the existing
CHEST-PLACE-BIND-1 binder against the exact activated World bundle and classify each of these
59 claim placements as served/bound, unbound with its typed reason, or left out. Then execute
representative chest claims through the real gameplay path and prove once-per-character,
reward delivery, reconnect and restart behavior before promoting any row to
PLAYABLE_VERIFIED.

No quest-specific chest bypass is authorized or proposed by this audit.
