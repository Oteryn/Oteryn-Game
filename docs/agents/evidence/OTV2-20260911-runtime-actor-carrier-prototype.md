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

## Namespace continuity across carrier loss

The current candidate source was materially repaired after independent review to prevent same-generation namespace resurrection.

`NamespaceContinuityGuard` is separate from the carrier backing and is the prototype evidence object that must survive carrier loss/reconstruction. Carrier materialization is private to a successful guard claim; the carrier cannot mint fresh namespace authority from raw scope/generation facts.

Once a guard has initialized one actor-local namespace for a `ScopeOwnershipGeneration`, losing/dropping the `ChannelActorCarrier` does **not** reset the guard. A second bootstrap through that surviving guard fails closed with `SAME_GENERATION_RECONSTRUCTION_BLOCKED`. A fresh namespace becomes available only after the guard consumes an independently authorized, strictly newer `ScopeOwnershipGeneration`. The new carrier then rejects every old exact actor reference on the outer-generation fence before local-slot acceptance.

This prototype does not implement an exact same-generation durable restore source; its selected safe behavior is fail closed. Production ownership/persistence of the continuity guard remains outside this allocation.

## Tested candidate points

The example tests independently exercise `M = 1, 2, 3, 4`. None is a production maximum. `cargo run --example runtime_actor_carrier_prototype` regenerates physical/work rows for all four points.

| M | expected retained slot bytes | lookup work | removal work | sparse insert work | full/fragmented insertion worst case | M+1 |
|---:|---:|---:|---:|---:|---:|---|
| 1 | 16 | 1 | 1 | 1 | 1 | reject before mutation |
| 2 | 32 | 1 | 1 | 1 | 2 | reject before mutation |
| 3 | 48 | 1 | 1 | 1 | 3 | reject before mutation |
| 4 | 64 | 1 | 1 | 1 | 4 | reject before mutation |

The lookup/index entry is physically the same bounded slot resource. Distinct-index bytes are therefore zero rather than silently counted as actor bytes.

## Correctness / authority matrix encoded by tests

The exact example contains focused tests for:

1. exact M success and M+1 capacity denial with complete pre-attempt state preservation for every tested M;
2. independent current-owner `RuntimeScopeRefV1::Channel + ScopeOwnershipGeneration` validation before local actor lookup;
3. cross-scope and stale outer-generation rejection;
4. removal retaining the slot generation, successful reuse advancing it, and stale prior references never resolving;
5. distinct local IDs remaining bound to distinct slot/generation cells;
6. injected failure after reusable-slot selection and valid successor derivation leaving the complete carrier state byte/field-equivalent at the semantic struct level;
7. checked local-generation exhaustion producing exactly `ACTOR_LOCAL_GENERATION_EXHAUSTED` in category `CAPACITY_EXCEEDED`, transitioning only the selected vacant max-generation slot to terminal `EXHAUSTED`, and preserving unrelated slots/actors;
8. churn across every configured slot keeping retained generation cells exactly `M` and independent retirement history exactly zero;
9. insertion work measured at sparse, full and fragmented boundaries; direct lookup/removal remain one-slot work;
10. carrier loss with a surviving `NamespaceContinuityGuard` blocking same-generation reconstruction, followed by a strictly newer outer generation permitting a fresh namespace and rejecting the old reference;
11. count/byte overflow rejecting through checked arithmetic.

## Minimal carried facts

Each occupied slot carries only the allocation-approved fixed facts:

- player-like / creature-like / npc-system-like discriminator;
- occupied/vacant/exhausted lifecycle;
- actionable flag;
- fixed local `x/y/z` position candidate;
- retained actor-local generation.

No inventory, combat values, AI memory, dialogue, loot, persistence, protocol handle, visibility state, timer collection or variable domain payload is added.

## Qualification truth

The current material candidate source repair is commit `1f6f1c271544d34e304785aaf6b3f13f2dd2433b`. The final PR head is intentionally bound by GitHub check/review evidence rather than self-referentially embedded as a PASS claim in this file.

The numeric values above are **candidate expectations encoded as executable assertions**, not yet PASS claims. They become qualified only when exact-head repository CI compiles/runs the applicable example tests and all selected repository gates succeed.

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
