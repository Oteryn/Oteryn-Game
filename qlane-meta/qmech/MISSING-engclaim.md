# ENGCLAIM checked blockers — PARTIAL (four authored recipes inspected)

Checked against main `82ff2d753` plus package metadata. The USE dispatcher and
PostgreSQL once-claim/container mint owner already exist (`bundle_chest.rs`,
`reward_claim_mint.rs`); the following authored recipes have no RewardClaim
placements in the embedded shards and remain `runtime_enabled: false`.
These are missing native authoring/allocation bindings, not missing wire commands.

| quest | missing mechanic (server or client) | evidence (the quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| `oteryn:quest.authored.kingdom_of_kormarak_quest` | server: authored corpse placement and complete four-item once-only allocation | `content/quests/definitions/quests-00300-00351.json`, record 9, s4/s5: unresolved dead-human target; empty `native_lowering.canonical_reward_refs`. No matching record in `content/interactions/reward_claims/*.json`; `world_reward_claims::candidate` at the checked base admits only plain ready singleton or specially pinned source variants. | fully: authored quest disabled |
| `oteryn:quest.authored.magic_sword_quest` | server: two independent personal vault allocations, including wand identity | same quest file, record 14, s5/s6/s7: unresolved north/south vault targets and Yellow Spell Wand intent; empty canonical reward refs. No matching RewardClaim shard record. | fully: authored quest disabled |
| `oteryn:quest.authored.mastermind_potion_quest` | server: two personal cache allocations and selected final 201-gold payout | same quest file, record 16, s2/s4/s5: unresolved wall/fire cache targets; empty canonical reward refs. No matching RewardClaim shard record. Existing durable mint owner supports explicit container children but does not infer allocations from recipe text. | fully: authored quest disabled |
| `oteryn:quest.authored.mystery_of_the_valley_quest` | server: three shrine fragment placement bindings and final statue reward binding | same quest file, s2/s3/s4 collect and s6 complete: named shrine/fragment targets without native claim placements and empty canonical reward refs. No matching RewardClaim shard record. | fully: authored quest disabled |

No client mechanic gap established by this inspection. Other ENGCLAIM ledger
entries require route/completion qualification; absence of proof alone is not
reported here as a missing engine mechanic.

## Native admission implementation slice

The new `native_authored_v1` reader accepts ready per-character once claims with
exact project position, native top-level ordinal and appearance. It carries
explicit leaf/container allocations, a native achievement key and the existing
quest-transition obligation through the shared binder and production USE owner.
Conflicting map-object claim authorities refuse every affected claim. Existing
PostgreSQL claim, mint, nested custody and achievement owners retain durability.

Four focused unit cases pass. Two production dispatch-owner PG cases passed in
40.476803198 s (201-coin container allocation) and 40.451163174 s (alternative
vault allocation with achievement), including stale-fence refusal, a new-command
second-placement refusal and cold-owner replay. Cases live in
`crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/engclaim_native.rs`;
they configure server-authored fixture shards and do not certify any full quest.

Remaining scope: independent multiple Item outputs (qready2's claimed participant),
selected native placements and complete stage/completion compositions for the
four recipes above. Full quests newly unblocked: **0**. No migrations or content
inputs changed in this slice. The open placement/allocation packet question
remains relevant before any complete quest qualification claim.
