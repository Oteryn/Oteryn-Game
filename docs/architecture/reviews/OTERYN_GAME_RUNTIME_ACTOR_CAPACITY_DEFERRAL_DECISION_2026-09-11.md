# Runtime actor capacity deferred until measurable implementation

- Decision: `RUNTIME-ACTOR-CAPACITY-DEFERRED-UNTIL-MEASURABLE-V1`
- Status: **CANDIDATE; acceptance requires independent exact-head review and protected integration**
- Source escalation: Issue #540; explicit owner direction in comment `5635836468`
- Coordination handoff: Issue #162 comment `5635840410`
- Protected decision baseline: `main@90a3f92434e32354ff1aaeac96d038bbc49eba9c` after PR #568
- `MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`

## Resolution packet

```yaml
classification: ARCHITECTURE_RESOLUTION
repository: Oteryn/Oteryn-Game
main_sha: 90a3f92434e32354ff1aaeac96d038bbc49eba9c
source_escalation: 540
blocking_question: >-
  May the smallest bounded non-production shared Channel actor carrier proceed
  before production hardware and RUNTIME-ACTOR-RL-01 are measurable, without
  inventing or publishing a production capacity?
facts:
  proven:
    - "The owner designates Synology only as the current non-production development, integration and measurement environment."
    - "The owner designates VPS as the production target class while deferring provider, SKU, resources and topology until representative measurements exist."
    - "Protected PR #568 supplies a bounded non-production actor-carrier prototype and explicitly selects no production M."
    - "ADR-0009 requires named environment and exact-artifact evidence and at least 30 percent headroom below measured saturation for production admission."
    - "RUNTIME-ACTOR-RL-01 remains PERF_REFERENCE_CELL_REQUIRED."
  derived:
    - "A finite fail-closed development bound can permit the representative implementation needed to measure capacity without asserting that the bound is production capacity."
    - "Production qualification can remain closed independently of bounded pre-production implementation."
  unknown:
    - "Exact Synology fingerprint until a measurement is run."
    - "Production VPS provider, SKU, CPU, RAM, storage, network, process/container allocation and topology."
    - "Accepted representative workload, numeric service objectives, measured saturation and production admission capacity."
  conflict: []
accepted_decision: RUNTIME-ACTOR-CAPACITY-DEFERRED-UNTIL-MEASURABLE-V1, conditional on protected integration
rejected_options:
  - block_all_preproduction_carrier_work_until_production_capacity_is_selected
  - promote_a_test_or_development_M_to_production_capacity
  - promote_early_synology_results_to_production_capacity
  - guess_a_vps_class_or_production_actor_maximum
  - create_a_second_actor_registry
affected_contracts:
  - ADR-0009
  - PERF-01
  - FND-03
  - RUNTIME-ACTOR-RL-01
  - CHANNEL_RUNTIME_ACTOR_CARRIER_V1
  - Issue #508 Phase A exact-target boundary
  - Issue #139 bounded Movement re-evaluation boundary
affected_paths:
  - docs/architecture/reviews/OTERYN_GAME_RUNTIME_ACTOR_CAPACITY_DEFERRAL_DECISION_2026-09-11.md
implementation_owner: "future #162-allocated pre-production shared Channel carrier worker"
implementation_scope: >-
  The smallest finite, fail-closed, non-production shared Channel carrier and
  exact-lookup slice needed to produce a representative measurable server;
  this architecture decision grants no implementation authority.
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
supersedes:
  - "Only prior sequencing language that made a production PERF-01 actor maximum a prerequisite to every executable pre-production shared-carrier slice."
required_validation:
  - finite_preproduction_bound_and_bound_plus_one_fail_closed
  - development_bound_cannot_be_read_or_exported_as_production_capacity
  - production_mode_rejects_without_an_accepted_production_capacity
  - complete_measurement_fingerprint_and_required_metric_schema
  - no_second_actor_registry
  - exact_target_lookup_has_no_geometry_or_movement_authority
required_independent_review: >-
  YES — exact-head architecture, resource-authority, production-fail-closed and
  #508/#139 boundary review; this author may not merge or enqueue the candidate.
next_action: >-
  Independently review this exact candidate and, if clean, return it to the
  unique #162 control plane for protected integration.
```

## Decision and scope

The production capacity decision is deliberately deferred until a representative
server implementation can be measured. This deferral is not a reason to prevent
the minimum implementation needed to create that evidence. It instead creates two
separate gates:

1. **Bounded pre-production implementation.** A later, explicit #162 allocation
   may authorize the smallest shared Channel actor carrier and exact lookup needed
   for a representative server. Every such allocation must use a finite test/dev
   configuration or an equivalent accepted finite mechanism, reject overflow and
   bound-plus-one before partial authoritative mutation, and remain fail closed.
2. **Production capacity acceptance.** Production qualification, production
   admission and any production capacity publication remain blocked until the
   measurement and acceptance requirements below are complete.

The first gate does not satisfy, weaken or bypass the second. A pre-production
bound is an implementation-safety input, not a capacity statement, promise, SLO,
deployment default or candidate value for production.

This candidate does not activate either gate as implementation work. It changes no
runtime, registry, Cargo/workflow, production configuration or hardware and grants
no production authority.

## Environment and evidence classification

Synology is the current **non-production** development, integration and
measurement environment only. Its model, CPU, RAM and other hardware properties
are not assumed by this decision. An exact sanitized fingerprint is captured when
a measurement is actually run. Early Synology measurements, like synthetic/test
values of `M`, are correctness, regression and development evidence only unless a
later accepted capacity methodology expressly qualifies them for a broader use.

Production targets a VPS, but this decision selects no provider, SKU, CPU class or
count, RAM, storage, network parameters, process/container allocation, topology or
cost/performance class. Candidate VPS classes and the final admission capacity are
selected later from measured evidence on representative implementation, not from
owner guesswork or convenience.

`RUNTIME-ACTOR-RL-01` remains exactly:

```text
PERF_REFERENCE_CELL_REQUIRED
```

No numeric production actor maximum is selected. In particular, the tested
`M = 1, 2, 3, 4` prototype points from protected #568 are correctness/regression
fixtures, not capacity evidence.

## Production fail-closed contract

`RESOURCE_LIMITS_REGISTRY` must not receive a production actor maximum until a
separate accepted resource decision is supported by qualifying evidence. No
production mode, deployment or readiness claim may use a development fallback,
compiled default, synthetic value, Synology convenience value or absent value as
the production actor limit. It must fail closed when an accepted production limit
is unavailable or does not bind the exact qualified deployment inputs.

The production admission limit must be derived from representative measured
saturation and preserve ADR-0009's current **at least 30% headroom** rule, together
with every lower accepted coupled resource limit. A measured saturation point is
not itself an admission value. Numeric objectives, repetition rules and workload
acceptance remain owned by PERF-01 and are not invented here.

## Later measurement and selection contract

Every result proposed for production capacity acceptance must bind, at minimum:

- exact binary/artifact identity, source revision, dependency lock, compiler and
  toolchain;
- exact server configuration and protocol, content, world and ruleset revisions;
- exact sanitized host hardware, operating-system/kernel and relevant firmware
  fingerprint;
- exact native-process or container/image boundary, CPU/memory allocation and
  limits, swap policy, storage/network constraints, tenancy and competing-load
  conditions;
- representative actor/gameplay workload, scheduling model, warm-up, repetitions,
  duration and occupancy progression;
- p50, p95 and p99 command/simulation latency, queue age, CPU, RSS/peak memory,
  rejection/degradation/error counts and measured saturation, including the first
  violated accepted objective.

The accepted methodology must make runs reproducible and must reject incomplete,
drifted or incomparable fingerprints. Results then inform evaluation of candidate
VPS class or classes. Qualifying measurements on the selected representative VPS
class, the existing headroom rule and lower coupled limits determine the final
admission proposal. A separate reviewed resource decision must accept that proposal
before registry serialization or production qualification.

## Actor-carrier and downstream boundaries

There is one shared Channel actor carrier, not a second Ability, Movement, AI or
capacity registry. It remains scoped by distinct `WorldId` and `ChannelId`, current
scope ownership generation, actor-local identity and actor-local generation.

Issue #508 Phase A may later consume exact actor lookup from that shared carrier
only under a separate allocation. Exact lookup adds no geometry, range, line of
sight, floor, pathfinding, dynamic retarget or target-collection authority. This
decision does not activate #508.

Issue #139 remains at its separately bounded Movement re-evaluation boundary. It
is not activated here and gains no pathfinding, occupancy, visibility, speed or
other Movement semantics from capacity deferral.

## Decision timing

**Must decide now: YES.** Conflating production capacity acceptance with all
pre-production implementation blocks the representative carrier/server needed to
produce honest measurements. Guessing a production value now would create a false
operational contract and expensive migration or unsafe admission later.

The correction freezes only the separation of gates and their fail-closed
relationship. It deliberately does not decide final hardware, topology, service
objectives, workload numbers, saturation, production `M`, or a registry entry.
Representative implementation measurements, accepted PERF-01 methodology and VPS
evaluation are the named evidence that completes—not silently supersedes—this
deferred decision.

## Exact handoff after protected readback

After this exact decision is independently reviewed, Merge-Queue integrated and
read back from protected `main`, the unique #162 coordinator must first perform a
fresh ownership, writable-custody and overlap reconciliation. Only then may #162
issue the smallest safe pre-production implementation allocation necessary to make
capacity measurable, limited to the shared Channel carrier/exact-lookup slice and
explicit finite fail-closed development bounds.

Production readiness, production configuration, `RESOURCE_LIMITS_REGISTRY`
serialization, VPS selection and production admission remain blocked until the
later representative capacity evidence and separate acceptance described above.

