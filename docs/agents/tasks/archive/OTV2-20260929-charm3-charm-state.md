# OTV2-20260929-charm3-charm-state

```yaml
task_id: OTV2-20260929-charm3-charm-state
title: CHARM-3 - Charm unlocks and assignments, derived Charm Points and Echoes (migration 0019)
mode: IMPLEMENT
status: blocked   # not freezable: blockers B1 and B2 below
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/charm3-charm-state
issue: 162
lane_id: GAME-CHAR durability (0009 guard-function chain)
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: 4ea220f
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "CHARM-3 hard worker (claude-code-session-012nzPTz29NThWJG45F2m5fP)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0019_character_charm_state.sql
  - apps/game-server/src/domain/charm.rs
  - apps/game-server/src/domain/mod.rs          # registration line only
  - apps/game-server/src/durability/charm_state.rs
  - apps/game-server/src/durability/mod.rs      # registration and linkage block only
  - apps/game-server/tests/charm_state_postgres.rs
  - apps/game-server/tests/support/charm_state_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20260929-charm3-charm-state.md
public_contracts: []
depends_on:
  - "CHARM-0 decision packet §4.2 and owner answers §7 (PR #1295)"
  - "#1293 static charm catalogue candidate (25 charms, oteryn:charm.<name>)"
  - "CHARM-2 Bestiary progress (claude/charm2-bestiary-progress, migration 0018)"
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
- Migration 0019: `game_character_charm_receipts`, `game_character_charm_unlocks`,
  `game_character_charm_assignments`; the 0017 chain guard with the charm kind added; a deferred
  charm projection guard; row guards; grants. Points and echoes are never stored.

## High-risk authority qualification

```yaml
applicable: true
authority_invariants:
  identity_binding: [occurrence -> one command binding, receipt <-> row (charm, stage/race, revision, occurrence)]
  current_liveness: [recovery fence, FND-04 session/connection/lease/scope, scope assignment + node incarnation, root at expected revision]
  temporal_provenance: [catalogue revision = root content revision, catalogue digest in the binding]
consumer_boundaries: [commit_charm_command, reconcile_charm_command, read_character_charm_state, SQL writes under 0019]
mutation_operators:
  applicable: [stale revision, stale connection/lease/scope generation, other session, ended session, revoked node, missing progression state, foreign catalogue revision, reused occurrence, concurrent distinct commands, concurrent same occurrence, injected rollback, row-only write, skipped or out-of-order stage, receipt without row, row without receipt, assign before unlock, second assign, race capacity, delete/update/truncate]
  considered_not_applicable: [expired/future time - no time-bounded input; the receipt time is the server statement time]
one_invariant_per_negative_case: true
record_derived_matching_helper: none
```

## Blockers

- **B1** `verify_character_integrity` (`durability/character_authority.rs`, outside the owned
  paths) requires XP receipts alone to explain every CharacterRevision. After the first charm
  (or death, stance, Bestiary) receipt, `open_character_authority` fails for every Character, so
  a restart cannot open Character authority. `restart_readback_after_charm_commands` reproduces it
  and is `#[ignore]`d until the check covers every receipt kind. #1278 (DEATH-1) rewrites that
  check; it needs a charm arm (`game_character_charm_receipts`), then the case is un-ignored.
- **B2** CHARM-2 migration 0018 also replaces `game_character_progression_consistency_guard`.
  0019 replaces it again from the 0017 body, so 0019 must be rebased onto 0018 (add the Bestiary
  kill receipts to the chain) before either merges. Prepared and verified locally: 0018 at
  `73f66421` plus 0019 with the 0018 guard body and the three charm arms passes
  `charm_state_postgres`, `character_stance_postgres` and `character_death_receipts_postgres`.

## Findings for the lead

- The gameplay fence is a copy of the private `character_progression::assert_gameplay_fence`;
  making that `pub(super)` and sharing it removes the copy (outside the owned paths).
- The PostgreSQL cases run in the standalone `charm_state_postgres` target. CI runs only the
  registered wrappers; including them in `character_authority_postgres` is one `#[path]` line
  outside the owned paths.
- No durable promotion or slot entitlement exists; production `CharmFacts` returns `false` and
  `Free` until one does. `rulesets/progression/charms/` stays unpopulated: the rules are code.

## Validation (local)

- `cargo fmt --all --check`, `cargo clippy --locked -p oteryn-game-server --all-targets -- -D
  warnings`, `cargo test --locked -p oteryn-game-server`: pass.
- PostgreSQL 17.6: `charm_state_postgres` 5 pass, 1 ignored (B1); `character_stance_postgres`,
  `character_death_receipts_postgres`, `character_progression_postgres`,
  `character_authority_postgres`: pass.
- RED: disabling the charm projection guard fails the SQL guard case; skipping the balance check
  fails the rules and the double-spend cases.
- `validate_governance.py`, governance unit tests, `validate_repository_policy.py`,
  `git diff --check`: pass.

## Closeout

- Review: independent exact-head review after B1 and B2 are resolved and the head is frozen.
- Merge commit/result: squash merge of the CHARM-3 PR (resolve with `git log --grep`).
