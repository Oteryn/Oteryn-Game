# OTV2-20260924-full-content-source-discovery-census

```yaml
task_id: OTV2-20260924-full-content-source-discovery-census
title: Full Tibia content source discovery and family census
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/full-content-source-discovery-census-20260923
pr: null
base_sha: eb90813043e4216865994c2f00f50ab240fcb28c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "single autonomous G1 content-census writer"
created_at: 2026-09-24T00:30:00+02:00
updated_at: 2026-09-24T00:30:00+02:00
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
  - "G0 PR #808 protected main eb90813043e4216865994c2f00f50ab240fcb28c"
  - "PR #803 protected wiki-first Item census; reuse, do not restart"
blocks:
  - "G2 GLOBAL_SOURCE_OVERLAP_AND_DEDUPLICATION"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Implement G1 `FULL_CONTENT_SOURCE_DISCOVERY_AND_FAMILY_CENSUS`: deterministically enumerate the in-scope TibiaWiki source universe from explicit navigation/category/list roots, retain only bounded structured discovery metadata, deduplicate exact source identities, and produce a compact manifest with source-shape/family-candidate counts and provenance.

## Architecture and source of truth

- PROVEN: G0 is protected-integrated through PR #808 / protected `main@eb90813043e4216865994c2f00f50ab240fcb28c`.
- PROVEN: G0 retains current WorldProject/v2 and establishes `tools/content-census/` for new G1+ tooling.
- PROVEN: PR #803 Item census remains the protected Item lane and must not be restarted.
- PROVEN: current TibiaWiki home still exposes the material gameplay/content navigation used to seed this census; source collection itself must use exact MediaWiki API revision/category/template metadata.
- DERIVED: G1 can discover and classify source provenance without identity crosswalk or semantic promotion.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: "Source evidence tooling only; no runtime authority, protocol, persistence, session/fence or production mutation."
```

## Acceptance criteria

- [ ] Explicit in-scope source-surface registry covers G0 navigation dispositions and contains no hard-exclusion roots.
- [ ] Category roots recursively enumerate namespace-0 pages and materially relevant subcategories under finite depth/request/page/category bounds.
- [ ] List/navigation roots discover bounded namespace-0 links without retaining page prose.
- [ ] Every retained source page has exact page id, title, revision id/timestamp, discovery surfaces, categories/templates where available, source shape and candidate family set.
- [ ] Page-id/title conflicts, malformed continuation, continuation loops, duplicate source identity, source snapshot instability and bound overflow fail closed.
- [ ] Hard exclusions are rejected before traversal/retention and never appear in output.
- [ ] Source identities are globally deduplicated before counts.
- [ ] Item root reuses the protected #803 manifest as authoritative Item census evidence; G1 does not refetch the full Item corpus.
- [ ] Compact repository manifest records counts, digests, invariants, limitations and next gate; bulk full output remains CI/scratch artifact.
- [ ] Focused synthetic tests pass.
- [ ] Hosted live source census passes on exact frozen head.
- [ ] Repository exact-head qualification passes before Merge Queue admission.

## Excluded scope

- No identity crosswalk.
- No semantic field verification/promotion.
- No identity minting.
- No WorldProject population.
- No runtime/client/balance mutation.
- No long-form article/quest/book/lore prose collection.
- No image/sprite/art asset collection.
- No crawl/inventory/evidence for Kalkulatory, Narzędzie do nasycania / Imbuement Tool, or Dostawca / reseller utility surfaces.
- No restart of the protected #803 Item wiki-first census.

## Implementation / findings

In progress.

## Validation

### Focused
- command/run: pending
- result: pending

### Component/integration
- command/run: pending
- result: pending

### E2E
- scenario: hosted bounded TibiaWiki API source census
- result: pending

### Exact-head CI
- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: source-evidence tooling
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: pending
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #803 #808 #810
- protected auto-merge: forbidden substitute; governed Merge Queue only
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: G0 integrated; G1 branch allocated from protected main
status: implementing
branch: agent/full-content-source-discovery-census-20260923
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
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
blocker: null
next_action: implement bounded source-universe registry, collector and synthetic tests
```
