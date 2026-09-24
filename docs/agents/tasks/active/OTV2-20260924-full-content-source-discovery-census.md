# OTV2-20260924-full-content-source-discovery-census

```yaml
task_id: OTV2-20260924-full-content-source-discovery-census
title: Full Tibia content source discovery and family census
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/full-content-source-discovery-census-20260923
pr: 812
base_sha: eb90813043e4216865994c2f00f50ab240fcb28c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "single autonomous G1 content-census writer"
created_at: 2026-09-24T00:30:00+02:00
updated_at: 2026-09-24T08:06:00+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/content-census/source_universe.py
  - tools/content-census/source_universe_self_test.py
  - tools/content-census/source-surfaces.json
  - .github/workflows/full-content-source-discovery.yml
  - docs/agents/evidence/OTV2-20260924-full-content-source-discovery-census.json
  - docs/agents/tasks/active/OTV2-20260924-full-content-source-discovery-census.md
public_contracts: []
depends_on:
  - "G0 PR #808 protected-integrated"
  - "PR #803 protected wiki-first Item census; reused, not restarted"
blocks:
  - "G2 GLOBAL_SOURCE_OVERLAP_AND_DEDUPLICATION until G1 protected integration"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

G1 `FULL_CONTENT_SOURCE_DISCOVERY_AND_FAMILY_CENSUS` deterministically discovers the in-scope TibiaWiki source universe from explicit bounded category/list roots, retains only structured identity/revision/category/template metadata, reuses the protected Item census, and emits compact reproducible evidence without identity resolution or semantic promotion.

## Architecture and source of truth

- PROVEN: G0 is protected-integrated through PR #808.
- PROVEN: PR #803 remains the sealed Item lane: 6,918 pages and stable digest `389875abd364aa9bcb0b09a591989c82ece5098d63b3c23376274048f6ac2f5a`.
- PROVEN: new G1 tooling lives under `tools/content-census/`; predecessor tools remain at protected paths.
- PROVEN: live hosted census run `35962506624` completed successfully on source head `5bb2b56e81a2cda00b71e9621ae8808e14b54e0e`.
- PROVEN: compact retained manifest is byte-identical to artifact `10792259950` manifest (Git blob `25f1ca001ad15ef560787380f6a69b5622b900fb`).
- DERIVED: cross-lane Item/non-Item overlap remains G2 work; G1 nominal total is intentionally not called a global unique total.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: "Source evidence tooling only; no runtime authority, protocol, persistence, session/fence or production mutation."
```

## Acceptance criteria

- [x] Explicit source registry covers all 29 G0 surfaces.
- [x] Hard exclusions are absent from roots/output: Kalkulatory, Narzędzie do nasycania, Dostawca.
- [x] Category roots use bounded recursive namespace-0/category traversal.
- [x] List/navigation roots use bounded namespace-0 links without prose retention.
- [x] Every live retained page has exact page id/title/revision timestamp, source surfaces, categories/templates, source shape and candidate families.
- [x] Page-id/title conflicts, malformed/looped continuation, request/page/category/template bounds and source revision drift fail closed.
- [x] Live page IDs/titles are deduplicated before G1 counts.
- [x] Protected Item manifest is digest-bound and reused; full Item census is not re-fetched.
- [x] Compact manifest retains counts, digests, invariants, limitations and next gate.
- [x] Bulk full output remains workflow artifact; it is not committed.
- [x] Focused synthetic suite passes 18/18.
- [x] Hosted live source census passes.
- [ ] Final frozen exact-head repository qualification passes before Merge Queue admission.

## Excluded scope

- No identity crosswalk, target identity selection or minting.
- No field verification or semantic promotion.
- No WorldProject population.
- No runtime/client/balance mutation.
- No article/quest/book/lore prose.
- No image/sprite/art asset collection.
- No hard-exclusion crawl or placeholder coverage.
- No restart of the protected Item wiki-first census.

## Implementation / findings

### Live result

Hosted run `35962506624` / artifact `10792259950`:

- registry surfaces: **29**;
- real collection roots: **21**;
- live non-Item unique pages: **6,640**;
- sealed protected Item pages: **6,918**;
- nominal pages before G2 cross-lane overlap: **13,558**;
- requests: **779**;
- source shapes:
  - `STRUCTURED_PRIMARY=6,337`;
  - `STRUCTURED_ALTERNATE=187`;
  - `REDIRECT=104`;
  - `SOURCE_CLASSIFICATION_UNRESOLVED=12`;
- candidate family state:
  - `EXACT_FAMILY=1,057`;
  - `MULTI_FAMILY_RELATION=5,583`;
- source snapshot SHA-256: `10ddc604de04fec01cf08f3b4e555f687c450be46b7937fbd7251e9c5c43308f`;
- full-output SHA-256: `1ae1a9d63c42819b90620edade5902323affc3ad6a740394f46864729f40e72e`;
- stable-without-retrieval-time SHA-256: `861bd8980ca3968dac72cfbdda3ec5c5ff05c5b82d8e6814b3d0ba81b1bc954c`;
- artifact ZIP SHA-256: `56813e421d90b6e671bf62a5162d1365df31cdd51a00f502db4d55667a7b44ef`.

The 12 `SOURCE_CLASSIFICATION_UNRESOLVED` records are an explicit source-shape state, not silently dropped entities. Family routing itself is complete for the live set; G3 owns later semantic family-classification closure.

### Source-driven repairs

Live evidence repaired six assumptions without weakening fail-closed behavior:

1. missing/red navigation links are counted and excluded from admitted page identities;
2. case-distinct canonical MediaWiki titles remain distinct identities;
3. non-existent `Casas e Guildhalls` navigation label is not invented as a root; house/building coverage uses canonical Geography discovery;
4. `Updates` remains registry/provenance-only instead of inventing a page root;
5. MediaWiki prop continuation may omit revision metadata after the first exact record, but may never change it;
6. large template sets are bounded at 4,096 entries/page, consistent with the existing 4 MiB API-response fence; 4,097 fails closed.

## Validation

### Focused

- `python -m py_compile tools/content-census/source_universe.py tools/content-census/source_universe_self_test.py`: PASS.
- `python tools/content-census/source_universe_self_test.py`: PASS, 18 tests.

### Component/integration

- live hosted TibiaWiki API census run `35962506624`: PASS.
- manifest invariant step: PASS.
- artifact: `10792259950`, 963,374-byte ZIP, 14-day retention.
- committed bulk corpus: NO.
- retained compact evidence:
  `docs/agents/evidence/OTV2-20260924-full-content-source-discovery-census.json`.

### E2E

- scenario: hosted bounded source-universe collection against current TibiaWiki structured API
- result: PASS on run `35962506624`.

### Exact-head CI

- final head: pending freeze after this write
- trigger source: pull_request on PR #812
- required workflows: Full Content Source Discovery, Agent Governance, Architecture Semantic Audit, Merge Gate / `game-gate`
- result: pending final frozen generation

## Self-review

- exact head: pending freeze after this write
- method/reviewer: implementing/coordinating agent
- material findings: six live-source assumptions repaired above; no runtime/schema/identity authority added; full corpus remains artifact-only.
- verdict: PASS subject to final exact-head repository qualification.

## Independent review

- required: NO — bounded source-evidence tooling only; no runtime/schema/public-contract/production authority change.
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- canonical PR: #812
- state: Draft until frozen exact-head qualification completes
- changed-file scope: six owned G1 paths only
- unresolved review threads: pending final readback
- related PRs: #803 #808 #810
- protected integration: governed Merge Queue only
- merge result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: live G1 census passed; compact manifest retained; this write is the final authoring freeze boundary
status: validating
branch: agent/full-content-source-discovery-census-20260923
head_sha: null
pr: 812
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending-final-freeze
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 6
ci_recovery_actions_for_current_head: 6
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: freeze returned successor head, verify full bounded diff, mark PR #812 ready, and qualify exact head
```
