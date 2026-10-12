# OTV2-20261005-progression-owner-1

```yaml
task_id: OTV2-20261005-progression-owner-1
title: "PROG-BIND-1: compose the Character progression owner into the gameplay seam"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/prog-bind-1-20261008
pr: 1964
base_sha: null
head_sha: "exact frozen head in the FREEZE report on the PR"
final_head_sha: "exact frozen head in the FREEZE report on the PR"
owner: claude-code-session-01Cdzo5t7PGTD7HXJQsrUXGx
created_at: 2026-10-08
updated_at: 2026-10-12
owned_paths:
  - apps/game-server/src/content/activation.rs
  - apps/game-server/src/combat/death_reward.rs
  - apps/game-server/src/durability/character_authority.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/character_progression_binding.rs
  - apps/game-server/src/gameplay_transport/character_progression_binding_tests.rs
  - apps/game-server/src/gameplay_transport/kill_reward.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/tests/support/character_progression_postgres_cases.rs
  - apps/game-server/tests/support/character_progression_admission_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20261005-progression-owner-1.md
public_contracts: []
external_repositories: []
```

## Outcome

ARCH-PROGRESSION-SOURCE-0 §1.3-§1.5 and §2.2 (`docs/architecture/reviews/OTERYN_GAME_ARCH_PROGRESSION_SOURCE_PACKETS_2026-10-05.md`),
coordination #1622, CP decisions D607 and D968.

- A fresh admission whose first entry is positioned or reconciled runs the progression step
  before the pending-respawn consumption: the policy is formed from the World's pinned
  `CharacterProgressionContent` with the revisions stored on the Character root
  (`read_character_root_interpretation`), the progression row is read under the session's
  gameplay fence, and a new Character (root revision 1) is initialized with the same request in
  up to `RECONCILE_ATTEMPTS` rounds.
- A root whose interpretation is no longer the newest global one (a newer interpretation was
  configured after the Character was created) is decided `progression_context_mismatch`: the
  gameplay fence of every durable progression write refuses that root (§0.2), so the session is
  left unbound and its deaths respawn non-durably instead of retrying a death write that can
  never commit. An interpretation configured while a bound session is live (#1964 Codex P1
  4243820239): the first death write the gameplay fence then refuses with
  `ProgressionContextMismatch` downgrades the session to unbound (`settle_session_death`, logged
  `progression_unbound reason=progression_context_mismatch`) and that death respawns non-durably,
  with no XP and no durable write, after exactly that one attempt; the fence is not widened.
- A bound session keeps an `Arc<SessionProgressionBinding>` in the seam's session map, removed
  in `retire` (so a resume reuses it) or by that context refusal. `AdmittedSession` is
  unchanged.
- An unbound session logs `progression_unbound reason=...` once; its deaths respawn non-durably
  and its kills award no XP (`NoProgressionBinding`). `progression_initialization_unavailable`
  also makes the first entry `RefusedUnavailable`.
- Player deaths with a binding are durable (`settle_player_death`); kill XP reads the same
  binding (table width `CHARACTER_EXPERIENCE_TABLE_LEVELS` on both paths).
- A refused session that holds a runtime actor is held at most `interval * missed_limit`, then
  closed with no frame (`hold_refused` in `connection.rs`; no dispatch arm touched).
- The futures that hold a policy by value across awaits (the durable kill settle, the death
  commit and the admission binding step) are pinned on the heap, so the admitted-session
  futures stay within the seam's 8 MiB stack (`serve_accepted` 758 KB inline, 136 KB boxed).

## Excluded scope

The death penalty (DEATH-3/4), the SPEED-1 player level (`PLAYER_LEVEL_UNTIL_PROGRESSION_OWNER`
unchanged), migrations, and `durability/character_progression.rs` are unchanged.

## Acceptance (D968 3a split)

- PostgreSQL (`character_progression_admission_postgres_cases`, a child of the shared P03 cases,
  so `character_authority_postgres` runs it in CI): a new Character binds, is initialized with the
  stored eight values and its death commits and replays under the binding; a matching row binds
  with no initializer call; a missing row past revision 1, a stale row, a changed table, a
  changed death policy and a global interpretation advanced past the root each decide without a
  write, and the advanced root's death is refused by the durable writer on every attempt; a failed round retries; a lost
  acknowledgement reconciles; rounds that all fail leave no row and a later admission
  initializes.
- Unit: unbound reason names, the binding's revisions, the death request formed from the
  binding, one table width on the kill and death paths, `AdmittedSession: Copy`, a binding whose
  death is refused mid-session for its context settling that death non-durably after one attempt
  and never attempting again (other refusals keep the binding and retry), and the bounded
  hold (silent client closed after the bound with no frame, a frame before it closes with an
  error, no actor is not bounded).

## Validation

On the exact tested head named in the PR body (PostgreSQL 17.6 local service):

- `cargo fmt --all --check`: pass
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --lib`: pass
- `cargo test --locked -p oteryn-game-server --test character_progression_postgres`: pass
- `cargo test --locked -p oteryn-game-server --test character_authority_postgres`: pass
- `cargo test --locked -p oteryn-game-server --test combat_death_reward_postgres`: pass
- `cargo test --locked -p oteryn-game-server --test character_death_receipts_postgres`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
