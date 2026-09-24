# OTV2-20260908-wp3-aws-lc-provider-resident-amendment-439

```yaml
task_id: OTV2-20260908-wp3-aws-lc-provider-resident-amendment-439
title: Resolve WP3 AWS-LC provider-resident lifetime and finite reservation boundary
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
issue: 439
issue_state: completed
architecture_pr: 451
architecture_final_head: 72c4a65a7a052f6310df00ae149a73fc26082ca1
architecture_merge_sha: 1829758d093aac826cebe469973246bfa2cd18e8
architecture_disposition: B_PROVIDER_FINITE_RESERVATION
canonical_material_pr: 356
canonical_material_branch: agent/sqlx-driver-budget-351
material_application_comment: 5592724634
material_result_comment: 5592947242
completed_at: 2026-09-09T21:48:51+02:00
classification: ARCHITECTURE_RESOLUTION
shared_lease: released_architecture_task_only
future_write_authority: none_completed_task
activation: architecture resolution protected and applied to the existing #351/#356 material lineage; ongoing material authority remains solely with that canonical worker and its protected grants
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_WP3_AWS_LC_PROVIDER_RESIDENT_ACCOUNTING_AMENDMENT_2026-09-08.md
  - docs/agents/tasks/archive/OTV2-20260908-wp3-aws-lc-provider-resident-amendment-439.md
public_contracts:
  - DUR-FRESH-RESOURCE-ENVELOPE-V1
blocks: []
```

## Terminal outcome

This architecture task is complete. Protected PR #451 replaced the prior exact-target
`D_BLOCKED_MISSING_PROVIDER_BOUNDARY` disposition with **B —
`PROVIDER_FINITE_RESERVATION`** for the pinned Linux x86-64 GNU, non-FIPS
rustls/AWS-LC production graph.

The durable architecture authority remains:

`docs/architecture/reviews/OTERYN_GAME_WP3_AWS_LC_PROVIDER_RESIDENT_ACCOUNTING_AMENDMENT_2026-09-08.md`.

This archived task record does not supersede that decision and does not create new runtime,
resource, merge, deployment or production authority.

## Protected architecture evidence

- **PROVEN:** architecture PR #451 final candidate was
  `72c4a65a7a052f6310df00ae149a73fc26082ca1`.
- **PROVEN:** #451 was protected as merge commit
  `1829758d093aac826cebe469973246bfa2cd18e8`.
- **PROVEN:** that merge remains an ancestor of protected Game main at closeout.
- **PROVEN:** the only #451 review thread is resolved; its historical pthread-pointer-table
  P1 was disproved from exact pinned AWS-LC source because `thread_pthread.c` uses direct
  libc `malloc/free`, so the charged table remains `L(64)=64`.
- **PROVEN:** the architecture selects one same-root resource model, not a second budget or
  owner plane.

## Accepted exact-target boundary

The protected decision keeps the existing non-RSS charged resident-work contract and pins:

- rustls `0.23.43`;
- aws-lc-rs `1.18.0`;
- aws-lc-sys `0.44.0`;
- AWS-LC `991e67ff4cf04df4dd89e407f8b920c6936cb56a`;
- target `x86_64-unknown-linux-gnu`, default non-FIPS graph;
- reachable KX groups `X25519MLKEM768`, X25519, P-256 and P-384.

Charged source-visible units remain:

```text
AWS-LC OPENSSL heap request: P(n) = n + 8
Direct libc heap request:    L(n) = n
Rust Box allocation:         exact target layout
Rust Vec/String backing:     actual allocated capacity
mmap backing:                page-rounded requested mapping capacity
```

Protected provider-resident bounds are:

```text
PROCESS_PROVIDER_RESIDENT(page_size) = 140208 + 2 * page_size
THREAD_PROVIDER_RESIDENT             = 1360
```

Protected full-lifetime operation KX bounds are:

| Reachable group | Full-lifetime reservation |
|---|---:|
| X25519 | 554 |
| P-256 | 1625 |
| P-384 | 1705 |
| ML-KEM-768 child | 6264 |
| X25519MLKEM768 | 7881 |

HRR requires old-full plus replacement-full overlap before replacement `start()` and no
early release of old custody.

## Material application evidence

The architecture task's final application gate is also satisfied without creating a new
worker:

- **PROVEN:** #356 comment `5592728858` records the continuation of the SAME canonical
  #351/#356 worker and cites fresh application authority #351 comment `5592724634` at
  protected `main@1829758d093aac826cebe469973246bfa2cd18e8`.
- **PROVEN:** that dispatch explicitly activates the protected #451 KX/provider-resident
  lease on the existing `agent/sqlx-driver-budget-351` lineage only.
- **PROVEN:** #356 result comment `5592947242` then records material implementation of the
  same-root provider-shared reservation capability and exact-target AWS-LC residency/KX
  accounting on that same lineage.

This satisfies the architecture task's requirement for fresh material application. It does
**not** make WP3 complete.

## Downstream state deliberately left open

Canonical #356 remains Draft/ACTIVE_WIP. The architecture task is no longer the blocker, but
WP3 must still complete its remaining TLS capacity/lifetime composition, resumed TLS1.2 and
compressed-certificate ownership cells, transcript/ClientHello obligations, real funded SQLx
AWS-LC TLS-positive evidence, PostgreSQL 17.6 qualification, whole-diff independent review,
canonical exact-head CI, governed Merge Queue integration and protected-main readback.

WP4/#335 and Server Seam/#247 therefore receive no release from this archive move.

## Historical findings retained

The original active task established and repaired these architecture findings before
integration:

1. start-only classical KX bounds were insufficient because `complete()` could exceed them;
   the protected decision uses full-lifetime bounds;
2. the pthread TLS pointer table is direct-libc `L(64)`, not prefixed `P(64)`;
3. provider-shared root reservation had to be additive on the existing
   `DeframerBufferOwner`, default-fail for unsupported owners, rather than a parallel owner
   trait;
4. first-slice thread/provider shared debits remain conservatively process-lifetime charged;
   direct `AWSLC_thread_local_clear()` implementation remains outside this task.

The complete pre-closeout working record remains recoverable from Git history at the parent
protected main preceding this archive move.

## Closeout acceptance

- [x] Exact provider graph and target pinned.
- [x] Native charged unit reconciled with `DUR-FRESH-RESOURCE-ENVELOPE-V1`.
- [x] Process and per-thread provider residency source-closed.
- [x] Full-lifetime KX bounds and HRR overlap defined.
- [x] Same-root provider-shared owner seam defined without a second budget.
- [x] Independent review finding resolved.
- [x] Canonical exact-head validation completed before protected #451 integration.
- [x] #451 protected-main integration/readback completed.
- [x] Fresh material application granted to the SAME #351/#356 lineage.
- [x] Material implementation consumed the protected decision without replacing the worker.
- [x] Issue #439 closed as `completed` with architecture-only closeout comment `5607780896`.

## Archive-move scope

This closeout changes only task metadata location/state:

`docs/agents/tasks/active/OTV2-20260908-wp3-aws-lc-provider-resident-amendment-439.md`
→
`docs/agents/tasks/archive/OTV2-20260908-wp3-aws-lc-provider-resident-amendment-439.md`

No runtime/client/server/protocol/persistence/Cargo/workflow/ruleset/META policy/registry,
Platform, Atlas, WP4, WP5, Server Seam, deployment, credential or live-data mutation is
included.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
