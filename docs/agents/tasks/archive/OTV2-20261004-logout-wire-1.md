# OTV2-20261004-logout-wire-1

```yaml
task_id: OTV2-20261004-logout-wire-1
title: "LOGOUT-WIRE-1: LOGOUT_INTENT, LOGOUT_V1 and TerminalRelease::Logout"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/logout-wire-1
pr: 2127
base_sha: a61cb623
owner: claude-code-session-01Y3ihfip9Q6PN6PWEdzucq9
control_plane: claude-code-session-01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
packet: docs/architecture/reviews/OTERYN_GAME_ARCH_KILL_REWARD_LOGOUT_PACKETS_2026-10-04.md
owned_paths:
  - crates/protocol-oteryn/src/lib.rs
  - crates/protocol-oteryn/src/logout.rs
  - crates/protocol-oteryn/src/logout_tests.rs
  - crates/session/src/lib.rs
  - apps/game-server/src/durability/fresh_admission.rs
  - apps/game-server/src/gameplay_transport/capabilities.rs
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/logout.rs
  - apps/game-server/src/gameplay_transport/logout_tests.rs
  - apps/game-server/src/gameplay_transport/logout_postgres_tests.rs
  - apps/game-server/src/gameplay_transport/monk_save_postgres_tests.rs
  - apps/game-server/src/gameplay_transport/familiar_cast.rs
  - apps/game-server/src/gameplay_transport/actor_spell.rs
  - apps/client/src/combat_input.rs
  - apps/client/src/input.rs
  - apps/client/src/play.rs
  - apps/client/src/windows_shell.rs
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/protocol-oteryn/v1/logout_v1.proto
  - docs/agents/tasks/archive/OTV2-20261004-logout-wire-1.md
public_contracts:
  - docs/contracts/protocol-oteryn/v1/logout_v1.proto
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
```

## Outcome

LOGOUT-WIRE-1 adds capability 21 `LOGOUT_V1` and command 23 `LOGOUT_INTENT`, following packet §1.6 and §2.2. The server routes a logout through `release_logout_session`, which is fenced on the session generation.

- **ACCEPTED:** commits TERMINAL with no loss epoch, saves the last position and retires the actor.
- **IN_FIGHT:** returns a bounded retry (1..=60000 ms).
- **BUSY:** returned for a superseded controller.
- **REJECTED with an empty payload:** the refusal happens before the logout authority is reached.
- **Unknown outcome:** ends as an ordinary control loss, reconciled from the row.

The session exposes `logout()`. The client binds Ctrl+L and Ctrl+Q to logout and leaves play on ACCEPTED. The fencing of ATTACK-1b #1798, KILL-REWARD-COMP-1 #2075 and CHAR-POSITION-1 #2006 is preserved.

## Notes

- **Number deviation:** the packet proposed capability 19 and command 20.
  - Capability 19 is already `WORLD_LIGHT_V1`.
  - 20 is leased to `CONNECTION_TRACE_V1`.
  - This PR uses the next free numbers, 21 and 23, and reported the conflict to the control plane.
- **Familiar save at logout (#2127 Codex P1 4278730847):** the actor-end familiar save was proven only after a grace expiry. A live familiar was left open by a logout, and the next session zeroed it as a crash. The save is now keyed by `FamiliarActorEnd`:
  - `GraceExpiry(epoch)` is unchanged: same proof, same occurrence.
  - `Logout(transport)` is proven by an ACTIVE row (a resumed row keeps its last loss epoch) on the exact current transport and a runtime actor with no control-loss mark (Codex P1 4278956541). It has its own occurrence domain `oteryn:familiar-logout-intent:v1`, keyed by the transport and the release's transition id: each attempt saves under its own occurrence, so a retry after a `BUSY` whose save committed is no conflicting replay (Codex P2 4279467193).
- **Familiar snapshot is the last actor-end save (#2127 Codex P1 4279636456):** the familiar save now runs after the monk, training and kill steps, directly before the durable release. A retryable step can no longer answer `BUSY` after a committed logout snapshot. After the snapshot, only a release that is not this logout's can leave the session running: a controller that superseded it in between. That controller's own actor end then re-snapshots the familiar.
- **Second logout on the accepted path (#2127 Codex P2 4279368526):** a second `LOGOUT_INTENT` read before an `ACCEPTED` result is still answered with `TOO_MANY_OUTSTANDING_COMMANDS` after the result. The connection still ends as a logout, because the release has committed.
- **Logout committed during the loss path (#2127 Codex P1 4279467185):** `commit_control_loss` also reports `Terminal` when the second session read, after the spell reconciliation, sees TERMINAL. The lifecycle then reconciles the committed release instead of leaving the actor in the Channel.
- **Compile-forced edit outside owned paths:** `tools/dev-client/src/lib.rs` gains a `DevClientError::Logout` arm.
- **Out of scope:**
  - Protection zones (PZ).
  - ACCEPTED while dead or with a pending respawn (DEATH-2).
  - The rate limit `LOGOUT-RL-01`, which is registered but stays outstanding.
- No migration.

## Validation

cargo fmt --all --check: pass
git diff --check: pass
cargo clippy --workspace --all-targets -- -D warnings: pass
cargo test -p oteryn-protocol-oteryn: pass
cargo test -p oteryn-session: pass
cargo test -p oteryn-client: pass
cargo test -p oteryn-game-server --lib logout: pass
python tools/agents/validate_governance.py: pass
python -m unittest discover -s tools/agents/tests: OK
