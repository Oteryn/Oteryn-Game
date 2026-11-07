# Runtime actor carrier resource evidence — #530

## Disposition

`EVIDENCE_RESULT: COMPLETE_WITH_NARROW_BLOCKERS`

This packet is the public-safe result of the activated evidence-only worker `OTV2-RUNTIME-ACTOR-CARRIER-RESOURCE-EVIDENCE-530`.

Protected admission and authority:

- protected `main@951f98746e2fb8b93be0a4f04518b74e266df188`;
- protected allocation blob `0b81a498f1553e61fca89eebd85a23df080e428b`;
- #162 activation comment `5619094154`;
- #532 real merge-group run `34478991801` — SUCCESS;
- no runtime, registry, Cargo/workflow, production, Movement/#139, Ability/#508 or InstanceRuntime mutation authority.

## Question answered

#508 already established that the semantic ingredients of an exact actor reference are not the architectural blocker. The missing production prerequisite is a physical shared runtime-owned carrier/current-owner resolver. #530 asks which finite resources appear when that carrier is made physical without borrowing unrelated Ability, AI, Movement or Content limits.

This worker does **not** select the production carrier. It compares synthetic shapes only to determine whether a resource can honestly be classified as the same resource, independently growing, not exercised or still evidence-gated.

## Required identity semantics

Every positive reference in the harness binds:

```text
WorldId
+ ChannelId
+ current ScopeOwnershipGeneration / owner fence
+ actor semantic/local identity
+ actor local generation
```

`WorldId` and `ChannelId` are distinct and inseparable for the modeled Channel scope. Same `ChannelId` under another `WorldId` rejects. A stale scope ownership generation rejects before actor state is exposed. Actor-local generation remains inseparable from the exact reference. AI-local/client/protocol-like scalar handles are not accepted as authority.

## Candidate models

Two candidate physical shapes are deliberately retained as **evidence models**, not product selections.

### Fused fixed bucket

One fixed bucket carries the modeled actor record, exact lookup location and retained generation/retirement state. Logical bucket width is explicitly 64 bytes. The table size is a deterministic power-of-two candidate derived from a tested synthetic active ceiling.

### Split record + fixed index

The actor record and lookup index are accounted separately: 56 logical bytes per active record and 24 logical bytes per fixed index entry. A separate 16-byte logical generation-history entry is listed only to make any independently retained history visible instead of hiding it inside active actor count.

Neither shape is a decision for a Rust type, ECS, hash map, vector, slab, allocator, protocol handle, persistence schema or module location.

## Measurement semantics

All byte values below are explicit fixed-width **logical accounting**. They are not `sys.getsizeof`, Python RSS, Rust ABI size, allocator overhead, cache footprint or a production memory benchmark. No wall-time result is used as capacity evidence.

Synthetic active ceilings `2, 3, 8, 64, 256` are tested points only. They are not proposed production limits and they deliberately include the existing AI number 256 only as a regression point proving that an adjacent limit cannot silently become Channel authority.

| tested M | buckets | fused logical bytes | split record+index bytes | observed max exact probes | structural probe upper bound |
|---:|---:|---:|---:|---:|---:|
| 2 | 4 | 256 | 208 | 1 | 4 |
| 3 | 8 | 512 | 360 | 1 | 8 |
| 8 | 16 | 1,024 | 832 | 2 | 16 |
| 64 | 128 | 8,192 | 6,656 | 7 | 128 |
| 256 | 512 | 32,768 | 26,624 | 10 | 512 |

For every tested M, M+1 admission rejects before partial actor/index state becomes visible and preserves every already admitted actor.

The exact lookup materializes no variable candidate collection and performs no world scan, nearest-N search, visibility query, range/LoS calculation, pathfinding or dynamic retargeting. Probe work is bounded by the fixed candidate table for the synthetic shape and therefore does not establish an independent variable-query collection.

## Mixed actor fixture and first functional lower bound

The measurement fixture cycles player-like, creature-like and NPC/system-like records so AI-only occupancy cannot define total Channel store semantics.

The first Reference functional lower bounds are intentionally much smaller than a production capacity claim:

- first exact-target component: 2 simultaneous actors (source + target);
- first static local-step component: 1 current actor;
- mixed-kind structural measurement: at least 3 records to represent player/creature/NPC-system kinds.

These are correctness lower bounds only.

## RL-03 independent-growth proof

The most important result is that actor generation/history cannot automatically be collapsed into the active actor count.

The fused model retains retired opaque local identities so that a later use of the same local identity can advance and validate actor generation. Reusing the *same* identity 1,000 times advances generation in the same retained slot with no retained-byte growth. In contrast, retiring a sequence of unique opaque identities consumes retired slots while the active count returns to zero after every removal.

Observed deterministic history exhaustion:

| active ceiling M | fixed buckets | unique retirements before history exhaustion | active at exhaustion | retired at exhaustion |
|---:|---:|---:|---:|---:|
| 1 | 2 | 2 | 0 | 2 |
| 2 | 4 | 4 | 0 | 4 |
| 3 | 8 | 8 | 0 | 8 |
| 8 | 16 | 16 | 0 | 16 |

This is not proof that production should retain tombstones forever. It proves the opposite: **production needs an accepted rule** for actor-local identity retirement/reuse and generation-retention/exhaustion before it can claim that RL-03 is either the same finite resource as RL-01 or a separately bounded finite resource.

A vector index or pointer cannot be used as the semantic identity simply to avoid this question; #530 explicitly forbids those authority shortcuts.

## Resource-row classifications

### RUNTIME-ACTOR-RL-01 — `PERF_REFERENCE_CELL_REQUIRED`

Candidate retained logical bytes, direct lookup work and M/M+1 behavior are reproducible. A production total-actor ceiling is not.

ADR-0009 explicitly defers `max_players_per_channel`, GameNode and world capacity to reproducible representative `PERF-01` evidence on named reference hardware/artifacts. The required workloads include movement/hunting/AI/combat/loot/pathfinding, crowded interest sets, mass combat, raids, reconnect storms, durable transaction pressure, multi-channel noisy neighbors, degraded/recovery pressure and soak/memory growth. Therefore no tested M from this harness may be promoted to a production maximum.

### RUNTIME-ACTOR-RL-02 — `MEASURED_CANDIDATE_EVIDENCE_AVAILABLE`

Physical shape matters. The fused candidate makes lookup location and actor/generation state one retained fixed table. The split candidate has a separately accountable fixed index. Both are structurally measurable, so a universal `SAME_RESOURCE_AS_RL01` claim would be premature before the production carrier representation is selected.

### RUNTIME-ACTOR-RL-03 — `ARCHITECTURE_ESCALATION_REQUIRED`

Unique opaque identity churn demonstrates a resource that can exhaust independently while active actor count is zero. Current protected semantics require stale-generation rejection but do not yet select actor-local identity retirement/reuse, generation retention horizon/representation or the terminal behavior when that namespace/history cannot safely advance.

The next architecture work should be narrow: settle that identity lifecycle/exhaustion rule without selecting a general ECS/container and without weakening stale-handle safety.

### RUNTIME-ACTOR-RL-04 — `NOT_EXERCISED_BY_FIRST_CARRIER`

The modeled exact lookup materializes no variable candidate list and no geometry/spatial scan. Any fixed-table probing is physical lookup work accounted with the chosen carrier/index shape, not a new Ability-style candidate collection. Existing Ability target occurrence ceilings remain separate.

### RUNTIME-ACTOR-RL-05 — `NOT_EXERCISED_BY_FIRST_CARRIER`

The candidate record is fixed-shape. AI memory, combat state, inventory, loot, dialogue, behavior graphs and other variable payloads stay with their owning domains. No variable per-actor payload is necessary for this first exact-ref/local-position evidence boundary.

## Deterministic negative evidence

`self_test.py` covers 14 focused cases and the generated JSON records the corresponding PASS results:

1. matching World/Channel/scope generation/actor generation resolves;
2. missing actor rejects;
3. stale actor generation rejects after removal/recycle;
4. same local identity with a newer generation rejects the old ref;
5. different Channel rejects;
6. same ChannelId under a different WorldId rejects;
7. stale/mismatched `ScopeOwnershipGeneration` rejects with carrier state unchanged;
8. M+1 admission rejects before partial state;
9. capacity rejection preserves existing actor resolutions;
10. checked u64 count/byte arithmetic rejects overflow;
11. lookup result is independent of insertion/enumeration order;
12. 1,000 same-identity recycle cycles do not grow retained storage;
13. unique-identity churn exposes independent retained-history exhaustion;
14. untyped scalar handle is rejected and the exact lookup creates no variable target collection.

Focused local result while authoring these exact bytes:

```text
PASS 14 tests
PASS evidence JSON matches deterministic generator
```

The deterministic generator is intentionally able to reproduce the checked JSON without inserting a self-referential commit SHA.

## Evidence classification

```yaml
RUNTIME-ACTOR-RL-01: PERF_REFERENCE_CELL_REQUIRED
RUNTIME-ACTOR-RL-02: MEASURED_CANDIDATE_EVIDENCE_AVAILABLE
RUNTIME-ACTOR-RL-03: ARCHITECTURE_ESCALATION_REQUIRED
RUNTIME-ACTOR-RL-04: NOT_EXERCISED_BY_FIRST_CARRIER
RUNTIME-ACTOR-RL-05: NOT_EXERCISED_BY_FIRST_CARRIER
first_reference_functional_lower_bound:
  exact_target_active_actors: 2
  local_step_active_actors: 1
  production_capacity_claim: false
accepted_production_maximum_selected: false
perf_reference_cell_required: true
```

## Blockers and next action

Primary blocker: `RUNTIME-ACTOR-RL-03` needs an accepted actor-local identity retirement/reuse and generation-retention/exhaustion disposition before production can prove same-resource identity or register a separate finite limit.

Independent capacity blocker: `RUNTIME-ACTOR-RL-01` still requires an accepted representative `PERF-01` reference cell before selecting a production Channel total-actor ceiling.

Recommended #530 disposition:

> Keep #139 and #508 unactivated. Resolve the narrow actor-local identity retirement/reuse plus generation-retention/exhaustion rule without selecting a generic ECS/container; then obtain the representative PERF-01 capacity cell required by ADR-0009 before registry serialization and shared carrier implementation allocation.

This result does not activate Movement, Ability, runtime, registry or production behavior.
