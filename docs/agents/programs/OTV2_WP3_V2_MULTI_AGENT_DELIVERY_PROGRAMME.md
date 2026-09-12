# OTV2 WP3-v2 Multi-Agent Delivery Programme

Status: PROPOSED execution programme
Date: 2026-09-12
Coordinator locators: #162, #364
Primary dependency chain: WP3-v2 architecture acceptance -> WP3 implementation -> Child B/WP4 -> Child C/WP5 -> fresh G0 -> existing Server Seam #247
Terminal programme target: `SERVER_SEAM_READY_FOR_INTEGRATION`

This document is a task-specific orchestration plan. It does not replace live Issues, accepted architecture, repository policy, task allocations, path leases, protected checks or Merge Queue authority.

## Live-state rule

Every lane must refresh the current protected `main`, its governing Issue/PR, active task ownership and exact path overlap before a material action. Historical SHAs/statuses in this document are audit coordinates only.

Current authoring/reconciliation facts:

- #162 remains the Game coordinator and #364 the vertical-slice remediation programme unless live state says otherwise.
- PR #590 has merged Revision 3 of `WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1` to protected `main`.
- Revision 3 explicitly remains `CANDIDATE / NOT ACCEPTED`; protected presence alone grants no A4 implementation authority.
- Revision 3 selects first-slice Option B and freezes the root ownership/overlap and recovery-trigger semantics described below.
- #329 / PR #335 remains the canonical Child B lineage unless live state supersedes it.
- #319 remains the source-readiness coordination locator.
- #247 preserves the existing Server Seam worker/branch and alias `Oteryn: sol server seam lead` unless live state supersedes it.
- PR #588 remains retained WP3 audit evidence; it is not implementation or architecture-acceptance authority.

## Operating principle

Parallelize discovery, independent evidence and genuinely path-disjoint work; serialize canonical architecture acceptance, shared-path implementation and protected integration. Do not create replacement workers merely because an existing canonical worker is blocked.

Prefer no more than 2-3 active agents at once. Exactly one mutating owner per material lane/path set.

## Agent map

| ID | Alias | Responsibility | Mutation class | Terminal lane outcome |
|---|---|---|---|---|
| A0 | `Oteryn: astra wp3-v2 programme coordinator` | dependency DAG, live reconciliation, allocations, gates, integration ordering | coordinator/governance only | programme reaches A7 handoff or one precise external blocker |
| A1 | `Oteryn: astra wp3-v2 architecture lead` | close/validate protected Revision-3 candidate; no duplicate architecture lineage | architecture/docs/review closure only | `WP3_V2_ARCHITECTURE_READY_FOR_ACCEPTANCE` or exact material finding |
| A2 | `Oteryn: sol wp3-v2 evidence auditor` | SQL corpus, locks, frozen connection lifecycle/resource model and exact evidence gaps | READ_ONLY | evidence package with `PROVEN/UNKNOWN/BLOCKING_EVIDENCE_GAP` |
| A3 | `Oteryn: astra platform native evidence hardening` | Platform witness/limiter/recovery/provenance/trust/mTLS hardening | Platform-only when separately admitted there | `PLATFORM_NATIVE_EVIDENCE_READY_FOR_REAL_INTEROP` |
| A4 | `Oteryn: astra wp3-v2 implementation lead` | implement accepted Revision-3 WP3-v2 substrate | Game WP3 paths only after acceptance + exact allocation | `WP3_IMPLEMENTATION_CANDIDATE` then protected WP3 |
| A5 | `Oteryn: astra child-b durability lead` | canonical Child B durability semantics on #335 lineage | Child B paths only after WP3/shared-custody release | protected Child B/WP4 |
| A6 | `Oteryn: astra wp5 source composition lead` | #319 source readiness, S1/S2/S3, WP5 composition, fresh G0 | exact source/WP5 allocations only | fresh G0 with real owners |
| A7 | `Oteryn: sol server seam lead` | resume existing #247 worker after G0 and finish seam | existing Server Seam allocation | `SERVER_SEAM_READY_FOR_INTEGRATION` |

A7 reuses the existing canonical prompt `docs/agents/prompts/OTV2_SOL_SERVER_SEAM_LEAD.md`; do not create a duplicate Server Seam worker or alias.

## Frozen WP3-v2 first-slice architecture baseline

Revision 3 currently fixes, for architecture-closure purposes:

- one logical Durability executor / one accepted DFR root ledger;
- lazy holder-style `PgPool`, `max_connections=1`, `min_connections=0`;
- root-owned serialized establishment outside active DFR work;
- active DB work uses ready-only `Pool::try_begin()` / `try_acquire()`;
- two logical active custody slots, at most one physical DB pass;
- `I + max(R,T) + Q + A <= 12 MiB` as the root feasibility equation;
- `R` and `T` mutually exclusive, including retirement tails; successful establishment is `T -> R` ownership transfer rather than overlap;
- coalesced `root_ready_demand` recovery trigger with no periodic/immediate autonomous reconnect loop;
- explicit no-ambient PostgreSQL configuration;
- literal-IP TCP with separate TLS server identity, VerifyFull, TLS1.3-only selected AWS-LC profile;
- SCRAM-SHA-256-only first-slice authentication;
- canonical Q01-Q75 remains binding.

A and C are no longer ordinary first-slice alternatives to be reconsidered by A1. Reopen them only if new material evidence meets the supersession conditions in Revision 3 or proves its frozen state machine cannot be implemented safely.

## Wave and gate schedule

### Wave 0 — A0 admission and reconciliation

A0 refreshes #162, #364, #247, #319, #329/#335, #351/#356, #588, #590, active tasks/path leases, required checks and protected main. A0 records which planned lanes are legally and technically startable. A0 does not implement WP3 or Child B.

### Wave 1 — A1 + A2 + A3 in parallel

A1 and A2 may run concurrently because A2 is read-only. A3 runs independently in Platform under Platform authority; this Game programme file does not grant Platform write permission.

A1 closes/validates the exact protected Revision-3 successor. It verifies its exact-head qualification/review state, resolves material findings through the correct owner, confirms that R/T overlap, retirement-tail classification and root recovery trigger are no longer A4 architecture choices, and prepares the smallest repository-native acceptance/closeout evidence permitted by live authority. It must not draft a second B-vs-C decision merely because #590 is not yet accepted.

A2 supplies exact SQL corpus, lock inventory, PostgreSQL/TLS/connection lifecycle and source-backed resource evidence. It tests the feasibility of the frozen Revision-3 state machine; if exact source contradicts it, A2 reports a material architecture contradiction rather than silently substituting the older multi-connection overlap equation.

A3 hardens the Platform native-evidence boundary identified by the cross-repository audit: witness durability prerequisites, rate-limit semantics, outer security-transaction recovery, witness provenance/restore, trust-profile recovery semantics and real mTLS qualification readiness.

### Gate 1 — Revision-3 architecture acceptance + A4 allocation

A4 must not begin material WP3 mutation until both are true:

1. live repository state proves Revision 3 accepted through the applicable architecture controls; and
2. #162/#364 has freshly established the exact A4 implementation lineage, path ownership and shared custody.

Protected presence of #590 alone is insufficient. Existing #356 evidence remains preserved and its disposition must follow the accepted decision/current control plane.

### Wave 2 — A4 implementation + A6 path-disjoint preparation

A4 is the sole mutating WP3 implementation worker selected by live allocation. A6 may perform source-readiness discovery and path-disjoint S1 preparation only where live allocations permit. A5 may prepare test/design evidence read-only but must not seize WP3/shared Durability/Cargo paths.

A4 implements the accepted frozen architecture. Required proof areas include root/executor ownership, exact `I/R/T/Q/A` byte values, serialized bounded connection establishment/finality, SQL protocol/result lifetime, deadlines, cancellation/rollback/cleanup ownership, root recovery, restart/takeover, configuration/credential lifetime and the frozen TLS/runtime profile.

### Gate 2 — exact WP3 qualification and protected delivery

Before WP3 terminal delivery, qualify canonical Q01-Q75 on the exact composed consumer. Use independent review where required by risk policy. `Q01-Q75 green` means truthful terminal classification for every item, not merely text completion.

No unresolved P0/P1 may remain. Integration proceeds only through current repository controls.

### Wave 3 — A5 + A6 + A3 interop in parallel

After protected WP3 and custody release:

- A5 resumes the existing #335 Child B lineage; no replacement B worker.
- A6 completes Game source S1 where authorized.
- A3 completes Platform-side real-interoperability readiness under Platform controls.

Child B target custody is one real path:

`producer -> bounded queue <= 8 -> active <= 2 -> DB pass -> commit/rollback/reconcile -> bounded completion -> owner acknowledgement -> release`.

No production semantic persistence entrypoint may bypass the accepted executor/custody model where DFR requires it.

### Gate 3 — protected Child B/WP4

Child B requires real configured PostgreSQL qualification, including fresh admission, reconnect, session replacement/nonreuse, lost response, restart, ambiguous outcomes, queue/slot saturation, deadline/cancellation and lock contention/recovery.

### Wave 4 — A6 S2/S3 and WP5 composition

Only after exact dependency gates in protected allocations are met, A6 composes durable source descriptors/high-water/pending state and exact real-source ingestion.

S3 must use an exact compatible Game + Platform pair and real authenticated transport. Fixtures or caller-provided claims are not production authority.

Required owner families include, as applicable to accepted contracts: Platform account/security/signing trust, Game Character Authority/current ownership, runtime-scope assignment/readiness and durable source ordering/non-rollback state.

### Gate 4 — fresh G0

A0/A6 perform a fresh protected-state composition check. G0 is not G1 and is not terminal programme completion. It only releases the existing Server Seam when all prerequisites are truthful.

### Wave 5 — A7 existing Server Seam

Resume, do not replace, `agent/otv2-gameplay-server-seam-01` under #247. Reconcile normally with then-current protected main and preserve valid existing work.

Finish the accepted boundary: TCP, TLS 1.3 selected profile, ALPN `oteryn-game/1`, bounded BE32 framing, Foundation decode, FND-04 trusted verification, canonical durable fresh admission, GameSession/ConnectionGeneration binding, reconnect/resync and fail-closed gameplay entry.

A7 terminal result is `SERVER_SEAM_READY_FOR_INTEGRATION`, not an inferred merge or G1 claim.

## Dependency DAG

```text
A0 live reconciliation
  |
  +--> A1 Revision-3 closure ------+
  |                                |
  +--> A2 exact evidence ----------+--> Gate 1: acceptance + allocation --> A4 WP3 --> Gate 2
  |
  +--> A3 Platform hardening -----------------------------------------------------+
                                                                                  |
                                      A6 source preparation -----------------------+
                                                                                  |
Gate 2 --> A5 Child B -------------------------------> Gate 3 --------------------+
                                                                                  |
                                          A6 S2/S3 + real source composition <-----+
                                                        |
                                                     fresh G0
                                                        |
                                       A7 existing Server Seam #247
                                                        |
                                  SERVER_SEAM_READY_FOR_INTEGRATION
```

## Shared-path collision rules

- A1 owns only allocated architecture/closure artifacts; it does not edit runtime.
- A2 is read-only.
- A4 is the only WP3 implementation writer after Gate 1 according to the live allocation.
- A5 remains read-only on shared WP3/Cargo/Durability surfaces until live custody is returned; once active it stays on the canonical #335 lineage.
- A6 never seizes Child B or Server Seam paths.
- A7 starts only after fresh G0 and retains the existing #247 allocation.
- A3 must obey Platform's own task/branch/path rules; this Game plan is a coordination locator, not Platform write authority.

## Audit obligations carried forward

- Existing WP3 Q01-Q75 remains the canonical WP3 qualification matrix.
- Cross-repository findings R01-R21 are retained in `docs/agents/evidence/OTV2_WP3_GAME_PLATFORM_CROSS_REPO_AUDIT_R01_R21_20260912.md`.
- R15-R21 are audit findings/qualification obligations, not claims that runtime failures were reproduced.
- Do not rewrite historical #588 evidence to make later conclusions appear original.

## Mandatory owner-facing successor footer

Every A0-A7 agent must end a completed or blocked final response with:

```text
NEXT_AGENT: <one exact alias, up to three independent aliases, or NONE>
RUN_WHEN: <exact live gate/state>
WHY: <one concise dependency reason>
```

The recommendation must come from fresh programme state. Do not route a mutating worker through an unmet gate. Uncertain cross-lane routing returns to A0. After `FRESH_G0_READY`, A6 normally routes to A7. After true `SERVER_SEAM_READY_FOR_INTEGRATION`, A7 uses `NONE` unless a remaining coordinator action is proven.

## Terminal programme criteria

Report `SERVER_SEAM_READY_FOR_INTEGRATION` only when:

1. Revision-3 WP3-v2 architecture is accepted;
2. WP3 exact implementation is protected and Q01-Q75 classifications are terminal;
3. no unresolved material WP3 finding remains;
4. Child B uses the accepted executor/resource architecture and its required real PostgreSQL qualification is terminal;
5. required real owning sources and Platform interoperability are qualified;
6. WP5 composition is terminal enough for fresh G0;
7. fresh G0 passes;
8. the existing #247 Server Seam worker is resumed and completes required product/security/resource qualification;
9. the Server Seam candidate is ready for the repository's current authorized integration lifecycle.

If progress stops, return one precise blocker naming the owner/capability, exact affected dependency, evidence and smallest required action. Do not collapse a temporary dependency wait into programme completion.