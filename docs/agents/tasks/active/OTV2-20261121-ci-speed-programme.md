# OTV2-20261121-ci-speed-programme

```yaml
task_id: OTV2-20261121-ci-speed-programme
title: Measure and shorten CI compilation and PostgreSQL qualification
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/ci-pg-flow-20261121
issue: 3352
pr: null
base_sha: b20391213cde54511233bd02fae3370e019c7bf4
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT standalone CI programme coordinator, issue 3352
created_at: 2026-11-21T09:29:01Z
updated_at: 2026-11-21T09:29:01Z
execution_policy: continuous_progress
owned_paths:
  - tools/ci/pg_test_rerun.py
  - tools/ci/test_pg_test_rerun.py
  - .github/workflows/pg-test-rerun.yml
  - tools/repository/benchmark_rust_cache.py
  - tools/repository/test_benchmark_rust_cache.py
  - .github/workflows/rust-cache-pilot.yml
  - apps/game-server/src/qualification.rs
  - apps/game-server/tests/
  - docs/agents/tasks/active/OTV2-20261121-ci-speed-programme.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Issue #3352 owns this multi-PR programme. Make small changes cheaper to qualify,
measure actual edited Rust builds, and remove redundant migration embedding.
Preserve test coverage, source provenance, canonical PostgreSQL 18.6, required
checks and protected-main integration. No achieved latency target is claimed.

## Architecture and source of truth

- PROVEN: admitted main is `b20391213cde54511233bd02fae3370e019c7bf4`.
- PROVEN: META policy binding is the repository's existing immutable 3.1.0 binding.
- PROVEN: `GAME_MIGRATOR` already owns the canonical embedded migration chain.
- PROVEN: PG producer emits five archive variants and bare filenames in SHA256SUMS;
  the rerun consumer selected only e2e/lib and expected a leading `./`.
- DERIVED: removing duplicate embedding can reduce compiler work; the speedup is
  UNKNOWN until representative compilation measurements exist.
- UNKNOWN: warm-builder end-to-end improvement, RAM high-water mark and PG 19
  benefit on the accepted workload. Experiments must not change qualification pins.

## High-risk authority/recovery qualification

NOT_APPLICABLE: this first batch changes diagnostic CI and reuses the identical
existing migrator. It does not change a production authority/fence operator,
migration SQL, durable-value rules or accepted merge eligibility. Archive source
identity, integrity, inventory ownership and actual runtime selection have
negative regression coverage in `tools/ci/test_pg_test_rerun.py`.

## Acceptance criteria

- [ ] Rerun every selected PG test exactly once per pass across all five archive
  variants, including legacy single-lib archives, with exact run/head custody.
- [ ] Reject bad checksums, wrong head/workspace, malformed/missing bundles,
  duplicate artifact identities and absent or unexpected actual nextest matches.
- [ ] Replace the 31 duplicate game-server migration embeds with the canonical
  migrator; compile affected targets and qualify migration refusal/history cases.
- [ ] Compare real edited builds with retained Cargo incremental targets and
  empty targets backed by isolated sccache; record source trees and exact patch.
- [ ] Qualify each final PR head through required checks and independent review;
  verify the post-merge full gate before claiming integration complete.
- [ ] Use observed timings, memory and storage to select further build/DAG work.

## Reference parity

NOT_APPLICABLE: build/CI infrastructure; no new player mechanic or reference value.

## Excluded scope

No PostgreSQL version or durability change, skipped assertions, weaker provenance,
protection bypass, paid runner purchase, unrelated PC cleanup or main workflow
pin rotation is part of the initial three branches. Later protected workflow
rotation requires its separately prepared exact audit/activation contract.

## Implementation / findings

One writer per branch. Workers prepare uncommitted changes and release their
worktree; the coordinator reviews, publishes, freezes and qualifies candidates.
Branches: `chatgpt/ci-pg-flow-20261121`,
`chatgpt/ci-migrator-reuse-20261121`, `chatgpt/ci-build-deltas-20261121`.
This card remains active across those PRs and later measured programme work.

Native local Git publication was attempted with a non-mutating dry run and failed
because no GitHub credentials were available. The coordinator selects the bound
META API-native new-candidate route: complete bounded tree, one sole-parent
successor, fresh expected-head lease, one non-force ref movement, live readback,
then fresh freeze/CI/review. This does not preserve any earlier candidate identity.

## Validation

### Focused

- `python -m unittest discover -s tools/ci -p test_pg_test_rerun.py`: pass, 7 tests.
- Existing archive affinity and partition planner regression suites: pass, 9 tests.
- Mocked workflow shell seam: pass for five variants, repeated execution, legacy
  single archive, and rejection before any run when nextest lacks a selected test.
- Baseline producer-format fixture against old consumer: failed as expected,
  exit 2 after checksum verification with no staged archive.
- `python tools/repository/validate_repository_policy.py`: pass, 23 files/76 workflows.
- `git diff --check`: pass before publication.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass, 135 tests.
- `python -m unittest discover -s tools/repository`: pass, 197 tests with Rust
  1.100.0 on PATH, including PG planner, shard and heavy-gate regressions.
- Broad tools/ci suite has a pre-existing failure at
  `test_pg_client_capture_replays.py:93`, reproduced on admitted main.

### Component/integration

Actual hosted Actions, nextest and PostgreSQL execution pending on the candidate.
Rust build/Clippy/tests and representative edited-build measurements pending.

### E2E

The PG rerun is diagnostic and grants no merge qualification. Full canonical PG
and applicable WP5 qualification remain authoritative on the migration-reuse PR.

### Exact-head CI

No final head exists yet. Exact SHAs, runs and dispositions belong to immutable
PR/check evidence after each final authoring commit; this file cannot contain
its own future commit SHA.

## Self-review

Coordinator reviews the full bounded delta and archive/selection failure paths
before publication. Worker summaries do not replace source inspection.

## Independent review

Required on each separately merging PR under OWNER_FUNDED_AI_POLICY.md. Root is
the unique review dispatcher after a live same-head de-duplication check. Pending.

## PR and closeout

Canonical PRs are created after initial candidate publication and linked in #3352.
Programme and ownership remain active. No merge, post-merge result, achieved
speedup or completion is claimed by this authoring record.
