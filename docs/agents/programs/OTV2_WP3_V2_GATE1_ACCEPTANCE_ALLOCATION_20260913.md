# OTV2 WP3-v2 Gate 1 Acceptance and A4 Allocation

Date: 2026-09-13
Status: `PROSPECTIVE_NOT_ACTIVE`
Repository: `Oteryn/Oteryn-Game`
Active control plane: `OTV2_WORK_DELIVERY_COORDINATOR`
Programme authorities: #162 / #364
Architecture: `WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1`, Revision 3
Architecture exact head: `60b2018ded8be9be9b404eafcd25eba9c43097cd`
Protected architecture merge: `253b5c0c464e9b73c8398bcf479bdcca9a1f0932`
Canonical implementation lineage: #351 / PR #356 / `agent/sqlx-driver-budget-351`
Observed implementation baseline at reconciliation: `fe7891989b1247012e32c89c10cff6a10bacb943`

This document records the prospective repository acceptance/allocation package required to move WP3-v2 from the protected Revision-3 architecture candidate into material A4 implementation. It deliberately does **not** transfer the active control plane and does **not** become implementation authority merely because this file or its parent PR exists.

## Authority rule

Protected `docs/agents/tasks/active/OTV2-20260825-work-delivery-coordinator.md` currently selects `OTV2_WORK_DELIVERY_COORDINATOR` as the active control plane for #162. That selector remains unchanged.

Therefore:

- alias invocation, model/session choice, this programme document or PR metadata do not activate A4;
- merge of this prospective package does not by itself activate A4;
- material A4 mutation remains forbidden until protected integration/readback of this package **and** a fresh explicit activation by the active control plane on the canonical #351/#356 lineage;
- no second coordinator, second resource ledger, replacement #356 worker, replacement branch or replacement PR is created by this package.

## Gate 1 architecture evidence

The architecture content is eligible for formal repository acceptance on the following exact evidence:

- PR #590 is protected-integrated as `253b5c0c464e9b73c8398bcf479bdcca9a1f0932`;
- exact architecture head is `60b2018ded8be9be9b404eafcd25eba9c43097cd`;
- Architecture Semantic Audit `34719732556`: `SUCCESS`;
- Agent Governance `34719818428`: `SUCCESS`;
- Merge Gate `34719818411`: `SUCCESS`, including canonical `game-gate`;
- genuinely independent HIGH-risk exact-head architecture/resource/security review reports `P0=0`, `P1=0`, `P2=0`, `BLOCKING_EVIDENCE_GAP=0` and `INDEPENDENT_ARCHITECTURE_REVIEW=PASS`;
- the independent review explicitly did not grant implementation authority, so this package preserves the separate repository activation step.

No further B-vs-C architecture round is required absent new material evidence that satisfies Revision-3 supersession conditions.

## Canonical A4 implementation lineage

A4 must reuse the existing canonical WP3 implementation/evidence lineage:

- Issue: #351
- PR: #356
- Branch: `agent/sqlx-driver-budget-351`
- Reconciled observed head: `fe7891989b1247012e32c89c10cff6a10bacb943`

Before any material mutation after activation, the SAME branch must be freshly read back and normally reconciled with then-current protected `main`. No reset, rebase, force-push, replacement branch, replacement PR or competing material worker is authorized.

The observed #356 head is a reconciliation coordinate, not a permanent admission SHA. Explicit activation must name the then-current protected main and the then-current canonical #356 head after the fresh overlap/custody check.

## #356 disposition

#356 remains the canonical research/evidence/material lineage until the accepted WP3-v2 implementation is protected. Its contents are not accepted wholesale.

### RETAIN

Retain and reuse where compatible with Revision 3 and exact current source:

- `ResourceBudget` / `ResourceReservation` and the same-ledger reservation model;
- applicable backing-lifetime/finality primitives that can prove the frozen Revision-3 ownership model;
- protected AWS-LC provider/KX work and matching exact-profile source/tests;
- PostgreSQL 17.6 and real TLS qualification harnesses;
- hostile frame/count/resource-denial/max+1 vectors;
- exact-source/provenance evidence that remains true on the final resolved graph;
- already-protected narrow path/symbol leases only where their semantics remain compatible with Revision 3.

### SUPERSEDED

Do not carry forward as architecture authority:

- broad operation-owned direct connection as terminal WP3 architecture;
- earlier `min_connections=2` / `max_connections=2` or generic two-connection first-slice recommendations;
- assumptions that caller timeout, transaction drop, `after_release`, pool wrapper drop or SQLx close alone proves backing finality;
- any implementation-selected overlap between a new connection attempt and prior `R`/`T` retirement descendants;
- timer/polling/unbounded or immediate autonomous reconnect policy substituted for the frozen demand-triggered recovery model;
- ring as the final first-slice provider after Revision-3 acceptance;
- broad rustls ownership outside exact retained/required seams;
- generic DNS/UDS work excluded by the literal-IP first-slice transport profile;
- per-operation direct-connect ownership assumptions superseded by root ownership;
- closed/unmerged PR #455 as architecture authority.

Historical evidence is preserved; superseded architecture assumptions must not be revived merely because implementation code already exists.

## Frozen A4 implementation contract

A4 implements Revision 3; it does not choose a new architecture.

The first-slice contract includes:

- one process-scoped logical Durability executor and one accepted DFR root ledger;
- lazy holder-style `PgPool` with `max_connections=1`, `min_connections=0`, idle timeout 10 minutes and max lifetime 30 minutes;
- active DB work through ready-only `Pool::try_begin()` / `try_acquire()` so an active pass cannot manufacture a connection;
- two logical active custody slots and at most one physical DB pass at a time;
- one-second queue deadline and one absolute two-second DB-pass deadline;
- root resource feasibility equation `I + max(R,T) + Q + A <= 12 MiB`;
- `I` as process/root backing independent of one physical connection generation;
- lifecycle-complete `T` for one establishment/failure generation through all descendants to finality;
- lifecycle-complete `R` for one established generation through fencing/reaper/close/return/reactor descendants to finality;
- strict mutual exclusion of `R` and `T`; no new `T` while any previous `R`/`T` retirement tail is non-final;
- successful establishment as a charged `T -> R` ownership transfer, never an overlap;
- no hidden third retirement term and no migration of delayed per-connection tails into `I` merely to evade the equation;
- one coalesced root-owned `root_ready_demand` latch;
- only startup/takeover demand, an otherwise-authorized ready-only miss, or explicit root-owned fencing/retirement may create first-slice demand;
- silent SQLx reaper loss detected by the next ready-only miss;
- one latched demand authorizes at most one five-second root recovery window;
- failed/timed-out/cancelled connect retirement does not self-trigger a periodic or immediate retry; a successor window requires new/coalesced demand after prior `R`/`T` finality;
- explicit bounded no-ambient PostgreSQL configuration: no `PG*` inheritance, `.pgpass` or OS-user fallback in the production profile;
- literal-IP TCP routing plus separately supplied TLS server identity;
- `VerifyFull`, TLS1.3-only owner-aware profile;
- exact non-FIPS Linux x86-64 GNU profile: Rust 1.94, rustls 0.23.43, aws-lc-rs 1.18.0, aws-lc-sys 0.44.0, SQLx 0.9.0, Tokio 1.53.1;
- SCRAM-SHA-256-only PostgreSQL authentication;
- canonical `WP3-Q01..WP3-Q75` remains binding;
- R21 remains binding: one physical connection is first-slice minimality, not a throughput conclusion.

Exact byte values for `I`, lifecycle `R`, lifecycle `T`, active SQL peak and metadata remain A4 exact-candidate qualification obligations. If the exact candidate cannot prove the frozen equation/model, A4 must stop with architecture escalation instead of adding overlap, a second ledger, another physical slot or weaker TLS/DFR semantics.

## Prospective custody / path allocation

After activation, the SAME #351/#356 lineage is the sole WP3 material writer.

A4 may consume only:

- exact paths/symbols already protected for #351/#356 that remain compatible with Revision 3;
- exact WP3/Durability integration paths named by the explicit activation;
- minimum new narrow seams proven necessary by the final exact source graph and separately granted under normal shared-lease discipline.

Shared Cargo/vendor/Durability surfaces remain serialized. This package grants no blanket repository or dependency-fork authority.

If implementation reaches a required path/symbol not covered by the explicit activation or a compatible protected lease, stop before mutation and return exactly:

`SHARED_LEASE_REQUIRED = <path> :: <symbol/scope> :: <reason>`

The following remain excluded without separate protected authority:

- #329 / PR #335 Child B material mutation;
- WP5/source material mutation outside an independently valid live allocation;
- #247 Server Seam material mutation;
- `Oteryn/Oteryn-Platform` writes;
- workflows/rulesets/merge-control changes;
- production/deployment/secret/certificate/live-data actions.

## Explicit activation gate

Material A4 mutation is forbidden until all of the following are true:

1. this Gate-1 package is integrated through the repository's normal protected process;
2. protected-main readback proves the exact package is present;
3. `OTV2_WORK_DELIVERY_COORDINATOR` freshly re-reads #162, #364, #351/#356, current protected main, open material PRs and Cargo/vendor/Durability overlaps;
4. that active control plane records an explicit activation on the programme/canonical WP3 lineage naming the exact protected-main SHA, exact #356 branch/head and bounded custody/path scope;
5. no competing material writer owns an overlapping required path set.

Only after those conditions may repository state truthfully record:

```text
ARCHITECTURE_ACCEPTED = YES
A4_IMPLEMENTATION_ALLOCATION = ACTIVE
```

Before that point the truthful state remains:

```text
GATE_1_PACKAGE = PROSPECTIVE_NOT_ACTIVE
A4_IMPLEMENTATION_AUTHORITY = NONE
```

## Terminal WP3 release gate

#335 / Child B must remain materially frozen until WP3 is protected and required shared custody is released.

Before WP3 release, the exact final #356 lineage must provide at minimum:

- actual funded owner-aware AWS-LC TLS-positive qualification on the exact resolved production graph;
- configured PostgreSQL 17.6 positive plus hostile/resource-denial qualification;
- exact `I/R/T/active` byte/lifetime proof under the frozen equation;
- terminal truthful classification of canonical Q01-Q75;
- cancellation/rollback/ambiguous COMMIT/reconciliation/restart/takeover/finality evidence required by the accepted contracts;
- genuinely independent HIGH-risk whole-diff review with explicit `PASS`, `P0=0`, `P1=0`, `P2=0` and `BLOCKING_EVIDENCE_GAP=0` on the exact final head;
- exact-head canonical repository CI;
- normal FULL native Merge Queue with real `merge_group` `game-gate`;
- protected-main source/readback after integration.

Only after that protected WP3 terminal state may the control plane release the canonical #335 Child B lineage. WP5 composition and #247 Server Seam remain downstream of their existing protected gates.

## Current disposition

```text
WP3_V2_ARCHITECTURE_CONTENT = CLEAN_FOR_ACCEPTANCE
GATE_1_PACKAGE = PROSPECTIVE_NOT_ACTIVE
ACTIVE_CONTROL_PLANE = OTV2_WORK_DELIVERY_COORDINATOR
CANONICAL_A4_LINEAGE = #351/#356 agent/sqlx-driver-budget-351
A4_IMPLEMENTATION_AUTHORITY = NONE
WP4_CHILD_B = HOLD
SERVER_SEAM_247 = WAITING_DEPENDENCY
```
