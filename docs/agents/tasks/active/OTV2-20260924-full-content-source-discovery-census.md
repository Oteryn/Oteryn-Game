# OTV2-20260924-full-content-source-discovery-census

```yaml
task_id: OTV2-20260924-full-content-source-discovery-census
correction_task_id: OTV2-20260924-full-content-source-roots-correction
title: Full Tibia content source discovery and family census — source-root correction
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/full-content-source-roots-correction-20260924
pr: 821
base_sha: 0c3445068a696883a35a248c98436f2af87a39b5
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "exclusive G1 source-root correction writer"
created_at: 2026-09-24T00:30:00+02:00
updated_at: 2026-09-24T10:42:00+02:00
execution_policy: api_native_authoring
owned_paths:
  - tools/content-census/source-surfaces.json
  - tools/content-census/source_universe_self_test.py
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

- [x] Explicit source registry covers all 29 G0 surfaces and 24 bounded collection roots, including Objetos, Livros, and Documentos e Papéis.
- [x] Hard exclusions are absent from roots/output: Kalkulatory, Narzędzie do nasycania, Dostawca.
- [x] Category roots use bounded recursive namespace-0/category traversal.
- [x] List/navigation roots use bounded namespace-0 links without prose retention.
- [x] Every live retained page has exact page id/title/revision timestamp, source surfaces, categories/templates, source shape and candidate families.
- [x] Page-id/title conflicts, malformed/looped continuation, request/page/category/template bounds and source revision drift fail closed.
- [x] Live page IDs/titles are deduplicated before G1 counts.
- [x] Protected Item manifest is digest-bound and reused; full Item census is not re-fetched.
- [x] Compact manifest retains counts, digests, invariants, limitations and next gate.
- [x] Bulk full output remains workflow artifact; it is not committed.
- [x] Focused synthetic suite passes 19/19 on the correction branch; baseline G1 suite passed 18/18.
- [x] Hosted live source census passes.
- [ ] Final frozen exact-head repository qualification passes before coordinator review and Merge Queue admission.

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

### Original protected G1 baseline before source-root correction

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

### Original live-source repairs (PR #812)

PR #812 documented six fail-closed repairs: redlink exclusion, case-distinct title handling, canonical roots, provenance-only Updates, revision-stable continuation, and template bounds (4,096/page).



### Corrected live source-root evidence

The bounded correction census ran on PR #821 authoring generation `b8dfd8d723ae108bdb74b379cbdba31aaedcce90`. This run collected the new source evidence; it is not the final frozen-head qualification.

- workflow run: `35976071662` — Full Content Source Discovery: SUCCESS;
- artifact: `10797829617`, ZIP size **1,147,358 bytes**, SHA-256 `d5feaf754382c5396d878e52bcf6035b2f4e805c4453df9087c267aa1dba75b5`;
- source universe: **9,373** live unique pages, **1,059** requests, **24 roots**, **29 surfaces**;
- root-member counts: Objetos **3,120**, Livros **77**, Documentos e Papéis **120**;
- registry SHA-256: `008bee49c72d5bcaf494dd05247dad4f49f16d5a258469c4f8941de807a7e74a`;
- source snapshot SHA-256: `29bfa1fb1b7fd2c8b29104fe90a20c4e6f3952cbc079b1c66c011856034c4b96`;
- full source-universe SHA-256: `a6f5d793d7e4cfde897b78a887bfd7fba5845edc6768fa745f65f6cef0293b8f`;
- stable-without-retrieval-time SHA-256: `17f72a8f63861244b3193e639c3e33a530b641b77a7539653cc066a70c093c6c`;
- compact retained manifest SHA-256: `fb6bb324e3204754df1dc51e2df509a8c1ec780db444015252c6ddf4411d699b`;
- timestamp: `2026-09-24T08:34:58Z`.

Comparison to the previous protected G1 artifact (`5bb2b56e81a2cda00b71e9621ae8808e14b54e0e`, 6,640 live IDs) found **2,733** additional live IDs and no lost IDs. Category-root memberships overlap: page IDs **60220** (`Book About Spells`) and **60222** (`Large Book of Chants`) are shared by Objetos and Livros; both are new relative to the old G1 output.

A provisional identity comparison against draft PR #807's exact Item crosswalk artifact (head `61d051a13329c51ae04d8a011655e279664c334c`, artifact `10778892407`, 6,918 source-page records) found old G1/Item overlap **311**, corrected G1/Item overlap **504**, and **2,540** corrected G1 IDs outside both the previous G1 set and that Item crosswalk. The resulting **15,787** union is provisional because #807 remains draft and cross-source G2 reconciliation is not complete. The G1 manifest's `global_unique_pages_after_g2_overlap` remains `null`; do not treat the provisional union as the final global count.

## Validation

### Focused

- Original protected G1: `python -m py_compile ...`: PASS; focused suite PASS, 18 tests.
- Correction PR #821 local scratch validation: `python -m py_compile tools/content-census/source_universe.py tools/content-census/source_universe_self_test.py`: PASS; focused suite PASS, 19 tests.

### Component/integration

- Original protected G1 live census run `35962506624`: PASS.
- Correction source-evidence run `35976071662`: PASS; full live manifest invariants and artifact upload passed on the initial correction authoring generation.
- manifest invariant step: PASS.
- artifact: `10792259950`, 963,374-byte ZIP, 14-day retention.
- committed bulk corpus: NO.
- retained compact evidence:
  `docs/agents/evidence/OTV2-20260924-full-content-source-discovery-census.json`.

### E2E

- Original scenario: hosted bounded source-universe collection against current TibiaWiki structured API; PASS on run `35962506624`.
- Corrected source roots: hosted live collection and manifest invariants PASS on run `35976071662`; final frozen-head rerun remains pending.

### Exact-head CI

- final correction head: pending freeze after the last authoring write
- trigger source: pull_request on PR #821
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

### Protected predecessor

- original G1 PR #812: **merged and protected-integrated** at merge commit `b9ad0f52f48b06394a5c661b6bdc02fd111964e2`;
- original source-census result is the historical baseline documented above;
- protected Item census #803 remains sealed at 6,918 pages and is reused.

### Bounded source-root correction

- correction task: `OTV2-20260924-full-content-source-roots-correction`;
- correction PR: #821, draft, branch `agent/full-content-source-roots-correction-20260924`;
- admission/base: `main@0c3445068a696883a35a248c98436f2af87a39b5`;
- changed files are the five allocated paths only;
- exact final head, freeze, and final candidate checks are pending after this checkpoint write;
- merge: not attempted; coordinator owns final review, Merge Queue and protected closeout;
- worker terminal state after final exact-head qualification: `READY_FOR_INTEGRATION`.

## Context checkpoint

```yaml
last_progress: corrected 24-root census evidence retained; source-root correction is on PR #821
status: validating
branch: agent/full-content-source-roots-correction-20260924
head_sha: null
pr: 821
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: final-candidate-pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: final exact-head workflows pending after final authoring write
next_action: read back exact new branch head, verify only five allocated paths, freeze that head, and qualify Full Content Source Discovery, Agent Governance, Architecture Semantic Audit and Merge Gate / game-gate
```
