# OTV2-20260814-ability-combat-first-evidence-fixtures — archived

```yaml
task_id: OTV2-20260814-ability-combat-first-evidence-fixtures
title: Add first ABILITY_COMBAT evidence cases and fixture specs
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
delivery_branch: docs/ability-combat-first-evidence-fixtures
delivery_pr: 255
delivery_base_sha: 996da4270beadc548781fb81e95ea342e84b6376
delivery_final_head_sha: 6744f655c6438eebeab70b30aae17d33b5bd2fa7
delivery_merge_sha: d04f0939f0078cb677ca3ad66f5949e9f3dadc8d
closeout_branch: docs/ability-combat-evidence-closeout
closeout_issue: 256
closeout_pr: 257
owner: released_after_closeout
created_at: 2026-08-14T12:47:53+02:00
delivery_merged_at: 2026-08-14T13:54:39+02:00
execution_budget_minutes: 60
implementation_status: NOT_STARTED
runtime_client_authority: NONE
postgresql_ddl_migration_authority: NONE
platform_write_authority: NONE
production_authority: NONE
owned_paths: []
public_contracts:
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
  - docs/architecture/GAME-ABILITY-01_FIRST_REFERENCE_EVIDENCE_FIXTURE_PACKAGE.md
  - docs/agents/evidence/OTV2-20260814-ability-combat-official-spell-library.md
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

The first bounded representative `ABILITY_COMBAT` Reference evidence package is delivered. Reference manifest v1 advances from revision 2 to revision 3, `ABILITY_COMBAT` becomes `MECHANIC_CASES_REGISTERED`, and four bounded cases are registered for Light Healing (`exura`) and Ice Strike (`exori frigo`).

The delivery is deliberately fail closed:

- target evidence: `UNKNOWN` for all four immutable-target cases;
- source provenance: `PENDING` for all four indexed official sources;
- case provenance: `PENDING`;
- legal review: `PENDING`;
- Oteryn implementation: `NOT_STARTED`;
- exact implementation revision: null;
- fixture/test links: empty;
- parity: `PARITY_PENDING_EVIDENCE`;
- no `PARITY_CONFIRMED` claim.

The source package records exact official Tibia Library locators, but content was surfaced through search with an approximately two-week-old index freshness signal while direct page opening from the research environment returned HTTP 403. Exact crawl/live observation time and equality with the live page are not claimed. Under the owning Reference manifest contract, indexed/post-target content cannot establish the immutable 2026-07-28 target without continuity evidence, and uncleared provenance independently blocks promotion.

## Review and repair history

1. Self-review found that post-boundary indexed official content had initially been classified too strongly as target `OBSERVED`; it was repaired to `UNKNOWN` because continuity from the immutable 2026-07-28 cut was not evidenced.
2. Self-review found that retrieval had initially been described too strongly as a current/live official page. It was repaired to state the actual retrieval mode: exact official locator, search-indexed content, approximate crawl freshness, direct-fetch HTTP 403, exact crawl/live time unknown.
3. Self-review found source/case `provenance_state=CLEARED` was too strong without direct source revalidation. Evidence, manifest and catalogue/blueprint projections were repaired to `PENDING`; provenance clearance became an explicit independent future prerequisite.
4. Final exact-head self-review on `6744f655c6438eebeab70b30aae17d33b5bd2fa7` passed with zero open material findings; immutable PR evidence is comment `5292724813`.
5. The owner explicitly authorized Codex review for **PR #255 only**. Automatic Codex review started on ready transition and completed with PR `+1` reaction `454048359`; no review finding/thread was produced.

The owner authorization for Codex was exact-use authorization for PR #255 and is not standing permission for closeout PR #257 or later work.

## Terminal delivery validation

Exact final head: `6744f655c6438eebeab70b30aae17d33b5bd2fa7`.

- changed scope: exactly four intended paths;
- compare to delivery base before merge: `behind_by=0`;
- Agent governance run `31795833321`: PASS;
- Merge authority audit run `31795833334`: PASS;
- Merge gate run `31795833324`: PASS, including `Merge gate / validate`, governance, Dependency Review, CodeQL actions and CodeQL python;
- Rust workspace/policy/supply-chain/Windows gates: correctly NOT_APPLICABLE/skipped for docs/contracts-only scope;
- unresolved review threads before merge: 0;
- owner-authorized Codex review: no findings (`+1` reaction);
- component/integration/runtime E2E: NOT_APPLICABLE — paper-only evidence/architecture package;
- squash merge: `d04f0939f0078cb677ca3ad66f5949e9f3dadc8d`;
- post-merge main verified at the same SHA;
- issue #254 closed as completed.

## Lifecycle closeout

Closeout issue #256 / draft PR #257 perform lifecycle/status bookkeeping only: move this task from active to archive, release advisory ownership, and reconcile stale coordination overlays so they no longer select creation of the Reference manifest or first representative ABILITY_COMBAT package as future work.

The next paper-only programme action after closeout is a bounded **target-continuity + provenance-clearance evidence package for the four registered ABILITY_COMBAT cases**. It must obtain provenance-cleared, time-appropriate evidence that directly bridges/captures the immutable 2026-07-28 target boundary and then either promotes or rejects the indexed hypotheses. It must not infer continuity from patch/search silence and must not broaden mechanics or freeze physical catalogue tooling first.

No runtime/client/protocol/content/DDL/Platform/production authority follows from delivery or closeout.

## Context checkpoint

```yaml
last_progress: PR #255 exact final head 6744f655c6438eebeab70b30aae17d33b5bd2fa7 passed self-review, owner-authorized Codex no-finding review and required exact-head gates, then squash-merged as d04f0939f0078cb677ca3ad66f5949e9f3dadc8d; main and issue #254 were verified post-merge; lifecycle closeout is issue #256 / draft PR #257.
status: completed
delivery_pr: 255
final_head_sha: 6744f655c6438eebeab70b30aae17d33b5bd2fa7
delivery_merge_sha: d04f0939f0078cb677ca3ad66f5949e9f3dadc8d
closeout_issue: 256
closeout_pr: 257
ci_run_ids:
  - 31795833321
  - 31795833334
  - 31795833324
codex_review_authorized_scope: PR_255_ONLY
codex_result: NO_FINDINGS_PLUS1
ownership_released: true
owner_action_required: false
blocker: null
next_action: none
```
