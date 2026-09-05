# OTV2-20260905-authority-invariant-harness

```yaml
task_id: OTV2-20260905-authority-invariant-harness
title: Independent authority invariant harness
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: test/authority-invariant-harness-281
pr: 302
issue: 281
parent_issue: 277
base_sha: 9631cbfe718e75d6bc530352fb811e08a444b6b0
head_sha: null
final_head_sha: null
owner: Programme continuation sole writer
created_at: 2026-09-05T10:00:00Z
updated_at: 2026-09-05T10:00:00Z
execution_budget_minutes: 120
large_budget_reason: Owner-authorized sequential completion of 281 and 282
owned_paths:
  - apps/game-server/tests/authority_invariants.rs
  - apps/game-server/tests/support/authority_matrix.rs
  - apps/game-server/tests/durability_postgres.rs
  - docs/agents/tasks/active/OTV2-20260905-authority-invariant-harness.md
depends_on:
  - issue: 280
    state: completed
external_repositories: []
```

## Outcome and authority

Issue281 governs allocation and acceptance under programme277. Independent live facts must be constructed without consuming a prepared record. The expected record may only be passed at the final production API binding. Keep all existing positive/E2E assertions. No production, schema, workflow, dependency or ruleset changes.

## High-risk authority/recovery qualification

Use AuthorityInvariant × ConsumerBoundary × MutationOperator. The registry enumerates identity/binding, current liveness and temporal/provenance classes; each mutation changes one raw source field. Every applicable boundary runs a positive control and every declared mutation. Explicit N/A reasons distinguish values not consumed by terminal PREPARE from COMMIT/reconciliation predicates. The same independent source feeds real PostgreSQL-reloaded committed snapshots. No record-derived matching helper is used by the new negative matrix.

## Execution and validation

One writer/exclusive checkout. Independent read-only Foundation analysis is complete; coupled registry/builders and PG wiring are serial. Fresh RED requires actual executed coverage at each declared authority boundary before implementing the registry. Then run focused matrix, all server tests, strict Clippy, repository/governance validators, exact-head PG17.6 and canonical gate; whole-diff/finding-family self-review and independent deep review precede protected Merge Queue/readback.

## Context checkpoint

Initial RED checkpoint: missing-boundary failure preserved before implementing the independent source and executable matrix. Final SHA and review/CI evidence live on Issue/PR; no post-freeze bookkeeping commit. Issue282 remains blocked until protected-main readback.


## Material candidate evidence

Fresh RED is preserved on GitHub `eeaca228f8103330347459d44cf35ec431ead9e7`: the focused test fails at missing terminal-prepare execution. GREEN adds separate prepared and live-source builders, a macro-derived enum registry that cannot silently omit newly declared variants, per-case single-field mutation assertions, positive controls on every consumer and V1/V2 negative projection phase checks. The PG target reloads actual committed snapshots through both public journal versions, then executes the same independently sourced mutation matrix. Existing assertions are unchanged.

Explicit N/A: terminal replacement authorization does not consume reconnect candidate attempt/transport/proof/security/time inputs; those are revalidated at COMMIT and InstallController. Predecessor receipt session/lease-character/state inputs belong to replacement PREPARE, while the reconnect API carries the candidate identity and scalar lease fence. Missing typed nonoptional fields are constructor/type constraints. A separate grace expiry is dominated by the valid prepared deadline; no artificial multi-invariant expiry case is counted. Runtime/source observation ordering follows current production semantics, not an invented requirement that commit time exceed source observation.

Next action: complete deterministic and exact-head PostgreSQL validation, perform whole-diff and independent review, then integrate through protected Merge Queue and release #282.

Local GREEN: 290 isolated negative cases across five positive boundaries; full game-server tests and strict all-target Clippy passed. Canonical PG target compiles; actual database execution remains a hosted exact-head requirement. Full four-file diff reviewed; no runtime or existing assertion changes. Missing isolated epoch/deadline is unrepresentable through the public snapshot builder (both set together), explicitly N/A instead of counting a compound absence as one invariant.

Verified review P2 repair: fresh focused RED failed at CommitV1/pending_present, proving independent FND02 membership was ignored. Add missing membership and substituted IDs for both pending commands and state domains through the independent resolver and shared registry. Constructors accept those values, so all unrelated facts remain valid. All four COMMIT/InstallController consumers plus PG reload inherit these cases. No production change. The repair moves the material head; refreshed exact-head validation/review is required.
