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
owner: "CHAR-NAME-1 hard worker (claude-code-session-01CAFRNutbD9G27BFGoBGBBd)"
created_at: 2026-09-30
updated_at: 2026-09-30
owned_paths:
  - apps/game-server/migrations/0021_character_name.sql
  - apps/game-server/src/domain/character_name.rs (+ mod.rs registration)
  - apps/game-server/src/character_bootstrap_intent.rs
  - apps/game-server/src/durability/character_authority.rs (+ mod.rs, node/serve.rs error variant)
  - apps/game-server/tests/character_authority_postgres.rs and tests/support/*_postgres_* root fixtures (name + v2 binding)
  - docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md §6.1, §15
  - docs/agents/tasks/archive/OTV2-20260930-char-name-1.md
external_repositories:
  - Oteryn/Oteryn-Platform: issuer must emit bootstrap intent contract_version 2 with requested_name (follow-up, see below)
jira: null   # sync pending (coordinator batch)
```

## Owner decisions (given directly to this worker, 2026-09-30)

1a repertoire (2..29 ASCII letters, single inner spaces); 2c key folds case and spaces; 3a one global namespace;
4b released names blocked 30 days for everyone (ships with rename/delete); 5a `requested_name` in bootstrap intent
contract version 2, lockstep with Platform. Stated assumption: existing Characters are preproduction only, so 0021
fails closed on a store that holds any Character. Recorded in the boundary contract §6.1; D-number to be assigned
by the control plane.

## Outcome

- 0021: `game_character_roots.name` (+ generated `name_key`), `game_character_name_reservations` (PK `name_key`,
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

## Follow-up / blocker

Platform issuer change (contract version 2 + `requested_name`) is not in this PR: the session could not clone
Oteryn-Platform. Until it lands, Game refuses Platform's version 1 intents (fail closed; no production impact).
