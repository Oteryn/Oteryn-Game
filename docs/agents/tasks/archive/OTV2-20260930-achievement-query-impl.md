# OTV2-20260930-achievement-query-impl

```yaml
task_id: OTV2-20260930-achievement-query-impl
title: ACCOUNT_ACHIEVEMENTS_QUERY wire, registry, fixtures and server read path (display contract §7 item 2)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/achievement-query-server   # (b), stacked on (a) claude/achievement-query-impl
pr: null   # (a) and (b) PRs are opened by the lane lead; see PR and closeout
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
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/tests/account_achievement_postgres.rs
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
rownolegle 1 2 3", item 1). Split by the 500-line batch rule into two stacked PRs.

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
    `error_code` (FND-02 §18).
- **(b) server** (`claude/achievement-query-server`):
  - `AchievementCatalogue` keeps name, description, grade, points and secret, and builds the page
    (`account_achievements_page`): one row per fact, retired = 0 points, `total_points` over all facts,
    `fact_count` = number of facts, order grade / name by code point / key, 64 rows a page, empty past the end;
    a fact under an unknown key fails closed (§4.3).
  - `DurabilityRoot::read_account_achievements`: facts joined to their own request on the composite key for
    `earned_at`, filtered by the given account, at most catalogue size + 1 rows; recovery fence only, no
    Character fence, no write (§4.1); runtime role `SELECT` (0021).
  - Dispatch in `serve_admitted` beside command types 1-3: the account is the admitted controller's
    (`ControllerBinding.account_id`, from the character authority record at admission), never the payload's.
    A page is `ACCEPTED`; a row over its byte bounds is `REJECTED` with `PAYLOAD_LIMIT_EXCEEDED` (1009); a
    malformed query, no controller, storage failure or an integrity fault is `REJECTED` with no payload.
  - The catalogue is passed to the gameplay seam (`GameplaySeamOwners.achievements`) from `serve`.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >-
  The query performs no production mutation, consumes no session, lease or generation fence for a write,
  installs no controller and interprets no recovery evidence. It reads two append-only immutable tables
  (display contract §4.1). The identity binding (own account only) is covered by the dispatch test and the
  PostgreSQL cases below.
```

## Excluded scope

The client panel (§7 item 3), the website and ranking export, `.github/**`, migrations and grants (0021 already
grants `SELECT`), and any `docs/architecture/**` text. Contract §7 lists a test for "a fact under a key absent
from a per-world subset": the runtime has no per-world catalogue subset, so there is nothing to test yet.

## Validation

- (a): `cargo fmt --all --check`; `cargo clippy --locked -p oteryn-protocol-oteryn --all-targets -- -D
  warnings`; `cargo test --locked -p oteryn-protocol-oteryn` (86 passed).
- (b): `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`; `cargo test --locked -p
  oteryn-game-server --lib` (1162 passed, 2 ignored); PostgreSQL 17.6: `account_achievement_postgres` 701
  passed, `character_authority_postgres` 883 passed, `chest_use_postgres` 763 passed.
- Both: governance and repository-policy validators, `python -m unittest discover -s tools/agents/tests`,
  `git diff --check`.
- Mutation evidence (PostgreSQL target, each reverted): reading every account's facts (account filter
  removed) fails `the_query_returns_only_the_accounts_own_earned_facts`; adding unearned catalogue records to
  the page fails that test, `a_retired_fact_shows_with_zero_points_and_moves_only_the_watermark` and
  `the_query_pages_by_64_rows`; joining requests on (account, key) only returns a later request's duplicate
  row and fails the own-account test and `grants_commit_under_the_runtime_role_grants`.

## Independent review

Required (wire contract, public registry, new server command). The control plane triggers it on each frozen
head.

## PR and closeout

- PRs: (a) `claude/achievement-query-impl` and (b) `claude/achievement-query-server` (stacked), opened by the
  lane lead; merge commit/result: squash merge of each PR.
- Review state: pending at authoring.
- Ownership release: on merge of (b).

## Context checkpoint

```yaml
last_progress: (a) and (b) authored, validated locally and pushed
status: completed
next_action: lane lead opens both PRs and requests review on the frozen heads
```
