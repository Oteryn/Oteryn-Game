# World item delivery

A source-owned `world_callbacks::Callback` can declare `effects.item_reward`
using `reward_callbacks::ItemReward`. Authored `MapUse` bindings reuse main's
`world_callbacks::primary_transition`, `RevisionSlot::commit_world_callback_with_items`
and `DurabilityRoot::commit_character_world_callback_with_items`; they do not
introduce a second mint owner or a duplicate migration.

The immutable selector, world occurrence and session generation authorize USE.
The ordinary backpack MINT participant enforces native identities, item counts,
container payloads, premium requirements, fluid outputs, weight and capacity.
Its receipt and the owned quest write commit in the same database transaction.
A per-character read-only gift preserves the shared object's revision. Replaying
an accepted command returns its original result without another reward.

Authored NPC/carried bindings also reuse the canonical primary transition.
`Bindings::attach` projects `Stage.completes` into their callback plans so main's
completion proof observes the same final stage. World storage preserves canonical
increment, raise-to and admission-bound operations.

Reuse: main `83acbdac3` (QUESTPC4), questpc3
`6146ee945bc2e01b5c7e751e6499f670062ad03a`, and questpc7
`653feac452d8dbb2cf03edbfc733a1bf59bd77d7`. The earlier duplicate world helpers and
migration1553 are removed in favor of main's existing transaction owner.

Synthetic real-dispatch cases live in `postgres_tests/authored_world_rewards.rs`.
The original PostgreSQL RED ran49.876s; the premerge GREEN run passed all three
selected cases in113.478s (normal gift56.625s, full backpack56.737s, retained
NPC/carried delivery98.925s). Merged-main PostgreSQL refresh is pending; this
checkpoint does not qualify a production quest or claim that refresh passed.
