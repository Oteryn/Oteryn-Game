# OTV2-20260912-wp3-v2-architecture-decision

```yaml
task_id: OTV2-20260912-wp3-v2-architecture-decision
title: Draft WP3-v2 superseding architecture decision
mode: CONTRACT
status: evidence_blocked
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/wp3-v2-superseding-decision-20260912
pr: pending
base_sha: 489e3e390a1bce1ce3439c66521ab75f8a826cd8
head_sha: null
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
  - wp3-v2:A2-exact-evidence
blocks:
  - WP3-v2 Gate 1 architecture acceptance
  - A4 WP3-v2 material implementation allocation
  - terminal composed WP3 qualification
cross_repository_coordination_id: WP3-V2-GAME-PLATFORM-20260912
external_repositories:
  - Oteryn/Oteryn-Platform
```

## Outcome

Produce one bounded superseding WP3-v2 architecture candidate for the real Child B consumer without granting implementation or architecture-acceptance authority.

Current candidate: `WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1`.

Current lane result:

```text
WP3_V2_ARCHITECTURE_CANDIDATE_OPTION_B
BLOCKING_EVIDENCE_GAP = A2 exact profile/corpus/finality evidence
ARCHITECTURE_ACCEPTED = NO
IMPLEMENTATION_AUTHORITY = NONE
```

The intended terminal worker result remains `WP3_V2_ARCHITECTURE_READY_FOR_ACCEPTANCE`, but it is not truthful until the exact A2 evidence obligations recorded below are closed.

## Architecture and source of truth

- `PROVEN`: protected admission is `main@489e3e390a1bce1ce3439c66521ab75f8a826cd8`.
- `PROVEN`: #162 remains product allocation/integration authority and #364 remains the active remediation programme.
- `PROVEN`: #351 / Draft PR #356 remains the canonical WP3 implementation/evidence lineage; this task does not mutate it.
- `PROVEN`: #329 / PR #335 remains the canonical Child B lineage; this task does not mutate it.
- `PROVEN`: PR #588 is retained audit evidence with no architecture-acceptance authority.
- `PROVEN`: the canonical WP3 qualification package remains `WP3-Q01..WP3-Q75`.
- `PROVEN`: the WP3-v2 programme records A1 as architecture/docs only and A2 as read-only exact-source evidence.
- `UNKNOWN`: no terminal A2 evidence package has been located for this candidate head.

## Selected option

Select Option B for the first slice:

- one logical Durability executor and one accepted DFR root ledger;
- bounded lazy `PgPool` as a single-ready-connection holder;
- `max_connections = 1`, `min_connections = 0` candidate topology;
- root-owned serialized connection establishment outside active DFR work;
- ready-only active checkout;
- two logical active custody slots but at most one physical DB pass at a time;
- minimal SQLx/PostgreSQL seams only where exact consumer/query/profile repair cannot close the bound.

Option A is rejected as terminal architecture but its useful evidence/primitives are retained. Option C remains a valid future supersession option if exact proof shows the holder-style pool requires a broader invasive seam or representative latency/contention evidence requires more physical DB concurrency.

## A2 evidence required before READY_FOR_ACCEPTANCE

The exact evidence handoff must close or explicitly classify:

1. every reachable production Child B/WP3 SQL statement family, including parameters, result cardinality/bytes, same-snapshot guards, retained backing, error lifetime and cache effects;
2. exact correctness-required lock footprint, ordering, parent/sibling participation and maxima;
3. exact production PostgreSQL connection lifecycle from bounded configuration through DNS/socket/TLS/startup/authentication/ReadyForQuery, checkout, COMMIT/rollback, return/ping, idle/reaping/replacement and lower-layer close/finality;
4. exact production startup/authentication mechanism allowlist; unlisted mechanisms require reachability proof rather than assumption;
5. exact resolved Cargo/TLS/provider/runtime feature graph and target/profile assumptions;
6. exact bounded configuration/credential source set, including PG environment/passfile/OS-fallback disposition and diagnostic redaction;
7. source-backed `I`, `R`, `T`, `Q`, `A` terms where derivable and every remaining `UNKNOWN`/`BLOCKING_EVIDENCE_GAP`;
8. finite pool idle/max-lifetime/retirement policy and proof that `min_connections = 0` cannot create automatic replacement;
9. deferred reactor/provider/socket finality term or one narrowly justified finality seam;
10. B-vs-C evidence respecting cross-repository finding R21: one connection is not declared performance-sufficient solely because current relation locks serialize writes.

## Acceptance criteria

- [x] Options A/B/C are compared as realistic alternatives.
- [x] One root/executor ownership model is defined.
- [x] Queue/active ownership and definitive release/finality semantics are defined.
- [x] PgPool establishment/replacement/retirement/checkout responsibilities are defined at architecture level.
- [x] Configuration/credential authority is narrowed to an explicit bounded profile; unresolved exact bounds are not invented.
- [x] PostgreSQL startup/authentication and TLS/runtime/provider profiles are explicitly evidence-gated.
- [x] Cancellation, rollback, ambiguous COMMIT and reconciliation ownership are defined.
- [x] Restart/takeover/predecessor fencing requirements are defined.
- [x] Canonical Q01-Q75 remains binding; later audit refinements do not silently replace it.
- [x] #356 artifacts have explicit RETAIN / REWORK-NARROW / HISTORICAL-EVIDENCE_ONLY / SUPERSEDED dispositions.
- [ ] A2 exact-source evidence package closes the blocking profile/corpus/finality gaps.
- [ ] Independent architecture review validates the Option B-vs-C decision and #356 disposition.
- [ ] Exact-head repository/governance checks are green.
- [ ] Candidate is protected-integrated/read back before any A4 material allocation.

## Excluded scope

No runtime Rust, vendor, Cargo/lockfile, SQL/migration, workflow/ruleset, #356/#335 worker mutation, Platform write, production/deployment/secret, merge, Merge Queue submission or architecture self-acceptance.

## Validation

### Focused source/readback

- protected `main@489e3e390a1bce1ce3439c66521ab75f8a826cd8`: PASS
- live #351/#356, #329/#335, #364, #588 readback: PASS
- accepted DFR resource authority readback: PASS
- Architecture Decision Discipline readback: PASS
- Revision-4 audit and cross-repository R01-R21 readback: PASS
- exact Child B `apps/game-server/src/durability/db.rs` readback at `#335@834db1d7118d751e31287715d3eaac7780a0c7b9`: PASS

### Component/integration/E2E

`NOT_APPLICABLE`: docs-only architecture candidate; no product behavior is changed or claimed qualified.

### Exact-head CI

Pending publication of this candidate as a draft PR.

## Self-review

- no runtime/product/external-repository path mutation;
- no architecture acceptance claim;
- no invented I/R/T/auth/provider bound;
- R21 correction preserved: max-one connection is a first-slice topology candidate, not a measured throughput conclusion;
- #356 evidence is preserved until replacement qualification.

## Context checkpoint

```yaml
last_progress: Option B architecture candidate committed on dedicated A1 branch
status: evidence_blocked
branch: arch/wp3-v2-superseding-decision-20260912
head_sha: null
pr: pending
blocker: A2 exact profile/corpus/finality evidence package not yet verified
next_action: publish draft PR, exact-head readback, then consume A2 evidence without guessing missing bounds
```
