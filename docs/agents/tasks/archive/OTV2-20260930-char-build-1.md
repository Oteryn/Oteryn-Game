# OTV2-20260930-char-build-1

```yaml
task_id: OTV2-20260930-char-build-1
title: "CHAR-BUILD-1a Character build state storage, guards and admission verifier (migration 0030)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
issue: 162
branch: claude/char-build-1
pr: "exact PR in the #162 FREEZE_SHA entry"
base_sha: 8e609a41
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session_011jPtp9mvLBYdXWiwQ58opr (oteryn-hard-worker)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0030_character_build_state.sql
  - apps/game-server/src/durability/character_authority.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - apps/game-server/tests/support/character_build_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20260930-char-build-1.md
public_contracts: []
depends_on: []
blocks: [CHAR-BUILD-1b, W2b, DAWNPORT-1, DEATH ML loss]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

CHAR-BUILD-1 is split (brief: ~500 hand-written lines). This PR is **CHAR-BUILD-1a**, migration
`0030` and its admission verifier, as `0017` preceded STANCE-1:

- `game_character_build_state` (A13 §4.1, SKILLS-0 §3.1): vocation, magic level with
  `mana_spent`, and seven skill (`level`, `tries`) pairs; no row means the seed (`none`, 0, 0,
  7 x (10, 0)); no creation insert, no backfill, never deleted, truncate rejected.
- `game_character_build_receipts` (A13 §4.2, SKILLS-0 §3.2): one receipt kind with a cause CHECK
  per direction (`training`, `vocation_choice`, `promotion`), stance prune fields, binding and
  policy digest, revision fields.
- Nullable death receipt build fields (A13 §4.6, SKILLS-0 §3.6): all or none, strict loss.
- The 0026 consistency guard carried verbatim with the build arms (#1271 F1): revision one, chain,
  stance chain, build chain and row, successor query (#1271 F3).
- `verify_character_integrity`: build receipts join the chain; named build-chain continuity and
  build-row checks cover the death build fields (#1271 F4).
- Grants as `0017`; no CHECK calls a new function, and a `SET ROLE oteryn_game_runtime` build
  commit is tested (#1271 F2).
- PG-COVERAGE-1: the cases run inside the CI-run `character_authority_postgres` target.

**CHAR-BUILD-1b** (next, no migration): `commit_character_build`, `reconcile_character_build`,
the admission load, the binding, `req(L)` and checked saturating arithmetic, and the writer tests
(replay, conflict, stale fence).

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. This record was archived in the PR's final
  authoring commit (`docs/agents/tasks/archive/README.md`).
- Size: 0030 is 703 lines, of which about 260 are the 0026 guard carried verbatim; the verifier
  adds about 110 lines and the cases about 700. Split from 1b as the brief asks.

## Architecture and source of truth

- `PROVEN`: A13 (D150, D151) §4.1, §4.2, §4.6; SKILLS-0 §3.1, §3.2, §3.6; #1271 5902863956
  (F1-F4); migrations 0009-0026 on main `8e609a41`.

## High-risk authority/recovery qualification

Persistence and chain guards only; no fenced writer in 1a. Every guard branch has one red run
with its exact message or constraint and an unchanged snapshot. Writer fence cases belong to 1b.

## Acceptance criteria

- [ ] Exact frozen head with passing CI.
- [ ] Independent persistence review (control plane).
- [ ] Protected Merge Queue integration.

## Excluded scope

- The writer, reconcile, admission load (CHAR-BUILD-1b); PROF-1; wire/protocol; content; Platform.

## Validation

- `cargo fmt --check`; `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`.
- PostgreSQL 17.6 (docker `postgres:17.6-bookworm`): `character_authority_postgres` 885 passed,
  `durability_postgres` 770, `runtime_scope_assignment_postgres` 705,
  `native_admission_source_postgres` 697; `--lib` 1170 passed.
- `python3 tools/agents/validate_governance.py`; `python3 tools/repository/validate_repository_policy.py`;
  `git diff --check`.

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/char-build-1
owner_action_required: null
blocker: null
next_action: null   # control plane: persistence review; then allocate CHAR-BUILD-1b
```
