# WP3 large-diff PostgreSQL target qualification repair allocation

Coordinator: #162. Programme: #364. Bounded blocker: #420. Consumer: #351 / #356.

## State

```yaml
allocation_id: OTV2-WP3-PG-TARGET-LARGE-DIFF-420
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 4d6139083179b8fd8c5d0497b2abf8c2545de599
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-pg-target-420-allocation
worker_branch: coord/wp3-pg-target-420
risk: CONTROL
required_status_name_change: false
merge_queue_semantics_change: false
ruleset_change: false
```

This is a prospective allocation, not a workflow lease. After independent review,
canonical checks, normal FULL Merge Queue and protected readback, Work may
explicitly apply it to one serialized CONTROL writer. It does not replace the
existing WP3 worker, branch, admission or partial implementation.

## Exact blocking evidence

PR #356 at `6438a8c0a168b722672ab4e48bffc83687da28a3` has 803 changed files.
Merge gate run `34165596593`, Linux job `101875956055`, fails at
`Classify Durability PostgreSQL target` with
`invalid or over-cap changed-files count` before Rust/PostgreSQL execution.

The protected classifier rejects more than 300 files, then requires complete
immutable compare enumeration. Raising that cap cannot establish completeness
of a truncated compare response. Paginated mutable PR files cannot replace
immutable evidence because mixed-generation/ABA behavior must remain rejected.

## Sole material lease after Work application

Exactly these existing files:

- `.github/workflows/merge-gate.yml`;
- `tools/repository/validate_pr_gate_pg_sim.py`;
- `tools/repository/test_validate_pr_gate_pg_sim.py`.

The writer may also create and maintain only its own task:
`docs/agents/tasks/active/OTV2-20260907-pg-target-large-diff-420.md`.
Its terminal archive counterpart is coordinator-owned at closeout.

The workflow lease is limited to the existing Linux PostgreSQL target classifier
and the immediately related target-execution consistency check. The validator
lease permits the matching Linux evidence-job digest and assertions of the
same new target contract. Keep every unrelated job, digest, trigger, dependency,
permission, required status, routing decision and MQ behavior unchanged.

No WP3 vendor/Cargo/source, WP2 Foundation, WP4 SQL/migration/shared PG test,
WP5 implementation/routing, Atlas trigger, registry, production, external
repository or `merge-authority-audit.yml` lease follows. If an existing binding
proves another file materially necessary, return exact `SHARED_LEASE_REQUIRED`
before changing it. Do not expand this allocation for adjacent cleanup.

## Minimum implementation contract

Replace only the PG classifier's full changed-file enumeration dependency with
a query of the exact canonical target
`apps/game-server/tests/durability_postgres.rs` at validated immutable base and
head commit SHAs.

- Only an authenticated exact-path/exact-commit response may establish presence.
- A valid file response establishes presence. A directory, malformed/mismatched
  path, malformed type or other unexpected payload fails closed.
- Treat a genuine target-not-found response as absence only within the already
  authenticated/validated repository and exact commit context. Authorization,
  transport, rate-limit and server errors must not become absence.
- A target present at base and absent at head is removal, including rename away,
  and fails qualification.
- A target present at head must execute the actual configured PostgreSQL target.
- Absence at both exact revisions is the only historical target-absence case;
  it must not skip a target present in the checked-out candidate.
- API target state and the verified exact-head checkout must agree before the
  current target-execution step may run or classify NOT_APPLICABLE.
- Preserve the current pre/post open-PR, same-repository, exact head/base and
  changed-file-count race checks. Do not derive authority from mutable pages.
- Large changed-file counts must not cause rejection merely because unrelated
  file enumeration is capped. Invalid count metadata remains a failure.
- Keep PostgreSQL 17.6 image/harness, Rust 1.94, existing target invocation and
  final `game-gate` failure propagation unchanged.

No generalized routing system or new framework is needed. The unchanged scope
classifier continues to own whole-PR impact routing.

## Required evidence

Use the actual inline classifier in the existing regression harness.

1. RED: a valid 803-file candidate with the canonical target present is rejected
   by the protected classifier before qualification.
2. GREEN: large-diff present, removed and renamed-away target; introduced target;
   genuine both-absent historical case.
3. Fail-closed negative controls: malformed/type/path responses, authorization,
   API/server/rate-limit errors, exact-head/base/repository/count movement and
   closed PR; missing/mismatched checkout target.
4. Immutable A-to-B-to-A evidence cannot mix target observations from different
   commits. No mutable PR-file pagination is used for target authority.
5. Preserve existing ordinary PR and deletion/rename controls; replace obsolete
   enumeration-specific expectations only with stronger exact-target evidence.
6. Valid workflow bytes plus the exact Linux evidence-job digest pass existing
   validation. A stale digest or removed mandatory test invocation fails.
7. A target failure reaches the existing required aggregate. No success/skip
   path may mask failure or bypass legacy PostgreSQL execution.
8. Run focused PG/SIM regressions, applicable repository policy/governance,
   independent exact-head CONTROL review and canonical CI.
9. Integrate through normal FULL MQ and verify protected readback. Qualify the
   actual large WP3 candidate through the corrected protected classifier; local
   mocks or command presence alone are not its hosted PostgreSQL evidence.

## Custody and release

Before application, refresh #420, all open PRs, active tasks and current main.
At preparation, #419 owns only its prospective Atlas semantic-trigger allocation;
WP1 special gate custody has been released. #416 remains a separate NOT_ACTIVE
WP5 routing allocation. Serialize any later overlap rather than transferring
those lanes implicitly.

WP2 and WP3 may continue their existing path-disjoint source work throughout this
CONTROL repair. WP4 and Server Seam release conditions remain unchanged.

Runtime product E2E is NOT_APPLICABLE to this allocation document. This document
does not prove the repair, PostgreSQL qualification, WP3 delivery or Server Seam
readiness.
