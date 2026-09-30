# OTV2-20260929-spell-w2a-followup-tests

```yaml
task_id: OTV2-20260929-spell-w2a-followup-tests
title: SPELL W2A follow-up - resume-after-cast and fail-closed spell book tests
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/spell-w2a-followup-tests
issue: 162
lane_id: spell cast
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: 50d75c65
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "impl worker (claude-code-session-01MnSvpbKjAZEdzEaFrwiu7D)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/spell/**
  - apps/game-server/src/gameplay_transport/** (#[cfg(test)] only)
  - docs/agents/tasks/archive/OTV2-20260929-spell-w2a-followup-tests.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Closes the two EVIDENCE_GAPs of the independent review of #1263 (#162 comment 5898225379).

- Resume: `spell_same_session_resume_snapshots_the_paid_vitals_and_keeps_the_cooldown`
  (`connection.rs` tests) drives the real `serve_admitted` loop over the real Channel owner with
  cast facts granted the way #1263's tests do. A wounded actor casts `exura`; a second
  `serve_admitted` with the owner's repeated initialization snapshots the paid vitals (mana 70/90,
  healed health, revision 2), and a second cast answers `CoolingDown` with no delta.
- Fail closed: `a_book_that_does_not_load_fails_closed_and_nothing_is_skipped` (`cast_tests.rs`).
  Seam: `v1_spell_book` now delegates to the private `book_from_bundles(&[(&str, &str)])` (same
  behaviour, production passes `V1_BUNDLES`); a bad spell, bad dependencies, a non-spell bundle, a
  bad bundle among good ones and a duplicate key all fail the whole book.
- Test-only helpers: `PlayerSpellState::set_health_for_test` and `tests::wound` (`#[cfg(test)]`).

## Limits

- The `serve.rs` mapping of a loader error to `BootError::ContentActivation("spell book")` is not
  exercised: `node/serve.rs` is outside the owned paths and needs a boot seam.
- An empty `SpellBook` is valid by construction; production's book is the non-empty const.

## Validation (local)

- Mutation checks (not committed): re-initializing a present actor resets it makes the resume test
  fail; skipping bad bundles makes the loader test fail.
- `cargo fmt --all --check`, `cargo clippy --locked -p oteryn-game-server --all-targets -D warnings`,
  `cargo test -p oteryn-game-server --lib spell`: 98 passed, 0 failed.
