# World item delivery

Implementation checkpoint: PostgreSQL GREEN is pending.

A source-owned `world_callbacks::Callback` can declare `effects.item_reward`
using the existing `reward_callbacks::ItemReward` shape. The shared dispatcher
uses `RevisionSlot::commit_world_reward` and
`DurabilityRoot::commit_character_world_reward` to combine the world occurrence,
owned quest storage and the existing complete backpack MINT participant.

The callback uses ordinary USE on a read-only source object. Its owned storage
advances out of the eligible range. The reward contains one native Item and an
optional bounded container payload (up to 20 entries); Item counts are 1–100.
The immutable selector and ordinary world/session fences still authorize USE.
The source object's shared revision remains unchanged after a per-character
claim. Replaying the same command returns the original result and reward.

The first shared path admits a single item-delivery effect with its quest guards.
Levers, passage changes, teleport, offerings, carried inputs, XP and other world
effects are separate callback modes. The native generation resolves exact Item
identities and applies the existing wheel-aware weight and backpack capacity
rules. A refusal leaves the quest storage and reward source uncommitted.

Reused implementation:

- questpc3 `6146ee945bc2e01b5c7e751e6499f670062ad03a`, bounded world gifts and
  shared Item validation;
- questpc7 `653feac452d8dbb2cf03edbfc733a1bf59bd77d7`, world USE value participant;
- questpc3 `1225_quest_world_reward_participants.sql`, exact callback/reward and
  same-transaction USE proof, adapted into QENGINE2 migration1553.

Existing NPC, paid/carried exchange and complete MINT handling remain the shared
owners. This adds no production quest data or whole-quest qualification. The
synthetic dispatch cases are in `postgres_tests/authored_world_rewards.rs`.
