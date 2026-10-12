# OTV2-20261012-death-restart-safety-1

```yaml
task_id: OTV2-20261012-death-restart-safety-1
title: Fix the two restart-safety defects of the first player death
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/death-restart-safety-20261012
pr: 2004
base_sha: 08e13f7dea83c43c13df85f901caf1e032ef3cd9
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
merge_commit: "squash merge of #2004"
final_head_frozen_at: "recorded in the FREEZE_SHA report"
owner: Claude worker for the programme control plane, single writer
created_at: 2026-10-12
updated_at: 2026-10-12
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/durability/character_death.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/character_progression_binding_tests.rs
  - apps/game-server/tests/support/character_death_writer_postgres_cases.rs
  - apps/game-server/tests/support/character_progression_admission_postgres_cases.rs
  - docs/architecture/reviews/OTERYN_GAME_ARCH_PROGRESSION_SOURCE_PACKETS_2026-10-05.md
  - docs/agents/tasks/archive/OTV2-20261012-death-restart-safety-1.md
public_contracts: []
depends_on: [DEATH-0, DEATH-2, DEATH-2b, ARCH-PROGRESSION-SOURCE-0]
blocks: [Reference first player death acceptance]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The owner chose option (a) for the Reference first player death: fix the two restart-safety
defects of the review, then accept with a DEATH-3 waiver (no corpse or loot yet).

1. `DurabilityRoot::settle_player_death` committed the death and consumed its own pending
   respawn before the actor was placed. A crash in between lost the recorded respawn. Settlement
   now leaves the row; `respawn_after_death` places the actor, then consumes the row under the
   death's occurrence, then refills the vitals. Any failed step leaves the actor dead and the row
   recorded for the next admission (DEATH-2b already places before consuming). The placement
   is volatile, so the in-session consumption runs only when the row records the cell a fresh
   admission writes; a restart after it still lands on the recorded respawn (Codex review of
   #2004). Any other recorded cell keeps the row and the actor dead for the next admission.
   The session's current durable authority is read fresh before the placement, since a
   replayed death receipt checks none, so a terminal or superseded session moves nothing (second
   Codex review of #2004). The authorization releases its revision slot before the placement
   waits for the runtime lock, so the placement revalidates that authority under the lock, with
   no I/O, against the owner's binding: the slot still bound to the session, no write fence (a
   terminal release fences first), the authorized lease and scope generation. Otherwise nothing
   moves, no respawn condition is cleared and the actor stays dead (third Codex review of #2004).
2. `settle_session_death` respawned an unbound session (no World section, no death policy, a
   context mismatch) with no durable death. It now returns `NoDurableDeath`: the actor stays dead
   and the session end recovers the Character normally.

## Architecture and source of truth

- PROVEN: first player death decision §4.5 and the review recorded in #1994 §3.
- PROVEN: DEATH-0 §3.4 pending respawn; DEATH-2b places before consuming at admission.
- CONFLICT resolved: ARCH-PROGRESSION-SOURCE-0 §1.2/§1.5 non-durable respawn; amended by A2.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants: [one durable death per occurrence, respawn only after a durable death, pending respawn consumed once after placement]
consumer_boundaries: [settle_player_death, respawn_after_death authorization, in-session placement under the runtime lock, settle_released_death, admission consumption]
mutation_operators:
  applicable: [node restart before placement, replay of the same occurrence, second death over a pending respawn, missing binding, context mismatch, session fenced terminal between authorization and placement, successor rebind between authorization and placement, authorized lease/Character/scope generation differing from the owner binding]
  considered_not_applicable: [protocol versions (no wire change), migrations (no schema change)]
one_invariant_per_negative_case: true
independent_current_fact_sources: [PostgreSQL rows via snapshot()]
finding_family_sweep:
  direct_and_reconciled_paths: settle, replay, reconcile after restart
  restart_retry_replay_concurrency_pg_reload: restart test in character_death_writer_postgres_cases
```

## Acceptance criteria

- [x] A restart between the durable death and the placement keeps the pending respawn, the
  replay loses experience once, and the consumption runs once.
- [x] An unbound session never respawns without a durable death.
- [x] The in-session order is settle, place, consume, refill; a failed step stops the chain.
- [x] The in-session consumption keeps the row unless a fresh admission reproduces its cell.
- [x] The in-session order is settle, authorize (fresh durable authority), place, consume, refill.
- [x] The placement revalidates the authorization under the runtime lock; a session fenced or
  superseded after the authorization moves nothing and clears no respawn condition.

## Excluded scope

DEATH-3 corpse and item loss (waived by the owner); durable character position (CHAR-POSITION).

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --quiet --lib`: pass (2800 passed, 0 failed)
- `cargo test --locked -p oteryn-game-server --test character_progression_postgres --quiet` (PostgreSQL 17.6): pass (1054 passed, 0 failed)
- `character_authority_postgres`: left to CI; the local PostgreSQL container stopped when the shared disk filled during the run
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
- Codex P1 repair (respawn row kept unless an admission reproduces it): `cargo fmt --all --check`,
  `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`: pass;
  `cargo test --locked -p oteryn-game-server --quiet --lib -- gameplay_transport::`: pass (470
  passed, 0 failed); PostgreSQL targets left to CI
- Codex P1 repair (authorize before placement): fmt and clippy as above: pass;
  `cargo test --locked -p oteryn-game-server --quiet --lib -- gameplay_transport::`: pass (476
  passed, 0 failed); PostgreSQL targets left to CI
- Codex P1 repair (revalidate at the placement): `rustfmt --check` on the touched files,
  `python tools/agents/validate_governance.py` and `git diff --check`: pass; no local cargo run
  in this round (control-plane D1011), so build, clippy and tests are left to CI

## Self-review

- method/reviewer: implementing agent, whole-diff adversarial review
- material findings: none open; unbound sessions now stay dead until the session ends
- verdict: ready for independent review

## Independent review

- required: YES, persistence and authority change to death and respawn restart safety
- verdict: recorded on the exact frozen head by the independent reviewer and CI; no open finding in this record

## Context checkpoint

```yaml
last_progress: fix, tests and amendment A2 complete; record archived in the final authoring commit
status: completed
branch: claude/death-restart-safety-20261012
pr: 2004
blocker: null
next_action: none; terminal. Task closed once PR 2004 squash-merges.
```
