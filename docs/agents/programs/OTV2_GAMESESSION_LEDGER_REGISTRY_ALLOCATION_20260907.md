# OTV2 GameSession Ledger Registry Allocation — 2026-09-07

Status: **PROSPECTIVE_ALLOCATION**

## Authority

- Coordinator: Issue #162, `OTV2_WORK_DELIVERY_COORDINATOR`.
- Bounded child: Issue #384.
- Protected allocation base: `main@6b07f96d47de37971bb54fed5bb9c12decd1be17`.
- Accepted architecture: PR #383 / `FND-DUR-GAMESESSION-NONREUSE-V1` plus its protected FND-04C ledger-capacity error amendment.
- This allocation PR changes coordination/task documents only. It does **not** mutate `RESOURCE_LIMITS_REGISTRY.json`.

## Prospective worker allocation

```yaml
lane_id: OTV2-GAMESESSION-LEDGER-RESOURCE-REGISTRY
task_id: OTV2-20260907-gamesession-ledger-resource-registry-384
issue: 384
coordinator_issue: 162
status: NOT_ADMITTED
admission_main_sha: null
worker_branch: agent/gamesession-ledger-resource-registry-384
task_packet: docs/agents/tasks/active/OTV2-20260907-gamesession-ledger-resource-registry-384.md
implementation_plan: docs/agents/programs/OTV2_GAMESESSION_LEDGER_RESOURCE_REGISTRY_20260907.md
owned_paths:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/active/OTV2-20260907-gamesession-ledger-resource-registry-384.md
  - docs/agents/programs/OTV2_GAMESESSION_LEDGER_RESOURCE_REGISTRY_20260907.md
shared_registry_lease: exclusive_after_protected_admission
external_repository_write_authority: none
```

## Activation gate

The prospective worker remains `NOT_ADMITTED` until all of the following are true:

1. this exact allocation is independently reviewed where required, passes applicable exact-head repository checks, integrates through the normal Merge Queue, and is read back from protected `main`;
2. Work re-reads protected #383 authority, Issue #384 and open PR/path ownership;
3. no competing writer owns any allocated path;
4. Work records the protected allocation merge SHA as immutable admission and explicitly grants the exclusive registry lease;
5. only then is `agent/gamesession-ledger-resource-registry-384` created from that exact protected admission SHA.

## Exact outcome

The admitted worker appends exactly the architecture-owned row `FND04-GAMESESSION-USED-IDS-PER-CHARACTER` with fixed hard maximum `65536` and the protected Decision A/FND-04C semantics. All pre-existing registry entry objects remain byte/semantic-equivalent. No value or error behavior may be invented or tuned by the worker.

## Excluded scope

No Foundation/runtime Rust, Durability/SQL/migrations, Cargo/lockfile, workflow/protection, Server Seam, Platform/Atlas/META, production, deployment, credentials, secrets or live data. This allocation does not release #361, #335, #356, #247, Decision B/WP3, WP5 or #308 frozen scope.

## Validation and closeout

Registry delivery requires JSON/required-field/unique-ID validation, governance validation, exact changed-file/diff self-review, one genuinely independent exact-head review because the resource affects session/recovery authority, normal exact-head CI and protected Merge Queue. After protected readback, archive the task and release the exclusive registry lease before #162 issues any WP2 implementation amendment.
