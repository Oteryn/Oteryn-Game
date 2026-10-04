# SPELL-LOCK-1

```yaml
task_id: SPELL-LOCK-1
title: "SPELL-LOCK-1: release channel guards before durable I/O; spell hardening"
mode: IMPLEMENT
status: frozen
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: spells
base_branch: main
branch: claude/spell-lock-1-20261004
pr: PENDING
base_sha: 95c93340
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-011LbKYMGrmvxG1eKSALySUP (oteryn-hard-worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
packet: "Codex finding 4178405815 on #1534 (P1, Channel guards across durable combat I/O) and the #1534 deep-review P2s (SPELL-HARDEN-1)"
leases: none
owned_paths:
  - apps/game-server/src/spell/cast.rs
  - apps/game-server/src/spell/cast_tests.rs
  - apps/game-server/src/gameplay_transport/ordinary_combat.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/durability/spell_field_policy.rs
  - crates/session/src/lib.rs
  - tools/content-schema/native-gameplay/verify_house_privacy_pg.py
  - docs/agents/tasks/active/SPELL-LOCK-1.md
  - docs/agents/tasks/archive/SPELL-LOCK-1.md
depends_on:
  - "spells #1534 merged"
  - "VIS-3 #1772 merged before any connection.rs edit"
public_contracts: []
external_repositories: []
```

## Scope (1): Channel guards across durable I/O

Not in this PR. Reported to the control plane as a BLOCKER: every semantic-pass spell path
interleaves transaction reads with `ChannelRuntimeV1` mutation, and `ChannelRuntimeV1` carries no
revision a re-acquire fence could check. The control plane moved Codex finding 4178405815 to a
separate task, SPELL-LOCK-2, which starts with a design proposal.

## Scope (2): #1534 deep-review hardening

- `prepare_ordinary_owner_cast_with_caster` takes the caster's `SpellBook` and rejects a spell
  outside it, like the focus and source-self paths. A new test proves both the book refusal and
  the uninitialized monk `accept_command` refusal.
- `crates/session`: the `vitals_pair` doc comment is back on `vitals_pair`, and the
  closed-before-admission comment is back on its test.
- The six `spell_*_pg_cases.sql` fixtures that no harness ran are wired into
  `verify_house_privacy_pg.py`. They run in their own schema in dependency order. The verifier
  applies every migration (the stale `> 48` cutoff failed `native_map_item_cases.sql`) and takes
  `--url` for a local psql.
- Migration 0048's `session_user` choice is documented at its Rust reader. The migration is
  unchanged.
- `vitals_delta` from revision 0: actor revisions start at 1, so 0 means the join carried no
  `ACTOR_VITALS` snapshot. The Serene and spell-result paths skip the delta then instead of
  disconnecting the session. A connection test proves a cast from such a session answers its
  result and the next command, and fails without the guard.

## Validation

- `cargo fmt --all -- --check`: pass.
- `cargo clippy -p oteryn-game-server -p oteryn-session --all-targets -- -D warnings`: pass.
- `cargo test -p oteryn-game-server --lib spell::`: pass.
- `cargo test -p oteryn-game-server --lib ordinary_combat`: pass.
- `cargo test -p oteryn-game-server --lib gameplay_transport::connection`: pass.
- `cargo test -p oteryn-session`: pass.
- `python3 tools/content-schema/native-gameplay/verify_house_privacy_pg.py --url <local PostgreSQL 17>`:
  every case passes, the six spell Item guard cases included.
- The CI-registered Rust PostgreSQL targets need the `oteryn_test_admin` role on port 5432, which
  this session could not create; they run in CI.
