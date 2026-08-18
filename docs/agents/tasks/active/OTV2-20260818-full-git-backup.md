# OTV2-20260818-full-git-backup

```yaml
task_id: OTV2-20260818-full-git-backup
title: Produce verified full Git backup of Oteryn-v2
mode: MIGRATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: ops/otv2-20260818-full-git-backup
pr: 338
base_sha: bf8a65ca0d6b0fbc1b6c521b16e613824b048f0d
head_sha: 63d9d04f9ac7e7540f42a1a37137264624daed71
final_head_sha: null
final_head_frozen_at: null
owner: chat-github-20260818-full-git-backup
created_at: 2026-08-18T09:36:00Z
updated_at: 2026-08-18T10:32:00Z
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260818-full-git-backup.md
  - .github/workflows/oteryn-v2-full-git-backup.yml
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: OTERYN-GAME-BACKUP-20260818
external_repositories:
  - Google Drive
```

## Outcome

Create a verified, restorable Git backup of the complete live `blakinio/Oteryn-v2` Git repository state using a GitHub-hosted runner. The backup contains a `git clone --mirror` archive, portable `git bundle --all`, GitHub pull-request refs as recovery evidence, complete refs metadata and SHA-256 checksums, and is copied outside GitHub to the owner's connected Google Drive.

## Acceptance criteria

- [x] Runner created a fresh `git clone --mirror` of `blakinio/Oteryn-v2`.
- [x] `git fsck --full` passed on the mirror.
- [x] `git bundle create --all` and `git bundle verify` passed.
- [x] Backup recorded source repository, repository ID, observed main SHA, refs, object statistics and SHA-256 checksums.
- [x] Artifact contained the mirror tarball, full bundle and verification metadata.
- [x] First artifact was downloaded through the GitHub connector and copied to the owner's connected Google Drive.
- [x] No Git LFS objects were observed in the first verified backup.
- [x] No repository runtime/product/production data or secrets were mutated.
- [ ] Refresh one final backup after source `main` advanced to `16afdf31a15bd49d454cdbcdd98fa7ec72213ef9` so the external copy matches the latest pre-migration source state.
- [ ] Upload the refreshed artifact to Google Drive and verify its digest.

## Excluded scope

No physical `Oteryn-Game` target creation/cutover in this temporary backup task, no package/deployment changes, no secrets, no runtime changes and no merge of the temporary backup workflow to `main`.

## Validation

### First verified backup generation

- workflow run: `32122981093` — SUCCESS
- artifact: `9319247804`, `Oteryn-v2-full-git-backup`
- artifact ZIP SHA-256: `b1dd29002740b64694d0cb098452ef9897939a48827783cfa1cd4296546a2454`
- observed source main: `bf8a65ca0d6b0fbc1b6c521b16e613824b048f0d`
- refs: `340`
- GitHub PR refs: `302`
- `git fsck --full`: PASS
- `git bundle verify`: PASS
- LFS: no objects observed
- Google Drive file ID: `1MjR4cwYrdxdcc2kmcVmLAdElKKtxAsdN`

### Refresh rationale

After the first artifact completed, source `main` advanced to `16afdf31a15bd49d454cdbcdd98fa7ec72213ef9`. A new workflow generation is intentionally triggered by this material checkpoint update so the final external backup captures that later source commit before `Oteryn-Game` creation/import.

### E2E

- scenario: restore-capable Git objects validated by `git fsck` and `git bundle verify`; no game runtime E2E applies
- first generation result: PASS
- refreshed generation: pending

## Context checkpoint

```yaml
last_progress: first verified full Git backup uploaded to Google Drive; source main later advanced and now requires one refreshed final generation before target import
status: validating
branch: ops/otv2-20260818-full-git-backup
head_sha: 63d9d04f9ac7e7540f42a1a37137264624daed71
pr: 338
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: push
ci_check_generation: refreshed-final-backup
ci_checks_for_current_head: 0
ci_run_ids:
  - 32122981093
ci_job_ids:
  - 95667293234
runner_assignment_state: first_generation_completed_success
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: target repository creation remains unavailable through the connected GitHub actions; backup refresh itself is unblocked
next_action: capture the refreshed backup artifact produced by this material source-drift update, upload it to Google Drive, then proceed to the owner-only target repository creation/import gate
```
