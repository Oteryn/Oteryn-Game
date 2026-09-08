# OTV2-20260908-wp3-aws-lc-provider-resident-amendment-439

```yaml
task_id: OTV2-20260908-wp3-aws-lc-provider-resident-amendment-439
title: Resolve WP3 AWS-LC provider-resident lifetime and finite reservation boundary
mode: CONTRACT
status: READY_FOR_REVIEW
repository: Oteryn/Oteryn-Game
base_branch: main
allocation_observed_main_sha: c9cec0f746e549ff96151bcf1e3582522dfea0ee
publication_base_sha: b6411e9bd280a1b48a8c356492a332d084ac7672
branch: arch/wp3-aws-lc-provider-resident-boundary-439
issue: 439
prior_decision_pr: 441
canonical_material_pr: 356
canonical_material_branch: agent/sqlx-driver-budget-351
canonical_material_head_observed: 8a97ed5e0bdfd42b934295ecf3f98584bb49a00f
owner: WP3 architecture amendment lane
allocation_comment: 5588086089
created_at: 2026-09-08T00:00:00Z
updated_at: 2026-09-08T18:26:00+02:00
execution_policy: continuous_progress
classification: ARCHITECTURE_RESOLUTION
architecture_disposition: B_PROVIDER_FINITE_RESERVATION
activation: architecture-only; material authority begins only after protected integration and fresh bounded application to existing #356
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_WP3_AWS_LC_PROVIDER_RESIDENT_ACCOUNTING_AMENDMENT_2026-09-08.md
  - docs/agents/tasks/active/OTV2-20260908-wp3-aws-lc-provider-resident-amendment-439.md
public_contracts:
  - DUR-FRESH-RESOURCE-ENVELOPE-V1
depends_on:
  - Issue 439
  - protected PR 441
  - provider evidence comments 5586413904, 5586624216, 5586628094, 5587745874
  - existing sole material worker PR 356
blocks:
  - WP3 KX continuation in #351/#356 until protected integration
  - dependent WP4 / Server Seam readiness
```

## Outcome

Publish a two-document architecture amendment that removes `BLOCKED_MISSING_PROVIDER_BOUNDARY` only for the exact Linux x86-64 non-FIPS production graph. The amendment selects **B — `PROVIDER_FINITE_RESERVATION`** and separates:

- operation-scaled KX full-lifetime custody; and
- provider-resident process/thread custody.

Both debit the same accepted executor-root ceiling. Provider-resident state is not declared free overhead and no second resource allowance is created.

No runtime mutation is performed by this architecture task. The sole material worker remains Draft PR #356 on `agent/sqlx-driver-budget-351`.

## Live publication reconciliation

The owner allocation was recorded while protected main was `c9cec0f746e549ff96151bcf1e3582522dfea0ee`. Before first branch commit, protected main advanced to `b6411e9bd280a1b48a8c356492a332d084ac7672` through unrelated control-plane changes. Fresh compare showed neither allocated documentation path changed. Because the architecture branch still had no own commit, it was fast-forwarded non-force to the new protected main before publication. No authored work or third-party path was overwritten.

## Exact provider graph

- rustls `0.23.43`;
- aws-lc-rs `1.18.0`, checksum `ce2b2dcc879c3bae0d371e77c99f2238400ef24ec001394befa67b6e543add9e`;
- aws-lc-sys `0.44.0`, checksum `f09fae7be8bb3174e05c6afdb34199e6dc0c7c04ba9fa237b1967adfbde27483`;
- aws-lc-rs VCS `f464440d1fd3983ce9fb023e9eaf1698530919a2`;
- AWS-LC `991e67ff4cf04df4dd89e407f8b920c6936cb56a`;
- target `x86_64-unknown-linux-gnu`, default non-FIPS aws-lc-sys;
- reachable groups `X25519MLKEM768`, X25519, P-256, P-384.

The durable analysis is `docs/architecture/reviews/OTERYN_GAME_WP3_AWS_LC_PROVIDER_RESIDENT_ACCOUNTING_AMENDMENT_2026-09-08.md`.

## Resolution summary

### Charged accounting unit

Under the existing non-RSS charged resident-work model:

- AWS-LC `OPENSSL_malloc(n)` charges exact `n + 8` at this pin;
- direct libc allocations charge source-requested capacity;
- Rust boxes charge exact target layout and collections charge actual capacity;
- mmap charges page-rounded requested mapping capacity;
- allocator/pthread/kernel implementation metadata with no caller-visible requested byte capacity is outside this non-RSS unit, while source-visible dependency allocations remain charged.

### Shared-root provider custody

The existing caller-supplied `ResourceBudget` remains the sole capability. A provider-shared reservation operation may be added to that same capability, but it must debit the same 12 MiB executor root rather than one operation's 4 MiB active-slot subledger. Shared provider custody reduces later root availability; it does not mint capacity. Unsupported budget implementations fail before AWS-LC use.

A root-backed shared token must not keep the initiating active slot occupied after that operation otherwise becomes releasable.

### Process provider bound

```text
JITTER_FIRST_USE_PEAK = 140208
PROCESS_PROVIDER_RESIDENT(page_size) = 140208 + 2 * page_size
```

The 140208 peak is source-derived from one global tree DRBG object plus the temporary Jitter health-test collector and concurrent 8 KiB GCD history. The temporary collector/history are freed before the smaller persistent collector is created. Holding the complete first-use peak until process teardown is conservative.

The two page terms cover Linux fork-UBE and optional SysGenID UBE mappings.

### Per-thread provider bound

```text
THREAD_PROVIDER_RESIDENT = 1360 bytes
```

This covers the direct-libc AWS-LC TLS pointer table, frontend RAND state, entropy-source object, tree-Jitter thread DRBG and fixed `ERR_STATE`. The 64-byte pointer table is direct libc `malloc` in the exact pinned `thread_pthread.c` (`_BORINGSSL_PROHIBIT_OPENSSL_MALLOC`), so it is `L(64)=64`, not `P(64)=72`. Registration bookkeeping is separately precharged.

Minimum implementation may conservatively retain thread registrations until process teardown; root exhaustion denies before another AWS-LC thread registration. An exact thread-exit release is optional only with proven free-before-release; the pinned x86-64 aws-lc-sys binding exposes `AWSLC_thread_local_clear()`.

### Full-lifetime operation KX bounds

A second exact-source audit after the first PR candidate found that the initial `385/744/744` classical values bounded `start()` but did **not** bound `complete()`. The candidate is corrected before integration. Classical completion retains the active private key while parsing the peer key, creating derive contexts, validating ECDH peer state and overlapping aws-lc-rs/rustls shared-secret backing.

Correct full-lifetime reservations acquired before `start()` are:

| group | bytes |
|---|---:|
| X25519 | **554** |
| P-256 | **1625** |
| P-384 | **1705** |
| ML-KEM-768 child | **6264** |
| X25519MLKEM768 | **7881** |

Derivation highlights:

- X25519: active retained 305 + parsed peer 137 + derive context 80 + aws secret 32 = 554.
- P-256: active retained 640 + retained parsed peer 401 + derive contexts 104 + aws secret 32 + ECDH temporary EC key/point 304 + two bounded coordinate BIGNUMs 144 = 1625.
- P-384: analogous 640 + 433 + 104 + 48 + 304 + 176 = 1705.
- ML-KEM: start peak 6264 remains larger than decapsulation complete peak 5112.
- Hybrid: conservative start reservation 7881 remains larger than sequential classical-complete 6850 and PQ-complete 6456 phases; classical/PQ complete temporaries do not coexist.

Initial KX keeps its full reservation through complete/error/drop. HRR reserves the complete replacement **full-lifetime** bound before replacement while the old full reservation remains held; old charge releases only after old backing destruction.

Private rustls sizes/capacities must be asserted from inside the exact vendored implementation. Pin/target/layout mismatch fails qualification.

## Review finding disposition on first PR head

Codex review of first candidate `ef7979f40eb65559e8a42ff921a56c781ce3249b` raised one P1 asserting that the 64-byte pthread TLS pointer table used `OPENSSL_calloc` and needed an 8-byte AWS-LC prefix. Exact pin readback disproves that specific finding: `crypto/thread_pthread.c` prohibits `OPENSSL_malloc` and calls direct libc `malloc(sizeof(void *) * NUM_OPENSSL_THREAD_LOCALS)`. The architecture document now states this explicitly.

Separately, self-review found the real P1 described above: classical `complete()` can exceed the start-only reservation. That finding is repaired in the same PR before integration and invalidates review/CI evidence tied only to the old exact head.

## Acceptance criteria

- [x] Fresh protected main and canonical #356 read back before allocation.
- [x] Exact architecture branch absence proved before creation.
- [x] One architecture-only allocation recorded in #439 comment `5588086089`.
- [x] Branch reconciled non-force to later protected main before first commit; allocated paths were path-disjoint.
- [x] No #356 mutation performed by this task.
- [x] Exact non-FIPS provider graph and target pinned.
- [x] Native charged unit reconciled with the accepted non-RSS resource envelope.
- [x] Jitter first-use overlap mechanically closed; no false double-collector overlap.
- [x] Process RNG/Jitter/UBE residency given a finite root-shared reservation.
- [x] Per-thread RNG/error/TLS residency given a finite root-shared reservation with exact direct-libc TLS table proof.
- [x] Shared residency explicitly debits the same executor root and does not occupy/mint an active slot.
- [x] Non-FIPS self-test/service-indicator and ML-KEM stack-workspace unknowns closed.
- [x] Exact target **full-lifetime** operation KX bounds and HRR overlap semantics defined.
- [x] Classical complete-phase peer parse/derive/ECDH validation and secret-copy overlap source-closed.
- [x] Ordinary owner-free behavior and custom-provider fail-closed rule preserved.
- [x] Exact future material path set defined for the same #356 worker.
- [ ] Independent exact-candidate review of repaired head.
- [ ] Required canonical CI for repaired head.
- [ ] Normal Merge Queue integration.
- [ ] Protected-main readback.
- [ ] Fresh material application to existing #356.

## Future material path set after protected integration only

Existing #351-owned SQLx surfaces:

- `vendor/sqlx-core-0.9.0/src/net/resource_budget.rs`
- `vendor/sqlx-core-0.9.0/src/net/tls/tls_rustls.rs`
- `vendor/sqlx-core-0.9.0/src/net/tls/resource_budget_tests.rs`

New KX crypto-cell surfaces:

- `vendor/rustls-0.23.43/src/client/hs.rs`
- `vendor/rustls-0.23.43/src/client/tls13.rs`
- `vendor/rustls-0.23.43/src/crypto/mod.rs`
- `vendor/rustls-0.23.43/src/crypto/ring/kx.rs`
- `vendor/rustls-0.23.43/src/crypto/aws_lc_rs/pq/hybrid.rs`
- `vendor/rustls-0.23.43/src/crypto/aws_lc_rs/pq/mlkem.rs`

Everything else remains unleased. Another required path must stop with exact `SHARED_LEASE_REQUIRED` evidence before mutation.

## Required future RED/GREEN

The existing #356 worker must prove at minimum:

1. provider-shared capability missing => denial before AWS-LC call;
2. process shared bound max/max+1, same-root debit, and active-slot reuse while process token persists;
3. concurrent first use cannot mint/double-debit process residency;
4. new-thread shared reservation/bookkeeping denial before provider call and no duplicate charge on repeat use;
5. root exhaustion under thread churn denies before provider use and registry growth;
6. X25519 `553/554`, P-256 `1624/1625`, P-384 `1704/1705`, ML-KEM `6263/6264`, hybrid `7880/7881` denial/success boundaries, with the full reservation acquired before start;
7. classical completion peer parse, derive contexts, ECDH validation/BIGNUM temporaries and secret-copy overlap fit the held reservation with no post-start growth;
8. HRR old-full + replacement-full overlap and no early old release;
9. hybrid child failure/unwind without leak or early release;
10. completion/error/cancellation/drop release of operation charge only after active backing destruction while process residency persists;
11. ordinary owner-free API unchanged;
12. owner-aware unsupported custom provider fails before ordinary start;
13. exact-target internal size/capacity assertions;
14. actual AWS-LC positive TLS handshake under the budgeted SQLx path;
15. configured PostgreSQL 17.6 qualification and all remaining #356 acceptance cells.

## Explicit exclusions

This docs task grants no mutation to #356, rustls/sqlx/Tokio/Game source, Cargo/lock, aws-lc-rs/aws-lc-sys, workflows, AGENTS, registry, rulesets/protection, production/external systems, or WP4/WP5/G0/Server Seam readiness. It grants no FIPS or cross-target proof.

## Validation and integration

The architecture candidate must contain exactly the two allocated documentation paths. Independent review is mandatory because the disposition changes from D to B. Review and CI from a superseded exact head do not authorize the repaired candidate. Canonical repository checks and normal Merge Queue remain integration authority. No direct protected-main update, bypass, protection weakening or self-approval is authorized.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.

## Context checkpoint

```yaml
last_progress: first candidate review completed; exact-source self-review found start-only classical bound defect and repaired full-lifetime bounds
status: READY_FOR_REVIEW
branch: arch/wp3-aws-lc-provider-resident-boundary-439
pr: 451
canonical_material_pr: 356
canonical_material_modified: false
blocker: independent review + canonical CI + protected integration of repaired architecture amendment before material KX lease
next_action: publish repaired two-file head, verify exact diff, reply to prior review with pinned-source correction, request new exact-head review, run canonical CI, integrate through Merge Queue, protected-read back, then apply bounded KX lease to existing #356
```
