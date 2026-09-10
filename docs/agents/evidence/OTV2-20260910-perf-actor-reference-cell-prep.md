# OTV2 PERF actor reference-cell preparation evidence

## Classification

```yaml
allocation: OTV2_PERF_ACTOR_REFERENCE_CELL_PREP_543
contract: PERF-01-ACTOR-CARRIER-CELL-V1
base: main@d2367d7088727d3c1d61ab77ea2df7fc01ed1050
classification: PREPARATION_TOOLING_ONLY
physical_measurement_performed: false
capacity_selected: false
production_authority: NONE
rl_01_status: PERF_REFERENCE_CELL_REQUIRED
perf_01_status: OPEN
```

## Prepared result

The package supplies a strict evidence schema, an intentionally unbound
placeholder, and a Python-stdlib processor. The processor canonicalizes the
approved sanitized fingerprint fields and SHA-256 digest independent of input
key ordering. Missing fields produce `CELL_FINGERPRINT_INCOMPLETE`; any
one-field observed/frozen difference produces `CELL_FINGERPRINT_MISMATCH`.

Every protected cell value is an input. There is no defaults mode. Exact
repository, source, artifact, dependency, toolchain, protocol, content, world,
ruleset, SIM and workload identities must equal their frozen comparison values.
The processor rejects #537 synthetic populations, correctness fixtures and
`AI01-ACTIVE-ACTORS=256` when named as the capacity evidence source.

Cell enforcement is represented as read-back evidence for one selected CPU,
soft and hard `RLIMIT_AS` no greater than 1 GiB, zero process-swap observations,
one exclusive benchmark instance, workflow run/job binding and native
non-container execution. An absent or false proof returns
`REFERENCE_CELL_ENFORCEMENT_UNAVAILABLE`.

Nearest-rank p50/p95/p99 uses rank `ceil(percentile * sample_count / 100)` on
sorted retained nonnegative integer observations. A declared minimum sample
count is mandatory. Every progressive population needs at least five clean
repetitions. Mixed results and non-reproducible objectives are inconclusive;
there is no pass inference. A complete result needs the first reproducibly
failing population `S`, a lower population `P` that passed every repetition,
and a passing continuous soak of `P` lasting at least 1,800 seconds.

Only then does checked unsigned-64-bit arithmetic derive the provisional bound:

```text
candidate_M = min(floor(7 * S / 10), P, every supplied lower coupled limit)
```

The exact configured `M` and atomic rejecting `M+1` boundary must prove no
partial mutation, eviction or recycle, stale-reference acceptance, state damage
or allocation-before-overflow-check. The output state is only
`MEASUREMENT_COMPLETE_NOT_ACCEPTED`, its authority is
`PROVISIONAL_EVIDENCE_ONLY`, production capacity is null, RL-01 stays required,
and broader PERF-01 stays open.

## Deliberately not performed

No Synology execution, workflow dispatch/edit, runner mutation, SSH, Remote
Desktop, Cargo/workspace change, runtime/application change, registry change,
production action, or numeric `S`, `P`, or `M` selection from invented evidence
was performed.
