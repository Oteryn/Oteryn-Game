# CHAT-MUTE-CAST-1

```yaml
task_id: CHAT-MUTE-CAST-1
title: "CHAT-MUTE-CAST-1: a muted character cannot cast a spell (CHAT-0 section 6)"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/chat-0-impl-20261012
pr: 2003
base_sha: 08e13f7de
head_sha: null
final_head_sha: null
owner: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
control_plane: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-12
updated_at: 2026-10-12
owned_paths:
  - apps/game-server/src/gameplay_transport/chat_intent.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - docs/agents/tasks/active/CHAT-MUTE-CAST-1.md
public_contracts: []
external_repositories: []
```

## Outcome

CHAT-0 section 6 says a muted character can neither chat nor cast any spell, and that a muted cast
answers the existing `SPELL_CAST_DISPOSITION_REJECTED`. The limiter and its mute shipped with
CHAT-WIRE-1, but nothing on the cast path read it (`ChatRuntime::is_muted` was `dead_code`). Now
`cast_spell_completion` asks `ChatRuntime::muted_cast` first and answers rejected while the
session's mute runs.

## Architecture and source of truth

- PROVEN: CHAT-0 decision `docs/architecture/reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md` section 6 (mute gates the spell path, command 3, no new disposition).
- DERIVED: the gate sits at the head of the single cast completion entry, so the familiar, world-item, native combat and plain cast arms are all covered; an unknown (not chat-admitted) session is never held back.

## Acceptance criteria

- [x] A muted session's cast answers `Rejected`; an unmuted, another and an unknown session are not affected; the answer ends with the mute (`a_muted_session_cannot_cast_until_the_mute_ends`).
- [x] `is_muted` no longer needs a dead-code allowance.

## Excluded scope

The durable mute row (`game_character_chat_mutes`), the World relay, private messages and rooms
stay CHAT-2. No protocol, wire, migration or content change.

## Validation

- `cargo fmt -p oteryn-game-server --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --lib -- chat_intent chat:: npc_talk chat_wire`: pass (38 passed, 0 failed)
- `python3 tools/agents/validate_governance.py`: pass

### Exact-head CI

Recorded in the PR checks of the frozen head.

## Self-review

- method: implementing agent read the whole diff against owned paths.
- material findings: none open.

## Independent review

- required: per the bound review policy, requested by the control plane on the frozen head.

## PR and closeout

- changed-file review: within owned paths
- merge commit/result: pending
