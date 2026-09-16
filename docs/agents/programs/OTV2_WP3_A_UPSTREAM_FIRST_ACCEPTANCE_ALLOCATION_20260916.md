# OTV2 WP3-A upstream-first programme / acceptance / allocation amendment

Date: 2026-09-16
Status: `PROSPECTIVE_NOT_ACTIVE`
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

- upstream Tokio `1.53.1` from the normal dependency graph; no Tokio source fork;
- upstream rustls `0.23.45`; no rustls source fork. Protected security PR #623 supersedes the historical Gate-1 `0.23.43` pin; do not downgrade;
- AWS-LC selected through supported features;
- explicit `rustls/prefer-post-quantum` feature unification where required by
  the accepted KX profile;
- upstream SQLx `0.9.0` semantics everywhere except the smallest source-proven
  seams listed below.

As of this amendment, upstream SQLx `v0.9.0` remains the latest stable upstream
tag. Open upstream work is retained as semantic provenance, not treated as a
released dependency:

- `transact-rs/sqlx#3832`: deterministic PostgreSQL options without ambient environment;
- `transact-rs/sqlx#4102`: separate `hostaddr` transport routing from TLS host identity;
- `transact-rs/sqlx#4350`: bounded return-to-pool ping on an unresponsive peer;
- `transact-rs/sqlx#4051`: draft custom rustls configuration direction only.

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
configuration/protocol seam needed by PostgreSQL. Do not wholesale backport the
current draft SQLx #4051 and do not fork rustls source.

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

## 4. Oteryn-owned WP3-A execution model

The application layer, not Tokio/rustls forks, owns semantic custody:

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
  symbols in `sqlx-postgres` and `sqlx-core`.

The implementation allocation explicitly excludes:

- all Tokio source/vendor changes;
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

## 6. Acceptance obligations

This amendment changes implementation strategy, not correctness meaning.
Current correctness/security/durability/compatibility invariants remain binding,
including exact original identity, fencing/nonreuse, replay/reconciliation,
ambiguous COMMIT, cancellation/completion ownership, reconnect/restart,
explicit TLS/auth/configuration and fail-closed error handling.

No canonical `WP3-Q01..WP3-Q75` item becomes PASS merely because this amendment
is protected. For closure, each item must be classified from exact candidate
evidence as one of:

- `PROVEN_CURRENT` — required now and proven on the exact WP3-A candidate;
- `CURRENT_BLOCKER` — required now and not yet proven;
- `WP3_B_TRIGGERED` — representative performance/resource/failure measurement
  whose current correctness/safety floor is already met; record owner, exact
  trigger and expected evidence;
- `SUPERSEDED_BY_PROTECTED_DECISION` — only with the exact superseding protected
  architecture/contract reference.

The historical equation `I + max(R,T) + Q + A <= 12 MiB`, root/retirement
ownership and hard resource-denial requirements are not silently waived. WP3-A
must prove the still-binding current safety floor with the minimum sufficient
mechanism. Broad per-allocation instrumentation and representative
performance/resource optimization move to WP3-B only where the protected
classification says they are not current blockers. Any reproduced current
resource/security failure triggers immediate re-entry to the owning lane.

## 7. Mandatory WP3-A regressions and qualification

At minimum the admitted exact candidate must cover:

### Configuration / TLS / auth

- hostile `PG*`, `.pgpass` and OS-user fallback negatives;
- literal-IP TCP + correct DNS `VerifyFull` positive;
- wrong DNS and wrong CA negatives;
- TLS1.2 rejected and TLS1.3 accepted;
- exact resolved AWS-LC provider plus accepted PQ KX order;
- Cleartext, MD5, passwordless and unsupported-auth downgrade rejection;
- ordinary SCRAM-SHA-256 positive;
- no plaintext fallback.

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
5. exact disposition of the retained #633 findings;
6. producer full-diff review;
7. genuinely independent HIGH-risk exact-head whole-diff review;
8. exact-head repository CI and zero unresolved material review threads;
9. integration only through the authenticated bound META 3.1 exact-head Merge
   Queue route;
10. real `merge_group` `game-gate` SUCCESS plus protected-main readback.

Only after protected integration/readback and coordinator closeout may WP3-A be
`DONE` and Child B/WP4 be considered for explicit release.

## 9. Activation state

At creation of this amendment candidate the truthful state is:

```text
WP3_A_STRATEGY = UPSTREAM_FIRST_MINIMAL_PATCH_ON_PROVEN_NEED
WP3_A_LINEAGE = CLEAN_FROM_PROTECTED_MAIN_AFTER_ACTIVATION
WP3_A_IMPLEMENTATION_BRANCH = agent/wp3-a-upstream-first-351 (RESERVED_NOT_CREATED)
WP3_356_DISPOSITION = PRESERVED_RESEARCH_EVIDENCE_NOT_TERMINAL_CANDIDATE
TOKIO_SOURCE_FORK = NOT_ALLOCATED
RUSTLS_SOURCE_FORK = NOT_ALLOCATED
SQLX_SOURCE_CUSTOMIZATION = MINIMUM_PROVEN_SEAMS_ONLY
FRESH_ADMISSION_SHARED_CUSTODY = UNCHANGED_CHILD_B
ALLOCATION_STATE = PROSPECTIVE_NOT_ACTIVE
WORKER_STATE = NOT_ADMITTED
TRUSTED_INTEGRATION_CAPABILITY = MUST_BE_FRESH_AT_WORKER_RELEASE
WP4_CHILD_B = HOLD
SERVER_SEAM_247 = WAITING_DEPENDENCY
```

Protected integration of this document is necessary but not sufficient for
source activation. The active Work coordinator must freshly read protected main,
#162/#364/#351/#356/#329/#335, open material PRs and path ownership, obtain the
current trusted capability decision, then record the exact admitted main/branch
and writer before source mutation.
