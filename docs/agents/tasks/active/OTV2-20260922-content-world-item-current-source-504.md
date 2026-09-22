# OTV2-20260922-content-world-item-current-source-504

```yaml
task_id: OTV2-20260922-content-world-item-current-source-504
title: D6-M1 Item current-source TibiaWiki crosswalk
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-item-current-source-504
pr: null
base_sha: 3ebdca55840573861a63e6e4de1436ac1a6e60ec
head_sha: pending
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: item current-source lead"
created_at: 2026-09-22T16:20:00Z
updated_at: 2026-09-22T17:15:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/reference-world-corridor-census/item_current_source_tibiawiki.py
  - tools/reference-world-corridor-census/item_current_source_tibiawiki_self_test.py
  - docs/agents/evidence/OTV2-20260922-content-world-item-current-source.json
  - docs/agents/tasks/active/OTV2-20260922-content-world-item-current-source-504.md
public_contracts: []
depends_on:
  - "#749 protected Item schema"
  - "#763 protected 38157 Item classification crosswalk"
blocks:
  - ITEM_FIELD_VERIFICATION
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Produce one bounded, reproducible TibiaWiki current-source evidence successor over the protected 38,157 Item identity closure. Every protected identity receives an explicit `WIKI_MATCHED | WIKI_NOT_FOUND | WIKI_AMBIGUOUS | WIKI_CONFLICT` disposition. Full row/page output remains scratch evidence; the repository retains only a compact provenance/count/digest manifest.

## Architecture and source of truth

- **PROVEN:** protected #749 supplies the canonical typed Item schema and bounded v4 artifact profile.
- **PROVEN:** protected #763 supplies the exact 38,157 native identity closure and source-signal profiles; this task consumes its reproducible full scratch output and never regenerates identity.
- **PROVEN:** protected B3 loot Item evidence supplies exact source-name discovery seeds for a bounded subset of B1 source identities. Names are discovery evidence only and never identity proof.
- **PROVEN:** TibiaWiki is `STRUCTURED_REFERENCE_DATA` under the protected Reference source registry, not gameplay truth.
- **PROVEN:** target cut remains 2026-07-28. A newer current revision is retained with `target_continuity=UNKNOWN` unless separately proven.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: read-only public-source evidence collection; no production/runtime/durable mutation authority
```

## Acceptance criteria

- [ ] Consume the exact protected #763 full scratch crosswalk digest and reject drift.
- [ ] Consume exact protected B3 evidence digest and reject drift.
- [ ] Use only public MediaWiki API with explicit identifying User-Agent.
- [ ] Bound requests, response bytes, page bytes, field counts, string lengths, retries/backoff and cache record bytes.
- [ ] Cache/resume by exact page/revision and avoid content refetch on exact-revision cache hit.
- [ ] Normalize current page/revision provenance deterministically without committing raw wiki corpus.
- [ ] Name-only discovery never yields `WIKI_MATCHED`.
- [ ] A match requires at least one admitted non-name comparable signal and zero admitted contradictions.
- [ ] Missing discovery evidence remains explicit residual rather than guessed identity.
- [ ] Post-target current revisions remain continuity `UNKNOWN` in this generation.
- [ ] Full disposition partition closes exactly 38,157 records.
- [ ] Compact manifest records counts/digests/limitations and explicitly claims no promotion.

## Excluded scope

No Crystal/B1 re-import, source XML reread, identity allocation/remint, schema/resource-profile mutation, semantic promotion, runtime/client/registry/persistence change, second Item model/parser, name-only identity resolution, majority voting, or automatic target-cut continuity claim.

## Implementation / findings

- Collector and self-test authored on the allocated branch.
- Local synthetic self-test: 15/15 PASS.
- Independent local DeepSeek-R1 32B read-only review identified two P1 concerns: contradiction handling and target-cut continuity. Both are covered by the implementation and dedicated tests; no AI output is authority.
- Real protected-input reproduction and live TibiaWiki collection are in progress.

## Validation

### Focused

- command/run: `python tools/reference-world-corridor-census/item_current_source_tibiawiki_self_test.py`
- result: PASS 15/15 locally; repository-host rerun pending after final authoring.

### Component/integration

- command/run: protected #763 native-map export + classification crosswalk reproduction + current-source collector against real TibiaWiki API
- result: in progress

### E2E

- scenario: current source only; semantic promotion/runtime/client E2E is a later programme gate
- result: NOT_APPLICABLE to this generation

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: parser nested-template close detection repaired before publication
- verdict: pending final whole-diff review

## Independent review

- required: NO for external paid review; local DeepSeek-R1 32B used only as advisory prefreeze review
- exact head: NOT_APPLICABLE
- method/auditor: local advisory model
- material findings: contradiction handling and target continuity, both already covered
- verdict: advisory PASS_WITH_REPAIRS; repairs present before candidate freeze

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #749, #763, #764 protected predecessors
- protected auto-merge: forbidden substitute; governed Merge Queue only
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: collector and self-test published prefreeze; real-input validation running
status: implementing
branch: agent/content-world-item-current-source-504
head_sha: pending
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
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: reproduce protected #763 full scratch crosswalk and execute live current-source collection
```
