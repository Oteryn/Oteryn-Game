# OTV2-20261006-arch-client-installer-0

```yaml
task_id: OTV2-20261006-arch-client-installer-0
title: "CLIENT-INSTALLER-0: Windows client installer, package manifest and updater architecture"
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-client-installer-0-20261006
issue: 1622
pr: null
head_sha: null
final_head_sha: null
owner: architect worker (control plane session_0114oBVR3osF1auvFMu6ksMH, decision D853)
created_at: 2026-10-06
updated_at: 2026-10-06
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/CLIENT-INSTALLER-0_WINDOWS_CLIENT_INSTALLER_CONTRACT_CANDIDATE.md
  - docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md (two reference lines)
  - docs/agents/tasks/archive/OTV2-20261006-arch-client-installer-0.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Accepted-candidate design for a real per-user Windows installer (Inno Setup) installing the client exe, release `client.env` and a versioned `packages.json`, with SHA-256/provenance rules, updater model, provider-neutral signing seam, CI gate extension and CLIENT-INSTALLER-1..5 slices. ALPHA-CLIENT-01 now points to it instead of deferring installer technology.

## Excluded scope

No code, workflow, protocol, identity or trust-root change. Signing provider, CI audit rotation and the Platform channel-pointer endpoint are owner questions in §9 of the doc.

## Validation

git diff --check: pass
python tools/agents/validate_governance.py: pending
python -m unittest discover -s tools/agents/tests: pending
