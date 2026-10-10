# OTV2-20261006-arch-client-installer-0

```yaml
task_id: OTV2-20261006-arch-client-installer-0
title: "CLIENT-INSTALLER-0: Windows client installer, package manifest and updater architecture"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-client-installer-0-20261006
issue: 1622
pr: 1894
head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
final_head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
owner: architect worker (control plane session_0114oBVR3osF1auvFMu6ksMH, decisions D853, D854)
created_at: 2026-10-06
updated_at: 2026-10-06
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/CLIENT-INSTALLER-0_WINDOWS_CLIENT_INSTALLER_CONTRACT_CANDIDATE.md
  - docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md (installer deferral references in §15, §17.4, §24 and the deferred list)
  - docs/agents/tasks/archive/OTV2-20261006-arch-client-installer-0.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This is an accepted-candidate design for a real per-user Windows installer (Inno Setup) that installs the client exe, a stable launcher, the release `client.env` and a versioned `packages.json`. It covers SHA-256 and provenance rules, a single atomic `current.txt` activation pointer, a sequence-based updater with authorized downgrade, a signing seam (unsigned until external alpha, D854), a `rust_windows` gate extension under an owner-authorized audit rotation (D854), and slices CLIENT-INSTALLER-1..5. ALPHA-CLIENT-01 now points to it everywhere it previously deferred these choices.

## Excluded scope

There is no change to code, workflows, protocol, identity or trust roots. The signing provider and the patch/CDN format stay deferred. The channel-pointer endpoint is Platform-owned under a future cross-repository contract (D607).

## Validation

git diff --check: pass
python tools/agents/validate_governance.py: pass
python -m unittest discover -s tools/agents/tests: OK

## Review and closeout

Codex review at cad2bbd0 raised three P1s, all fixed in the final authoring commit:
- one atomic activation pointer, with no reliance on post-install rollback (4200003735);
- the ALPHA-CLIENT-01 §24 and deferred-list references amended (4200003741);
- rollback eligibility now goes by sequence (4200003717).

Review is decided by the control plane on the frozen head. Merge result: squash merge of #1894, with CI and the Merge Queue still pending at authoring.
