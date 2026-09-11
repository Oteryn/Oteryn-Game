# OTV2-20260911-runtime-actor-carrier-prototype-530

```yaml
task_id: OTV2-20260911-runtime-actor-carrier-prototype-530
title: Prove bounded Channel actor-carrier prototype
mode: IMPLEMENTATION_EVIDENCE
status: validating
repository: Oteryn/Oteryn-Game
issue: 530
coordinator_issue: 162
base_branch: main
branch: agent/runtime-actor-carrier-prototype-530
pr: 568
base_sha: 32a055ac9b2c4e69773f1d6cb9746ea412164866
head_sha: null
final_head_sha: null
owner: OTV2_WORK_COORDINATOR_ACTIVATED_WORKER
created_at: 2026-09-11
updated_at: 2026-09-11
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/examples/runtime_actor_carrier_prototype.rs
  - docs/agents/evidence/OTV2-20260911-runtime-actor-carrier-prototype.json
  - docs/agents/evidence/OTV2-20260911-runtime-actor-carrier-prototype.md
  - docs/agents/tasks/active/OTV2-20260911-runtime-actor-carrier-prototype-530.md
depends_on:
  - issue: 530
  - pr: 564
  - pr: 541
implementation_authority: NON_PRODUCTION_EXAMPLE_ONLY
registry_mutation_authority: NONE
production_authority: NONE
external_repository_write_authority: NONE
```

## Protected activation

PR #564 is protected as `main@32a055ac9b2c4e69773f1d6cb9746ea412164866`. Coordinator #162 performed a fresh custody census and explicitly activated this exact lineage in comment `5632224967`; Issue #530 records the same activation in comment `5632227300`.

No competing worker branch or open implementation PR existed at activation. WP3/WP4/client work is path-disjoint. Everything outside the four owned paths is read-only.

## Outcome

Implement and qualify the smallest executable `CHANNEL_RUNTIME_ACTOR_CARRIER_V1` prototype using the real public Foundation `RuntimeScopeRefV1::Channel`, `WorldId`, `ChannelId`, and `ScopeOwnershipGeneration` types while remaining outside production runtime composition.

The candidate uses a fixed const-generic slot array. `ActorLocalId` maps directly and immutably to one logical slot/generation cell, so there is no separately growing lookup index or tombstone/history store. Admission performs deterministic slot probing; lookup and removal are direct single-slot operations.

`ScopeOwnershipGeneration` itself is the namespace incarnation fence. The externally supplied live `NamespaceContinuityGuard` is the single surviving continuity authority supplied to lookup/removal and kept outside carrier backing. A move-only `AuthorizedNewerScopeGenerationGrant` may advance it only to a strictly newer generation. The prototype exposes no grant issuer, same-generation grant path, or guard constructor from raw facts. The guard survives carrier loss and is neither `Clone` nor `Copy`; same-generation rebootstrap fails closed, while only an independently authorized strictly newer `ScopeOwnershipGeneration` may establish a fresh namespace. The authority-bearing carrier is also non-Clone; rollback checks use a separate non-authoritative fixed-value snapshot. The carrier, continuity guard, newer-generation grant, and all authority-dependent operations are test-only; the non-test executable has no namespace bootstrap path. The prototype does not implement or claim same-generation durable restore or production ownership/persistence of that guard.

## Required acceptance matrix

- [ ] Exact `WorldId + ChannelId + ScopeOwnershipGeneration + ActorLocalId + ActorLocalGeneration` reference shape is exercised.
- [ ] Same-Channel cross-World, same-World cross-Channel and stale live outer-generation cases reject independently before actor acceptance.
- [ ] Lookup/removal consume the live continuity guard rather than a replayable copied owner-generation snapshot.
- [ ] Missing/out-of-range actor IDs and valid-but-vacant slots reject fail-closed.
- [ ] Distinct local IDs cannot alias one generation cell.
- [ ] Removal preserves generation; successful reuse advances generation; well-formed retired/reused refs reject as `STALE_GENERATION`, while malformed/out-of-range/never-used vacant identity rejects as `INVALID_REFERENCE`.
- [ ] Injected post-selection failure preserves complete carrier state.
- [ ] Local-generation exhaustion returns `ACTOR_LOCAL_GENERATION_EXHAUSTED / CAPACITY_EXCEEDED`, marks only the selected slot terminal and preserves unrelated state.
- [ ] The externally supplied `NamespaceContinuityGuard` is the one surviving authority; carrier loss leaves it initialized; no issuer or raw-fact constructor can establish another namespace under that same generation; same-generation reconstruction fails closed; a genuinely newer independently authorized outer generation may initialize a fresh namespace and fences old refs.
- [ ] Churn across every configured slot retains exactly `M` generation cells and zero independent retirement-history entries.
- [ ] Actor-ID one-based count/index overflow and retained-byte arithmetic overflow reject before mutation/allocation evidence.
- [ ] M and M+1 are independently exercised for every tested M.
- [ ] Focused example tests execute exact-M admission, M+1 denial, full-state preservation, lookup, removal, and fragmented reinsertion outcomes for every M=1,2,3,4.
- [ ] `cargo run --example runtime_actor_carrier_prototype` emits only authority-free M=1..4 physical representation/readback (checked retained bytes, type sizes, and direct-index metadata), explicitly without executing or qualifying namespace, admission, lookup, removal, or other authority-dependent behavior.
- [ ] Lookup/removal work and sparse/full/fragmented insertion work are exercised independently for every tested M.
- [ ] Physical slot/index backing is reported honestly; no separate index backing is hidden.
- [ ] No production capacity, registry, #508 Phase A, #139 Movement, Cargo/workspace or runtime-source claim is made.
- [ ] Focused example tests pass on exact head.
- [ ] Repository-selected fmt/Clippy/workspace/Windows/governance gates pass on exact head.
- [ ] Whole-diff adversarial self-review has no unresolved P0/P1/P2.
- [ ] Genuinely independent exact-head review has no unresolved material finding.

## Validation target

Primary focused commands, subject to live protected build-matrix selection:

```text
cargo test --locked -p oteryn-game-server --example runtime_actor_carrier_prototype
cargo run --locked -p oteryn-game-server --example runtime_actor_carrier_prototype
cargo fmt --check
cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
python tools/agents/validate_governance.py
```

No local or hosted result is claimed until observed from the exact candidate. Integration remains native Merge Queue only; direct merge/generic auto-merge/bypass are forbidden.
