# FIRST_PRODUCTION_CONTENT_PROFILE/v1 — Amendment 01: spawn population hard bounds

- Date: 2026-09-09
- Tracking: Issue #433 / PR #462 / CONTENT #54.
- Applies to: `OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md`.
- Normative status: **part of the same `FIRST_PRODUCTION_CONTENT_PROFILE/v1` architecture decision**. This amendment closes the exact-head P1 finding on PR #462 that the base decision bounded spawn-definition count but not the actor population controlled by the spawn record.
- Authority: identical to the base decision. Architecture implementation authority is bounded; repository integration remains normal reviewed PR + required checks + FULL Merge Queue; **live deployment / production activation authority remains NONE**.

## 1. Evidence and why the accepted maximum is 1

Fresh protected `main@10ce3393a51dac14105b831040e1f4faa3ca565f` proves all of the following:

- `apps/game-server/src/content/model.rs` exposes `SpawnDefinition::fixture_population_limit: u16` as content-controlled data;
- `apps/game-server/src/content/compiler.rs` serializes that population field into the spawn artifact record;
- VSL-CONTENT-01 §13 requires population/placement bounds to fail closed;
- `FIRST_PRODUCTION_CONTENT_PROFILE/v1` already permits exactly one spawn and exactly one creature definition and intentionally claims only the smallest production-capable movement/combat/value-source slice;
- no accepted broad-world or production population sizing evidence justifies a value greater than one.

Therefore the first production profile accepts **exactly one concurrent content-spawned actor for its one allowed spawn**. This is not copied from an evidence/test ceiling and is not a throughput/capacity estimate. It is the smallest positive population that can actually exercise the required production spawn/value-source seam. Zero would make that seam unreachable; any value greater than one would widen runtime actor allocation/work without accepted production evidence.

Because the profile has exactly one spawn, the mechanically derived aggregate content-spawned population ceiling for one authoritative scope is also **1 actor**.

Any future requirement for population >1, multiple spawns, encounter packs, density scaling or broad-world spawn capacity requires a fresh accepted sizing/architecture decision and corresponding registry update before implementation.

## 2. Normative corrections to the base decision

The base decision remains normative except for the additions below. Any statement that its resource inventory was complete is to be read as complete **with this amendment included**.

### 2.1 Compiler/build IN scope addition

The single allowed production spawn has:

- `population_limit = 1` concurrent content-spawned actor;
- aggregate content-spawned population per authoritative scope = 1 actor;
- no config, artifact capability, runtime default or omitted field may increase either value.

The production source type must not preserve the fixture name as authority-bearing semantics. It must expose a production `population_limit` field (or an equivalently explicit production-only typed field) and validate it before artifact lowering.

### 2.2 `REQUIRED_NOW` additions

These two resource dimensions are part of §5.1 of the base decision:

| Resource dimension | Unit | Hard maximum | Evidence / derivation | Mandatory boundary behavior |
|---|---:|---:|---|---|
| concurrent population for one spawn | content-spawned actors per spawn | 1 | minimum positive population required to exercise the one-spawn production slice; no accepted evidence supports >1 | `1` accepted; `0` rejected as semantically incomplete; `2` / max+1 rejected before artifact lowering or runtime actor allocation |
| aggregate content-spawned population in one authoritative scope | content-spawned actors per authoritative scope | 1 | exactly one allowed spawn × per-spawn population maximum 1 | cumulative `1` accepted; cumulative `2` rejected before staging/runtime actor allocation; checked aggregate arithmetic required |

Both violations fail closed before actor/container/task allocation or authoritative activation. They are capacity/profile errors, not values that may be clamped, truncated, defaulted or silently reinterpreted.

### 2.3 `EXCLUDED_FAIL_CLOSED` / deferred clarification

The following is unreachable in v1 and therefore cannot be enabled by input/config/artifact:

- spawn population >1;
- a second spawn contributing population;
- encounter-pack/density multipliers or population scaling;
- per-region/per-cell population expansion beyond the single explicit actor.

Broad-world spawn density/population sizing is `DEFERRED_REQUIRES_FUTURE_DECISION`. Trigger: the first accepted requirement for >1 concurrent content-spawned actor, multiple production spawns, encounter packs, or representative broad-world spawn sizing.

## 3. Exact serialized `RESOURCE_LIMITS_REGISTRY.json` additions

When the base decision is protected and the separate serialized registry PR is prepared, the exact array from base §6 must include the following two objects immediately after `DUR04-FIRST-PROD-SPAWNS`. No other interpretation or owner choice is required.

```json
[
  {
    "id": "DUR04-FIRST-PROD-SPAWN-POPULATION-PER-SPAWN",
    "owner_contract": "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md+AMENDMENT_01",
    "resource": "Concurrent content-spawned population for one FIRST_PRODUCTION_CONTENT_PROFILE/v1 spawn",
    "unit": "content-spawned actors per spawn",
    "hard_maximum": 1,
    "configurable_range": {"minimum": 1, "maximum": 1},
    "failure_category": "CAPACITY_EXCEEDED",
    "allocation_impact": "Validate the production spawn population before artifact lowering or any runtime actor/container/task allocation; never clamp/default a value outside the profile.",
    "client_visible": false,
    "boundary_tests": [
      "population 1 accepted for the single semantically valid production spawn",
      "population 0 rejected as semantically incomplete",
      "population 2 rejected before artifact lowering/runtime actor allocation"
    ]
  },
  {
    "id": "DUR04-FIRST-PROD-SPAWN-POPULATION-PER-SCOPE",
    "owner_contract": "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md+AMENDMENT_01",
    "resource": "Aggregate concurrent content-spawned population in one authoritative scope",
    "unit": "content-spawned actors per authoritative scope",
    "hard_maximum": 1,
    "configurable_range": {"minimum": 1, "maximum": 1},
    "failure_category": "CAPACITY_EXCEEDED",
    "allocation_impact": "Use checked aggregate arithmetic across all accepted spawn records before staging/runtime actor allocation; the first profile has exactly one spawn and aggregate population 1.",
    "client_visible": false,
    "boundary_tests": [
      "aggregate population 1 accepted",
      "aggregate population 2 rejected before staging/runtime actor allocation",
      "aggregate population checked-add overflow rejected"
    ]
  }
]
```

The registry serialization remains a separate serial PR after protected readback of the complete #433 decision. This amendment does not itself mutate `RESOURCE_LIMITS_REGISTRY.json`.

## 4. Exact CONTENT #54 allocation/test additions

The post-registry CONTENT #54 allocation from base §12 additionally requires:

- production source/model symbol for `population_limit` with accepted value exactly `1`;
- compile-time checked aggregate population calculation before lowering;
- staging/runtime revalidation that the artifact cannot claim population >1 or aggregate >1;
- no compatibility path that interprets `fixture_population_limit` from evidence/test profiles as production authority.

Mandatory focused tests added to the base list:

21. production spawn population `1` is accepted; `0` is rejected as semantically incomplete; `2` is rejected before artifact lowering/runtime actor allocation;
22. aggregate per-scope population `1` is accepted; aggregate `2` and checked-add overflow are rejected before staging/runtime actor allocation; evidence/test fixture population never becomes production by profile renaming.

## 5. Activation/recovery/coexistence/security impact

No activation, rollback, restart, coexistence or authority semantic from the base decision changes.

The population limits are validated before a candidate can become `staged` and are revalidated as part of profile/semantic validation. Therefore malformed or oversize population input cannot reach the atomic activation commit point. A failed population check leaves the current active generation unchanged.

On restart or rollback, the exact primary/fallback artifact pair must pass the same population limits again; an older or otherwise authorized digest does not bypass them.

Only one authoritative generation remains possible, and that generation can create at most one content-spawned actor from the first-profile content graph. Runtime/player/system actor budgets outside content-spawned population remain owned by their respective runtime contracts and are not silently redefined here.

## 6. Criterion-13 state

This amendment moves PR #462 to a new exact head. All previous exact-head review evidence for `a1b3fea3...` is historical and cannot satisfy #433 criterion 13 after this commit.

Before #433 can become `completed`, the **new exact PR head** must again pass:

1. independent architecture/security review with P0=0, P1=0, P2=0;
2. applicable canonical CI/governance;
3. normal FULL Merge Queue;
4. protected-main readback.

Until then #433 remains open.