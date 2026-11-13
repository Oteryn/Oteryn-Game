# OTV2-20261113-bestiary-client-gap-audit

```yaml
task_id: OTV2-20261113-bestiary-client-gap-audit
title: Retain source-first Bestiary inventory and qualified initial UI gaps
mode: AUDIT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/bestiary-client-gap-audit-20261113
pr: null
base_sha: 3e168ce6afe374485da20ee0ac13d3749558e2f8
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: codex-bestiary-evidence-publication
created_at: 2026-11-13T10:30:00Z
updated_at: 2026-11-13T10:35:17Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/evidence/bestiary-client-gap-audit-20261113/
  - docs/agents/tasks/OTV2-20261113-bestiary-client-gap-audit.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Preserve the source inventory and qualified initial UI findings so later
Bestiary reading targets actual gaps rather than repeating known source rows.
The owner requested a separate publication worker; the lead alone controls the
test-client UI. This worker neither clicks nor maintains the game session.

## Architecture and source of truth

- PROVEN: comparison is pinned to the base SHA above; parser/source receipts
  identify the exact repository blobs and private client-file hashes.
- PROVEN: current client has 845 records/21 classes; all 833 earlier imported
  records have identical raw hashes; twelve were added.
- DERIVED: 813 unique name/outfit candidates and 32 unresolved rows are source
  joins, not client-ID authority or a missing-entity count.
- UNKNOWN: remaining UI detail, full literal locations, and pixel/animation
  parity. The whole Bestiary audit remains incomplete.
- CONFLICT: Rat detailed UI locations differ from the generic profile text.
  This packet retains evidence; it does not select a new canonical value.

## High-risk authority/recovery qualification

NOT_APPLICABLE: this change is retained documentation and validation only.
No protocol, persistence, game input, runtime admission or production change.

## Acceptance criteria

- [x] Retain bounded source counts, exact deltas and qualified candidate status.
- [x] Keep private image bytes and proprietary original assets out of Git.
- [x] Preserve proof hashes and explicit partial/unknown boundaries.
- [x] Include the lead's final immutable continuation checkpoint.
- [x] Run required local governance checks.
- [x] Prepare one separate draft PR candidate; whole Bestiary audit remains partial.

## Reference parity

NOT_APPLICABLE as mechanic implementation. Evidence comparison is recorded in
the package README; this audit adds no game rule or runtime behavior.

## Excluded scope

Items worktrees/PRs, Cloud branch writes, raw PNG/APNG/DAT assets, gameplay,
anti-logout, runtime stat correction, deployment and merge.

## Implementation / findings

The existing clean spell worktree was reused on a dedicated new branch from
fresh main. The original spell branch remains preserved. Others WIP and the
historical Mantra worktree were left intact. Cloud Items PR #3170 has disjoint
paths; the targeted open-PR query found no existing Bestiary audit PR.

The owner switched to Charms after the final immutable checkpoint. Retained
continuation: 19 search attempts, only nomad/butterfly results qualified,
Wafer Paper Butterfly detail, and 78 further private image receipts. Earlier
full-grid pixel classifications are superseded because failed searches can
retain animated results. No additional Bestiary collection is planned here.

## Validation

- Focused validator with private image rehash and repository blob rehash: PASS.
- Ten evidence-boundary control/negative tests: PASS.
- `python tools/agents/validate_governance.py`: PASS.
- `python -m unittest discover -s tools/agents/tests`: PASS, 129 tests.
- `git diff --check`: PASS.
- Component/integration/E2E: NOT_APPLICABLE, no game/runtime/content input changes.
- Exact-head CI: pending first publication; no green/mergeability claim.
