# OTV2-20260912-wp3-v2-architecture-decision

```yaml
task_id: OTV2-20260912-wp3-v2-architecture-decision
title: Draft WP3-v2 superseding architecture decision
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/wp3-v2-superseding-decision-20260912
pr: 590
base_sha: 489e3e390a1bce1ce3439c66521ab75f8a826cd8
head_sha: external_pr_evidence
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
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

`WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1`, Revision 3.

Current lane result:

```text
WP3_V2_ARCHITECTURE_SUCCESSOR_VALIDATING
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
- one root-owned connect/reconnect generation at a time, five-second root connect deadline per recovery window;
- `R` and `T` are mutually exclusive lifecycle generations, including every connection-attributable retirement tail; no new `T` begins until prior `R`/`T` descendants are final;
- successful establishment is a charged `T -> R` ownership transfer, not an overlap;
- root recovery is event-driven through one coalesced `root_ready_demand` latch: startup/takeover demand, ready-only `try_begin()==None`, or explicit root-owned retirement; one demand authorizes at most one five-second recovery window and there is no periodic/immediate reconnect loop after failure;
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

Revision 3 freezes the architectural consequences rather than leaving them to A4: connection-attributable reaper/close/reactor tails stay in the owning `R`/`T` generation until finality; generations do not overlap; a silent reaper is recovered on the next coalesced ready-miss demand; a failed root connect window does not self-loop and requires a new/coalesced demand before a successor window.

### Configuration

Pinned SQLx ordinary options can consume ambient `PG*`, OS username and `.pgpass`; `.pgpass` uses an unbounded `read_line(String)` and may log the full malformed line. Therefore the production profile forbids ambient configuration and uses an explicit Oteryn-owned bounded configuration path.

### PostgreSQL startup/authentication

Pinned source reaches AuthenticationOk, CleartextPassword, MD5Password and SASL; other methods reject. SASL implements non-channel-binding SCRAM-SHA-256. `SCRAM-SHA-256-PLUS` is not qualified because the implementation sends `plus:false`.

Revision 3 retains SCRAM-SHA-256 only and the narrow fail-closed owner-aware auth-profile seam from Revision 2.

### TLS/provider graph

Current root Game Server profile selects ring, but exact #356 owner-aware TLS code requires the qualified AWS-LC feature and fails otherwise. Protected #451/#453/#458 plus exact #356 source make the AWS-LC profile the smallest evidence-backed first-slice provider. This is a future Gate-1 implementation profile change, not a claim that protected main already uses AWS-LC.

## Independent-review remediation

The independent review of predecessor exact head `8f5e45dd1a7bdaf28dbf088543518f40cea38e29` produced exactly the two findings assigned to this repair lane. No additional architecture scope is admitted here.

### P1 — accepted and repaired

**Finding:** the prior `I + max(R,T) + Q + A` equation did not unambiguously classify or prohibit overlap between settled/retiring `R`, connect `T`, and asynchronous retirement tails, leaving A4 to choose architecture semantics.

**Repair:** Revision 3 freezes `I`, lifecycle-complete `R`, lifecycle-complete `T`, the `T -> R` transfer point, and strict non-overlap. Per-connection retirement descendants remain in the generation that created them; shared process residency is `I`; a new `T` is blocked until every prior `R`/`T` tail is final. The equation is therefore complete under a single fixed architecture model. Failure to implement/prove that model is escalation, not implementation discretion.

### P2 — fixed

**Finding:** the prior root-maintenance text did not freeze the exact recovery trigger after a silent reaper close or a failed five-second connect window.

**Repair:** Revision 3 freezes one coalesced `root_ready_demand` event source set and one-window consumption rule. Startup/takeover demand, ready-only `try_begin()==None`, or explicit root-owned retirement may set the latch. A silent reaper is detected by the next ready miss. One latched demand authorizes one five-second window; a failed/timed-out window retires fully as `T` and never self-retries. Only a new/coalesced demand may authorize a later window.

Because P1 is a material accepted repair, the predecessor review does not qualify the successor exact head. A fresh genuinely independent HIGH-risk architecture/resource/security re-review remains required before repository architecture acceptance.

## Resource qualification state

Architecture defines:

```text
I + max(R,T) + Q + A <= 12 MiB
```

`Q <= 4 MiB` and `A <= 8 MiB` are accepted DFR ceilings. Architecture now fixes the complete lifecycle classification and mutual exclusion of `R` and `T`: no separate retirement-tail term exists outside them, and per-connection deferred tails cannot be moved into `I` to evade the bound. Complete byte values for `I`, lifecycle `R`, lifecycle `T`, active SQL peak and metadata remain `UNKNOWN` until the exact A4 candidate.

Those byte values are implementation qualification obligations, not architecture choices. Failure of the frozen root equation requires escalation; it never authorizes overlap, a tail exemption, a second ledger, extra slot or weakened TLS/DFR semantics.

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
- pool/bootstrap to lazy max1/min0 + root maintenance + active try-begin + coalesced root-demand/finality gate;
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
- implementation-selected overlap between new connects and prior connection/attempt retirement tails;
- timer/polling/unbounded reconnect policy substituted for the frozen demand-triggered recovery model;
- current ring profile as the final first-slice WP3-v2 provider once this candidate is accepted and A4 is allocated.

## Acceptance criteria

- [x] A/B/C compared with real implementation/maintenance/finality trade-offs.
- [x] one root/executor ownership model defined.
- [x] queue/active ownership and exact release/finality semantics defined.
- [x] pool construction/connect/replacement/retirement/ready-only active checkout defined.
- [x] P1: exact `I/R/T` lifecycle and retirement-tail classification plus non-overlap frozen so A4 has no architecture choice.
- [x] P2: exact coalesced root-maintenance recovery trigger/window semantics frozen for reaper absence and failed connect windows.
- [x] configuration/credential sources and lifetimes defined without ambient libpq discovery.
- [x] PostgreSQL startup/authentication profile frozen to SCRAM-SHA-256 only.
- [x] exact TLS/runtime/provider first-slice profile frozen.
- [x] cancellation/rollback/ambiguous COMMIT/reconciliation ownership defined.
- [x] restart/takeover/predecessor fencing defined.
- [x] canonical Q01-Q75 preserved as final qualification matrix.
- [x] #356 retention/supersession map recorded.
- [x] exact-source SQL/lock/pool/config/auth/provider evidence used rather than assumptions.
- [ ] successor exact-head repository/governance checks green.
- [ ] fresh independent exact-head HIGH-risk architecture/resource/security re-review complete with no blocking material finding.
- [ ] candidate protected-integrated/read back before A4 material allocation.

## Excluded scope

No runtime Rust, vendor, Cargo/lockfile, SQL/migration, workflow/ruleset, #356/#335 material mutation, Platform write, production/deployment/secret, merge, Merge Queue submission or architecture self-acceptance.

## Validation

### Focused readback

PASS for protected main, live #351/#356, #329/#335, #364, #451/#453/#458, #588/#589, accepted DFR, architecture discipline, exact Child B durability source and exact #356 SQLx/rustls/pool/config/auth source. Revision-3 changes are confined to the accepted independent-review P1/P2 findings and task lifecycle metadata.

### Component/integration/E2E

`NOT_APPLICABLE`: this PR remains docs-only and changes no product behavior.

### Previous exact-head evidence

Predecessor candidate head `8f5e45dd1a7bdaf28dbf088543518f40cea38e29` is historical after the P1/P2 repair. Its checks/review evidence cannot qualify the Revision-3 successor exact head.

### Final exact-head evidence convention

Per `docs/agents/tasks/TASK_TEMPLATE.md`, a commit cannot contain its own SHA. The final exact candidate SHA and freeze time are therefore recorded in immutable PR/check evidence after the final commit exists. `head_sha`, `final_head_sha` and `final_head_frozen_at` use `external_pr_evidence` here to avoid an impossible self-referential follow-up commit.

## Self-review

- no runtime/product/external-repository path mutation;
- no architecture acceptance claim;
- no fabricated `I/R/T` byte bound;
- P1 is resolved by a single explicit lifecycle/non-overlap model rather than adding a hidden retirement allowance;
- P2 is resolved without timer polling, unbounded reconnect spin or active-path connect;
- R21 retained: one connection is not claimed performance-sufficient from serialization alone;
- protected AWS-LC authority is distinguished from unmerged #455 and unprotected #356 material code;
- current protected ring profile is distinguished from the proposed accepted first-slice AWS-LC profile;
- all broad-removal actions remain conditional on replacement proof.

## Context checkpoint

```yaml
last_progress: Revision-3 repairs only independent-review P1/P2 by freezing lifecycle-complete R/T non-overlap and exact demand-triggered root recovery; successor exact-head qualification is in progress
status: validating
branch: arch/wp3-v2-superseding-decision-20260912
head_sha: external_pr_evidence
pr: 590
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
blocker: fresh successor exact-head repository checks plus genuinely independent exact-head re-review before architecture acceptance
next_action: read the final successor PR #590 head from immutable GitHub evidence, run all required exact-head checks on it, then stop before architecture acceptance pending independent successor re-review
```