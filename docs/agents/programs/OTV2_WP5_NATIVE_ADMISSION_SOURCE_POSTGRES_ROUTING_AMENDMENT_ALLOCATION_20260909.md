# WP5 native admission source PostgreSQL routing amendment allocation

Coordinator: #162. Source readiness: #319. Programme: #364. Parent routing allocation: #416.

## Status

Prospective Game-only control-plane amendment. **NOT_ACTIVE**.

This amendment registers exactly one additional future dedicated PostgreSQL
qualification target under the protected #416 allocation:

```yaml
amendment_id: OTV2-WP5-NATIVE-ADMISSION-SOURCE-PG-ROUTING-20260909
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
source_issue: 319
programme_issue: 364
parent_allocation_pr: 416
allocation_base_main_sha: 85ba329ccbdf51338a672fcac8ca0836726dc71b
allocation_state: NOT_ACTIVE
worker_launch: NONE_UNTIL_EXPLICIT_WORK_APPLICATION
registered_target: native_admission_source_postgres
target_test_path: apps/game-server/tests/native_admission_source_postgres.rs
workflow_write_authority: FORBIDDEN
runtime_write_authority: FORBIDDEN
sql_migration_write_authority: FORBIDDEN
platform_write_authority: FORBIDDEN
production_authority: FORBIDDEN
server_seam_authority: FORBIDDEN
```

Protection of this document is **target registration only**. It does not activate
routing, create the target test, allocate a migration, change policy pins, start a
worker, grant workflow custody, or make any hosted PostgreSQL result canonical.

## Why this amendment is required

Protected #416 registers only these exact dedicated WP5 targets:

```text
character_authority_postgres
runtime_scope_assignment_postgres
```

Its fail-closed contract forbids activating an unregistered target without a
reviewed amendment. The WP5 native admission source-ingestion allocation prepared
from #319 needs a third exact target for durable descriptor/source-high-water /
pending-publication qualification:

```text
native_admission_source_postgres
```

Reusing `durability_postgres`, `character_authority_postgres`,
`runtime_scope_assignment_postgres`, an alias, a generic glob, or a skipped job
would not prove this target and is forbidden.

## Exact registration

When this amendment is protected and read back, #416 is amended only for the
following target identity:

```yaml
id: native_admission_source_postgres
expected_test_path: apps/game-server/tests/native_admission_source_postgres.rs
required_postgres_major_minor: "17.6"
required_pr_lane: true
required_merge_group_lane: true
skipped_counts_as_success: false
nonzero_counts_as_success: false
hosted_stub_counts_as_success: false
material_routing_state: NOT_ACTIVE
```

The target name and expected path are immutable for this allocation. Any rename,
alternate path, wildcard expansion or additional target requires a new reviewed
amendment.

## Relationship to WP5 source ingestion

The sibling source-ingestion allocation prepared from #319 separates:

- S1 authenticated bounded source transport/descriptor validation;
- S2 durable descriptor/source-floor/pending-slot bootstrap;
- S3 sealed Foundation publication/composition.

This amendment is only the **registration prerequisite** for S2. It grants no S2
implementation path and does not make S2 active.

The ordering is deliberately asymmetric to avoid a dependency cycle:

```text
this target-registration amendment protected
-> source-ingestion/S2 activation gates independently become satisfiable
-> S2 may create native_admission_source_postgres on a held branch
-> only then may a separately serialized #416 writer activate material routing
-> routing review/pins/canonical CI/FULL MQ/protected readback
-> held S2 reconciles with protected routing
-> only post-routing PostgreSQL 17.6 PR/MQ results may count
```

Material #416 routing therefore is **not** a prerequisite for creating the held
target. Protected target registration is. Material routing remains mandatory
before S2 may treat PostgreSQL evidence as canonical or integrate a target whose
acceptance depends on that evidence.

## Future material routing custody — not granted here

After the exact held target exists, a separate Work application must refresh all
live ownership and acquire sole-writer leases before touching any #416 control
surface. Candidate paths inherited from the protected #416 contract include:

```text
.github/workflows/merge-gate.yml
.github/workflows/merge-group-gate.yml
.github/workflows/rust.yml
tools/repository/validate_repository_policy.py
tools/repository/validate_repository_policy_core.py
tools/repository/validate_pr_gate_pg_sim.py
tools/repository/POLICY_CORE_SHA256
tools/repository/AUDITED_VALIDATE_REPOSITORY_POLICY_SHA256
tools/repository/AUDITED_PG_SIM_HARNESS_SHA256
```

None of those paths is leased by this amendment. The future material writer must
refresh protected main, #416, active PRs/tasks and exact path custody. Any overlap
with another workflow/policy/pin writer is a hard hold, not authority to combine
lanes.

## Material activation gates

A future material #416 writer may activate this target only when all of the
following are true:

1. this amendment is protected on `main` and the exact target registration is read
   back;
2. WP3 PostgreSQL 17.6 driver qualification is terminal/protected and shared
   Cargo/test-target custody needed by the dedicated lane is released;
3. the source-ingestion S2 allocation is itself protected/authorized, WP4/#335 is
   terminal/protected, and the exact next migration/path custody is known;
4. `apps/game-server/tests/native_admission_source_postgres.rs` exists on a held
   S2 branch with a stable exact target contract that the routing writer can read;
5. the routing writer holds explicit sole-writer leases for every workflow,
   classifier, policy-core, pin and harness path it actually changes;
6. no #351/#356, #335, Server Seam, Platform, production or unrelated WP5 path is
   pulled into that material routing change.

If any gate is false, material routing stays `NOT_ACTIVE`.

## Required material proof sequence

The later material change must preserve #416's fail-closed sequence rather than
shortcut it:

1. add/refresh regression coverage for the exact target identity and exact held
   target path;
2. prove a test-only RED where the target exists but is intentionally not routed;
3. add the smallest exact PR / Merge Queue / push routing needed for
   `native_admission_source_postgres` without broadening unrelated paths;
4. preserve trusted protected-base classification and fail closed on unknown,
   renamed, missing, skipped or untrusted target state;
5. run the target against real PostgreSQL 17.6 in canonical PR and merge-group
   lanes; a stub, parser-only fixture, SQLite substitute, skipped job or unrelated
   `durability_postgres` result never counts;
6. independently review the exact material head;
7. update policy-core pins only through the separately serialized pin-owning
   procedure required by #416;
8. rotate audited policy/harness pins only after their predecessor material source
   is protected and re-read;
9. obtain exact canonical repository checks, FULL Merge Queue and protected-main
   readback.

Only after that protected material readback may the held S2 target reconcile and
produce acceptance evidence that counts.

## Failure semantics

The routing contract fails closed if any of the following occurs:

- target id or expected path differs from this registration;
- target file is missing from the held S2 lineage when material routing starts;
- target is classified by an unreviewed/unprotected head-side classifier;
- the PR lane or merge-group lane is omitted;
- PostgreSQL is not exactly the accepted 17.6 execution for qualification;
- a required job is skipped, neutral, cancelled or nonzero;
- a generic/other target result is substituted;
- policy-core or audited pin custody cannot be proven;
- protected-base SHA or target contract changes without fresh reconciliation;
- S2 attempts to count PG evidence produced before routing was protected.

Failure leaves the target held and noncanonical. It never authorizes a bypass,
fallback target, direct merge, production test, or weakened gate.

## Path and authority exclusions

This amendment changes only this new documentation path. It does not authorize
writes to:

```text
apps/game-server/**
.github/workflows/**
tools/repository/**
Cargo.toml
Cargo.lock
```

It also grants no write/custody change to #351/#356, #335, Server Seam/#247,
Oteryn/Oteryn-Platform, production infrastructure, credentials, certificates,
databases or live data.

## Integration / lifecycle

```text
this allocation-only amendment
-> exact-head independent review
-> exact-head canonical game-gate
-> FULL Merge Queue
-> protected-main readback
-> registration state becomes PROTECTED_BUT_NOT_ACTIVE
-> wait for exact held S2 target + all material gates
-> separate Work application for #416 material routing
```

Review/CI/Merge-Queue waiting is a checkpoint, not permission to launch a material
worker early.

## Validation for this allocation PR

Documentation/control-plane only:

- exact changed-file/full-diff review;
- repository governance/policy checks;
- independent exact-head review because the document amends a fail-closed CI
  routing authority;
- exact-head `game-gate`;
- normal FULL Merge Queue;
- protected-main readback before the registration is considered protected.

Runtime E2E: `NOT_APPLICABLE` — this PR registers one prospective target only and
changes no runtime, SQL/migration, workflow, policy pin, Platform or production
behavior.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
