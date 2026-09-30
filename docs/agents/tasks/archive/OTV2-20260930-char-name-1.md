# OTV2-20260930-char-name-1

```yaml
task_id: OTV2-20260930-char-name-1
title: CHAR-NAME-1 - Character name column, global name reservation and bootstrap intent v2 (D166)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/char-name-1
issue: 162
lane_id: Character Authority / N4-P
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: cf251bb2
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
owner: "CHAR-NAME-1 hard worker (repair generation 1: claude-code-session-011MXdyGBxZwZmAJZWW9Ddja)"
created_at: 2026-09-30
updated_at: 2026-09-30
owned_paths:
  - apps/game-server/migrations/0022_character_name.sql
  - apps/game-server/src/domain/character_name.rs (+ mod.rs registration)
  - apps/game-server/src/character_bootstrap_intent.rs
  - apps/game-server/src/durability/character_authority.rs (+ mod.rs, node/serve.rs error variant)
  - apps/game-server/tests/character_authority_postgres.rs and tests/support/*_postgres_* root fixtures (name + v2 binding)
  - docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md §6.1, §15
  - docs/architecture/reviews/OTERYN_CHARACTER_AUTHENTICATED_BOOTSTRAP_INTENT_DECISION_2026-09-23.md (amendment note; extension granted by the control plane)
  - Platform pin (D187, Q43a): .github/workflows/{merge-gate,merge-group-gate,node-boot-qualification,gameplay-server-seam,wp5-s3b-composition}.yml
  - candidate-side pins (owner-approved 2026-09-30): tools/repository/validate_repository_policy_core.py, tools/repository/test_validate_merge_group_pg_sim.py, PLATFORM_SOURCE in tests/wp5_s3b_composition.rs and src/gameplay_transport/qualification.rs
  - tools/qualification/{wp5_s3b,node_boot}/run.sh (pass --requested-name)
  - docs/agents/tasks/archive/OTV2-20260930-char-name-1.md
external_repositories:
  - Oteryn/Oteryn-Platform#1428: issuer emits bootstrap intent contract_version 2 with requested_name (lockstep)
jira: null   # sync pending (coordinator batch)
```

## Owner decisions (given directly to this worker, 2026-09-30; numbered D180-D184 by the control plane)

1a repertoire (2..29 ASCII letters, single inner spaces); 2c key folds case and spaces; 3a one global namespace;
4b released names blocked 30 days for everyone (ships with rename/delete); 5a `requested_name` in bootstrap intent
contract version 2, lockstep with Platform. Stated assumption: existing Characters are preproduction only, so 0022
fails closed on a store that holds any Character. Recorded in the boundary contract §6.1 as D180-D184.
D187 (Q43a) moves the pinned Platform commit to the #1428 merge commit; the owner approved the matching candidate-side
pin rotation and PLATFORM_SOURCE labels in this session.

## Outcome

- 0022: `game_character_roots.name` (+ generated `name_key`), `game_character_name_reservations` (PK `name_key`,
  immutable, no truncate), definer AFTER INSERT trigger reserving the key, revision guard now holds `name` equal,
  runtime EXECUTE on both CHECK/generated-column functions (PRIV-GUARD-1), no runtime write on reservations.
- Bootstrap: name lock + reservation check before the intent floor, `CharacterAuthorityError::NameUnavailable`
  with zero authoritative writes; integrity check binds the v2 binding (tag 2 + name) and requires each root's
  reservation.
- Intent decoder: contract version 2 only, required `requested_name`; read request carries `contract_version 2`.

## Validation (local, PostgreSQL 17.6 pinned image)

- fmt, clippy `-D warnings`, `cargo test -p oteryn-game-server`: pass.
- Postgres targets: character_authority_postgres (new name matrix, race repeated 8x), check_function_privileges,
  and every target using the updated root fixtures: pass.

## Repair generation 1 (review FIX, #1316 comment 5906978940)

- Migration renumbered 0022 (#1314 holds 0021; main merged normally). LCFA-1 is 0023, GOLD-FEE-1a 0024.
- Key computed as `lower(replace(value, ' ', '') COLLATE "C")`.
- §6.1 cites D180-D184; blocked words, key versioning and "only finer" are derived engineering notes;
  preproduction-only assumption stated.
- Test `name_migration_refuses_a_store_that_already_holds_a_character`: 0022 on a store with a root fails with
  23502, the root stays and the migration history stops at 21.
- Bootstrap intent decision amended (contract version 2, required `requested_name`, binding tag 2, pointer to §6.1).
- Platform pin 9147bfd3 -> 5d4883ac (merge commit of Oteryn/Oteryn-Platform#1428 on Platform main), with the
  candidate-side job digests, merge-group gate blob and PLATFORM_SOURCE labels.
- Local: fmt, clippy, full `oteryn-game-server` tests with PostgreSQL 17.6 (character_authority_postgres 734/734),
  repository policy and governance validators pass; pg-sim tests need `pwsh` (left to CI).

## Follow-up / blocker

The protected-base `merge-authority-audit.yml` on main pins both workflow blobs; a control-plane stage A on main must
set `EXPECTED_MERGE_GATE_BLOB` ea38b9ccbbfb171aeca4b38762554497f8557dc3 and `EXPECTED_MERGE_GROUP_GATE_BLOB`
82b0d53592db4b030c65915a12f29ff3da718ad1 before this PR's audit passes.
