# OTV2-20260917-ability-exact-actor-resolution-v1-508

```yaml
task_id: OTV2-20260917-ability-exact-actor-resolution-v1-508
title: Shared read-only exact actor resolver
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/ability-exact-actor-resolution-508-20260924
issue: 508
pr: 837
base_sha: a9c72f5db14f48f428df9b12d200f9566ad36955
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Sol 6 allocation A
created_at: 2026-09-24T12:53:49Z
updated_at: 2026-09-24T12:53:49Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/ability/exact_actor_resolution.rs
  - apps/game-server/src/ability/mod.rs
  - apps/game-server/src/foundation/ability_exact_actor_resolution_tests.rs
  - docs/agents/evidence/OTV2-20260917-ability-exact-actor-resolution-v1.md
  - docs/agents/tasks/active/OTV2-20260917-ability-exact-actor-resolution-v1-508.md
public_contracts: []
depends_on: ["#508", "#573"]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

One preproduction Foundation carrier offers an opaque current-owner exact reference lookup to a single Ability resolver. Client and AI proposals share it. No production composition or grant is added.

## Architecture and source of truth

- `PROVEN`: accepted GAME-ABILITY-01 whole-gate baseline, #508 Phase A and terminal preflight `5712567820` select exact lookup without geometry.
- `PROVEN`: protected #162 allocation `5814505244` assigns this seven-path lease on the recorded base SHA.
- `DERIVED`: existing direct slot/generation lookup bounds this child to one candidate and at most one result under AB-RL-01/02.
- `UNKNOWN`: production actor continuity, ingestion and legality are excluded and require independent future composition.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` to production mutation: this task introduces a read-only lookup and test-only composition; it cannot admit actors, issue current-owner grants, mutate durable state, authorize PREPARE/COMMIT or restore controllers. The test matrix nevertheless changes each identity and current-owner fact independently; see the accompanying evidence file.

## Acceptance criteria

- [x] Current reference resolves; missing, vacant, recycled and cross-scope references reject in focused Foundation tests.
- [x] Client and AI proposal origins share one resolver; resolved output cannot be directly constructed by adapters.
- [x] Exact occurrence/revisions/target bind retry; one candidate and at most one result; no geometry or scanning.
- [x] Focused Rust 1.94 tests, fmt and strict Clippy pass on the repaired candidate.
- [ ] Exact-head protected gate passes after publication.
- [ ] Independent exact-head high-risk review disposition before MQ.

## Excluded scope

No second actor store, identity mint, production actor admission, `lib.rs`, position, Movement, registry, persistence, protocol, geometry, Reference spell or live deployment.

## Implementation / findings

Tests were written first. Foundation exports an opaque crate-visible ref and read-only borrowed current-owner lookup; Ability produces a one-result occurrence-bound snapshot. Focused validation passed on the repaired candidate; exact-head independent review and protected gate await coordinator publication. Evidence: `docs/agents/evidence/OTV2-20260917-ability-exact-actor-resolution-v1.md`.

## Validation

- Focused RED: authored first; not executed because local cargo/rustc are unavailable.
- Focused GREEN: three Foundation exact-actor tests and ten existing standalone Ability integration tests passed under isolated Linux Rust 1.94.
- Repaired candidate: three Foundation exact-actor tests and ten standalone Ability integration tests passed again under isolated Linux Rust 1.94. Strict Clippy (`cargo +1.94.0 clippy --locked -p oteryn-game-server --lib --tests -- -D warnings`) and `cargo +1.94.0 fmt --all -- --check` passed, process exit 0. Independent review's crate-visible trait finding was fixed by a concrete Foundation ref/lookup API; new exact-head review remains pending publication.
- Whole-diff self-review: complete, seven owned paths only, `git diff --check` clean; see evidence file.
- Protected exact-head gate and independent review: pending publication and freeze.
