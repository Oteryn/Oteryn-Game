# OTV2-20260907-remediation-wp7-ai

```yaml
task_id: OTV2-20260907-remediation-wp7-ai
title: Repair AI perception candidate identity uniqueness
mode: REPAIR
status: implementation_validated
repository: Oteryn/Oteryn-Game
base_branch: main
branch: fix/remediation-wp7-ai-364
issue: 364
pr: 369
admission_main_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
base_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
head_sha: null
final_head_sha: null
owner: WP7_AI_REPAIR
created_at: 2026-09-07
updated_at: 2026-09-07
owned_paths:
  - apps/game-server/src/ai/perception.rs
  - apps/game-server/src/ai/tests.rs
  - docs/agents/tasks/active/OTV2-20260907-remediation-wp7-ai.md
public_contracts: []
depends_on: []
blocks: [WP7_AI_correctness]
external_repositories: []
```

## Outcome

Make CandidateId uniqueness independent of priority/order before any Perception is published, while preserving the existing deterministic valid ranking and registered capacity limit. This repairs the underlying AI source defect only; production reachability and G1 remain separate WP8 evidence.

## Architecture and source of truth

PROVEN on admission main: `canonicalize_perception` sorts by priority then ID and only afterward checks adjacent IDs, so the same ID can evade detection when separated by a differently ranked candidate. Existing focused test covers only a two-element adjacent duplicate case. Exact allocation is #162 comment `5567031784`; fresh open-PR search found no overlapping AI runtime mutation lineage.

## High-risk authority/recovery qualification

NOT_APPLICABLE: no session authority, PREPARE/COMMIT, durable schema/recovery, production/live state or protected control-plane mutation.

## Acceptance criteria

- [x] Duplicate CandidateId is rejected independently of priority and input position before Perception publication.
- [x] At least the six historical priority-separated permutations plus additional head/middle/tail duplicate positions fail closed.
- [x] Unique candidates retain exactly the existing canonical priority-descending/id-ascending order.
- [x] Existing capacity/error behavior is unchanged.
- [ ] Focused tests, Rust 1.94 fmt, strict game-server Clippy/tests and local diff checks pass; exact-head CI remains pending as GitHub-owned readiness evidence.

## Excluded scope

No AI activation/registration, gameplay transport, Foundation/Durability, Ability, Cargo/lock, lib/composition, workflow/protection, production/live data or external-repository changes.

## Validation

- RED: `cargo +1.94.0 test -p oteryn-game-server ai::tests::priority_separated_duplicate_candidate_ids_fail_for_every_input_permutation -- --exact` failed as expected because the priority-separated duplicate returned `Ok(Perception { .. })` instead of `Err(AiError::InvalidInput)`.
- GREEN focused AI suite: `cargo +1.94.0 test -p oteryn-game-server --test ai_bootstrap` passed 13 tests.
- Final local gate: `cargo +1.94.0 fmt --all --check && cargo +1.94.0 clippy -p oteryn-game-server --all-targets --all-features -- -D warnings && cargo +1.94.0 test -p oteryn-game-server && git diff --check` passed in full (364 library tests, 69 migration-binary tests, 13 AI integration tests, 4 authority-invariant tests, 124 durability tests, 17 evidence-shell tests, 15 interaction tests, 2 qualification tests, and 30 doc tests).
- Exact-head GitHub CI is not claimed by this local implementation record and remains pending after publication.

## Context checkpoint

```yaml
last_progress: WP7A uniqueness repair and bounded regression matrix validated locally for publication
status: implementation_validated
branch: fix/remediation-wp7-ai-364
head_sha: null
pr: 369
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
owner_action_required: null
blocker: null
next_action: publish one coherent commit to PR 369 for remediation-lead exact-head inspection
```
