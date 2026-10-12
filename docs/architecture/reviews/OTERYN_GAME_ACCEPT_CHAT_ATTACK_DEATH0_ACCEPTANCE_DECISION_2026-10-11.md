# ACCEPT-CHAT-ATTACK-DEATH-0 Acceptance of the chat, attack and player death decisions

- Decision: `ACCEPT-CHAT-ATTACK-DEATH0-V1`
- Status: **PARTIALLY ACCEPTED BY OWNER 2026-10-11, after independent review 2026-10-12**. ATTACK-0
  and DEATH-0 are ACCEPTED. CHAT-0 and the Reference first player death decision stay CANDIDATE
  because their independent review found defects (section 3). It adds no rule text and changes no
  code, wire, migration or content.
- Answers: the audit finding that four decisions were still CANDIDATE while the Game server already
  implements them.
- Accepts (ATTACK-0 and DEATH-0) or reopens (CHAT-0 and Reference first player death, section 3):
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

- The ATTACK-0 and DEATH-0 status lines read ACCEPTED, each pointing here. CHAT-0 and the Reference first player death decision read CANDIDATE with their review result. No other line of the four documents
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

## 3. Independent review (2026-10-12)

Each decision's own gate requires an independent review. One read-only reviewer per decision, none
of them the author, reported.

- Round 1 read `a1d2ee2d8b9f4b573c90c7496b53caf925d32e13`. It found the CHAT-0 and Reference
  first player death defects below, and passed ATTACK-0 and DEATH-0.
- Round 2 (ATTACK-0 and DEATH-0 only) read `77762114d8dbc869679f5305c96cb9105fe82b0a`, the
  pre-record parent commit that the record commit builds on. Each reviewer confirmed with
  `git diff a1d2ee2d..77762114` that only status lines and review-record text changed in six
  documentation files, with no change to code, migrations, protocol, registry, content or decision
  rules, and then repeated the review. Both passed again, with the same residual notes.
- The only change after the reviewed commit is the record commit itself. It touches exactly these
  files, by status lines and review-record text: this acceptance decision (section 3), the ATTACK-0
  and DEATH-0 decision documents (their review-record line), and the task record
  `OTV2-20261011-accept-chat-attack-death-0.md` (independent review section). The record cannot
  contain its own SHA; the final head is the PR head.
- CHAT-0 and the Reference first player death decision stay CANDIDATE, so no acceptance rests on
  their round 1 review.

| Decision | Review required by its gate | Verdict | Result |
|---|---|---|---|
| ATTACK-0 | protocol, combat | PASS, PASS | ACCEPTED. Residual notes in the decision document, for ATTACK-PARITY-1. |
| DEATH-0 | receipt chain, guards, fencing | PASS, PASS, PASS | ACCEPTED. One rewording suggested for section 3.4. |
| CHAT-0 | protocol, security, privacy | PASS, DEFECT, PASS | CANDIDATE. No rule requires a rate gate before the Channel lock and entity clone for a `say`; the cast-path mute of section 6 is not wired. |
| Reference first player death | character durability, DUR-03 transfers, restart safety | PASS, DEFECT, DEFECT | CANDIDATE. Item loss (DEATH-3) is not implemented; the pending respawn is consumed before placement; a missing death policy lets a player respawn without a durable death. |

A reviewer ran in a separate read-only session and read code and migrations without running tests.
Reopened decisions need the defects fixed or the owner's explicit waiver, then a new review.
