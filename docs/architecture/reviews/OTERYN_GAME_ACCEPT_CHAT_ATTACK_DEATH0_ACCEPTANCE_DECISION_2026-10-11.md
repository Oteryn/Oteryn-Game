# ACCEPT-CHAT-ATTACK-DEATH-0 Acceptance of the chat, attack and player death decisions

- Decision: `ACCEPT-CHAT-ATTACK-DEATH0-V1`
- Status: **ACCEPTED BY OWNER 2026-10-11**. This records the owner's decision (audit points 2 and 5,
  option a). It adds no rule text and changes no code, wire, migration or content.
- Answers: the audit finding that four decisions were still CANDIDATE while the Game server already
  implements them.
- Accepts:
  - `reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md`
    (`CHAT0-LOCAL-PRIVATE-WORLD-ROOMS-V1`);
  - `reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md`
    (`ATTACK0-ATTACK-TARGET-AND-AUTO-ATTACK-V1`);
  - `reviews/OTERYN_GAME_DEATH0_CHARACTER_DEATH_RECEIPT_DECISION_2026-09-28.md`
    (`DEATH0-CHARACTER-DEATH-RECEIPT-V1`);
  - `reviews/OTERYN_GAME_REFERENCE_FIRST_PLAYER_DEATH_DECISION_2026-09-28.md`
    (`REFERENCE-FIRST-PLAYER-DEATH-V1`), including respawn.
- Runtime, migration, wire and production authority: NONE new. Each decision is already
  implemented (`apps/game-server/src/chat/`, `gameplay_transport/attack.rs`,
  `domain/death.rs`, migrations `0016` and `0018`, `crates/protocol-oteryn/src/chat.rs`).
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. What becomes effective

- The four status lines read ACCEPTED, each pointing here. No other line of the four documents
  changes.
- Amendments that other decisions wrote as "pending on acceptance" of the target stay worded as
  they are. Each takes effect in the target only when its own source decision is accepted, as
  ACCEPT-SOCIAL-MAP-0 §1 already rules for PARTY-PVP-0 and GUILD-0.
- `PROTOCOL_OTERYN_V1_REGISTRY.json` and `CROSS_REPOSITORY_CONTRACT_LOCK.json` carry no status for
  these decisions, so neither changes. The wire digests are untouched.

## 2. Not done here

The entry room does not let a player attack and does not let rats fight. That is a content and
pin change, not a documentation change. `native_spell_entry_room.json` is qualified by exact
flags in `native_entry.rs` (four cells, all `protection_zone: false`) and feeds pinned digests and
goldens. It is carried by the follow-up task records `ENTRY-SPELL-TILES-1` and
`BOOT-RAT-POLICY-1` (`docs/agents/tasks/`), which need their own allocation.
