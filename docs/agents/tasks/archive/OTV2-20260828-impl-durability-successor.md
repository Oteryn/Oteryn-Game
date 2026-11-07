# OTV2-20260828-impl-durability-successor

```yaml
task_id: OTV2-20260828-impl-durability-successor
title: Recover and complete journal-only Durability on clean history
mode: IMPLEMENT
status: superseded
integration_state: HISTORICAL_EVIDENCE_ONLY_NOT_FOR_INTEGRATION
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 167
recovery_issue: 240
parent_coordinator_issue: 162
historical_pr: 212
pr: 243
owner: null
allocation_pr: 241
allocation_merge_sha: a171410de07c2dab718f52f780d4314bdcc53604
admission_main_sha: a171410de07c2dab718f52f780d4314bdcc53604
historical_branch: impl/game-durability-journal-recovery-240
historical_head_sha: eb28c42125c346e7f6f1c72e69d51af35af8fc1f
source_snapshot_pr: 212
source_snapshot_head_at_allocation: fb30fba2a888835dfc7cbde27f940b79d7bfe05d
source_snapshot_mode: read_only_file_content_only_no_commit_inheritance
superseded_by_issue: 250
successor_pr: 252
write_authority: none
owned_paths: []
shared_paths: []
blocks: []
external_repositories: []
updated_at: 2026-09-07
```

## Terminal disposition

SUPERSEDED, not implementation-completed. PR #243 is retained historical
source evidence at the exact head above. Do not resume, merge up, rebase,
re-review for integration, replay its old reconstruction instructions or merge
its source. Closing the obsolete PR is lifecycle reconciliation only; its
branch, commits, review findings and original incident evidence are retained.
No claim is made that #243 itself reached accepted final qualification.

## Governing supersession

Issue #167 comments `5456369600`, `5456381241` and `5462519470` explicitly move
execution to #250/#252 under protected architecture #249 and allocation #251.
The successor was admitted at `12d4ca5326d62a7a2c46d80cd5e167e99f109d1d`;
it copied/adapted only the admitted nine file contents and inherited neither
#243 commits nor its review/CI qualification.

The successor is terminal, according to current Issue #250:
- PR #252 protected merge: `b67f4425e9e9c5bbf9f7bc94c422cd7478edcdd3`;
- task archival PR #290: `47faa84152ffdcac00d8e2173d582aa54be2cbcf`;
- successor branch deleted and its implementation/composition ownership released.

Those are successor facts, not retrospective acceptance of this old candidate.
This archival neither reopens #250 nor closes the later full Durability/Server
Seam programme. Current #329/#335, #351/#356, #353/#361 and #247 custody is
unchanged; no old path is handed to another writer here.

## Correction to the R5 locator

R5/#381 comment `5570203562` correctly observed that #241 was merged, but
its protected cache incorrectly labelled #243 an implementing live candidate
and directed the owner to continue. That comment explicitly granted no new
allocation; an open draft and a metadata correction cannot override the
accepted execution supersession above. Work's exact correction scope is #162
comment `5573970828`.

Removing this stale record from active discovery eliminates an apparent second
writer over Durability files and the now-released 0001 migration. Released
migration immutability and every current worker's exact scope remain intact.
The coordinator changes only this active-to-archive task pair, not source,
SQL, workflow, Cargo, registry, architecture, production or external repositories.

## Preserved recovery evidence

The original complete record, including all reconstruction blob IDs, RED/GREEN
requirements, ten historical paths and R5 locator, remains available at
[protected a793457](https://github.com/Oteryn/Oteryn-Game/blob/a793457cf3001df37109acb2c4b4a772b53db97a/docs/agents/tasks/active/OTV2-20260828-impl-durability-successor.md),
blob `89d2e788c8da98ac75eadda132409741cfcd905d`.
The source-only #243 branch/head and the #212 incident history remain unchanged.
Original #241 admission and the file-content-only source boundary are preserved
above; they are historical provenance, not permission to reconstruct again.

## Validation and closeout

This is a low-risk lifecycle correction enforcing an already explicit
supersession, not a new allocation or safety-policy change. Validate terminal
metadata, exact source/head references, scope and full diff; require ordinary
exact-head repository checks, normal protected Merge Queue and main readback.
Runtime E2E is NOT_APPLICABLE because no executable or data bytes change.
No new external AI review or replay of obsolete runtime qualification is required
solely to retire this stale dispatch record.

The archive candidate is not protected completion until its own merge/readback.
Then Work closes #243 without merging it, retains historical source, and records
the result on #162/#167/#240. This record grants no new worker, consumer
activation, production permission or protected-gate exception. Broader parent
issues require their own separate outcome reconciliation.
