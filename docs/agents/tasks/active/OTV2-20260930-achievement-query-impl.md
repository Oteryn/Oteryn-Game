# OTV2-20260930-achievement-query-impl

```yaml
task_id: OTV2-20260930-achievement-query-impl
title: ACCOUNT_ACHIEVEMENTS_QUERY wire, registry, fixtures and server read path (display contract §7 item 2)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/achievement-query-impl   # (a) wire; (b) server stacked on claude/achievement-query-server
pr: null
base_sha: de1a6226
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "ACHIEVEMENT lane worker (Claude Code, session_01L687XUJAozgAPkNZ7B8GMG)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  # (a) claude/achievement-query-impl
  - docs/contracts/protocol-oteryn/v1/account_achievements_v1.proto
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - crates/protocol-oteryn/src/account_achievements.rs
  - crates/protocol-oteryn/src/lib.rs
  - docs/agents/tasks/active/OTV2-20260930-achievement-query-impl.md
  # (b) claude/achievement-query-server, stacked on (a)
  - apps/game-server/src/achievement_catalogue.rs
  - apps/game-server/src/durability/account_achievement.rs
  - apps/game-server/src/gameplay_transport/account_achievements.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/tests/account_achievement_postgres.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - apps/game-server/tests/support/account_achievement_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20260930-achievement-query-impl.md
public_contracts:
  - docs/contracts/protocol-oteryn/v1/account_achievements_v1.proto
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
depends_on:
  - docs/architecture/OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1.md (#1343; D223-D228, #162 5911933242)
  - "#1348 (runtime AchievementCatalogue)"
blocks:
  - the client achievements panel (display contract §7 item 3)
cross_repository_coordination_id: null
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

Delivery order item 2 of `OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1.md`, owner authorization 2026-09-30 ("rob
rownolegle 1 2 3", item 1). Split by the 500-line batch rule into two stacked PRs:

- **(a) wire** (`claude/achievement-query-impl`):
  - `account_achievements_v1.proto`: `AccountAchievementsQuery { page }`, `AccountAchievementRow`,
    `AccountAchievementsResult { total_points, fact_count, page, has_more, rows }` (§3.2).
  - Registry `command_types` id 10 `ACCOUNT_ACHIEVEMENTS_QUERY` (reserved on #162), owner decision D223-D228,
    `max_payload_bytes` 6, `max_result_payload_bytes` 32788 (64 worst-case rows of 509 bytes + framing; inside
    the 64 KiB FND-02 §19 parents, checked at compile time).
  - `crates/protocol-oteryn/src/account_achievements.rs`: strict codecs (key grammar, UTF-8, per-field byte
    bounds, grade 1-4, at most 64 rows, `has_more` only on a full page), fixtures from the reference protobuf
    runtime (protoc-generated Python, protobuf 7.36.2, outside the repository), an independent raw-byte walk
    of the worst case, the malformed and oversize corpus, a seeded round-trip property test, and the registry
    binding test.
  - `encode_command_error_result`: a `REJECTED` `CommandResult` with an `OPERATION_TERMINAL` code in
    `error_code` (FND-02 §18), for the over-bound reply of §3.3.
- **(b) server** (`claude/achievement-query-server`, stacked): catalogue display fields, the read-only facts
  read, the page builder, command dispatch and the PostgreSQL cases.

## High-risk authority/recovery qualification

```yaml
applicable: pending   # (b): read-only, no fence; recorded with the server PR
```

## Excluded scope

The client panel (§7 item 3), the website and ranking export, `.github/**`, migrations and grants (0021 already
grants `SELECT`), and any `docs/architecture/**` text.

## Validation

- (a): `cargo fmt --all --check`; `cargo clippy --locked -p oteryn-protocol-oteryn --all-targets -- -D
  warnings`; `cargo test --locked -p oteryn-protocol-oteryn` (86 passed); governance and repository-policy
  validators; `git diff --check`.

## Independent review

Required (wire contract, public registry). The control plane triggers it on the frozen head.

## Context checkpoint

```yaml
last_progress: (a) wire authored and validated locally
status: implementing
next_action: author (b) server on the stacked branch
```
