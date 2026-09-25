# Runtime actor first-slice bound application decision

- Decision: `RUNTIME-ACTOR-FIRST-SLICE-NONPRODUCTION-BOUND-V1`
- Status: **CANDIDATE; acceptance requires exact-head validation, independent review and protected integration**
- Source escalation: Jira `KAN-28`
- Owner direction: Issue #162 comment `5837353246`
- Admission baseline: `main@1ecdafb91f238bbdee691c08e25208d1dfef36d5`
- Extends: `RUNTIME-ACTOR-CAPACITY-DEFERRED-UNTIL-MEASURABLE-V1` from protected PR #570
- Related: Issues #162 and #530; Jira `KAN-27` and `KAN-26`
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Resolution packet

```yaml
classification: ARCHITECTURE_RESOLUTION
repository: Oteryn/Oteryn-Game
main_sha: 1ecdafb91f238bbdee691c08e25208d1dfef36d5
blocking_question: >-
  May CHANNEL_RUNTIME_COMPOSITION_V1 proceed with the owner-accepted finite
  first-slice bound M=131072 without serializing that number into
  RESOURCE_LIMITS_REGISTRY v1 as an ambiguous production hard maximum?
facts:
  proven:
    - "Protected #570 permits finite fail-closed pre-production implementation before production capacity is measurable."
    - "Protected #570 forbids publishing a development/test bound as production actor capacity and keeps RESOURCE_LIMITS_REGISTRY production serialization blocked until representative measurement and separate acceptance."
    - "RESOURCE_LIMITS_REGISTRY schema v1 requires an absolute numeric hard_maximum and has no machine-enforced applicability, environment or qualification discriminator."
    - "Issue #530 accepts 131072 actors/Channel only as a conservative first-slice policy bound, not as measured production capacity."
    - "PR #899 integrated O(1) actor admission and measured the pre-production carrier at M=131072 without selecting production M."
  derived:
    - "Writing 131072 into registry v1 now would create an ambiguous machine-readable hard maximum that current consumers cannot prove is non-production."
    - "The smallest safe continuation is to keep 131072 outside registry v1 and inject it only into the explicitly non-production first-slice composition."
  unknown:
    - "Representative production workload and measured saturation."
    - "Accepted production VPS fingerprint and final actor admission capacity."
    - "Future registry representation, if any, for environment-qualified non-production limits."
accepted_decision: RUNTIME-ACTOR-FIRST-SLICE-NONPRODUCTION-BOUND-V1
rejected_options:
  - serialize_131072_into_registry_v1_with_notes_only
  - treat_131072_as_production_capacity
  - expand_registry_schema_now_only_to_unblock_the_first_slice
  - block_all_preproduction_channel_composition_until_production_measurement
sequencing:
  KAN-27: DEFERRED_UNTIL_PRODUCTION_CAPACITY_ACCEPTANCE
  KAN-26: MAY_REALLOCATE_AFTER_PROTECTED_READBACK_AND_FRESH_OWNERSHIP_CUSTODY_OVERLAP_RECONCILIATION
production_authority_changed: false
registry_schema_changed: false
resource_values_changed_for_production: false
```

## Decision

The owner-accepted value `M = 131072 actors/Channel` is authorized only as an
explicit finite **PREPRODUCTION_FIRST_SLICE** implementation-safety bound for the
next `CHANNEL_RUNTIME_COMPOSITION_V1` allocation.

It is not:

- a measured production capacity;
- a production default;
- a deployment or readiness value;
- a `RESOURCE_LIMITS_REGISTRY` entry;
- evidence that ADR-0009 headroom or PERF-01 production qualification is complete.

The current `RESOURCE_LIMITS_REGISTRY.json` schema v1 remains unchanged. The
registry requires a numeric `hard_maximum` and does not carry a machine-enforced
applicability/qualification discriminator. Therefore explanatory `notes` are
insufficient to make `131072` safe there while protected #570 requires
production capacity to remain fail closed.

## KAN-27 disposition

KAN-27 is superseded only in sequencing, not in eventual purpose.

Do not serialize `RUNTIME-ACTOR-RL-01..05` into the current registry as part of
the first-slice unblock. Reopen the production registry step only after
representative measurement plus a separately reviewed capacity acceptance, or
after a separately accepted registry contract explicitly introduces a
machine-enforced applicability model needed by a real consumer.

No registry mutation is required by this decision.

## KAN-26 release boundary

After this decision reaches protected `main`, #162 must first perform the fresh
ownership, writable-custody and path/lease-overlap reconciliation required by
protected #570. Only after that reconciliation is clean may #162 freshly allocate
`CHANNEL_RUNTIME_COMPOSITION_V1` with `131072` as its explicit finite
non-production bound.

The implementation allocation must preserve all of the following:

1. The bound is supplied only through an explicitly non-production
   composition/configuration boundary; it is not read from or exported as a
   production capacity registry value.
2. There is no fallback that turns the first-slice value into a production
   default when an accepted production capacity is absent.
3. Any production/readiness path that requires actor capacity remains fail
   closed until the later production capacity decision is accepted.
4. Admission at `M` succeeds only when all other accepted bounds permit it;
   `M+1` rejects before partial authoritative actor/index mutation.
5. RL-02 and RL-03 remain the same physical bounded slot resource when the
   implementation keeps lookup/generation in the accepted fixed-slot carrier;
   no second actor registry is introduced.
6. RL-04 remains direct exact O(1) lookup after protected #899; no scan or
   variable candidate collection is introduced by this decision.
7. RL-05 remains outside multi-creature production admission; broaden it only
   under a later exact allocation/evidence decision.
8. Movement, Ability, Server Seam reconnect, production deployment and final
   production capacity remain outside this release.

The exact Rust/config representation is intentionally not frozen here. KAN-26
must use the smallest representation that makes the non-production applicability
unambiguous and testable without creating a second capacity authority.

## Relationship to protected #570

This decision does not weaken or replace
`RUNTIME-ACTOR-CAPACITY-DEFERRED-UNTIL-MEASURABLE-V1`.

It applies that protected decision to the later owner-selected first-slice value:

```text
finite pre-production implementation bound = 131072
production capacity = UNKNOWN / FAIL_CLOSED
registry production serialization = DEFERRED
```

The separation is deliberate. A finite development bound exists so the real
runtime can be composed and measured; production remains blocked until the
representative measurement and separate acceptance required by #570.

## Validation and integration

Before this decision can release KAN-26:

- exact changed-path review must show this architecture document only;
- Agent Governance and Architecture Semantic Audit must succeed on the frozen
  candidate;
- Merge Gate and aggregate `game-gate` must succeed on the frozen candidate;
- one independent exact-head architecture/resource-authority review must report
  no unresolved material finding;
- integration must use the governed Merge Queue;
- a real `merge_group` `game-gate` must succeed;
- protected-main readback must contain this exact decision, and #162 must then
  complete fresh ownership, writable-custody and overlap reconciliation before it
  issues a fresh KAN-26 runtime source lease.

No direct merge, generic auto-merge, bypass, force/rebase or production action is
authorized by this decision.
