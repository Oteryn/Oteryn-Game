# OTV2-20260912-wp3-v2-architecture-decision

```yaml
task_id: OTV2-20260912-wp3-v2-architecture-decision
title: Draft WP3-v2 superseding architecture decision
mode: CONTRACT
status: ready_for_acceptance
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/wp3-v2-superseding-decision-20260912
pr: 590
base_sha: 489e3e390a1bce1ce3439c66521ab75f8a826cd8
head_sha: e106ac49952112685553b46861b82f75b44f7d8c
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: astra wp3-v2 architecture lead
created_at: 2026-09-12
updated_at: 2026-09-12
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_WP3_V2_SUPERSEDING_ARCHITECTURE_DECISION_2026-09-12.md
  - docs/agents/tasks/active/OTV2-20260912-wp3-v2-architecture-decision.md
public_contracts:
  - docs/architecture/reviews/OTERYN_GAME_DURABLE_FRESH_RESOURCE_ENVELOPE_DECISION_2026-09-06.md
depends_on:
  - issue:162
  - issue:351
  - pr:356
  - issue:364
  - pr:588
  - pr:589
blocks:
  - WP3-v2 Gate 1 repository acceptance
  - A4 WP3-v2 material implementation allocation
  - terminal composed WP3 qualification
cross_repository_coordination_id: WP3-V2-GAME-PLATFORM-20260912
external_repositories:
  - Oteryn/Oteryn-Platform
```

## Outcome

Produce one coherent superseding WP3-v2 architecture decision for the real Child B consumer without granting implementation or self-acceptance authority.

Current candidate:

`WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1`, Revision 2.

Current lane result:

```text
WP3_V2_ARCHITECTURE_READY_FOR_ACCEPTANCE
ARCHITECTURE_ACCEPTED = NO
IMPLEMENTATION_AUTHORITY = NONE
```

## Architecture and source of truth

- `PROVEN`: protected admission remains `main@489e3e390a1bce1ce3439c66521ab75f8a826cd8`.
- `PROVEN`: #162 remains product allocation/integration authority and #364 remains the active remediation programme.
- `PROVEN`: #351 / Draft PR #356 remains the canonical WP3 implementation/evidence lineage and was not mutated by A1.
- `PROVEN`: #329 / PR #335 remains the canonical Child B lineage and was not mutated by A1.
- `PROVEN`: PR #588 remains retained audit evidence; canonical `WP3-Q01..WP3-Q75` remains binding.
- `PROVEN`: PR #451 is merged and protects exact-target finite AWS-LC provider/KX accounting architecture.
- `PROVEN`: PR #453 is merged and protects provider-config owner allocation.
- `PROVEN`: PR #458 is merged and protects the AWS-LC PQ-first feature-edge allocation.
- `PROVEN`: PR #455 is closed/unmerged and is not architecture authority.

## Selected architecture

Option B is selected for the first slice:

- one process-scoped logical Durability executor and one accepted DFR root ledger;
- lazy `PgPool` used as one-ready-connection holder;
- `max_connections=1`, `min_connections=0`;
- SQLx default finite retirement retained: idle 10 minutes, max lifetime 30 minutes;
- one root-owned connect/reconnect transient at a time, five-second root connect deadline;
- active DB work uses `Pool::try_begin()` so it cannot manufacture a connection;
- two logical active custody slots, at most one physical DB pass at a time;
- one-second queue deadline and one absolute two-second DB-pass deadline;
- narrow PostgreSQL root-owner seam instead of generic SQLx/Tokio ownership redesign;
- explicit no-ambient PostgreSQL config;
- literal-IP TCP transport + separate TLS server name;
- `VerifyFull`, TLS1.3-only owner-aware profile;
- exact non-FIPS AWS-LC Linux x86-64 GNU profile, Rust 1.94 / rustls 0.23.43 / aws-lc-rs 1.18.0 / aws-lc-sys 0.44.0 / SQLx 0.9.0 / Tokio 1.53.1;
- SCRAM-SHA-256-only PostgreSQL authentication;
- built-in PostgreSQL type profile and statement-cache capacity 100 initially;
- broad dependency forks removed only after replacement proof.

Option A is rejected as terminal architecture but useful primitives/evidence remain. Option C remains a supersession option if exact implementation evidence shows the narrow holder-style pool seam is worse than an actor or if later measured product evidence requires additional physical DB concurrency.

## Exact-source evidence closure

The missing A2 evidence categories were closed through a read-only exact-source pass against #335/#356 and protected architecture. This did not mutate an A2 branch or claim independent review.

### SQL corpus

Exact production call-site census at #335:

- `admission_journal.rs`: 33 `sqlx::query*` call-sites;
- `mod.rs`: 22;
- `fresh_admission.rs`: 10;
- `admission_authority_guards.rs`: 4;
- total base call-sites: 69.

Exact repair families identified:

- schema migration-ledger unbounded `fetch_all` -> embedded `N+1` sentinel bound;
- two pending-command child `fetch_all` families -> bounded ordered aggregate preserving accepted 64 commands;
- active committed binding vector -> exact-cardinality `LIMIT 2` shape;
- all unguarded reconnect `record_json` reads -> same-snapshot byte guard before transfer;
- SQLx error/dependency logging -> bounded normalized Durability result and bounded/redacted sink;
- 15 relation-lock statements -> one static multi-table statement preserving exact order/mode.

Conservative statement-shape bound is 95 before lock consolidation and 81 after it, below pinned statement-cache count 100. Metadata bytes remain an A4 qualification obligation.

### Lock footprint

Exact current maximum normal V1/V2 footprint:

```text
1 shared executor-custody advisory root
+ up to 8 domain advisory roots
+ 15 relation classes
= 9 logical advisory roots / 15 relation classes
```

Executor takeover uses one exclusive executor advisory root plus the same 15 relation classes. This fits accepted DFR `64 / 16` without inventing capacity.

### Pool lifecycle

Pinned SQLx proves:

- ordinary `acquire()` may open a connection and retries ConnectionRefused/transient connect errors with exponential backoff;
- `try_acquire()` only pops an existing idle connection;
- `Pool::try_begin()` uses `try_acquire()` and returns an owned transaction, preserving the existing transaction-oriented Child B boundary without connection creation;
- `after_release` runs before the final return `ping()`;
- drop can spawn asynchronous return-to-pool work;
- reaper may close idle/lifetime-expired connections, but with `min_connections=0` its minimum-maintenance call does not create a replacement.

### Configuration

Pinned SQLx ordinary options can consume ambient `PG*`, OS username and `.pgpass`; `.pgpass` uses an unbounded `read_line(String)` and may log the full malformed line. Therefore the production profile forbids ambient configuration and uses an explicit Oteryn-owned bounded configuration path.

### PostgreSQL startup/authentication

Pinned source reaches AuthenticationOk, CleartextPassword, MD5Password and SASL; other methods reject. SASL implements non-channel-binding SCRAM-SHA-256. `SCRAM-SHA-256-PLUS` is not qualified because the implementation sends `plus:false`.

Revision 2 therefore freezes SCRAM-SHA-256 only and authorizes a narrow fail-closed owner-aware auth-profile seam.

### TLS/provider graph

Current root Game Server profile selects ring, but exact #356 owner-aware TLS code requires the qualified AWS-LC feature and fails otherwise. Protected #451/#453/#458 plus exact #356 source make the AWS-LC profile the smallest evidence-backed first-slice provider. This is a future Gate-1 implementation profile change, not a claim that protected main already uses AWS-LC.

## Resource qualification state

Architecture defines:

```text
I + max(R,T) + Q + A <= 12 MiB
```

`Q <= 4 MiB` and `A <= 8 MiB` are accepted DFR ceilings. One settled connection and one connect transient are architecture maxima. Complete `I`, `R`, `T`, active SQL peak, deferred reactor tail and metadata-byte bounds remain `UNKNOWN` until the exact A4 candidate.

Those are implementation qualification obligations, not guessed architecture constants. Failure of the root equation requires escalation; it never authorizes a second ledger, extra slot or weakened TLS/DFR semantics.

## #356 disposition

### RETAIN

- ResourceBudget/ResourceReservation and applicable backing/finality primitives;
- protected AWS-LC provider/KX work and matching exact-profile source/tests;
- PostgreSQL 17.6 / real TLS harness;
- hostile frame/count/denial vectors;
- provenance/source census;
- valid runtime/socket/reactor finality research;
- direct owner-aware establishment primitives reused by root-owned pool creation.

### REWORK / NARROW

- SQLx core/PostgreSQL changes to exact root-owner, receive/count/status/cache/finality seams;
- final Game Server TLS feature from ring to selected AWS-LC profile after acceptance/allocation;
- pool/bootstrap to lazy max1/min0 + root maintenance + active try-begin;
- config/auth/TLS setup to no-ambient + VerifyFull/TLS1.3/SCRAM-only.

### HISTORICAL EVIDENCE ONLY / removable after replacement proof

- generic Tokio blocking-owner expansion not required by the frozen profile;
- broad rustls ownership outside retained exact AWS-LC seams;
- generic DNS/UDS work excluded by the first-slice transport profile;
- operation-owned direct-connect architecture assumptions;
- closed/unmerged #455 as authority.

### SUPERSEDED

- broad operation-owned direct connection as terminal WP3 architecture;
- earlier two-connection first-slice recommendations;
- timeout/drop/after_release/SQLx-close-as-finality assumptions;
- current ring profile as the final first-slice WP3-v2 provider once this candidate is accepted and A4 is allocated.

## Acceptance criteria

- [x] A/B/C compared with real implementation/maintenance/finality trade-offs.
- [x] one root/executor ownership model defined.
- [x] queue/active ownership and exact release/finality semantics defined.
- [x] pool construction/connect/replacement/retirement/ready-only active checkout defined.
- [x] configuration/credential sources and lifetimes defined without ambient libpq discovery.
- [x] PostgreSQL startup/authentication profile frozen to SCRAM-SHA-256 only.
- [x] exact TLS/runtime/provider first-slice profile frozen.
- [x] cancellation/rollback/ambiguous COMMIT/reconciliation ownership defined.
- [x] restart/takeover/predecessor fencing defined.
- [x] canonical Q01-Q75 preserved as final qualification matrix.
- [x] #356 retention/supersession map recorded.
- [x] exact-source SQL/lock/pool/config/auth/provider evidence used rather than assumptions.
- [ ] independent exact-head HIGH-risk architecture/resource/security review complete.
- [ ] exact-head repository checks green for the Revision-2 head.
- [ ] candidate protected-integrated/read back before A4 material allocation.

## Excluded scope

No runtime Rust, vendor, Cargo/lockfile, SQL/migration, workflow/ruleset, #356/#335 material mutation, Platform write, production/deployment/secret, merge, Merge Queue submission or architecture self-acceptance.

## Validation

### Focused readback

PASS for protected main, live #351/#356, #329/#335, #364, #451/#453/#458, #588/#589, accepted DFR, architecture discipline, exact Child B durability source and exact #356 SQLx/rustls/pool/config/auth source.

### Component/integration/E2E

`NOT_APPLICABLE`: this PR remains docs-only and changes no product behavior.

### Previous exact-head CI

PR #590 head `3f4965404b28065325382d2a4ef8399e0790cb45` had:

- Agent Governance: SUCCESS;
- Architecture Semantic Audit: SUCCESS;
- Merge Gate: SUCCESS.

Those runs are stale after Revision-2 mutation and are **not** claimed for the new head.

## Self-review

- no runtime/product/external-repository path mutation;
- no architecture acceptance claim;
- no fabricated I/R/T bound;
- R21 retained: one connection is not claimed performance-sufficient from serialization alone;
- protected AWS-LC authority is distinguished from unmerged #455 and unprotected #356 material code;
- current protected ring profile is distinguished from the proposed accepted first-slice AWS-LC profile;
- all broad-removal actions remain conditional on replacement proof.

## Context checkpoint

```yaml
last_progress: Revision-2 exact-source closure committed; A1 candidate is ready for repository acceptance
status: ready_for_acceptance
branch: arch/wp3-v2-superseding-decision-20260912
head_sha: e106ac49952112685553b46861b82f75b44f7d8c
pr: 590
final_head_sha: null
final_head_frozen_at: null
blocker: independent exact-head architecture review and fresh Revision-2 exact-head CI/protected acceptance
next_action: re-read PR #590 exact head, exact changed files and new checks; repair only concrete findings
```
