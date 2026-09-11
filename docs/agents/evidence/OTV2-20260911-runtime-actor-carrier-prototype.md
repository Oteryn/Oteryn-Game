# Runtime actor-carrier prototype evidence

Status: **PENDING_HOSTED_QUALIFICATION**

Protected activation baseline: `main@32a055ac9b2c4e69773f1d6cb9746ea412164866` after #564. Active worker: `agent/runtime-actor-carrier-prototype-530`.

## Candidate shape

The candidate is a non-production const-generic Channel-local carrier in `apps/game-server/examples/runtime_actor_carrier_prototype.rs`.

It consumes the real public Foundation types:

- `RuntimeScopeRefV1::Channel`;
- `WorldId`;
- `ChannelId`;
- `ScopeOwnershipGeneration`.

Prototype-local actor identity is fixed-shape only. A one-based `ActorLocalId(u32)` maps directly and immutably to one of `M` slots. There is no `HashMap`, `BTreeMap`, tombstone table, retirement history or independently growing lookup backing. Lookup and removal address exactly one slot. Admission scans the fixed slot array deterministically and reports every slot probe as one insertion work unit.

`ActorSlot` is `#[repr(C)]` with four one-byte fields followed by three `i32` position fields. The exact candidate asserts `size_of::<ActorSlot>() == 16`. Therefore retained slot backing is expected to be `16 * M` bytes for the tested candidate points. All multiplication is checked before the evidence helper accepts the derived byte count.

## Live authority and namespace continuity

`ScopeOwnershipGeneration` itself is the namespace incarnation fence. The externally supplied `NamespaceContinuityGuard` is the single surviving continuity authority, separate from carrier backing and retained across carrier loss. A move-only `AuthorizedNewerScopeGenerationGrant` may advance it only to a strictly newer generation. The prototype intentionally exposes no issuer, no same-generation grant path, and no guard constructor from raw scope/generation facts. Both the newer-generation grant and guard are deliberately neither `Clone` nor `Copy`. Production grant issuance, ownership, persistence, and exact same-generation restore remain explicitly unselected.

`lookup` and `remove` consume a live `&NamespaceContinuityGuard`, not a copied `CurrentOwnerFacts` value. A legitimate outer-generation transition therefore immediately fences the old carrier even if a caller retains the old actor reference or the old carrier object. The executable matrix separately changes only `WorldId`, only `ChannelId`, and only the live outer generation to prove all three fences independently.

Carrier materialization is private to a successful guard claim. Once the guard has initialized one actor-local namespace for a `ScopeOwnershipGeneration`, dropping the `ChannelActorCarrier` does not reset the guard. Same-generation rebootstrap fails closed with `SAME_GENERATION_RECONSTRUCTION_BLOCKED`. Advancing an existing guard requires it to consume an already-authorized grant bound to the same scope and a strictly newer `ScopeOwnershipGeneration`.

This prototype does not implement an exact same-generation durable restore source; its selected safe behavior is fail closed. Production ownership/persistence of the continuity guard remains outside this allocation.

## Tested candidate points

The example tests independently exercise `M = 1, 2, 3, 4`. None is a production maximum. Authority-bearing carrier, guard, grant, and authority-dependent operations exist only in the focused test build. `cargo run --example runtime_actor_carrier_prototype` emits only authority-free physical representation/readback for every point: checked retained bytes, type sizes, and direct-index metadata. It explicitly does not execute or qualify namespace, admission, lookup, removal, or other authority-dependent behavior.

| M | expected retained slot bytes | lookup work | removal work | sparse insert work | full/fragmented insertion worst case | M+1 |
|---:|---:|---:|---:|---:|---:|---|
| 1 | 16 | 1 | 1 | 1 | 1 | reject before mutation |
| 2 | 32 | 1 | 1 | 1 | 2 | reject before mutation |
| 3 | 48 | 1 | 1 | 1 | 3 | reject before mutation |
| 4 | 64 | 1 | 1 | 1 | 4 | reject before mutation |

The authority-bearing `ChannelActorCarrier` is not cloneable. Rollback checks compare a separate, fixed-size, non-authoritative `CarrierStateSnapshot`.

The common `prove_m_boundary<M>()` executable matrix asserts, for every tested M, sparse insertion work=1, exact-M fill, full M+1 denial work=M with complete state preservation, direct lookup work=1, removal work=1, and fragmented-last-slot reinsertion work=M. The separate M=4 boundary test remains an additional concrete check, not the sole evidence for occupancy-dependent work.

The lookup/index entry is physically the same bounded slot resource. Distinct-index bytes are therefore zero rather than silently counted as actor bytes.

## Correctness / authority matrix encoded by tests

The exact example contains focused tests for:

1. exact M success and M+1 capacity denial with complete pre-attempt state preservation for every tested M;
2. live `NamespaceContinuityGuard` validation before local actor lookup/removal;
3. same-Channel `WorldId` mismatch rejection;
4. same-World `ChannelId` mismatch rejection;
5. legitimate live outer-generation transition immediately fencing the old carrier/reference;
6. out-of-range/missing actor ID rejection;
7. valid actor ID rejection while its slot is vacant;
8. removal retaining slot generation, successful reuse advancing it, and well-formed retired/reused references rejecting as `STALE_GENERATION`; malformed, out-of-range, and never-used vacant identities remain `INVALID_REFERENCE`;
9. distinct local IDs remaining bound to distinct slot/generation cells;
10. injected failure after reusable-slot selection and valid successor derivation leaving complete carrier state unchanged;
11. checked local-generation exhaustion producing exactly `ACTOR_LOCAL_GENERATION_EXHAUSTED` in category `CAPACITY_EXCEEDED`, transitioning only the selected vacant max-generation slot to terminal `EXHAUSTED`, and preserving unrelated slots/actors;
12. churn across every configured slot keeping retained generation cells exactly `M` and independent retirement history exactly zero;
13. lookup/insertion/removal work measured for every tested M, including sparse, full and fragmented occupancy boundaries;
14. carrier loss with the surviving guard blocking same-generation reconstruction, no runtime API capable of issuing a replacement same-generation authority from raw facts, and an independently authorized strictly newer outer generation permitting a fresh namespace;
15. actor-ID one-based count/index overflow rejection plus retained-byte multiplication overflow rejection.

## Minimal carried facts

Each occupied slot carries only the allocation-approved fixed facts:

- player-like / creature-like / npc-system-like discriminator;
- occupied/vacant/exhausted lifecycle;
- actionable flag;
- fixed local `x/y/z` position candidate;
- retained actor-local generation.

No inventory, combat values, AI memory, dialogue, loot, persistence, protocol handle, visibility state, timer collection or variable domain payload is added.

## Qualification truth

The current material candidate source commit is bound to the exact GitHub PR head at qualification. The final PR head is intentionally bound by GitHub check/review evidence rather than self-referentially embedded as a PASS claim in this file.

The numeric and authority-dependent operation values above are **candidate expectations encoded as focused-test executable assertions**, not `cargo run` claims and not yet PASS claims. They become qualified only when exact-head repository CI compiles/runs the applicable example tests and all selected repository gates succeed.

## Explicit non-claims

```text
accepted_production_maximum_selected = false
resource_registry_mutated = false
production_capacity_claim = false
ability_508_phase_a_activated = false
movement_139_activated = false
production_runtime_implemented = false
same_generation_durable_restore_implemented = false
```

No result here authorizes a production `ChannelRuntime`, registry serialization, #508 Phase A, #139 Movement, Cargo/workspace mutation, runtime-source mutation or external-repository write.
