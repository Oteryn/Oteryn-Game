# OPS-SAVE-2

```yaml
task_id: OPS-SAVE-2
title: "OPS-SAVE-2: quiesce admissions and input before the shutdown snapshot; flush spell training"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ops-save-2-20261013
issue: 1622
pr: 2083
base_sha: 574bcd31
owner: claude-code-session-017fjxzdaPfpqwDKCiU8ZzKw
control_plane: claude-code-session-01XYC239givrMonrYoBwwECV
created_at: 2026-10-13
updated_at: 2026-10-13
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/input_gate.rs
  - apps/game-server/src/gameplay_transport/last_position.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/kill_reward_postgres_e2e_tests.rs
  - apps/game-server/src/gameplay_transport/monk_save_postgres_tests.rs
  - apps/game-server/src/gameplay_transport/actor_spell.rs
  - apps/game-server/src/spell/mana_training.rs
  - docs/agents/tasks/archive/OPS-SAVE-2.md
public_contracts: []
depends_on: [OPS-SAVE-1]
blocks: []
```

## Outcome

Residue of OPS-SAVE-1 (#2073), confirmed open on `main` 574bcd31: the shutdown flush took its
snapshots while admissions and player commands still ran, and it did not write spell training.

- A shared `InputGate` wraps every fresh admission, resume and player command in a permit. The
  flush closes the gate and waits for the permits in flight (`quiesce`) before its first write;
  later work is refused (admission `Unavailable`, command rejected with no effect). The node's
  existing request, 3 s budget and `last_position_flush_incomplete` event cover the wait.
- The flush also writes the spell-training checkpoint of every tracked live actor through the
  existing fenced `save_spell_training(.., force)`.

## Decisions

- Familiar state is not flushed: `save_familiar_at_actor_end` is a logout save, valid only after
  the grace deadline, so a live session has none to write.
- `serve.rs` is unchanged: the flush rides `LastPositionFlush` (OPS-SAVE-1). `connection.rs` gets
  the single command-dispatch gate (about 10 lines), outside the packet's named files.
- Server-driven work (monster AI, cadences) is not input and keeps running; it cannot move a
  player's saved values except through already-fenced paths.
- No migration, no registry change.
- Codex P1 (serial store reads per idle actor): the training flush pre-filters on an in-memory,
  read-only `has_training_to_flush` (retained checkpoint or unbanked mana); fixed in one commit.

## Validation

- `cargo +1.95.0 fmt --all`: pass
- `cargo +1.95.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo +1.95.0 test --locked -p oteryn-game-server --lib -- node:: gameplay_transport::`: pass (584 passed, 6 ignored)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

Review: Codex complete, one P1 fixed and resolved. Postgres-backed tests are CI-authoritative.
