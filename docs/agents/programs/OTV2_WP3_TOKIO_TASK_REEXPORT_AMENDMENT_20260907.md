# WP3 Tokio task re-export amendment

Coordinator: #162. Existing worker: #351 / PR #356. Programme: #364.
Preparation authority: #351 comment `5575814487`.
Protected parent amendment: `OTV2_WP3_TOKIO_BLOCKING_OWNER_AMENDMENT_20260907.md` / PR #412.

## Status

Prospective one-path amendment of the SAME #351/#356 worker. **NOT_ACTIVE**.
It creates no new worker, budget, runtime topology, dependency version, B authority,
production authority or external-repository authority.

```yaml
amendment_id: OTV2-WP3-TOKIO-TASK-REEXPORT-20260907
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
worker_issue: 351
worker_pr: 356
worker_branch: refs/heads/agent/sqlx-driver-budget-351
observed_blocker_head_sha: bec5d681f678f065bf0012dbb87c72c9c4cfaf19
allocation_base_main_sha: e286291173dfadd963fd9fdd2cd71fe6211b40f2
amendment_state: NOT_ACTIVE
worker_launch: EXISTING_WORKER_ONLY
parent_amendment: OTV2-WP3-TOKIO-BLOCKING-OWNER-20260907
new_authorized_path:
  - vendor/tokio-1.53.1/src/task/mod.rs
new_resource_maximum: FORBIDDEN
ordinary_tokio_spawn_blocking_semantics: PRESERVE
b_activation: FORBIDDEN
external_repositories: []
```

## Exact proven need

Protected #412 authorizes the complete exact Tokio 1.53.1 package and the smallest
blocking/runtime/task owner implementation surfaces, including
`vendor/tokio-1.53.1/src/task/blocking.rs`, but it does not authorize
`vendor/tokio-1.53.1/src/task/mod.rs`.

The same worker produced a compilation-valid amended RED and stopped before GREEN.
Fresh exact-source readback of Tokio 1.53.1 proves `src/task/mod.rs` contains the
runtime-gated public task exports:

```text
cfg_rt! {
    mod blocking;
    pub use blocking::spawn_blocking;
    ...
}
```

The Oteryn resource-owned fallible blocking API must be implemented in the already
allowed private `task/blocking.rs`, but SQLx cannot name that API through
`tokio::task` unless the task module re-exports it. No existing public hook exposes
the protected owner capability. Therefore one re-export surface is the smallest
implementation path that closes the exact blocker.

## Authorized mutation

After protected Work application, the existing #351/#356 writer may modify only:

```text
vendor/tokio-1.53.1/src/task/mod.rs
```

and only to re-export the Oteryn resource-owned blocking API implemented under the
already protected #412 allowlist.

Permitted shape is equivalent to adding the new owned API to the existing
`cfg_rt!` blocking re-export surface. The exact symbol name may follow the
implementation in `src/task/blocking.rs`, but it must be Oteryn-specific,
non-default and fallible/resource-owner aware.

No unrelated task documentation, module visibility, existing export, ordinary
`spawn_blocking`, `block_in_place`, `spawn`, `JoinHandle`, scheduler behavior or
feature semantics may change from this amendment. No new public general-purpose
resource-management API is authorized.

## Invariants inherited unchanged from #412

All parent requirements remain binding:

- exact Tokio 1.53.1 version, crates.io checksum
  `202caea871b69668250d242070849eb495be178ed697a3e98aebce5bc81a0bed`,
  upstream commit `75fef53d0a8590c2d1dbb63672aa7b7d1ef51155`;
- ordinary upstream `spawn_blocking` semantics remain compatible;
- owned task allocation, owner queue and attributable worker/thread backing are
  reserved before allocation/creation and retain custody through actual release;
- denial never falls back to ordinary unowned blocking work;
- all accounting uses the existing `DUR-FRESH-RESOURCE-ENVELOPE-V1`; no numeric
  resource maximum is added or widened;
- non-Tokio enabled SQLx runtime backends remain fail closed unless separately
  proven;
- existing #351 Cargo/shared PostgreSQL custody remains serialized;
- B/Foundation/SQL migration/workflow/registry/production/external scope remains
  excluded.

## Required proof after activation

The SAME worker resumes from its preserved RED/history and must prove at least:

1. without the `src/task/mod.rs` re-export, SQLx cannot resolve the owned API;
2. with the exact re-export and already-authorized implementation, the funded
   owned Tokio path compiles and is callable from the admitted SQLx owner adapter;
3. ordinary `tokio::task::spawn_blocking` behavior remains unchanged;
4. the new owned API is unavailable without the intended runtime/feature context
   and cannot silently route to the unowned path;
5. all #412 task/queue/worker preallocation, cancellation, idle-retention and
   shutdown tests remain mandatory;
6. strict provenance/delta manifest records this file as an Oteryn-authored
   semantic delta.

A successful re-export compilation is not WP3 completion. Complete TLS
composition, real TLS-positive evidence, PostgreSQL accounting/PG17.6, independent
whole-diff review, canonical CI/MQ and protected readback still apply.

## Activation gate

```text
this one-path amendment reviewed
-> canonical exact-head checks PASS
-> normal FULL Merge Queue
-> protected main readback
-> fresh #356/Cargo/B/shared-PG custody verification
-> explicit Work application to the existing #351/#356 writer
-> resume GREEN from preserved counters/history
```

Until then `vendor/tokio-1.53.1/src/task/mod.rs` remains outside the worker's
write lease.

## Excluded scope

No new Tokio source path beyond the one listed above; no Tokio version/features,
rustls, new dependency, B/#335, Foundation, SQL/migration, Server Seam, workflow,
ruleset, registry, production/live-data or external-repository mutation.

Runtime E2E is NOT_APPLICABLE to this allocation-only document; later material
vendor/SQLx implementation carries the full high-risk qualification burden.
