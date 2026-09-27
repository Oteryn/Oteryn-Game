# OTV2-20260927-client-asset-version-pin

```yaml
task_id: OTV2-20260927-client-asset-version-pin
title: Pin target client graphics to Tibia 15.30 via a checksum-only asset manifest
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: 1025
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session
created_at: 2026-09-27T00:00:00Z
updated_at: 2026-09-27T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - imports/official/index.json
  - imports/official/client-assets/15.30/manifest.json
  - imports/official/client-assets/15.30/README.md
  - docs/architecture/OTERYN_CLIENT_ASSET_VERSION_OWNER_DECISION_2026-09-27.md
  - docs/agents/tasks/active/OTV2-20260927-client-asset-version-pin.md
  - docs/agents/tasks/archive/OTV2-20260927-cw2-b1-crystal-item-identity-bindings.md
  - docs/agents/tasks/archive/OTV2-20260927-item-authoring-followups-lowering-v1.md
public_contracts: []
depends_on:
  - "docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: KAN-16
```

## Outcome

Record the owner's 2026-09-27 client-graphics version-pin decision (target Tibia
client 15.30, Summer Update 2026) and commit its evidence as a checksum-only
manifest of the owner's local official client `assets` folder, with no proprietary
asset content in the repository. Close two finished predecessor task records whose
PRs already merged.

## Architecture and source of truth

- PROVEN: `docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md` §8
  names `imports/official/` as one of the four import-provenance source
  directories; `imports/official/index.json` (`OTERYN_GAME_TREE_DIRECTORY/v1`)
  described it as `READY_UNPOPULATED` with notes "Official/public source evidence
  snapshots/manifests as legally permitted."
- PROVEN: `tools/repository/classify_pr_test_lanes.py`
  (`auxiliary_path`/`routing_surface`) and `tools/repository/test_classify_content_routing.py`
  route every `imports/**` path (already covering `imports/crystalserver/*.json`,
  `imports/tibiawiki/*.json`) as auxiliary/import-provenance, not product or atlas;
  no per-subdirectory special case exists or is needed for `imports/official/**`.
- PROVEN: server-side Item data is pinned to Canary
  `47dfd51f45280a59a1d3e50ba7edd573d7234446` / Crystal
  `ff7ede593c69d4c658b382c97443e8155926924a` (`CLIENT_VERSION` 1525 = 15.25), per
  the archived Item authoring/G4 identity-binding task records.
- DERIVED: `imports/official/client-assets/15.30/manifest.json` is the correct home
  for the owner-supplied checksum manifest: it matches the `official` directory's
  documented purpose, needs no new routing/lane rule, and mirrors the sibling
  `imports/<source>/index.json` + data-file layout already used by
  `imports/crystalserver/` and `imports/tibiawiki/`.
- UNKNOWN: the future Content/World client asset compiler/loader that will consume
  this manifest; not built by this task.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline documentation/import-provenance evidence only (a
checksum manifest and owner-decision record); no production mutation, session
fence, authority-bearing controller, protocol, persistence or client/runtime
implementation is touched.

## Acceptance criteria

- [x] `imports/official/client-assets/15.30/manifest.json` holds the owner-supplied
  `OTERYN_CLIENT_ASSET_MANIFEST/v1` manifest unchanged in content (verified equal
  by parsed JSON comparison), reserialized canonically (`sort_keys`, compact
  separators, matching sibling `imports/*/bindings/*.json` bulk-listing style).
- [x] A README in that directory states: checksum-only, no proprietary files, how
  it is produced, what consumes it, and the 1,409-object 15.30/15.25 gap.
- [x] `imports/official/index.json` is updated (`population_state: POPULATED`) to
  reflect the new manifest.
- [x] `docs/architecture/OTERYN_CLIENT_ASSET_VERSION_OWNER_DECISION_2026-09-27.md`
  records owner decisions 1-3, non-claims and follow-ups, mirroring the header
  style of an existing owner decision doc, at 72 lines (<= 80).
- [x] The two finished predecessor task records
  (`OTV2-20260927-cw2-b1-crystal-item-identity-bindings`,
  `OTV2-20260927-item-authoring-followups-lowering-v1`) are archived with their
  real merge commits/dates taken from `git log origin/main` (PR #989 -> `0c7098eb`,
  repair PR #996 -> `0e48ae51`, PR #1018 -> `d8285019`).
- [ ] `python3 tools/agents/validate_governance.py` and the imports-path-covering
  validators pass on the frozen head.
- [ ] Changed-path checks pass on the frozen head.

## Excluded scope

No proprietary client asset file (sprite, `.dat`/`.spr`/`.bin` binary, atlas) is or
will be committed. No client asset compiler/loader, atlas build, runtime or
protocol implementation. No server-side Item/Presentation/identity-binding change
and no re-pin of Canary/Crystal. No TibiaWiki population of the 1,409 new 15.30
object ids (separate future task, per the owner decision doc).

## Implementation / findings

- Reformatted the owner-supplied manifest input canonically (`sort_keys=True`,
  compact separators, trailing newline); confirmed byte-for-byte JSON-value
  equality with the original input before and after.
- Confirmed via `tools/repository/classify_pr_test_lanes.py` /
  `test_classify_content_routing.py` that `imports/**` already routes generically
  as auxiliary/import-provenance regardless of subdirectory, so no new routing
  rule was needed for `imports/official/client-assets/**`.
- Confirmed `tools/architecture-check` (Rust workspace-boundary check) and
  `.github/workflows/architecture-semantic-audit.yml` do not reference
  `docs/architecture/README.md` registration or any fixed doc index, so no
  machine-enforced registration applies to the new owner-decision doc; left
  `docs/architecture/README.md` unchanged, consistent with comparable owner
  decision docs (`DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md`,
  `FND-ID-01_OWNER_DECISION_CHECKPOINT_2026-08-07.md`) that are also not listed
  there.
- Archived both finished predecessor task records with terminal closeout sections
  citing exact `git log` commit/parent/tree/date/file-count facts only (no
  GitHub API call), following the closeout pattern in
  `docs/agents/tasks/archive/OTV2-20260926-item-authoring-formal-schema-v1.md`.

## Validation

### Focused

- command/run: `python3 -c "import json; assert json.load(open('manifest-input.json')) == json.load(open('imports/official/client-assets/15.30/manifest.json'))"`
- result: PASS (parsed JSON values identical after canonical reserialization)

### Component/integration

- command/run: `python3 tools/agents/validate_governance.py`
- result: pending (run and recorded before freeze)
- command/run: `python3 tools/repository/test_classify_content_routing.py`
- result: pending (run and recorded before freeze)

### E2E

- scenario: `NOT_APPLICABLE`; auxiliary import-provenance evidence and docs only,
  no Rust/game-gate build consumes `imports/official/**` yet

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing session
- material findings: confirmed no validator special-cases `imports/official/**`;
  confirmed no docs/architecture registration is machine-enforced
- verdict: PASS on the unpublished candidate; exact remote head review remains
  before freeze

## Independent review

- required: NO; checksum-only import-provenance evidence and documentation, no
  gameplay identity, protocol, persistence or production surface touched
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none known
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: manifest placed and reformatted canonically, README and owner-decision doc written, two finished predecessor task records archived with git-log-sourced closeout facts
status: implementing
branch: claude/compassionate-albattani-s29syw
head_sha: null
pr: 1025
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
next_action: run validate_governance.py and the imports-covering content-routing tests, then publish and freeze the exact remote head
```
