# OTV2-20261005-spell-lock-2a

```yaml
task_id: OTV2-20261005-spell-lock-2a
title: SPELL-LOCK-2a spell lane, commit window and complete reservation
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/spell-lock-2a-20261005
pr: null
base_sha: aecb02c5
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: oteryn-hard-worker session_01ARgwFxy96wwU3MEiSbVPCd
created_at: 2026-10-06T00:00:00Z
updated_at: 2026-10-06T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_SPELL_LOCK_2_2026-10-05.md §2.1 owned_paths
  - apps/game-server/src/durability/creature_source_items.rs  # CP amendment D848
  - apps/game-server/tests/support/type2_audit_activation_postgres_cases.rs  # CP amendment D848, lane permit before commit_item_mint only
  - apps/game-server/src/gameplay_transport/spell_access_facts.rs  # CP amendment D849 (owner-approved), pub(crate) async read / sync qualify split only
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Implements `ARCH-SPELL-LOCK-2-V1` §1.1-§1.4 and §1.6 (packet §2.1).

## CP amendments

- D848: `creature_source_items.rs` and the type2 audit activation PostgreSQL case join
  owned_paths, because both reach key 33 (`commit_creature_source_items` through the scope
  assert, the test case through `commit_item_mint`) and the compiler requires them to take a
  permit.

- D849 (owner-approved for this file only): `spell_access_facts.rs` joins owned_paths for the
  minimal split of the owned-fact load into an async database read and a sync qualification, so
  the native post-commit transaction runs without Channel guards; 2b keeps the rest of the file.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: no durable value, wire format, identity or authority changes; the change is an
in-process concurrency invariant of the Channel owner (decision §1.7).

## Validation

- cargo fmt --all -- --check: pending
- cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings: pending
- cargo test --locked -p oteryn-game-server: pending
- python tools/agents/validate_governance.py: pending
- git diff --check: pending
