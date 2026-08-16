# OTV2-20260816-game-atlas-export-contract-v1

```yaml
task_id: OTV2-20260816-game-atlas-export-contract-v1
title: Define Game -> Atlas immutable export v1 semantic contract
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
delivery_branch: docs/game-atlas-export-contract-v1
closeout_branch: docs/game-atlas-export-contract-v1-closeout
issue: 286
delivery_pr: 287
closeout_pr: 289
base_sha: afcbf8585ba23c506242978c38b2b51f9ea6f1b6
reconciled_main_sha: 005e31d7ddb137e77bc6825c248ec4b78e55b9cc
delivery_final_head_sha: 36075da1698df0be6e1c89edc2cce9e08f3e21e6
delivery_final_head_frozen_at: 2026-08-16T09:35:21+02:00
delivery_merge_sha: f19f543144693960c66e4ebff384d323ccdcbb56
owner: SENIOR OTERYN ECOSYSTEM ARCHITECT / MIGRATION COORDINATOR
owner_state: released_by_closeout
created_at: 2026-08-16T09:30:43+02:00
updated_at: 2026-08-16T09:52:00+02:00
execution_budget_minutes: 60
owned_paths: []
historical_owned_paths:
  - docs/agents/tasks/active/OTV2-20260816-game-atlas-export-contract-v1.md
  - docs/contracts/OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md
public_contracts:
  - docs/contracts/OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md
depends_on:
  - blakinio/Oteryn-Platform@b549e42041eda426bbf88db469862a92df930860:docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md
  - blakinio/Oteryn-v2@afcbf8585ba23c506242978c38b2b51f9ea6f1b6:docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md
  - blakinio/Otheryn@39cb2ce4ff427e7c3760eb6112b45efc0c1f73b8:docs/architecture/OTERYN_ATLAS_EXTRACTION_REVIEW_2026-08-15.md
blocks: []
cross_repository_coordination_id: OTERYN-GAME-ATLAS-V1
delivery_source_branch_disposition: auto_deleted_after_merge
delivery_source_branch_evidence: exact branch search after PR #287 merge returned no docs/game-atlas-export-contract-v1 ref
```

## Outcome

The first formal Game-owned semantic contract for the immutable public-safe dataset consumed by future `Oteryn-Atlas` is merged on protected `main` as `f19f543144693960c66e4ebff384d323ccdcbb56` at `docs/contracts/OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md`.

The contract freezes semantic requirements required before producer/consumer implementation and Atlas extraction while deliberately deferring physical serialization, compression, chunk dimensions, storage/CDN, Atlas framework, delta encoding, repository transfers and production deployment.

## Architecture and evidence

- **PROVEN** — Platform ADR 0041 assigns Game ownership of Atlas export semantics/public allowlist/deterministic exporter/producer validation/provenance and Atlas ownership of consumer validation/index/cache/render/publication.
- **PROVEN** — Oteryn-v2 ADR-0005 keeps OTBM/legacy formats behind bounded importer boundaries and makes canonical Oteryn World/Content authoritative.
- **PROVEN** — the merged Otheryn Atlas extraction audit is `EXTRACTABLE_WITH_REFACTOR` and identified the formal Game -> Atlas contract as P0.
- **PROVEN** — delivery PR #287 merged after exact-head CI/review; Issue #286 is closed completed; the delivery source branch was automatically deleted and verified absent.
- **DERIVED** — exporter/consumer design can now use one canonical semantic boundary, but runtime compatibility remains unimplemented and unproven.
- **UNKNOWN / deferred** — exact bytes/schema profile, compression, digest algorithm/profile, chunk dimensions, storage/CDN, Atlas framework, retention, deltas and third-party asset redistribution.
- **CONFLICT / MODEL LIMITATION** — `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json` requires a `LOCKED` entry to carry physical `schema_revision` and `schema_sha256`, while semantic v1 intentionally does not define a physical schema. This task therefore does not fabricate a lock record. A successor must either accept the first physical Game -> Atlas schema/profile and lock its immutable digest, or formally evolve the lock model to represent semantic-contract revisions separately from physical schema revisions.

## Acceptance criteria

- [x] Canonical Game-owned semantic v1 contract exists and pins exact authority/evidence revisions.
- [x] Game/Atlas/Platform ownership boundaries are explicit.
- [x] Full deterministic immutable snapshots, provenance, stable identities, coordinate/floor/order profile requirements, default-deny public projection, ambiguity states, compatibility, limits and rollback are specified.
- [x] OTBM, Legacy IR, canonical World Project wholesale, undocumented Game DB tables and live GameNode state are forbidden Atlas truth/fallback sources.
- [x] Physical serialization/storage/delta/repository migration choices remain explicitly deferred.
- [x] Exact delivery head `36075da1698df0be6e1c89edc2cce9e08f3e21e6` received PASS self-review with zero material findings and zero unresolved review threads.
- [x] Required exact-head `Merge gate / validate` passed before delivery merge.
- [x] Delivery PR #287 squash-merged as `f19f543144693960c66e4ebff384d323ccdcbb56`.
- [x] Issue #286 closed completed.
- [x] Delivery source branch was auto-deleted and verified absent.
- [x] Runtime/component/browser E2E is `NOT_APPLICABLE` because delivery changed documentation/semantic contract only.
- [x] Closeout is represented by lifecycle-only PR #289: it removes the active task record, adds this terminal archive, changes no accepted contract/runtime content, and may enter protected `main` only through normal required exact-head governance/merge checks.

## Delivery validation

Final delivery head: `36075da1698df0be6e1c89edc2cce9e08f3e21e6`.

- Agent governance run `31934241122`: PASS.
- Merge authority audit run `31934241126`: PASS.
- Merge gate run `31934241117`: PASS.
- Aggregate `Merge gate / validate` job `95133750537`: PASS.
- Self-review PR review `4945689816`: PASS; zero material findings; zero unresolved review threads.
- Runtime/component/E2E: `NOT_APPLICABLE` — no executable producer/consumer/browser/deployment behavior changed.
- Independent different-agent review: `NOT_REQUIRED` for this documentation-only formalization of already accepted architecture; no runtime protocol, auth/session, durable-data, production, repository-authority or governance-gate behavior changed.

## Closeout evidence semantics

PR #289 is intentionally the canonical source for its own final exact-head SHA, required-check generation and merge SHA. Embedding those self-referential values into this file before merge would require changing the head being certified and would make the recorded evidence stale by construction.

This archive therefore uses a fail-closed invariant instead: **if this archive exists on protected `main` and the corresponding active task path is absent, then PR #289 has completed through the repository's required protected-branch merge path.** The final PR head, required-check IDs/conclusions, merge SHA and closeout-branch disposition must be verified directly from GitHub when reporting live lifecycle state.

Historical failed closeout evidence is retained: Agent governance run `31934553639`, job `95134345744`, failed at PR metadata preflight because the initial PR body lacked `## Scope` and `## Validation`. That generation is not counted as successful evidence. The PR body was repaired before the final closeout generation.

## Excluded scope

- No Rust exporter or Atlas consumer implementation.
- No physical schema/profile or synthetic schema digest.
- No OTBM/Crystal/Canary code movement or history rewrite.
- No repository creation/rename/transfer/extraction.
- No CI/GHCR/deployment namespace mutation.
- No Platform/Otheryn mutation.
- No Synology, DNS, secret, protected-environment or production mutation.
- No third-party asset redistribution decision.

## PR and lifecycle state

- Issue #286: CLOSED / completed.
- Delivery PR #287: MERGED.
- Delivery head: `36075da1698df0be6e1c89edc2cce9e08f3e21e6`.
- Delivery merge: `f19f543144693960c66e4ebff384d323ccdcbb56`.
- Delivery source branch: auto-deleted; verified absent.
- Closeout PR #289: lifecycle-only archive + active-task deletion; accepted contract unchanged.
- Cross-repository lock: intentionally unchanged because no physical schema/profile identity exists yet.
- Ownership: no active owned paths remain once PR #289 is present on protected `main`.

## Post-merge interpretation

Presence of this file on protected `main`, together with absence of `docs/agents/tasks/active/OTV2-20260816-game-atlas-export-contract-v1.md`, is the durable repository-state proof that lifecycle closeout completed. Live GitHub state remains the authority for PR #289's exact final head, required checks, merge SHA and source-branch deletion/disposition.