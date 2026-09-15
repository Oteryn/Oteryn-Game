# OTV2-20260818-target-coordinate-reconciliation

```yaml
task_id: OTV2-20260818-target-coordinate-reconciliation
title: Reconcile Oteryn-Game target repository identity
mode: MIGRATE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/otv2-20260818-target-coordinate-reconciliation
pr: 4
base_sha: 16afdf31a15bd49d454cdbcdd98fa7ec72213ef9
final_head_sha: 0b1f8288c20a69a50628e401fe3a7fb60681f050
merge_sha: d85a5a075aaf72ec88cf2f4167f1aab2ab2ba3a9
owner: chat-github-20260818-target-coordinate-reconciliation
created_at: 2026-08-18T11:06:00Z
completed_at: 2026-08-18T11:43:00Z
execution_budget_minutes: 60
cross_repository_coordination_id: OTERYN-GAME-COPY-20260818
external_repositories:
  - blakinio/Oteryn-v2
  - Oteryn/Oteryn
```

## Outcome

`Oteryn/Oteryn-Game` is now the canonical repository identity for the native Oteryn gameplay stack. The repository was populated by a history-preserving copy from `blakinio/Oteryn-v2`, and the live governance/navigation surfaces were reconciled to the target coordinate in PR #4.

The original `blakinio/Oteryn-v2` repository was not transferred, deleted or archived by this migration. It remains a separate legacy/migration source and historical provenance repository.

## Delivered state

- Target repository: `Oteryn/Oteryn-Game`, repository ID `1338291140`.
- Preserved source snapshot used for the copy: `16afdf31a15bd49d454cdbcdd98fa7ec72213ef9`.
- Target `main` initially resolved to that exact preserved source commit before coordinate reconciliation.
- All 36 observed source branch refs in the migration snapshot were recreated at their original commit SHAs.
- Verified external backup `Oteryn-v2-full-git-backup-2026-08-18-final.zip` remains on the owner's connected Google Drive.
- Root governance and machine-readable governance now authorize routine writes only to `Oteryn/Oteryn-Game` and classify `blakinio/Oteryn-v2` as read-only legacy/migration provenance.
- Current repository links/navigation were reconciled in `README.md`, `Cargo.toml`, security issue routing, agent governance/map/template/lanes and the governance validator.
- Historical ADRs, archived task evidence and immutable source-era provenance were intentionally not mass-rewritten.

## Validation

### Migration / focused

- history-preserving source-to-target commit identity: PASS
- 36 source branch snapshot refs recreated at original SHAs: PASS
- source repository preservation: PASS
- target governance validator: PASS
- changed-file inventory for PR #4: PASS, exactly 12 declared live governance/navigation/task paths
- full-diff self-review: PASS, zero open material findings after removing formatting-only JSON churn
- temporary migration/probe workflows absent from final delivery diff: PASS

### Exact-head CI

Delivery head: `0b1f8288c20a69a50628e401fe3a7fb60681f050`.

- Agent governance run `32131529550`: SUCCESS after PR metadata repair.
- Merge gate run `32131529475`: SUCCESS on the unchanged delivery head after the owner enabled Dependency graph.
- `Merge gate / dependency review`: SUCCESS after Dependency graph enablement.
- Architecture semantic audit run `32132473138`: SUCCESS on the delivery head.
- Merge authority audit run `32131473603`: SUCCESS on the delivery head.
- Rust Linux workspace, Rust Windows client, CodeQL actions/python, supply-chain, repository policy/metadata and aggregate `Merge gate / validate`: SUCCESS in the successful exact-head merge-gate generation.
- Reviews / inline review threads / PR comments before merge: `0 / 0 / 0`.

A redundant Ready-triggered merge-gate generation was started on the same unchanged head after the successful exact-head generation. It did not invalidate the already completed successful exact-head evidence used for merge.

### E2E

`NOT_APPLICABLE`: the delivery changes repository identity/governance metadata only and does not change game runtime, protocol, persistence or user-visible gameplay behavior.

## Merge

PR #4 was marked Ready and squash-merged with expected head `0b1f8288c20a69a50628e401fe3a7fb60681f050`.

Resulting canonical target main merge commit:

`d85a5a075aaf72ec88cf2f4167f1aab2ab2ba3a9`

## Remaining administration debt

The newly created target repository still inherited GitHub creation defaults rather than every live setting encoded in `.github/repository-policy.json`. This includes merge-policy and automatic source-branch deletion behavior. No source repository secret or `REPO_ADMIN_TOKEN` was copied or reused.

Repository-settings reconciliation is a separate administration-capable follow-up. It must apply and verify the target's canonical repository policy without weakening controls or consuming/reusing unapproved credentials.

## Source branch closeout

- Delivery PR #4: merged.
- Delivery branch `docs/otv2-20260818-target-coordinate-reconciliation`: still present because the target's live `delete_branch_on_merge` setting is currently `false` and the connected GitHub capability exposes no delete-ref operation.
- Closeout branch: `docs/otv2-20260818-target-coordinate-reconciliation-closeout`.
- Branch retention is non-semantic cleanup debt and does not leave unmerged repository authority; canonical authority is on `main`.
- Do not force-move or repurpose either historical migration branch. Remove them when target repository administration is reconciled or a delete-ref-capable path is available.

## Ownership release

All task-owned live paths are released after this archive movement merges. No migration task remains authorized to mutate `blakinio/Oteryn-v2`.

## Next action

Run a separate target repository administration task for `Oteryn/Oteryn-Game` to reconcile live GitHub repository settings with `.github/repository-policy.json`, verify main protection/merge policy/Actions/security settings and enable automatic merged-branch deletion.