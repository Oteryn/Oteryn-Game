# ADR-0017: Repository-local terminal source-branch lifecycle

- Status: Accepted
- Date: 2026-08-23
- Decision owner: repository owner
- Lifecycle issue: #65
- Organization implementation: `Oteryn/Oteryn-Platform@e145f7c03bd0b15f0b0fecc0f6fae7884fe3e0db`

## Context

GitHub's automatic branch deletion closes only the ordinary merged-PR path. Game also creates implementation, recovery, diagnostic, migration, and agent branches that may intentionally terminate through a closed-unmerged or superseded PR. Retaining those refs indefinitely makes repository state ambiguous, while deleting by age or prefix alone is unsafe.

Oteryn Platform exposes the existing exact-head Terminal Branch Lifecycle through separate reusable workflows for read-only inventory and write-capable close/apply operations.

## Decision

Game adopts those workflows through a repository-local caller pinned to the exact merged Platform SHA above. All live inventory and deletion authority comes from this repository's `GITHUB_TOKEN`; no organization-wide destructive token is introduced.

Read-only inventory uses the physically separate Platform read reusable workflow. Write-capable close/apply operations use the separate write reusable workflow. An intentionally closed same-repository PR must contain exactly one `Branch-Disposition: delete` or `Branch-Disposition: retain` and one non-empty `Branch-Disposition-Reason`. A delete disposition permits cleanup only after trusted-main automation revalidates the exact PR/branch/head identity, no open PR or active claim, protection/retention state, and recovery-sensitive branch-name policy. Ambiguity fails closed.

Merged PR branches remain handled by `delete_branch_on_merge=true`. Scheduled/manual inventory is read-only. Historical orphan cleanup requires a separately reviewed manifest and approval; adoption itself does not remove existing ambiguous refs.

## Shared policy compatibility

`docs/agents/BRANCH_LIFECYCLE_POLICY.json` preserves the shared Platform classifier schema, including the schema compatibility marker `issue: 658`. Game lifecycle authority is Issue #65 and this ADR, not that compatibility marker.

## Consequences

Terminal branch state becomes a deterministic repository lifecycle concern. Protected/default, open-PR, active, retained, release, rollback, recovery, backup-sensitive, moved, or ambiguous refs remain untouched. Upgrading the implementation requires a reviewed PR that changes both reusable workflow references and all `platform_ref` values together to one merged Platform commit SHA.
