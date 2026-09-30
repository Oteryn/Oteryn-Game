# OTV2-20260929-charm3-charm-state

```yaml
task_id: OTV2-20260929-charm3-charm-state
title: CHARM-3 - Charm unlocks and assignments, derived Charm Points and Echoes (migration 0020)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/charm3-charm-state
issue: 162
lane_id: GAME-CHAR durability (0009 guard-function chain)
pr: 1307
base_sha: 4ea220f
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "CHARM-3 hard worker (claude-code-session-012nzPTz29NThWJG45F2m5fP)"
created_at: 2026-09-29
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0020_character_charm_state.sql
  - apps/game-server/src/domain/charm.rs
  - apps/game-server/src/domain/mod.rs          # registration line only
  - apps/game-server/src/durability/charm_state.rs
  - apps/game-server/src/durability/mod.rs      # registration and linkage block only
  - apps/game-server/tests/charm_state_postgres.rs
  - apps/game-server/tests/support/charm_state_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs   # shared: one #[path] include only (lead decision)
  - apps/game-server/src/durability/character_authority.rs  # charm_receipts arm of verify_character_integrity only (lead decision)
  - docs/agents/tasks/archive/OTV2-20260929-charm3-charm-state.md
public_contracts: []
depends_on:
  - "CHARM-0 decision packet §4.2 and owner answers §7 (PR #1295)"
  - "#1293 static charm catalogue candidate (25 charms, oteryn:charm.<name>)"
  - "#1278 DEATH-1 (multi-kind integrity check, pub(super) gameplay fence)"
  - "#1306 CHARM-2 Bestiary progress (migration 0019)"
blocks: [CHARM-4, CHARM-5]
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

- `domain::charm` (pure): charm and Bestiary race keys, stages, the catalogue, and the rules:
  - derived balance (owner answer 2a): points earned = sum of `charm_points` of completed entries;
    echoes earned = 50/100/200 per major stage unlocked + 100 if promoted; spent = sum of the
    unlocked stage costs (majors in points, minors in echoes); available = earned − spent;
  - unlock the next stage if the currency affords its cost; assign an unlocked, unassigned charm
    (no unassign or re-assign, owner answer 3c) to a race at stage ≥ 3 (major) or ≥ 2 (minor);
  - a race holds one major and one minor charm at once (TibiaWiki Updates/14.10, Canary
    `iobestiary.cpp`, resolved by the coordinator on 2026-09-29);
  - ASSUMPTION: all assigned charms share one slot limit: free 2, Premium 6, Charm Expansion
    unlimited (Canary; the wiki text predates 14.10).
- `durability::charm_state`: `commit_charm_command` (one fenced Character transaction per
  command, CharacterRevision + 1, receipt + unlock or assignment row), `reconcile_charm_command`,
  `read_character_charm_state`, and the `CharmFacts` trait for facts owned elsewhere (completed
  Bestiary stage and entries from CHARM-2, promotion, slot entitlement), read inside the
  command's transaction after the root lock.
- Migration 0020: `game_character_charm_receipts`, `game_character_charm_unlocks`,
  `game_character_charm_assignments`; the 0019 chain guard with the charm kind added; a deferred
  charm projection guard; row guards; grants. Points and echoes are never stored.

## High-risk authority qualification

```yaml
applicable: true
authority_invariants:
  identity_binding: [occurrence -> one command binding, receipt <-> row (charm, stage/race, revision, occurrence)]
  current_liveness: [recovery fence, FND-04 session/connection/lease/scope, scope assignment + node incarnation, root at expected revision]
  temporal_provenance: [catalogue revision = root content revision, catalogue digest in the binding]
consumer_boundaries: [commit_charm_command, reconcile_charm_command, read_character_charm_state, open_character_authority integrity, SQL writes under 0020]
mutation_operators:
  applicable: [stale revision, stale connection/lease/scope generation, other session, ended session, revoked node, missing progression state, foreign catalogue revision, reused occurrence, concurrent distinct commands, concurrent same occurrence, injected rollback, row-only write, skipped or out-of-order stage, receipt without row, row without receipt, assign before unlock, second assign, race capacity, delete/update/truncate]
  considered_not_applicable: [expired/future time - no time-bounded input; the receipt time is the server statement time]
one_invariant_per_negative_case: true
record_derived_matching_helper: none
```

## Resolved blockers

- **B1** `verify_character_integrity` counted only XP receipts, so after the first charm receipt
  no Character authority reopened. #1278 made it multi-kind; this PR adds the
  `game_character_charm_receipts` arm (path granted for that arm only).
  `restart_readback_after_charm_commands` is GREEN.
- **B2** #1306 (CHARM-2, now migration 0019) replaces the chain guard too. After merging `main`,
  migration 0020 rebuilds the guard on main's 0019 body with every arm (XP, death, stance,
  Bestiary) kept and the charm arms added.

## Review repair (FIX on 461a7c73, review comment 5903659640)

- Material: 0020 revoked EXECUTE on the two key functions behind the charm CHECKs without granting
  it to `oteryn_game_runtime`, so the runtime login could not insert (42501). 0020 now grants it.
- Material test gap: `charm_commands_commit_under_the_runtime_role_grants` runs unlock, replay,
  assign and read as a real login in the runtime group; the guard case asserts
  `has_function_privilege` for both functions. RED without the grant (both fail, and
  PRIV-GUARD-1 names the charm CHECKs), GREEN with it.
- Hardening: the charm guard rejects receipts of one charm with differing categories, so an
  assign cannot record a major charm as minor to pass the race bound. New negative case; RED
  without the arm.

## Rollback

Applied migrations are immutable, so rollback is a new migration. Before any charm receipt exists
it drops the three charm tables and four charm functions and restores the 0019 guard body. After
charm receipts exist they are part of the CharacterRevision chain that the guard and
`verify_character_integrity` count: a rollback keeps the receipt table and both chain arms and
only stops new writes (revoke the runtime INSERT grants). The Rust side reverts with the PR.

## Findings for the lead

- The gameplay fence is the shared `pub(super)` `character_progression::assert_gameplay_fence`
  (#1278); the copy is gone. The catalogue revision is checked against the progression state's
  content revision, which the shared `state_matches_root` binds to the fenced root.
- `BestiaryCharmFacts` is the production `CharmFacts` over the CHARM-2 kill counters, read in the
  command's transaction. Its entries (race + `charm_points`) come from the Creature definitions
  loaded by the caller. No durable promotion or slot entitlement exists: it reports `false` and
  `Free` until one does.
- Slot limits (free 2, Premium 6, Charm Expansion unlimited) are a Canary-sourced assumption
  pending the owner's answer.
- The cases run in the standalone `charm_state_postgres` target and, through one `#[path]`
  include, in the protected `character_authority_postgres` lane.
- `rulesets/progression/charms/` stays unpopulated: the rules are code.

## Validation (local)

- `cargo fmt --all --check`, `cargo clippy --locked -p oteryn-game-server --all-targets -- -D
  warnings`, `cargo test --locked -p oteryn-game-server`: pass.
- PostgreSQL 17.6: `charm_state_postgres` 8/8 (runtime-role commits; restart readback; real
  CHARM-2 kill counters through `BestiaryCharmFacts`), `check_function_privileges_postgres`
  (PRIV-GUARD-1) 1/1, `character_authority_postgres` 720/720,
  `bestiary_progress_postgres` 628/628, `combat_bestiary_postgres` 645/645,
  `character_stance_postgres` 5/5, `character_death_receipts_postgres` 6/6,
  `character_progression_postgres` 635/635.
- RED: disabling the charm projection guard fails the SQL guard case; skipping the balance check
  fails the rules and the double-spend cases; without the integrity arm the restart case fails;
  without the runtime EXECUTE grant the runtime-role case, the privilege assertion and
  PRIV-GUARD-1 fail; without the category arm the category case fails.
- `validate_governance.py`, governance unit tests, `validate_repository_policy.py`,
  `git diff --check`: pass.

## Closeout

- Review: independent exact-head persistence review, routed by the control plane on the frozen
  head (no owner-funded review triggered by the worker).
- Merge commit/result: squash merge of the CHARM-3 PR (resolve with `git log --grep`).
