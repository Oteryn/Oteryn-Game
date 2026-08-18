# OTV2-20260818-full-git-backup

```yaml
task_id: OTV2-20260818-full-git-backup
title: Produce verified full Git backup of Oteryn-v2
mode: MIGRATE
status: implementing
repository: blakinio/Oteryn-v2
base_branch: main
branch: ops/otv2-20260818-full-git-backup
pr: null
base_sha: bf8a65ca0d6b0fbc1b6c521b16e613824b048f0d
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chat-github-20260818-full-git-backup
created_at: 2026-08-18T09:36:00Z
updated_at: 2026-08-18T09:36:00Z
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260818-full-git-backup.md
  - .github/workflows/oteryn-v2-full-git-backup.yml
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: OTERYN-GAME-BACKUP-20260818
external_repositories: []
```

## Outcome

Create a verified, restorable Git backup of the complete live `blakinio/Oteryn-v2` Git repository state using a GitHub-hosted runner, because the assistant local runtime cannot resolve `github.com`. The backup must contain a `git clone --mirror` archive, a `git bundle --all`, complete refs metadata and SHA-256 checksums, and must be downloadable as a GitHub Actions artifact for subsequent storage outside GitHub.

## Acceptance criteria

- [ ] Runner creates a fresh `git clone --mirror` of `blakinio/Oteryn-v2`.
- [ ] `git fsck --full` passes on the mirror.
- [ ] `git bundle create --all` and `git bundle verify` pass.
- [ ] Backup records source repository, repository ID, observed main SHA, all refs, object statistics and SHA-256 checksums.
- [ ] Artifact contains the mirror tarball, full bundle and verification metadata.
- [ ] Artifact is downloaded through the GitHub connector and copied to the owner's connected Google Drive.
- [ ] No repository runtime/product/production data or secrets are mutated.

## Excluded scope

No physical `Oteryn-Game` cutover, no package/deployment changes, no secrets, no runtime changes and no merge of the temporary backup workflow to `main`.

## Validation

### Focused

- runner mirror `git fsck --full`: pending
- runner `git bundle verify`: pending
- SHA-256 manifest: pending

### Component/integration

- GitHub Actions artifact download: pending
- Google Drive upload: pending

### E2E

- scenario: restore-capable Git objects validated by `git fsck` and `git bundle verify`; no game runtime E2E is applicable
- result: pending

## Context checkpoint

```yaml
last_progress: dedicated backup branch created from main bf8a65ca0d6b0fbc1b6c521b16e613824b048f0d and task ownership claimed
status: implementing
branch: ops/otv2-20260818-full-git-backup
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: not_started
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: add the bounded temporary backup workflow and open a Draft PR so the PR event creates the verified backup artifact
```
