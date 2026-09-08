# OTV2-20260907-remediation-wp7-ai

```yaml
task_id: OTV2-20260907-remediation-wp7-ai
title: Repair AI perception candidate identity uniqueness
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 364
pr: 369
admission_main_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
base_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
head_sha: a11f9a0f836aff57986e4391f516b2ff8ccfef02
final_head_sha: a11f9a0f836aff57986e4391f516b2ff8ccfef02
owner: WP7_AI_REPAIR
created_at: 2026-09-07
updated_at: 2026-09-07
owned_paths: []
public_contracts: []
depends_on: []
blocks: []
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
- [x] Focused tests, Rust 1.94 fmt, strict game-server Clippy/tests and local diff checks pass; exact-head CI and Merge Queue passed.

## Excluded scope

No AI activation/registration, gameplay transport, Foundation/Durability, Ability, Cargo/lock, lib/composition, workflow/protection, production/live data or external-repository changes.

## Validation

- RED: `cargo +1.94.0 test -p oteryn-game-server ai::tests::priority_separated_duplicate_candidate_ids_fail_for_every_input_permutation -- --exact` failed as expected because the priority-separated duplicate returned `Ok(Perception { .. })` instead of `Err(AiError::InvalidInput)`.
- GREEN focused AI suite: `cargo +1.94.0 test -p oteryn-game-server --test ai_bootstrap` passed 13 tests.
- Final local gate: `cargo +1.94.0 fmt --all --check && cargo +1.94.0 clippy -p oteryn-game-server --all-targets --all-features -- -D warnings && cargo +1.94.0 test -p oteryn-game-server && git diff --check` passed in full (364 library tests, 69 migration-binary tests, 13 AI integration tests, 4 authority-invariant tests, 124 durability tests, 17 evidence-shell tests, 15 interaction tests, 2 qualification tests, and 30 doc tests).
- Exact delivery head `a11f9a0f836aff57986e4391f516b2ff8ccfef02`: Merge gate `34102713814`, Architecture semantic audit `34102713829` and Agent governance `34102713887` passed.
- Full Merge Queue run `34103268048` passed; PR #369 squash-merged as `71688357934c7e2dbc4515801dc019a26f1be01d` and the exact three-file result was read back from protected `main` (subsequently `728f25461d5a2b029ed60f7db4b14151d31776d7`). Durable evidence: PR #369 comment `5568189049`.

## Self-review and closeout

- Exact delivery head: `a11f9a0f836aff57986e4391f516b2ff8ccfef02`.
- Full changed-file and effective-diff review: PASS; zero open material findings, no unresolved review threads and no scope outside the three allocated files.
- Independent review: NOT_REQUIRED under the META-owned policy for this bounded local AI-kernel repair; exact-head repository gates and Merge Queue passed.
- Ownership: released after protected-main readback; the implementation branch has no continuing provenance role.
- Branch disposition: merged task branch deleted; live matching-ref readback returned no branch.

## Context checkpoint

```yaml
last_progress: PR 369 integrated through successful Merge Queue and read back from protected main; task archived and ownership released
status: completed
branch: null
head_sha: a11f9a0f836aff57986e4391f516b2ff8ccfef02
final_head_sha: a11f9a0f836aff57986e4391f516b2ff8ccfef02
pr: 369
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
owner_action_required: null
blocker: null
next_action: null
```
