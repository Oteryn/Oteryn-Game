# OTV2 PERF actor reference-cell preparation allocation — 2026-09-10

```yaml
allocation_id: OTV2_PERF_ACTOR_REFERENCE_CELL_PREP_543
issue: 543
parent_gate: 540
coordinator: 162
repository: Oteryn/Oteryn-Game
allocation_state: NOT_ACTIVE_CONDITIONAL
worker_launch: NOT_STARTED
protected_construction_base: 72c997b3f71c180437f6b40e2c5e2abbe0f45273
runtime_authority: NONE
workflow_authority: NONE
registry_mutation_authority: NONE
production_authority: NONE
architecture_decision_authority: NONE
```

## Purpose

Prepare only the fail-closed, non-production evidence tooling that #540's
Supervising Architect explicitly declared safe to build while
`RUNTIME-ACTOR-RL-01` remains `PERF_REFERENCE_CELL_REQUIRED`.

This allocation does not select a Channel actor maximum, does not make a PERF-01
architecture decision, and does not run on or mutate the self-hosted Game runner.
It exists so the eventual protected #540 method contract can be consumed by one
small evidence worker without reopening scope or inventing capacity.

## Activation conditions

The SAME worker lineage may become active only after all of the following are
true on fresh GitHub LIVE readback:

1. this exact allocation is protected-integrated through the repository-native
   Merge Queue and real merge-group `game-gate`;
2. the canonical #540 architecture/method decision is protected-integrated and
   supplies the required reference-cell fields/method semantics;
3. #162 verifies no active writer owns any path listed below;
4. the protected #540 decision still permits evidence-only preparation on these
   paths and does not require a different schema/tool boundary.

If #540 changes the required preparation shape materially, do not stretch this
allocation. Return `ALLOCATION_REFINEMENT_REQUIRED` to #162.

## Exact future writable paths

After explicit #162 activation, the preparation worker may write only:

- `tools/perf-actor-reference-cell/**`;
- `docs/agents/evidence/OTV2-20260910-perf-actor-reference-cell-prep.schema.json`;
- `docs/agents/evidence/OTV2-20260910-perf-actor-reference-cell-placeholder.json`;
- `docs/agents/evidence/OTV2-20260910-perf-actor-reference-cell-prep.md`;
- `docs/agents/tasks/active/OTV2-20260910-perf-actor-reference-cell-prep-543.md`.

No other path is implicitly writable.

## Required tooling boundary

The future tool package must remain stdlib/standalone where practical and must
not require a Cargo/workspace or workflow edit merely for convenience.

It may provide:

- a versioned reference-cell manifest schema;
- a placeholder manifest generator;
- sanitized environment/hardware fingerprint collection functions that can be
  invoked later on an authorized execution surface;
- exact artifact/source/toolchain/content/ruleset/workload identity validation;
- deterministic percentile calculation from supplied samples;
- progressive-population result validation;
- first-objective-violation/saturation calculation from complete supplied runs;
- the existing ADR-0009 30-percent-headroom calculation only after valid
  physical saturation evidence exists;
- repeat-count and soak-duration validation;
- max/max+1 and checked-arithmetic validation helpers;
- deterministic evidence serialization/check mode.

The tools are evidence processors. They are not a scheduler, benchmark daemon,
runtime actor store, workload generator with production authority, or CI control
plane.

## Mandatory fail-closed state model

At minimum distinguish:

```text
PLACEHOLDER_UNBOUND
CELL_BOUND_NOT_MEASURED
MEASUREMENT_INCOMPLETE
MEASUREMENT_INVALID
MEASUREMENT_COMPLETE_NOT_ACCEPTED
```

This preparation worker must not emit `ACCEPTED_CAPACITY`,
`PARITY_CONFIRMED`, `PRODUCTION_READY`, or an equivalent terminal production
claim.

A placeholder or incomplete result must never contain a non-null authoritative
actor maximum.

## Required reference-cell fields

The schema must require, before a physical measurement can even be classified as
complete, fields equivalent to:

### Repository/artifact identity

- repository;
- protected source revision;
- exact benchmark artifact revision/digest;
- lockfile/dependency identity where applicable;
- compiler/toolchain identity;
- protocol/content/world/ruleset/SIM or equivalent exercised revision set.

### Runner/hardware fingerprint

- expected runner group, label and persistent name;
- CPU model/vendor;
- architecture;
- logical and physical core/topology facts available to the process;
- SMT state;
- CPU frequency/governor/turbo facts where readable;
- total RAM visible to the host/process;
- kernel/OS identity;
- NUMA facts where applicable;
- process/cgroup/resource-limit facts;
- a canonical sanitized fingerprint digest over the normalized fields.

Missing required fingerprint evidence is `CELL_FINGERPRINT_INCOMPLETE`.
A later run with a different normalized fingerprint is
`CELL_FINGERPRINT_MISMATCH`; it cannot silently update the baseline.

### Process-cell enforcement

The schema must represent the protected #540-selected process/container mode,
CPU allocation, memory hard limit, swap policy, benchmark-job exclusivity and
any required host-noise precondition as explicit values, never defaults hidden
inside code.

If an execution surface cannot prove/enforce a required constraint, classify it
`REFERENCE_CELL_ENFORCEMENT_UNAVAILABLE` and produce no capacity result.

### Objectives and workload

All latency/queue/CPU/memory/memory-growth objectives and workload rates/mix
must be manifest fields sourced from protected #540 authority. The tooling may
validate them; it must not substitute its own values.

A generic `--defaults` path that fills missing owner/architecture inputs is
forbidden.

## Measurement validation

For every tested population, retain enough normalized data to recompute:

- repetition count;
- p50/p95/p99 requested metrics;
- queue age;
- CPU utilization;
- RSS/peak memory;
- swap use;
- memory-growth result for the declared soak;
- rejection/degradation counts;
- fingerprint/artifact identity;
- objective pass/fail disposition.

Percentile selection and rounding must be explicit and deterministic. If sample
count is insufficient for the declared percentile method, fail the measurement
rather than fabricate an interpolated success.

Saturation is the smallest tested population at which at least one declared
objective is reproducibly violated according to the protected #540 method.
Missing intermediate/progressive evidence must not be guessed.

Only a complete physical evidence packet may calculate a provisional first-slice
candidate:

```text
candidate_M <= floor(0.70 * measured_saturation)
```

and even that value is evidence output awaiting separate acceptance/registry
serialization; this worker does not register or activate it.

## Explicit anti-promotion controls

The tooling/evidence must reject or visibly classify as non-authoritative any
attempt to use the following as the RL-01 capacity source:

- `AI01-ACTIVE-ACTORS=256`;
- synthetic tested populations from protected #537;
- correctness lower bounds of 1 local-step actor, 2 exact-target actors or 3
  mixed actor kinds;
- GitHub-hosted runner hardware merely because CI ran there;
- a self-hosted run lacking the complete protected cell fingerprint;
- a run violating required CPU/memory/swap/exclusivity constraints;
- a run with incomplete repetitions or soak;
- a result from another source/build/content/workload revision;
- manually typed percentile/capacity conclusions not derivable from retained
  samples.

## Broader PERF-01 boundary

Even after a first actor-carrier measurement succeeds, broader PERF-01 remains
open for representative movement, hunting, creature AI, combat, loot,
pathfinding, crowded-interest, raid/boss concentration, reconnect storms,
durable transaction pressure, multi-channel noisy-neighbor, recovery and long
soak evidence.

A later broader result may lower or supersede the first-slice actor maximum.
This preparation tooling must preserve that distinction in evidence metadata.

## Explicit exclusions

This allocation grants no authority to modify:

- `.github/workflows/**` or any runner registration/configuration;
- `apps/**`, `crates/**`, Cargo files or existing product tests;
- `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`;
- `docs/architecture/**` or any public contract;
- runtime actor storage or identity implementation;
- #508 Ability or #139 Movement implementation;
- production deployment, hardware, credentials or live processes;
- Platform, Atlas, META or any external repository.

It does not authorize Remote Desktop, SSH, shell access to the Synology host,
workflow dispatch, hardware changes, CPU governor/turbo changes, cgroup changes,
container/Docker control, or production traffic.

## Required worker evidence

Before the future worker may return `PREP_HARNESS_READY`, it must prove at least:

1. placeholder manifests with any required field absent remain unbound and
   cannot produce a capacity value;
2. normalized hardware fingerprint is stable under field-order changes and
   rejects a one-field drift;
3. artifact/source/workload revision drift rejects comparison;
4. explicit objective fields are mandatory and no code defaults fill them;
5. deterministic percentile results are independently testable on boundary
   sample sets;
6. incomplete repetition/soak sets reject;
7. the first reproducibly failing population, not the largest completing
   population, is selected as saturation;
8. 70-percent headroom uses checked integer arithmetic and cannot exceed
   saturation;
9. synthetic #537 / AI01/correctness-lower-bound data cannot be promoted to
   physical capacity evidence;
10. generated JSON/evidence is byte-stable under the deterministic check mode.

No actual Synology capacity measurement is required or authorized by this prep
worker itself.

## Handoff

If the allocation is protected and #540 is later protected, #162 may activate
one preparation worker on the exact paths above. The resulting harness then
waits for a separately authorized repository-native execution route capable of
running the protected cell on `oteryn-synology-game` and retaining sanitized
physical evidence.

`RUNTIME-ACTOR-RL-01` remains `PERF_REFERENCE_CELL_REQUIRED` until that later
physical evidence is independently qualified and accepted.

Refs #162 #530 #539 #540 #543 #508 #139.

`IMPLEMENTATION_AUTHORITY: NONE_BY_THIS_ALLOCATION_UNTIL_EXPLICIT_ACTIVATION`
`WORKFLOW_AUTHORITY: NONE`
`REGISTRY_MUTATION_AUTHORITY: NONE`
`PRODUCTION_AUTHORITY: NONE`
