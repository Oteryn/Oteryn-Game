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
The original PostgreSQL RED ran for 49.876s. After merging main `6f61ac15d`, all six
selected real-dispatch PostgreSQL cases passed in 364.679s on `aa85b2500`:
world delivery/replay 65.493s, full-backpack rollback 66.639s, NPC/carried
exchange 121.746s, sequential kills 122.941s, queued burst 118.673s, and retained
Dragon Slayer 123.873s (completion-marker durations). The normal world case also
proves the completed USE and MINT receipts share one database transaction.
All 320 quest unit tests passed in 40.75s, including distinct definition encoding
for authored kill thresholds (tag `11`) and main's bound-set operation (tag `7`).

This qualifies the shared delivery mechanic. It does not qualify a production
quest or complete the separate native authored-binding boot admission feature.
