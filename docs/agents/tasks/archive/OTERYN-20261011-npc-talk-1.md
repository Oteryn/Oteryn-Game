# OTERYN-20261011-npc-talk-1

```yaml
task_id: NPC-TALK-1
title: "NPC-TALK-1: a say near a runtime NPC is answered from catalogue replies over the chat path"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/npc-talk-1-20261011
pr: 1976
base_sha: null
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
control_plane: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
coordination: "#1622 (plan comment 6056843803, row N3)"
created_at: 2026-10-11
updated_at: 2026-10-11
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/npc_talk.rs
  - apps/game-server/src/gameplay_transport/chat_intent.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/chat/greeting.rs
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/content/npc_catalogue.rs
  - apps/game-server/src/foundation/mod.rs
  - docs/agents/tasks/active/OTERYN-20261011-npc-talk-1.md
  - docs/agents/tasks/archive/OTERYN-20261011-npc-talk-1.md
public_contracts: []
external_repositories: []
```

## Outcome

A `say` from a player within 4 tiles of a runtime NPC is answered by that NPC with a local line
built from the NPC's generated catalogue replies (NPC-0 section 3.3): a greeting starts a
conversation and is answered with the greeting line, a topic word (`job`, `offer`, `trade`,
`travel`) is answered with the NPC's service lines, a farewell ends the conversation. Walking out
of range ends it. The reply is an ordinary domain 12 local line spoken by the NPC, heard by every
session in range. No wire change.

## Architecture and source of truth

- PROVEN: CHAT-0 section 3 (`chat::greeting`, `CHAT0-RL-08` range 4); NPC-RT-1 runtime NPC actors
  (`visible_entities().npcs`, `npc_key`); NPC-0 section 3.3 generated replies.
- DERIVED: `gameplay_transport/npc_talk.rs` is the pure conversation rule plus `NpcTalkBook`
  (built from `NpcServiceModel`); `chat_intent.rs` calls it after the speaker's own line and keeps
  the per-session engaged NPC in `ChatSession`. NPC replies bypass the limiter (they answer a
  limited player line).

## Excluded scope

Trade and bank dialogue (NPC-TRADE-1, BANK-NPC-1); admitted Dialogue records (such NPCs get no
book entry); a per-NPC greeting list; whisper/yell talk; boot wiring and `install_npc_talk` caller
(no runtime NPC exists before the NPC-PLACE-1b boot follow-up of NPC-RT-1).

Follow-up NPC-TALK-2: end the conversation as soon as the speaker walks out of range (today it
ends at the speaker's next `say`).

## Acceptance criteria

- [x] A greeting, topic and farewell near a placed NPC produce NPC lines heard by all in range
  (`a_greeting_near_an_npc_is_answered_by_the_npc_to_everyone_in_range`).
- [x] Out-of-range ends the conversation; nearest greeted NPC answers (`npc_talk` unit tests).

## Implementation / findings

Content and foundation edits are visibility re-exports only.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: not run locally (killed by memory contention); CI game-gate runs it
- `cargo test --locked -p oteryn-game-server --lib -- npc_talk chat_intent`: pass (13 passed, 0 failed)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass (60 tests)

## Self-review

- method: implementing agent read the whole diff against owned paths.
- material findings: none open.

## Independent review

- required: per the bound review policy, requested by the control plane on the frozen head.

## PR and closeout

- changed-file review: within owned paths
- merge commit/result: squash merge of #1976
