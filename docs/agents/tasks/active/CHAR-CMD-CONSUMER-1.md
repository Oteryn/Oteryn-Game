# CHAR-CMD-CONSUMER-1

```yaml
task_id: CHAR-CMD-CONSUMER-1
title: "CHAR-CMD-CONSUMER-1: Game consumer of Platform character command intents (CREATE path, disabled by default)"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/char-cmd-consumer-1
issue: null
governing_issue: 1622
pr: null
base_sha: a94831d0
created_at: 2026-10-14
updated_at: 2026-10-14
execution_policy: continuous_progress
migration_lease: "0086"
owned_paths:
  - apps/game-server/migrations/0086_character_commands.sql
  - apps/game-server/src/native_admission_source/character_commands.rs
  - apps/game-server/src/native_admission_source/character_commands_tests.rs
  - apps/game-server/src/native_admission_source/descriptor.rs
  - apps/game-server/src/native_admission_source/http1_mtls.rs
  - apps/game-server/src/native_admission_source/mod.rs
  - apps/game-server/src/durability/character_command.rs
  - apps/game-server/src/durability/character_command_tests.rs
  - apps/game-server/src/durability/character_authority.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/node/config.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/active/CHAR-CMD-CONSUMER-1.md
public_contracts: [OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md, RESOURCE_LIMITS_REGISTRY.json]
external_repositories: []
```

## Outcome

Contract step 2 of `OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` (§14, lock entry GAME-CHAR-CMD-0):
the Game consumer of Platform character command intents. It lists pending intents
(`ListPendingCharacterCommandsV1`), reads each (`ReadCharacterCommandIntentV1`), decides it under the
reconciled Character authority and publishes the stored receipt (`PublishCharacterCommandReceiptV1`),
byte-identical on every retry. Only `CreateCharacter` (`PLATFORM_USER_CREATE`) commits; any other
command is stored `CHAR_CMD_UNSUPPORTED`. The consumer runs only with an explicit
`[platform.character_commands]` section and is disabled by default; enablement stays gated by U-CC7
and U-CC9 and is not part of this task.

## Architecture and source of truth

- PROVEN: contract §3 (one exchange in flight, 1 s poll, back-off to 10 s on 429/503, read 404 alarm
  without receipt, read 409 stored `CHAR_CMD_OPERATION_CONFLICT`, publish 404/409 keep receipt and alarm),
  §5.2 receipt store, §5.3 decision order and per-revision conflict, §6 transient dependencies, §6.3
  lock order, §8 no AccountId, name or Character list in logs, §12 limits.
- DERIVED: a create command reuses audit event type `CHARACTER_AUTHORITY_BOOTSTRAPPED` with the
  `encode_bootstrap` payload, since the audit integrity check requires that payload for every outbox
  row; the LCFA projection change is produced by the existing migration 0024 trigger on the root insert.
- DERIVED: `game_character_creation_receipts` (a view over the operator bootstrap receipts and the
  committed command receipts) is what Character reads and the integrity check join, so a
  command-created root has a creation receipt like any other; an `operation_id` is held by at most
  one of the two tables, checked under the operation advisory lock by both writers.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - one receipt per operation_id across bootstrap and command stores
  - at most one non-conflict receipt per (variant, source_revision)
  - commit only under the recovery fence and the current node incarnation
  - at most 64 listed Characters per account, checked under the portfolio lock
consumer_boundaries: [Platform intent read, receipt publish, S2 account security, world assignment]
mutation_operators:
  applicable: [missing facts, stale facts, mismatched identity/binding, expired/future time, replay, concurrency]
  considered_not_applicable: [protocol versions beyond v1 (stored UNSUPPORTED)]
one_invariant_per_negative_case: true
independent_current_fact_sources: [S2 account security, runtime scope assignment, interpretation]
```

## Acceptance criteria

- [ ] Wire, cursor, rescan and retry behaviour covered by `character_commands_tests.rs`.
- [ ] Intent classification and receipt fixtures match the contract (`character_command_tests.rs`).
- [ ] PostgreSQL: commit, exact republish, conflicts, every rejection code, transient without a
      receipt, the 65th Character limit and integrity after reopen (`character_authority_postgres.rs`).
- [ ] Disabled unless configured; config validation keeps the identity separate.
- [ ] Limits registered as `CHARCMD-*` in `RESOURCE_LIMITS_REGISTRY.json`.

## Excluded scope

Enabling the consumer, transfer/rename/delete commands, the Platform side and any workflow,
ruleset or branch-protection change.

## Validation

- pending

## Self-review

- pending

## Independent review

- required: YES (persistence and authority); Codex review on the final head.

## PR and closeout

- pending

## Context checkpoint

```yaml
status: implementing
branch: claude/char-cmd-consumer-1
pr: null
blocker: null
next_action: run local checks, open the PR
```
