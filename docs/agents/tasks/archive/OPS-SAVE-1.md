# OPS-SAVE-1

```yaml
task_id: OPS-SAVE-1
title: "OPS-SAVE-1: shutdown flush of live monk state (server save / graceful restart)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ops-save-1-20261013
issue: 2068
pr: 2073
base_sha: 4ac13390
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_frozen_at: null
owner: claude-code-session-019bxZf2wtYm1q9fQkdu8p36
control_plane: claude-code-session-01XYC239givrMonrYoBwwECV
created_at: 2026-10-13
updated_at: 2026-10-13
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/monk_save.rs
  - apps/game-server/src/gameplay_transport/monk_save_postgres_tests.rs
  - apps/game-server/src/gameplay_transport/last_position.rs
  - docs/agents/tasks/archive/OPS-SAVE-1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

On SIGTERM/SIGINT the node, before it withdraws readiness (which fences every session write), saves
the monk Harmony and forced Serene time of every live tracked actor through the existing fenced
`commit_character_monk_state_save` writer. Before this, those values were saved only at session
release, so connected and grace-held sessions lost them on a planned restart.

## Architecture and source of truth

- PROVEN: SPELL-D8 §8.2 save point 1 (`monk_save.rs`); CHAR-POSITION-1 shutdown flush
  (`LastPositionFlush`, node budget `LAST_POSITION_FLUSH_BUDGET`).
- DERIVED: the flush rides `LastPositionFlush::serve`, so the node's existing request, 3 s budget and
  `last_position_flush_incomplete` event cover it; `serve.rs` needs no change. No migration, no new
  durability module, no registry change; the other writers' files are untouched.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - current liveness/authority: the flush reads the fence from the durable GameSession per attempt
  - identity/binding: the write is bound to the Character revision slot cursor (CHAR-REV-SEQ-1)
consumer_boundaries:
  - commit_character_monk_state_save (reused unchanged, same path as the actor-end save)
mutation_operators:
  applicable:
    - terminal/ended session: fenced out, nothing written, not retried (test)
    - unavailable store: reported unproven, flush retried, no revision movement (test)
    - untracked or non-monk actor: no durable read or write (test)
  considered_not_applicable:
    - stale generation / mismatched identity: no new write path; the reused writer's own fence tests cover them
one_invariant_per_negative_case: true
```

## Acceptance criteria

- [x] Live tracked monk values are durable after the flush while the session stays Active.
- [x] A terminal session is fenced out; an unavailable store is reported as failed.
- [x] Untracked actors write nothing.

## Excluded scope

Familiar and spell-training saves (`actor_spell.rs`, other lanes); client (#1942).

## Validation

- `cargo +1.95.0 fmt --all --check`: pass
- `cargo +1.95.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo +1.95.0 test --locked -p oteryn-game-server --lib node:: gameplay_transport::monk_save`: pass (PostgreSQL 17.11 locally; harness pins 17.6, CI authoritative)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

## Self-review

- method: implementing agent read the whole diff against owned paths; one extra file (`last_position.rs`, 21 lines) reported to the control plane.
- material findings: none open.

## Independent review

- required: per the bound review policy, requested by the control plane on the frozen head.

## PR and closeout

- merge commit/result: squash merge of #2073
