# OTV2 WP3-A upstream-first programme / acceptance / allocation amendment

Date: 2026-09-16
P1-A amendment date: 2026-09-19
P1-A finite-topology amendment date: 2026-09-20
Status: `PROSPECTIVE_NOT_ACTIVE`
P1-A amendment authority: #162 comments `5745227522` + `5748461271`; the latter accepts `WP3_DEDICATED_BOUNDED_TOKIO_RUNTIME/v1` and closes review P1 `4054801878` at the architecture-decision layer
Repository: `Oteryn/Oteryn-Game`
Owning control plane: `OTV2_WORK_DELIVERY_COORDINATOR` / #162
Policy base: protected `main@1995bd97460774ea9fc136959d5548471b81c987` / #634
META binding: `Oteryn/Oteryn@d9419b05eb98c81279297563c11fc90e4fe708ac`, policy 3.1.0

This is the minimum tracked amendment required to execute the protected
`PLAYABLE_FIRST / MINIMUM_SUFFICIENT_CHANGE / UPSTREAM_FIRST /
PATCH_ON_PROVEN_NEED` direction for WP3. It does not activate a writer merely by
existing on a branch or PR. Protected integration/readback plus a fresh #162
capability/custody admission remains required before source mutation.

When protected, this document supersedes only the broad-fork-first dispatch and
allocation assumptions in the current WP3 programme, Gate-1 allocation, live
allocation record, #351 task and SQLx-driver plan where they conflict with this
amendment. Historical evidence and already-proven test results remain evidence;
they are not erased.

The companion `OTV2_WP3_A_Q01_Q75_TRANSITION_DISPOSITION_20260916.md` is part of
this amendment. Its authority-backed per-property routing also supersedes the
historical blanket Q01-Q75-before-WP3-A release rule after protected adoption.
It does not supersede accepted security, durability or numeric resource values.
Owner correction #635 comment `5698458209` and the unmerged #636 prompt delta
motivate this repair; neither is being represented as protected acceptance.

## 1. Selected WP3-A lineage

WP3-A SHALL use a clean candidate based on the protected Game `main` selected at
fresh worker admission, under the existing #351 programme issue.

Planned implementation branch after protected readback and explicit #162
admission:

`agent/wp3-a-upstream-first-351`

Do not create or mutate that implementation branch before activation.

PR #356 / `agent/sqlx-driver-budget-351` remains OPEN/DRAFT research and evidence.
Preserve its complete history, source census, hostile regressions, PostgreSQL/TLS
evidence and useful mechanisms. It is not the default terminal production
candidate and SHALL NOT be reset, rebased, force-pushed, auto-merged or completed
wholesale merely because the old implementation exists.

Rationale: current #356 is a broad four-dependency research lineage. The
protected policy requires the smallest sufficient final candidate, and a clean
line makes the integration/review surface equal to the mechanisms actually
retained instead of requiring normal commits that unwind hundreds of unrelated
vendor changes.

## 2. Dependency strategy

The default WP3-A production dependency set is:

- pinned Tokio `1.53.1` keeps upstream runtime semantics; #162 comment `5748461271` fixes one WP3-owned dedicated MultiThread runtime (`worker_threads=1`, `max_blocking_threads=1`, `thread_stack_size=2 MiB`) and admits only the exact read-only dedicated-runtime + maintenance representation proof required by section 3.7; no generic Tokio owner/allocator/scheduler fork is admitted;
- upstream rustls `0.23.45`; no rustls source fork. Protected security PR #623 supersedes the historical Gate-1 `0.23.43` pin; do not downgrade;
- AWS-LC selected through supported features;
- explicit `rustls/prefer-post-quantum` feature unification where required by
  the accepted KX profile;
- upstream SQLx `0.9.0` semantics everywhere except the smallest source-proven
  seams listed below.

SQLx `0.9.0` is the selected version, not a claim about the latest public
release. The following upstream references remain historical semantic
provenance; their current merge/release state is not asserted here:

- `transact-rs/sqlx#3832`: deterministic PostgreSQL options without ambient environment;
- `transact-rs/sqlx#4102`: separate `hostaddr` transport routing from TLS host identity;
- `transact-rs/sqlx#4350`: bounded return-to-pool ping on an unresponsive peer;
- `transact-rs/sqlx#4051`: custom rustls configuration direction only.

No downstream dependency customization is authorized for a hypothetical
benefit. Each retained seam below is tied to an accepted current requirement
that exact SQLx 0.9.0 cannot express through its current public API.

## 3. Minimum justified SQLx seams

### 3.1 Deterministic no-ambient PostgreSQL options

The production root must not inherit `PG*`, `.pgpass` or the OS username. SQLx
0.9.0 `new_without_pgpass()` still reads ambient PostgreSQL environment state.
Retain only the smallest generic deterministic constructor/builder semantics.
Do not publish Oteryn-specific public fields or freeze SQLx internal
representation.

### 3.2 Literal transport IP with separate TLS DNS identity

The accepted first slice uses literal-IP TCP routing and a separately supplied
TLS server name under `VerifyFull`. SQLx 0.9.0 conflates the connection host and
TLS hostname; its `hostaddr` parser overwrites the host used for TLS identity.
Retain only the smallest generic `host_addr`-style seam so TCP uses the literal
address while certificate/hostname verification uses the configured host.

### 3.3 Strict TLS profile

The accepted production root requires `VerifyFull`, inline CA material,
TLS1.3-only and the accepted AWS-LC/PQ provider profile. SQLx 0.9.0's rustls path
uses safe default protocol versions and does not expose a stable public hook that
can express the complete profile. Add only the smallest generic TLS
configuration/protocol seam needed by PostgreSQL. Do not wholesale backport
SQLx #4051 and do not fork rustls source.

Preserve the accepted Revision-3 profile restrictions, not just the positive TLS
test: resumption is disabled unless separately source-bounded; certificate
compression is not admitted without exact reachability/bounds proof; no client
certificate/key authentication or implicit root-file discovery. Prove these
through the selected configuration and source graph. Historical hook tests for
excluded features do not become WP3-A requirements, but an unenforced exclusion
is not a valid qualification result.

### 3.4 SCRAM-SHA-256-only authentication

SQLx 0.9.0 can accept passwordless, cleartext and MD5 authentication during
PostgreSQL establishment. The production root must reject those downgrade
modes. Retain only a neutral strict authentication policy in PostgreSQL options
and establishment. Existing SQLx SASL encoding already uses ordinary
`SCRAM-SHA-256` when `plus=false`; no custom SASL algorithm is authorized.

### 3.5 Same-generation holder finality

WP3-A must distinguish terminal return-to-idle, terminal retirement/close and no
terminal evidence for the exact holder generation that performed the semantic
transaction. SQLx 0.9.0 privately computes return-vs-close but its public
`PoolConnection::return_to_pool()` erases that result.

Expose only a generic observed return/retirement disposition. `self.live == None`
and repeated calls are no-evidence, not fabricated terminal success. Preserve
ordinary SQLx pool behavior and do not modify `PoolInner::release` unless fresh
source proof shows it is unavoidable.

The same path must be bounded by the unchanged semantic-pass deadline. A silent
peer cannot hold the pool permit indefinitely. Deadline expiry must hard-retire
the exact holder before `RetiredClosed` is considered terminal; dropping a
future is not finality evidence.

### 3.6 Transaction ownership

WP3-A SHALL first use a root-owned `PoolConnection` with a borrowed SQLx
transaction. If Rust 1.94 compile and PostgreSQL qualification prove this shape,
standard `Transaction::commit/rollback` is used and the Oteryn-specific
`oteryn_m05_commit` / `oteryn_m05_rollback` transaction-extraction hooks from
#356 are not carried forward.

If exact compile/runtime evidence proves a missing generic SQLx primitive,
return `SHARED_LEASE_REQUIRED` with the exact path/symbol before expanding the
dependency patch. Do not assume the old M05 transaction implementation is
required merely because #356 contains it.

### 3.7 Dedicated WP3 runtime + root-maintenance backing: exact P1-A exception

The owner decision in #162 comment `5748461271` supersedes the insufficient
MultiThread-flavor-only closure while preserving the narrow Revision-4
maintenance-task exception.

Production WP3 root qualification now requires one **WP3-owned dedicated Tokio
`1.53.1` MultiThread runtime** with exactly:

```text
worker_threads = 1
max_blocking_threads = 1
thread_stack_size = 2 MiB
```

The root must not qualify an arbitrary ambient Tokio `Handle`. Current-thread
runtimes remain test-only/non-production.

For the exact accepted WP3 runtime/root graph only:

- all production root-owned Durability spawning must bind to the explicit
  root-owned runtime/handle rather than ambient `tokio::spawn` selection;
- pinned Tokio may expose/read source-derived representation facts sufficient to
  prove the exact one-worker scheduler/control backing, enabled driver/runtime
  control backing, configured stack topology, reachable blocking-pool backing,
  and the exact SQLx root-maintenance future/task allocation;
- the existing maintenance-task query remains limited to the
  `BOX_FUTURE_THRESHOLD` optional future-box request and exact MultiThread
  task-cell request;
- the representation seam provides facts only: no allowance, allocator
  interception, generic scheduler/task API, owner propagation, retry policy or
  lifetime policy;
- SQLx retains one root-specific maintenance construction that removes the
  heap-allocating `CloseEvent/EventListener` dependency while preserving the
  accepted silent reaper/shutdown semantics;
- Game must reserve every attributable source-derived dedicated-runtime and
  maintenance backing charge against the same root `I` ledger before root
  acceptance/allocation and retain it through complete runtime/task/shared-tail
  finality;
- `max_blocking_threads = 1` is a finite cap, not authorization to introduce
  blocking work; exact reachability must prove whether a blocking worker can
  exist in the accepted graph;
- the explicit `2 MiB` stack removes `RUST_MIN_STACK`/platform selection from
  the WP3 contract, while evidence must keep virtual stack reservation distinct
  from resident/committed DFR accounting;
- denial is fail-closed; no second budget or resource-registry value is introduced.

If the exact selected graph cannot fit
`I + max(R,T) + Q + A <= 12 MiB`, implementation stops and escalates rather than
changing the topology, transferring scheduler backing outside `I`, or widening
the budget.

The seam does not authorize reuse by unrelated Tokio tasks, Game-wide runtime
topology, broad SQLx/Tokio resource-owner propagation, rustls changes, alternate
reaper policy, WP4/WP5/Server-Seam, or `fresh_admission.rs` mutation.

## 4. Oteryn-owned WP3-A execution model

The application layer owns semantic custody; the exact dependency seams below expose only the minimum representation/finality mechanics and do not own policy:

- production WP3 durability root qualification requires the owner-accepted dedicated Tokio `1.53.1` MultiThread runtime with `worker_threads=1`, `max_blocking_threads=1`, `thread_stack_size=2 MiB`; current-thread and arbitrary ambient runtime/Handle qualification are forbidden in production;
- one process-scoped durability root and lazy max-one holder pool;
- ready-only active checkout; an active pass never establishes a new connection;
- a miss returns the accepted unavailable classification and coalesces one
  `root_ready_demand`;
- connection establishment/recovery runs only in serialized root maintenance
  outside active work and only after prior holder tails are terminal;
- one semantic pass binds exact backend/slot/kind/original/incarnation and one
  immutable absolute deadline;
- the same deadline covers begin/setup, generation/fencing SQL, relation and
  advisory locks, semantic SQL, COMMIT/ROLLBACK, same-generation finality and
  any reconciliation that is part of that same authorized semantic pass; it is
  never reset per phase;
- a persisted ambiguous original that outlives its mutation deadline may enter
  only a separately authorized bounded reconciliation-only window. That window
  may determine durable outcome/cleanup for the same original but cannot
  re-authorize the expired mutation or mint a new original identity;
- root-owned Tokio task lifetime retains issued semantic work after caller
  cancellation; caller disappearance cannot acknowledge or release custody;
- healthy definitive holders return to idle;
- broken, ambiguous or return-timeout holders hard-retire before capacity and
  finality are released;
- `ReturnedToIdle` may make the existing generation ready; `RetiredClosed`
  leaves the root not ready;
- failed root maintenance does not self-retry/spin. A later establishment window
  requires a genuine new/coalesced demand under the protected root state
  machine;
- active-slot reuse requires definitive semantic outcome, exact same-generation
  holder finality and exact owner acknowledgement.

This model preserves the correctness intent of the useful #356 mechanisms while
moving scheduler/cancellation ownership into Oteryn-owned code over upstream
Tokio.

Retain the accepted bounded no-ambient configuration and built-in PostgreSQL
type profile, including the selected statement-cache capacity. Exact numbers,
source authority and local-versus-composed proof are in companion N01-N32.
No arbitrary options/application name, generic type discovery or extra resource
allowance is introduced by moving ownership into the application layer.

## 5. WP3-A source allocation after activation

After this amendment is protected and freshly read back, #162 may activate one
sole WP3-A writer only after current overlap/custody reconciliation and trusted
integration capability are positive.

The initial allowed production surface is bounded to:

- `Cargo.toml` and `Cargo.lock` only for the exact dependency-feature/path patch
  consequences selected by this amendment;
- `apps/game-server/src/durability/db.rs`;
- `apps/game-server/src/durability/mod.rs`;
- `apps/game-server/src/durability/schema.rs`;
- `apps/game-server/src/durability/admission_journal.rs`;
- `apps/game-server/tests/durability_postgres.rs`, only the exact WP3 registered
  root/finality/recovery/TLS-auth qualification modules and named tests;
- clean exact-upstream SQLx 0.9.0 source/provenance needed to provide the seams
  in section 3, with authored changes restricted to the smallest affected
  symbols in `sqlx-postgres` and `sqlx-core`, including only the root-specific
  maintenance construction required by section 3.7;
- pinned Tokio 1.53.1 source/provenance only for the section-3.7 read-only
  representation proof of the exact accepted dedicated one-worker runtime and the
  exact SQLx root-maintenance future/task, including only source facts needed for
  scheduler/control/driver backing, configured stack topology, blocking reachability,
  optional future-box and task-cell sizing.

The implementation allocation explicitly excludes:

- all Tokio source/vendor changes except the exact section-3.7 read-only representation query; generic task/allocator/scheduler/resource-owner instrumentation remains forbidden;
- all rustls source/vendor changes;
- broad SQLx resource-owner/decoder/cache rewrites unless a still-current hard
  requirement is newly reproduced and separately admitted;
- SQLx `transaction.rs` when the borrowed-transaction holder shape qualifies;
- workflows, rulesets, protection, deployment, production, secret/certificate
  or live-data mutation;
- WP4/WP5/Server-Seam source paths.

`apps/game-server/src/durability/fresh_admission.rs`, including
`FreshAdmissionStore::commit_fresh_loss`, remains in Child-B/shared custody.
This amendment does not transfer it. If the final WP3-A implementation proves
that exact symbol is a prerequisite rather than a downstream B consumer, #162
must protect a separate exact shared-custody amendment before mutation.

## 6. Authority-backed acceptance obligations

The companion is the complete per-Q and numeric routing ledger for this
amendment. No unlisted remainder of Q01-Q75 is automatically required or marked
blocked. Classify the underlying property, not the existence of a #356 hook:

- A: current correctness/security/durability/compatibility/resource invariant,
  with exact authority and minimum selected-root proof;
- H: historical mechanism evidence only, with the surviving A property named;
- B: representative qualification at the real-product T-P trigger;
- D: exact downstream T-B/T-G composition, not a circular B-before-B admission;
- S: accepted profile supersession, whose enforcement still requires Q61 proof;
- U: exact missing authority/reachability evidence at T-U, not an invented
  blocker or an assumed safe exclusion.

Routing and execution result are different. The 75 row dispositions do not
create PASS claims. For an actually required A property, record exact candidate
proof or the precise missing proof/defect and affected release. Do not create a
new failing-test claim simply because material implementation has not started.
No historical hook needs reimplementation unless it is retained or independently
necessary for that current property. Current resource-denial, hostile-input and
failure regressions are not deferred as representative performance work.

### Numeric authority, including 12 MiB

Protected `RESOURCE_LIMITS_REGISTRY.json` explicitly registers
`DFR-TOTAL-RESIDENT-BYTES = 12,582,912`: queue plus active charged resident work,
not process RSS. Revision 3's `I + max(R,T) + Q + A <= 12 MiB` root model was
accepted through protected #589 and #162 comment `5652184410`; historical
candidate headings do not negate that acceptance. These are independently
verified current requirements, not inherited merely because #356 used them.

Preserve the value, one-root custody, strict R/T generation non-overlap and
lifecycle-complete charges. Prove the actual WP3-A root and owned work; full B
producer/consumer composition closes at T-B. I/R/T/active component byte values
are UNKNOWN until candidate proof, not invented blocker numbers or zero. Q/A
mean actual coupled charges, not mandatory simultaneous occupancy at every
individual maximum. Representative sizing/tuning is B, but does not waive the
accepted safety bound.

The required proof may use defensible finite reservations, bounded upstream APIs
and focused source/boundary/failure evidence. It does not mandate a broad vendored
Tokio/rustls per-allocation instrumented runtime. The P1-A amendment explicitly
admits only the section-3.7 pinned-Tokio representation query and root-specific
SQLx maintenance construction; no additional dependency seam may be inferred.
If that exact selected graph cannot meet an accepted bound or preserve the frozen
reaper/finality semantics, identify the gap and return to architecture/lease
discipline rather than widening implementation custody. The companion separates
every relevant numeric limit, historical observation, profile pin and missing
numeric proof.

## 7. Mandatory WP3-A regressions and qualification

At minimum the admitted exact candidate must cover the current A properties
below and in the companion, restricted to its actual selected root/owned paths.
The full B consumer is qualified at T-B; no fake mini-server or B fixture may
stand in for either the real WP3-A root or later composed production proof.

### Configuration / TLS / auth

- hostile `PG*`, `.pgpass` and OS-user fallback negatives;
- literal-IP TCP + correct DNS `VerifyFull` positive;
- wrong DNS and wrong CA negatives;
- TLS1.2 rejected and TLS1.3 accepted;
- exact resolved AWS-LC provider plus accepted PQ KX order;
- Cleartext, MD5, passwordless and unsupported-auth downgrade rejection;
- ordinary SCRAM-SHA-256 positive;
- no plaintext fallback;
- enforced first-slice exclusions and bounded retained config/diagnostics,
  including the resumption/compression/client-certificate restrictions above.

### Finality / cancellation / recovery

- caller cancellation after issued semantic work does not free custody or cancel
  the finality obligation;
- COMMIT success + exact returned-idle finality;
- ROLLBACK success + exact returned-idle finality;
- ambiguous COMMIT + exact hard retirement + durable reconciliation;
- silent peer during return -> bounded hard retirement without permit overlap;
- no successor connection can serve as evidence for original-generation
  finality;
- empty-holder ready miss cannot connect inside an active pass;
- one coalesced root recovery window, no self-retry, and a separate case where a
  genuine event during the first window authorizes exactly one later window;
- third sequential operation only after exact completion/finality/owner ACK;
- restart/takeover fencing and retained original identity.

### Maintenance-task/runtime qualification

- production root owns exactly one Tokio `1.53.1` MultiThread runtime configured
  with `worker_threads=1`, `max_blocking_threads=1` and
  `thread_stack_size=2 MiB`;
- production root rejects current-thread, unavailable or arbitrary ambient runtime
  substitution before root/pool acceptance;
- all production WP3 root-owned spawned work uses the explicit root-owned
  runtime/handle;
- source-derived prospective proof covers the selected one-worker scheduler/control
  backing, enabled runtime driver/control backing, exact configured stack topology,
  any actually reachable blocking-worker backing, and the exact SQLx maintenance
  future/task backing;
- blocking-worker reachability is proven explicitly; a finite cap is not evidence
  that the worker exists or that its backing may be ignored;
- exact SQLx maintenance future uses source-derived optional future-box and
  MultiThread task-cell allocation requests with no literal task-byte constant;
- the root-specific SQLx maintenance path removes the
  `CloseEvent/EventListener` heap dependency for this task without changing
  shutdown/reaper/finality behavior;
- same-root `I` reservation max/max+1 denial occurs before runtime/root/
  maintenance allocation and the reservation remains charged through complete
  runtime/task/shared-tail finality;
- virtual stack reservation is reported separately from resident/committed
  evidence; process RSS is not DFR accounting proof;
- 10-minute idle / 30-minute max-lifetime, max1/min0, ready-only active work,
  strict R/T non-overlap and coalesced demand/recovery behavior remain unchanged.

### Current resource and compatibility boundary

Prove accepted local root/queue/active bounds, reachable hostile PG lengths and
counts before uncontrolled allocation, partial denial/cleanup, bounded SQL and
migration transfers, and no hidden reduction of accepted reconnect maxima.
These are current properties, not a demand to complete discarded hook tests.
Full B SQL corpus, sibling lock inventory and producer-to-completion ownership
are explicit T-B/T-G obligations, with local A portions preserved in the ledger.

### Retained audit findings

`AUDIT-LIVE-RECONCILE` must receive an exact protected disposition. If the
relevant reconciliation/executor path is present in WP3-A, prove same-runtime
resolution of a persisted ambiguous original after its old mutation deadline
expired, without a new identity, duplicate effect, blind mutation-deadline reset
or premature slot release. Any new reconciliation-only window must be separately
authorized and must not re-authorize the original mutation. If the only reachable
reproducer remains in B-owned `fresh_admission.rs`, retain this finding as an
explicit Child-B release prerequisite with the same evidence requirements; this
amendment does not seize that B-owned path merely to close the audit finding.

`AUDIT-ROOT-DEMAND` must prove a failed maintenance window does not re-arm itself
without a new event, while preserving a genuine independently arriving/coalesced
ready demand.

`AUDIT-ERROR-CLASSIFICATION` must be resolved against the exact protected
acceptance boundary: preserve required source-specific inspection before bounded
redaction and never retain/format arbitrary untrusted SQLx/TLS payload merely to
match historical #356 implementation shape.

## 8. Required gates

After explicit #162 activation:

1. Rust 1.94 compile and focused tests on the exact selected source graph;
2. exact Cargo dependency/feature/provenance verification;
3. configured PostgreSQL 17.6 registered-root execution; an unconfigured skip is
   not evidence;
4. TLS/auth downgrade qualification and same-generation finality/recovery tests;
5. exact disposition of the retained #633 findings and every current A property;
6. producer full-diff review;
7. genuinely independent HIGH-risk exact-head whole-diff review: explicit PASS,
   P0=0, P1=0, P2=0 and BLOCKING_EVIDENCE_GAP=0 for the affected current gate;
8. exact-head repository CI and zero unresolved material review threads;
9. integration only through the authenticated bound META 3.1 exact-head Merge
   Queue route;
10. real `merge_group` `game-gate` SUCCESS plus protected-main readback.

Only after protected integration/readback and coordinator closeout may WP3-A be
`DONE` and Child B/WP4 be considered for explicit release. Downstream D evidence
is required at its named release, not before its own readmission. Representative
B work remains triggered; no current invariant or existing required gate is waived.

## 9. Activation state

At this amendment candidate the truthful state is:

```text
WP3_A_STRATEGY = UPSTREAM_FIRST_MINIMAL_PATCH_ON_PROVEN_NEED
WP3_A_LINEAGE = CLEAN_FROM_PROTECTED_MAIN_AFTER_ACTIVATION
WP3_A_IMPLEMENTATION_BRANCH = agent/wp3-a-upstream-first-351 (RESERVED_NOT_CREATED)
WP3_356_DISPOSITION = PRESERVED_RESEARCH_EVIDENCE_NOT_TERMINAL_CANDIDATE
PRODUCTION_WP3_RUNTIME = DEDICATED_TOKIO_1_53_1_MULTI_THREAD__WORKERS_1__BLOCKING_1__STACK_2_MIB
AMBIENT_TOKIO_HANDLE = FORBIDDEN_FOR_PRODUCTION_ROOT_OWNERSHIP
TOKIO_SOURCE_CUSTOMIZATION = EXACT_P1A_READ_ONLY_DEDICATED_RUNTIME_AND_MAINTENANCE_REPRESENTATION_QUERY_ONLY_AFTER_PROTECTED_AMENDMENT_AND_FRESH_LEASE
RUSTLS_SOURCE_FORK = NOT_ALLOCATED
SQLX_SOURCE_CUSTOMIZATION = MINIMUM_PROVEN_SEAMS_PLUS_ROOT_SPECIFIC_P1A_MAINTENANCE_CONSTRUCTION_ONLY
P1A_MAINTENANCE_I_CHARGE = SAME_ROOT_LEDGER_PREALLOCATION_THROUGH_SHARED_TAIL_FINALITY
FRESH_ADMISSION_SHARED_CUSTODY = UNCHANGED_CHILD_B
Q_ROUTING = EXPLICIT_PROPERTY_AUTHORITY_AND_MILESTONE_IN_COMPANION
NUMERIC_ROUTING = N01_N32_AUTHORITY_AND_SCOPE_IN_COMPANION
ALLOCATION_STATE = PROSPECTIVE_NOT_ACTIVE
CURRENT_P1A_IMPLEMENTATION_AUTHORITY = NONE_UNTIL_REVISION5_PROTECTED_AND_FRESH_COORDINATOR_LEASE
WORKER_STATE = NOT_ADMITTED
TRUSTED_INTEGRATION_CAPABILITY = MUST_BE_FRESH_AT_WORKER_RELEASE
WP4_CHILD_B = HOLD
SERVER_SEAM_247 = WAITING_DEPENDENCY
```

Protected integration of these two documents is necessary but not sufficient for
source activation. The active Work coordinator must freshly read protected main,
#162/#364/#351/#356/#329/#335, open material PRs and path ownership, obtain the
current trusted capability decision, then record the exact admitted main/branch
and writer before source mutation.
