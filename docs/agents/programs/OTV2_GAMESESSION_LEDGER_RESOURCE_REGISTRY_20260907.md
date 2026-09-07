# OTV2 GameSession Ledger Resource Registry Plan — 2026-09-07

Issue: #384
Coordinator: #162
Protected prerequisite: `main@6b07f96d47de37971bb54fed5bb9c12decd1be17` / PR #383

## Goal

Deliver the smallest contract-only step required by protected Decision A: register its fixed `GameSessionUseLedgerV1` lifetime resource bound without modifying any implementation or any pre-existing registry semantics.

## Admission

This plan is inert until the coordinator allocation containing it is independently qualified where required, merged through the normal Merge Queue and read back from protected `main`. Work then records that protected merge SHA as immutable admission, rechecks overlap and grants one exclusive registry lease to `agent/gamesession-ledger-resource-registry-384`.

Do not create the worker branch from the pre-allocation main and do not mutate the registry on the coordinator allocation branch.

## Exact implementation

On the admitted worker branch:

1. Read the protected `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` and protected Decision A Section 8.
2. Preserve the complete ordered set of existing registry entry objects unchanged.
3. Append exactly the architecture-owned row `FND04-GAMESESSION-USED-IDS-PER-CHARACTER`.
4. Use fixed `hard_maximum = 65536` and `configurable_range.minimum = configurable_range.maximum = 65536`.
5. Preserve the protected permanent-exhaustion semantics: `CAPACITY_EXCEEDED`, family-specific terminal FND-04C ledger-exhaustion codes, public class `SESSION_UNAVAILABLE`, no independent retry under the same ledger version/ceiling, no new authority effects on denial, and exact committed replay reconcilable at capacity.
6. Do not modify another resource row or broaden the registry schema.

## Validation

Before final-head freeze:

- parse the complete JSON;
- prove every required entry field is present and every ID is unique;
- prove the new row equals the protected architecture-owned semantic row;
- prove every pre-existing entry object is unchanged relative to the exact admitted base;
- run `python tools/agents/validate_governance.py` and any current repository/contract validation selected for `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`;
- inspect the complete changed-file list and full diff;
- perform mandatory whole-diff self-review.

The frozen head requires one genuinely independent review because the resource controls future session/recovery capacity behavior, then normal exact-head repository CI and protected Merge Queue. Runtime/PostgreSQL E2E is `NOT_APPLICABLE` to this contract-only delivery; the protected Decision A implementation acceptance still requires real Foundation + PostgreSQL 17.6 evidence later.

## Closeout

After protected merge/readback:

- verify the protected registry row and all unchanged existing rows;
- move the task from `docs/agents/tasks/active/` to the matching `archive/` path in a bounded closeout if merge facts were unknowable before integration;
- release the exclusive registry lease and delete the terminal source branch when no longer needed;
- update #384 and #162 with exact delivery/Merge Queue/protected-main evidence;
- only then may #162 issue the exact WP2/#361 Decision A allocation amendment.

## Exclusions

No runtime, Foundation, Durability, SQL/migration, Cargo, workflow/protection, Server Seam, external repository, production, deployment, credentials, secrets or live data. Decision B/WP3, WP5 and #308 remain unchanged blockers.
