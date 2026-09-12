# OTV2 WP3 — Round 3: consumer lifecycle and proof-boundary audit

- Date: 2026-09-12.
- Repository: `Oteryn/Oteryn-Game`.
- Mode: source audit and retained evidence; documentation changes only.
- Parent: [shared architecture audit](OTV2_WP3_ARCHITECTURE_COMPLEXITY_AUDIT_20260912.md), including its second-agent section 16.
- Parent report revision: `aea03a69b5b6526eea4af1b413286f3ed2865ccd`.
- Protected main readback: `489e3e390a1bce1ce3439c66521ab75f8a826cd8`.
- WP3 #351 / #356: `agent/sqlx-driver-budget-351@fe7891989b1247012e32c89c10cff6a10bacb943`.
- Child B / WP4 #335: `agent/durable-fresh-admission-child-b-329@834db1d7118d751e31287715d3eaac7780a0c7b9`.
- Authority: NONE. This supplement neither supersedes protected contracts nor freezes, activates, implements or integrates another worker's allocation.

## 1. Executive verdict

```text
WP3 architectural health: RED
Recommended WP3 path: REDESIGN_BOUNDARY
Confidence in the source-backed NO-GO: HIGH
Completeness of executable qualification: NOT ESTABLISHED
```

The second-agent pool-lifecycle analysis identifies a real missing boundary: bootstrap and idle pooled connections exist outside a particular active operation. However, its proposed smaller architecture is not yet a proven replacement. Source inspection found overlapping pool-maintenance entry points, work after an explicitly awaited return, asynchronous TLS work even in `close_hard`, an earlier unbounded schema inspection, and ambient configuration input outside the proposed address/certificate profile. These do not justify abandoning the useful budget primitive. They require a more precise ownership and qualification contract before replacing either the current forks or the accepted consumer topology.

This round contains twelve focused observations and twenty-four refinements to the earlier forty-six verification obligations. They are not twelve reproduced vulnerabilities or seventy executed tests. No new native Rust, PostgreSQL, concurrency or performance test was executed by this audit. Findings about Child B refer to its unmerged revision, not deployed production.

## 2. Verified live state and evidence boundary

#356 remains open, draft and unmerged. #335 is open and unmerged; fresh API metadata reports `draft=false` and `mergeable=false`, correcting the parent addendum's draft label without changing its source findings. The audit branch remains separate from the implementation branch.

The exact WP3 head still has successful PR runs: Merge gate `34699293933`, Agent governance `34699293903`, Architecture semantic audit `34699293887`. The previously inspected Linux job `103568134958` contains an explicit PostgreSQL step with 125 passed / 0 failed and the funded AWS-LC TLS qualification. These are existing CI results, not new executions. The artifact-list response for run `34699293933` was empty. A successful dispatcher/workflow must not be reclassified as proof of an unselected semantic profile. [BT]

The sandbox has Git but no available Cargo/rustc; a public GitHub HTTPS request fails DNS resolution. Repository content is readable through the GitHub connector. Available workflow actions expose reads and retries, not an arbitrary audit execution/dispatch interface. No unchanged successful heavy run was retriggered. Complete local repository governance, byte-for-byte upstream comparison, new Rust tests and benchmarks are not claimed.

Bound root and `docs/agents/AGENTS.md` instructions were retained at the verified immutable revision, together with META policy 3.1.0 at `Oteryn/Oteryn@3b39e0be05aef008f1bd442821daefa898a201dd`. Retained evidence belongs here, not in the accepted architecture tree. Preserve the original report and the second agent's additions.

Evidence labels below mean: **PROVEN** source/API observation; **DERIVED** a stated consequence of those observations; **RECOMMENDATION** proposed action; **UNKNOWN** missing qualification. Severity ranks delivery risk, not a CVSS score or a claim of exploitation.

## 3. Real requirement: protect the complete consumer, not one connect call

The accepted root covers queued and active work and operation-attributable runtime/driver backing. It is not a process-RSS limit. It requires complete immutable operation custody before effectful submission, bounded transfer in the protected SQL snapshot, and no slot reuse while unresolved work or completion remains. Two ambiguous slots intentionally may stop new work; that is not itself a defect. [DFR]

**PROVEN:** `registered_backend` creates the pool, inspects schema and acquires durable executor custody before constructing `WorkCustody`. The accepted consumer therefore needs funding before any later queued/active operation. The direct owner-aware `PgConnection` helper cannot alone qualify that lifecycle. [BDB] [BS] [BMOD] [HR]

**PROVEN positive control:** `DurabilityCustody::read_pending` already uses a guarded payload projection and exactly two named slots. Do not label every `fetch_all` as unbounded merely because that method appears. The schema-inspection finding below concerns a different query without that guard. [BMOD] [BS]

## 4. Current lifecycle, including previously omitted work

```text
configuration / PG environment / optional pgpass
  -> pool control allocation
  -> eager connect AND possible maintenance connect
  -> schema inspection / executor takeover / pending restoration
  -> queue and active-operation custody
  -> checkout / transaction / result / possible ambiguous commit
  -> cleanup and after_release decision
  -> ping / rollback flush / additional receive work
  -> idle release OR asynchronous close
  -> minimum-connection repair and possible wrapper-drop task
  -> last result/error/cache/driver/runtime owner
```

The existence of this sequence is source-grounded; its full budget and latency qualification remains open. [BDB] [PC] [PI] [PO] [PG] [CFG] [PARSE] [PASS]

## 5. Additional observations

### WP3-R3-01 — HIGH — schema bootstrap transfers before validating the ledger

**PROVEN:** `schema::inspect` runs `SELECT version, checksum, success FROM _sqlx_migrations ORDER BY version ASC` with `fetch_all`, then compares the returned count to the embedded migration set and materializes checksums as `Vec<u8>`. It executes during `connect_runtime`, before `WorkCustody`. [BS] [BDB]

**DERIVED:** unexpected row count or checksum size can consume driver/consumer memory before incompatibility is reported. This is an uncovered bootstrap boundary, not evidence of a new protocol regression introduced by WP3.

**RECOMMENDATION:** bound the compatibility read before transfer, preserve complete mismatch detection and the same-snapshot requirement, and reserve bootstrap working memory. Do not apply a migration-ledger bound to unrelated permanent admission history. **Closure:** oversized/incompatible ledger is rejected without unbounded client materialization; ordinary runtime inspection never performs migration DDL.

### WP3-R3-02 — HIGH — the proposed bounded configuration profile still has ambient inputs

**PROVEN:** URL parsing starts with `new_without_pgpass`, which still reads `PG*` environment values and may use OS username discovery; parsing ends with `apply_pgpass`. If no password is present, pgpass uses synchronous file I/O and an uncapped `read_line` into a retained String. A 4096-byte URL bound does not bound these other inputs. The `hostaddr` and `host` URL fields also currently converge on the same host member. [CFG] [PARSE] [PASS] [BDB]

**DERIVED:** a literal transport address plus preloaded TLS certificates does not by itself remove all configuration I/O or establish finite configuration residency. This is conditional on the exercised configuration path, not a claim that every supplied URL opens a passfile.

**RECOMMENDATION:** explicitly decide allowed ambient configuration and credential sources before initial construction; preserve bounded TLS server identity independently from routing. Do not silently disable a required credential mechanism.

The same review found that malformed pgpass records are logged with the entire line, and unknown URL options with their values. This is a conditional sensitive-data/diagnostic risk; no actual credential leak was observed. Redaction must happen before those library sinks, not only when the final error reaches B. [PASS] [PARSE]

### WP3-R3-03 — HIGH — sequential maintenance loops do not imply one handshake at a time

**PROVEN:** `PoolInner::new_arc` invokes `spawn_maintenance_tasks`; with both lifetime timers disabled, nonzero `min_connections` still spawns initial maintenance. `PoolOptions::connect_with` separately calls `try_min_connections`. That method's loop awaits each connect, but multiple invocations are not globally serialized. The eager-connect source itself notes the race. [PI] [PO]

**DERIVED, high confidence:** the section 16.12 argument cannot use one transient handshake allowance solely because each individual loop is sequential. A proposed two-connection pool can have two establishment attempts in flight. `max_connections` remains a useful connection-count cap; this finding does not claim it is bypassed.

**Closure:** prove a shared connection-establishment admission bound across bootstrap, checkout, maintenance and replacement, or fund the permitted overlap. Any new serialization policy needs review rather than an assumed scheduling order.

### WP3-R3-04 — HIGH — awaiting return does not prove pool quiescence

**PROVEN:** after `PoolConnection::return_to_pool().await` removes `live`, dropping the wrapper still spawns a return/maintenance future when `min_connections > 0`. The explicit return future itself invokes minimum-connection repair when the connection was not returned. Maintenance with no supplied deadline chooses 300 seconds. [PC] [PI]

**DERIVED:** disabling idle/lifetime timers eliminates those periodic checks, not every maintenance task, reconnect attempt or long-lived root reference. The 300-second internal maintenance deadline is not automatically the DFR business-operation deadline and must not be treated as permission to hold an active slot for that period.

**Closure:** operation-specific cleanup and root-funded maintenance need distinct finality evidence. Background repair must neither inherit a released operation budget nor multiply untracked tasks. `return_to_pool` is public but doc-hidden, so any reliance on it should be isolated and pinned. [PC]

### WP3-R3-05 — HIGH — the residual-state hook is before the final ping

**PROVEN:** internal return invokes `after_release`, then pings before releasing to idle. PostgreSQL ping sends Sync and drains pending protocol work. `clear_cached_statements` removes statement entries and clears `cache_type_oid`, but does not reset `cache_type_info`, `cache_elem_type_to_array` or `cache_table_data`. `shrink_buffers` addresses stream buffers, not those maps. [PC] [PG]

**DERIVED:** an `after_release` measurement is not necessarily the final resident state; later receive/status/error work still needs coverage. Zero cached statements is not zero retained connection memory.

**Closure:** establish residency after all cleanup/protocol work, or reserve an upper bound that includes it; close connections whose final accepted residency cannot be established. Do not assume a shrink request proves every backing allocation was reclaimed.

### WP3-R3-06 — HIGH — hard close is still an asynchronous TLS shutdown path

**PROVEN:** `PgConnection::close_hard` awaits stream shutdown. `RustlsSocket::poll_shutdown` sends close-notify and drives `complete_io` before shutting down the underlying socket; readiness may be Pending. [PG] [TLS]

**DERIVED:** the name `close_hard` does not certify immediate physical deallocation or a finite wait under an unresponsive peer. No hang was reproduced by this audit.

**Closure:** test bounded cleanup under stalled I/O and cancellation while retaining the true resource owner. Timeout must not release accounting for still-live resources or reinterpret an uncertain transaction as uncommitted. Do not replace the protocol with an unreviewed abort policy.

### WP3-R3-07 — HIGH — production custody and repeatable slot release remain incomplete

**PROVEN:** `WorkCustody` reserves/promotes entries but cancellation only removes queued work. `checkpoint` explicitly defers clearing to a future definitive-disposition plus owner-acknowledgement layer. `AdmissionRuntime` says semantic APIs are not yet fully routed through that queue. Registration stays `Starting` after failed/uncertain initialization by design. [BDB] [BMOD]

**DERIVED:** the inspected checkpoint does not demonstrate repeated completed operations returning active capacity, nor complete recovery of two occupied slots. This is an explicit unfinished integration obligation, not a reason to clear slots on cancellation.

**Closure:** verify more than two sequential acknowledged operations in one process, same-slot reconciliation with both slots occupied, and safe capacity recovery without a third slot. Separate known pre-effect bootstrap failure from ambiguous takeover in the operational error contract; do not add a blind retry/reset of `Starting`.

### WP3-R3-08 — HIGH — current test successes have narrower finality boundaries

**PROVEN:** the helper creates a runtime before its ledger and checks the final balance only after both connection and runtime destruction. Child B's `backend_for_constructor` uses `LegacyFixture` under `cfg(test)` instead of registered production wiring; explicit `AdmissionRuntime::connect` remains a route to that wiring. [HR] [BDB]

**DERIVED:** combined teardown cannot distinguish prompt connection release from retention until runtime teardown, nor detect a premature debit release merely by a final zero. Legacy fixture success is not production registration/custody success. Cross-crate or explicitly registered tests can still provide valid evidence; they are not invalidated wholesale.

**Closure:** long-lived-runtime churn, positive reuse, final-owner ordering, warm/cold provider state, and production-constructor coverage. Keep the existing real TLS positive and denial tests as useful, narrower controls. [H] [HR]

### WP3-R3-09 — HIGH — shared residency has no automatically available extra allowance

**PROVEN:** the accepted envelope is conjunctive: queue total 4 MiB, two active ceilings of 4 MiB each, root 12 MiB. Attributable infrastructure must fit the existing budgets; a new connection/runtime does not mint another allowance. [DFR]

**DERIVED arithmetic:** reserving every queue/active maximum already totals 12 MiB. Adding any separately counted positive shared reservation to those full reservations exceeds the root. This is not proof that a valid workload is impossible: actual simultaneous residency may be smaller and lawful capacity denial remains possible.

**Closure:** publish a non-double-counted joint reservation schedule and a funded successful witness for required semantic maxima. Completion, retry/ambiguity bookkeeping and cleanup must remain funded before irreversible COMMIT; they are not bonus budgets. Numbers for the proposed replacement remain UNKNOWN.

### WP3-R3-10 — MEDIUM — feature absence needs a resolved production graph

**PROVEN external contract:** enabled features can be unified across dependency edges; `default-features = false` on one edge does not alone establish absence in the complete build. Resolver, target and build/dev/normal dependency contexts matter. [CB]

**DERIVED:** section 16.9 needs a resolved graph and reachability witness before excluding compression or another optional family from the proof. This audit does not assert that compression is enabled. Helper-only features must not be silently promoted to the product profile. Global patches also retain a regression obligation for other workspace consumers even if a B-specific profile is narrower.

**Closure:** fingerprint exact commit, lockfile, resolved packages/features, target, compiler, panic/optimization settings, allocator and selected TLS configuration. Current public library documentation is not a substitute for source/layout proof for the pinned Rust 1.94 build.

### WP3-R3-11 — HIGH — a measured peak is not a complete phase-bound proof

**PROVEN contract:** DFR permits a defensible explicit finite reservation, not an arbitrary whole-slot guess. `ResourceReservation` deliberately does not infer the dependency bound or prove enclosing-allocation finality. [DFR] [QB]

**RECOMMENDATION:** each proposed phase envelope must identify entry admission, all reachable allocations and overlapping lifetimes, external descendants, exit transfer/finality, failure exits and concurrent re-entry. Independent measurements are corroboration and regression evidence; a successful sample maximum alone does not establish a universal bound.

For collection capacity, avoid confusing logical length, requested layout and physical/process memory. Public Vec documentation distinguishes capacity and allocation behavior; it does not justify a pinned private-layout constant for this project. [CV]

**Closure:** a source-derived bound plus independent observations over the frozen profile, including reallocations, retained errors/results, resumed/post-handshake work where reachable, cancellation and failed cleanup. Unproven escaping storage stays charged or blocks the proposal; it cannot disappear at a nominal phase boundary.

### WP3-R3-12 — HIGH — provider debt depends on thread lifetime history

**PROVEN:** the owner-aware AWS-LC path records process initialization and a thread-local first-use flag. Successful shared debits are intentionally retained for process lifetime; current-thread first use charges 1360 bytes. The provider configuration is also cached process-wide. [AWSR] [AWS]

**DERIVED:** a limit on simultaneously live connections or threads alone does not prove a bounded lifetime sequence of those charges. Fresh-thread churn can consume additional root accounting even after earlier threads exit. This is an accounting/availability concern, not a claim that actual native memory leaks by the same amount or that the current B workload necessarily creates unbounded threads.

**Closure:** prove the accepted crypto-thread cohort and process-root lifetime, or adopt a reviewed true-finality model. Cold initialization, warm reuse, failed initialization and lifetime churn need separate observations.

## 6. Essential versus accidental complexity

Essential: full backing lifetime, overlap during conversion/reallocation, truthful uncertain-commit custody, and shared infrastructure surviving operations. Accidental: treating a per-loop sequence as global serialization, assuming a hook is the final allocation point, using runtime destruction to mask a long-lived-consumer gap, and treating one manifest edge as complete reachability evidence.

The operation/pool split remains useful, but correctness still requires one owner for each physical resource and a small set of explicit transitions. Do not replace many ownership wrappers with equally opaque blanket phase allowances.

## 7. Minimum professional architecture and joint proof

RECOMMENDATION, not an accepted implementation:

```text
one root, established before configuration/driver bootstrap
  + funded shared runtime/provider/pool residency
  + bounded connection-establishment concurrency
  + resident connection allowances across idle/checkout/return
  + eight bounded queued identities
  + two end-to-end active identities
  + explicit transfer of surviving backing, never an uncharged escape
```

For every reachable time/state, use a disjoint accounting model such as:

```text
S + P(n) + sum(C_i) + sum(H_j) + Q + A_1 + A_2 + O <= 12,582,912
```

`S` is attributable shared infrastructure; `P(n)` is provider residency under the permitted thread history; `C_i` are resident connections; `H_j` are concurrent transient establishment peaks; `Q` is queued work; `A_i` includes each active operation, completion and ambiguity/reconciliation; `O` is any separately identified replacement overlap not already included. These are proof categories, not additional pools or policy values. A byte must not appear twice. Existing individual ceilings continue to apply. Source-derived bounds and positive fitting examples remain missing. [DFR]

A dedicated current-thread runtime is only a candidate. It needs a continuously driven owner loop while deferred pool work exists; intermittent `block_on` calls do not independently drive spawned tasks between calls. [TB] CPU-heavy parsing/crypto and synchronous configuration work also need measured scheduling impact and bounded admission; no latency improvement is claimed.

## 8. Reconciliation with the second agent and strategy choice

| Parent section | Disposition after source review |
|---|---|
| 16.3–16.5: pool/bootstrap versus active lifetime | CONFIRMED; this is the main additional architectural fact. |
| 16.6–16.7: root/shared plus phase envelopes | PROMISING, UNPROVEN; needs disjoint numerical fit and complete phase exits. |
| 16.9: unreachable optional features | CONDITIONAL; verify the resolved target graph, not only one edge. |
| 16.10: literal address and preloaded material | INCOMPLETE; include environment, pgpass, username discovery and TLS identity. |
| 16.11: dedicated current-thread runtime | CANDIDATE; continuous driving, actual footprint and scheduling need proof. |
| 16.12: two connections, sequential establishment | TWO is a proposal; global serialization is NOT established by the cited loops. |
| 16.13 and 16.17: awaited return / residual-state hook | INCOMPLETE; later ping, wrapper Drop, maintenance and TLS shutdown remain. |
| 16.14–16.16: consumer query/error/cache repair | RETAIN; add bootstrap inspection and diagnostic sinks before error normalization. |
| 16.18: remove broad dependency forks | CONDITIONAL outcome, not a safe action before the replacement is qualified. |

Continue-current has no demonstrated closure for these boundaries. Salvage helps locally but does not settle the consumer contract. Partial rewrite is likely appropriate after the boundary decision; a clean rewrite is not justified by line count or aesthetic preference. Relative effort remains unmeasured, and no percentages of retained code or hour estimates are asserted.

## 9. Preserve / simplify / replace

| Component | Disposition |
|---|---|
| `ResourceBudget` / `ResourceReservation` | KEEP, with their explicit caller proof obligations. [QB] |
| Real funded TLS helper and denial control | KEEP; add long-lived consumer and intermediate-finality observations. [H] [HR] |
| Correct final-owner fixes from the parent report | KEEP until a proven replacement makes them unnecessary. |
| B guarded two-slot pending reload and generation fence | KEEP; complete acknowledgement and same-slot reconciliation. [BMOD] |
| Pool-return and cache cleanup assumptions | REWORK the lifecycle proof, not unrelated upstream semantics. [PC] [PG] |
| New broad Tokio/rustls ownership expansion | NEEDS_DECISION; this audit itself does not revoke an allocation. |
| Unsupported blanket phase bounds or silent profile restrictions | REJECT as acceptance evidence. |
| Whole forks | PRESERVE EVIDENCE; remove only if the final qualified consumer no longer needs them. |

Current Tokio source has `OwnedQueue::remove_if_idle` with external node-charge release after destruction. Do not repeat an older assertion that every queue node necessarily survives until runtime shutdown. Provenance checkpoints are historical, and the seven-path semantic manifest is a declared inventory rather than independent verification of every imported byte. [TP] [TM]

## 10. Coverage ledger: reviewed topics are not passed qualification

| Area | Evidence status and remaining boundary |
|---|---|
| 01. Canonical revisions and report continuity | PROVEN API/source snapshots; refresh at any future mutation. |
| 02. Accepted requirement and integration authority | Read; no new policy, topology or numeric authority granted. |
| 03. Full authored/imported delta | PARTIAL; complete independent byte/LOC census missing. |
| 04. Authored unsafe and native crypto/FFI | UNKNOWN whole-diff qualification; workspace lint is not sufficient. |
| 05. Exact feature/target/build graph | PARTIAL; helper and production profile differ. |
| 06. Ambient configuration and secrets | Source gap identified; bounded intake/redaction tests missing. |
| 07. Bootstrap schema and custody reads | Mixed: pending guard present, migration read gap identified. |
| 08. DNS / transport address / TLS identity | Source reconciliation; accepted final deployment profile unknown. |
| 09. TCP/UDS socket and reactor finality | Earlier blocking cells still open. |
| 10. TLS modes, verification and downgrade behavior | Earlier source issues retained; full runtime matrix missing. |
| 11. Certificate reload, key material and rotation overlap | Required if reachable; no new proof executed. |
| 12. Provider initialization and thread churn | Source model identified; lifetime availability proof missing. |
| 13. Pool bootstrap/reconnect concurrency | Global single-handshake assumption corrected; tests missing. |
| 14. Return, hook, ping and maintenance | Source order verified; complete budget/finality proof missing. |
| 15. Close, stalled I/O and cancellation | Async close path verified; bounded execution proof missing. |
| 16. PG lengths, rows, columns and auth | Parent decoder/SASL gaps retained; no new native parser test. |
| 17. Status, errors, notifications and logging | Parent gaps plus pre-normalization sinks; tests missing. |
| 18. Statement/type/table/cache residency | Cleanup scope verified; final resident bound unknown. |
| 19. Shared rows, values, copies and errors | End-to-end last-owner qualification missing. |
| 20. Atomic ledger admission/transfer/release | Primitive source supports intent; production composition unqualified. |
| 21. Root joint fit and allowed concurrency | Arithmetic constraint explicit; actual fitting witness unknown. |
| 22. Queue, active slots, ACK and restart | Explicit incomplete integration; preserve uncertain custody. |
| 23. COMMIT, replay and cleanup funding | Contract retained; full production resource-failure matrix missing. |
| 24. SQL result aggregate and all sibling writers | Parent query repairs retained; full query census missing. |
| 25. Real PG/TLS test isolation | Existing test success preserved; target guard/cleanup hardening needed. |
| 26. Test/production equivalence | Both rustls and B construction distinctions recorded. |
| 27. Independent allocation and release-order observation | NOT EXECUTED; end counters are not the independent oracle. |
| 28. Load, scheduler fairness, latency and long-run churn | NOT MEASURED; no throughput or player-latency promise. |
| 29. Dependency updates, regression and platform coverage | PARTIAL; target/profile-bound requalification required. |
| 30. Exact consumer review, CI and protected integration | NOT QUALIFIED; successful WP3 CI is narrower evidence. |

This table is a scope ledger. It is not a percentage of all code inspected, and an entry does not imply that every function in that area was read.

## 11. Verification refinements and next execution sequence

The original `WP3-Q01`–`WP3-Q46` remain. The following twenty-four criteria refine them rather than claiming twenty-four independent new suites. Every row has status **REQUIRES_EXACT_HEAD_EVIDENCE** and **new_test_executed_by_this_audit=false**.

| ID | Refines | Required observation |
|---|---|---|
| WP3-Q47 | Q39, Q46 | Fund configuration, pool bootstrap and custody restoration before new work admission. |
| WP3-Q48 | Q31, Q40 | Bound migration-ledger count/checksum transfer and detect incompatibility before client materialization. |
| WP3-Q49 | Q01, Q29 | Bound and explicitly authorize PG environment, passfile and username/configuration inputs. |
| WP3-Q50 | Q33, Q41 | Redact diagnostics before library sinks, including malformed local credential records. |
| WP3-Q51 | Q39, Q44 | Exercise concurrent eager/maintenance/checkout connection attempts under one admission bound. |
| WP3-Q52 | Q13, Q44 | Observe maintenance after explicit return and wrapper Drop, with lifetime timers disabled. |
| WP3-Q53 | Q34, Q36 | Check post-ping residency, not only the earlier after_release hook. |
| WP3-Q54 | Q13, Q36 | Close during stalled TLS I/O; keep truthful ownership across deadline and cancellation. |
| WP3-Q55 | Q34, Q35 | Verify all reachable retained maps, not only statement count or stream shrink. |
| WP3-Q56 | Q39, Q46 | Complete and acknowledge more than two sequential operations without restarting the runtime. |
| WP3-Q57 | Q38, Q39 | Reconcile two occupied slots without a third slot or a replacement identity. |
| WP3-Q58 | Q38, Q46 | Classify pre-effect initialization failure versus uncertain takeover without blind reset. |
| WP3-Q59 | Q03, Q46 | Exercise registered B construction separately from LegacyFixture behavior. |
| WP3-Q60 | Q04, Q44 | Observe release order and reuse while the runtime stays alive, then separately audit shutdown. |
| WP3-Q61 | Q01, Q26, Q43 | Record resolved production features and prove excluded-family reachability claims. |
| WP3-Q62 | Q04, Q45 | Bind layout/phase evidence to compiler, allocator, target, panic and optimization profile. |
| WP3-Q63 | Q04, Q36 | Prove each phase's entry, overlap, escaping descendants and every failure exit. |
| WP3-Q64 | Q39, Q40 | Show simultaneous root/sub-budget fit at required semantic maxima, without double counting. |
| WP3-Q65 | Q37, Q38 | Reserve completion and cleanup before COMMIT and retain ambiguity when delivery fails. |
| WP3-Q66 | Q04, Q44 | Separate cold/warm provider initialization and lifetime crypto-thread churn accounting. |
| WP3-Q67 | Q29, Q44 | Charge old/new configuration and connection overlap during permitted rotation/failover. |
| WP3-Q68 | Q09, Q44 | Prove continuous runtime driving, bounded work admission and scheduling under crypto/parser load. |
| WP3-Q69 | Q43, Q45 | Run affected vendored and ordinary-consumer tests in the exact supported profiles. |
| WP3-Q70 | Q04, Q46 | Retain source identities, selected test names, skips, independent observations and cleanup outcomes. |

Ordered RECOMMENDATION: first protect the minimal superseding boundary/profile decision; next complete the real B byte/custody/cleanup substrate and its bootstrap/query guards; then implement only necessary SQLx seams; finally qualify phase bounds before deciding which dependency forks can be removed. Keep all changes in the existing authorized implementation lineage with properly allocated paths. Do not turn this evidence supplement into a competing worker or automatic merge request.

## 12. Integration and decision timing

WP2's accepted semantics remain binding. WP4 owns query shape, durable classification, checkpoint/acknowledgement and original-operation reconciliation. WP3 owns its allocated driver/resource seam, not a replacement durability authority. WP5 source/bootstrap readiness and fresh G0 composition remain separate. Neither positive TLS CI nor this report releases Server Seam.

**Must decide now? YES**, before committing to the proposed replacement consumer ownership model. It blocks truthful B/driver composition and numeric/finality qualification. Delaying the decision risks retaining incompatible operation-versus-pool lifetimes and expanding dependency forks without closure. Evidence that could supersede this recommendation: a complete qualified existing design, or a smaller demonstrated profile with source-backed bounds and real consumer tests. Deliberately not decided here: new byte ceilings, pool size, scheduler topology, provider switch, production connection policy, history retention, rollout or merge authority.

## Source anchors

Repository links bind the exact inspected revision. Symbols/ranges referenced above identify the relevant source observations; whole-file links do not assert whole-file semantic coverage. External documentation below is explanatory primary-source context, not a substitute for pinned private implementation proof.

[BDB]: https://github.com/Oteryn/Oteryn-Game/blob/834db1d7118d751e31287715d3eaac7780a0c7b9/apps/game-server/src/durability/db.rs
[BS]: https://github.com/Oteryn/Oteryn-Game/blob/834db1d7118d751e31287715d3eaac7780a0c7b9/apps/game-server/src/durability/schema.rs
[BMOD]: https://github.com/Oteryn/Oteryn-Game/blob/834db1d7118d751e31287715d3eaac7780a0c7b9/apps/game-server/src/durability/mod.rs
[PC]: https://github.com/Oteryn/Oteryn-Game/blob/fe7891989b1247012e32c89c10cff6a10bacb943/vendor/sqlx-core-0.9.0/src/pool/connection.rs
[PI]: https://github.com/Oteryn/Oteryn-Game/blob/fe7891989b1247012e32c89c10cff6a10bacb943/vendor/sqlx-core-0.9.0/src/pool/inner.rs
[PO]: https://github.com/Oteryn/Oteryn-Game/blob/fe7891989b1247012e32c89c10cff6a10bacb943/vendor/sqlx-core-0.9.0/src/pool/options.rs
[PG]: https://github.com/Oteryn/Oteryn-Game/blob/fe7891989b1247012e32c89c10cff6a10bacb943/vendor/sqlx-postgres-0.9.0/src/connection/mod.rs
[TLS]: https://github.com/Oteryn/Oteryn-Game/blob/fe7891989b1247012e32c89c10cff6a10bacb943/vendor/sqlx-core-0.9.0/src/net/tls/tls_rustls.rs
[CFG]: https://github.com/Oteryn/Oteryn-Game/blob/fe7891989b1247012e32c89c10cff6a10bacb943/vendor/sqlx-postgres-0.9.0/src/options/mod.rs
[PARSE]: https://github.com/Oteryn/Oteryn-Game/blob/fe7891989b1247012e32c89c10cff6a10bacb943/vendor/sqlx-postgres-0.9.0/src/options/parse.rs
[PASS]: https://github.com/Oteryn/Oteryn-Game/blob/fe7891989b1247012e32c89c10cff6a10bacb943/vendor/sqlx-postgres-0.9.0/src/options/pgpass.rs
[H]: https://github.com/Oteryn/Oteryn-Game/blob/fe7891989b1247012e32c89c10cff6a10bacb943/vendor/sqlx-postgres-0.9.0/tests/oteryn_resource_budget.rs
[HR]: https://github.com/Oteryn/Oteryn-Game/blob/fe7891989b1247012e32c89c10cff6a10bacb943/vendor/sqlx-postgres-0.9.0/tests/oteryn_resource_budget_helper.rs
[AWS]: https://github.com/Oteryn/Oteryn-Game/blob/fe7891989b1247012e32c89c10cff6a10bacb943/vendor/rustls-0.23.43/src/crypto/aws_lc_rs/mod.rs
[AWSR]: https://github.com/Oteryn/Oteryn-Game/blob/fe7891989b1247012e32c89c10cff6a10bacb943/vendor/rustls-0.23.43/src/crypto/mod.rs
[QB]: https://github.com/Oteryn/Oteryn-Game/blob/fe7891989b1247012e32c89c10cff6a10bacb943/vendor/sqlx-core-0.9.0/src/net/resource_budget.rs
[TP]: https://github.com/Oteryn/Oteryn-Game/blob/fe7891989b1247012e32c89c10cff6a10bacb943/vendor/tokio-1.53.1/src/runtime/blocking/pool.rs
[TM]: https://github.com/Oteryn/Oteryn-Game/blob/fe7891989b1247012e32c89c10cff6a10bacb943/vendor/tokio-1.53.1/OTERYN_DELTA_MANIFEST.json
[DFR]: https://github.com/Oteryn/Oteryn-Game/blob/489e3e390a1bce1ce3439c66521ab75f8a826cd8/docs/architecture/reviews/OTERYN_GAME_DURABLE_FRESH_RESOURCE_ENVELOPE_DECISION_2026-09-06.md
[BT]: https://github.com/Oteryn/Oteryn-Game/blob/489e3e390a1bce1ce3439c66521ab75f8a826cd8/docs/agents/BUILD_TEST_MATRIX.md
[CB]: https://doc.rust-lang.org/cargo/reference/features.html
[CV]: https://doc.rust-lang.org/std/vec/struct.Vec.html
[TB]: https://tokio.rs/tokio/topics/bridging

## 13. Final go/no-go

The verified consumer and pool source make the earlier NO-GO more precise and expose corrections needed in the proposed simplification. No replacement bounds, full authored-diff review, independent release-order instrumentation, long-lived-runtime qualification or production-composition test has been established by this round. Unknowns remain explicit rather than scored as passed.

Keep the useful core and historical evidence; qualify the smallest correct boundary before integrating. This is a stronger source audit, not a claim that every possible defect has been found or that the implementation is now 10/10.

```text
WP3_BOUNDARY_REDESIGN_REQUIRED
```
