# CLIENT-COMBAT-INPUT-1

```yaml
task_id: CLIENT-COMBAT-INPUT-1
title: "CLIENT-COMBAT-INPUT-1: the client sends attack-target and fight-mode intents and casts from a spell bar"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/client-combat-input-1-20261008
pr: 1938
base_sha: null
head_sha: "exact frozen head in the FREEZE report on the PR"
final_head_sha: "exact frozen head in the FREEZE report on the PR"
owner: claude-code-session-01GzBHiFkoC38DZPvXQZsMYM
created_at: 2026-10-08
updated_at: 2026-10-08
owned_paths:
  - crates/session/src/lib.rs
  - apps/client/src/play.rs
  - apps/client/src/spell.rs
  - apps/client/src/combat_input.rs
  - apps/client/src/lib.rs
  - apps/client/src/windows_shell.rs
  - tools/dev-client/src/lib.rs
  - tools/synthetic-client-harness/src/live/model.rs
  - docs/agents/tasks/archive/CLIENT-COMBAT-INPUT-1.md
public_contracts: []
external_repositories: []
```

## Outcome

The client reaches the server's attack and cast dispatch: digits 1-4 cast spell-book entries (cmd 3, aimed at the attack target), in press order, and the
answer shows in the window title. The session layer has senders for attack-target (cmd 11) and
fight modes (cmd 12) and keeps the actor combat state (domain 10). Click-to-attack is out of this
task: the view carries no visible entities yet, so it returns in CLIENT-ENTITY-VIEW-1. D964 packet S1, root finding 1 (coordination #1622).

## Architecture and source of truth

- Capability 17 `ATTACK_V1` (requires 6) owns cmd 11, cmd 12 and domain 10: PROVEN
  (`apps/game-server` capabilities). Protocol unchanged.
- The server owns the target; the client only names an `EntityRef` taken from the session's own
  entity set: DERIVED from `Session::world_entities`.

## Acceptance criteria

- [x] Session sends cmd 11 and cmd 12 and keeps domain 10; refuses both before sending when cap 17
  was not selected (session tests).
- [x] Hotkeys reach the session task in press order and their answers reach the view
  (`play::tests::a_hotkey_reaches_the_session_task_in_press_order_and_its_answer_reaches_the_view`).
- [x] An empty `Rejected` attack answer (server rate limit) is a normal refusal.
- [x] `windows_shell.rs` routes hotkeys and shows the combat line (`cfg(windows)`, not compiled here).

## Excluded scope

`apps/game-server/**` and `protocol-oteryn` unchanged. No fight-mode control and no click-to-attack.
`tools/synthetic-client-harness` got one no-op `ActorCombatState` arm. `tools/dev-client` got one `SessionError::Attack` mapping arm, forced by its exhaustive match.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy -p oteryn-client -p oteryn-session -p oteryn-session-tcp -p oteryn-dev-client --all-targets -- -D warnings`: pass
- `cargo test -p oteryn-client -p oteryn-session -p oteryn-session-tcp -p oteryn-dev-client`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- E2E (attack the qualification rat, cast exura): `NOT_APPLICABLE` here; it needs a game server
  with PostgreSQL 17.6, the platform login and a browser sign-in, none available in this session.
- Windows shell: the repository Windows CI job on the PR head.
