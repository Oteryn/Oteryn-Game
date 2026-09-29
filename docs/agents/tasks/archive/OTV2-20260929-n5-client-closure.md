# OTV2-20260929-n5-client-closure

```yaml
task_id: OTV2-20260929-n5-client-closure
title: Native client N5 - oteryn-client closure may reach protocol-oteryn only through the session crate, plus CodeQL action bump
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
pr: null
base_branch: main
branch: claude/n5-client-closure
base_sha: cf025b7a
owner: "Oteryn: native client" (Claude Code)
created_at: 2026-09-29T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-gate.yml
  - .github/workflows/merge-group-gate.yml
  - .github/workflows/rust.yml
  - .github/workflows/codeql.yml
  - tools/repository/**
  - docs/agents/tasks/archive/OTV2-20260929-n5-client-closure.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

ADR-0020 section 7 N5. The production-closure check in `merge-gate.yml`, `merge-group-gate.yml` and `rust.yml` now
parses `cargo tree` and lets `oteryn-client` reach `oteryn-protocol-oteryn` only as a direct child of `oteryn-session`
or `oteryn-session-tcp`. Canary, core, transport, game-session, client-domain, client-simulation, synthetic-assets,
test-support and the synthetic harness stay forbidden; `oteryn-dev-client` is added as a dev-only negative for both
roots. `oteryn-game-server` keeps its `protocol-oteryn` allowance. The three copies are identical. Local reproduction
against real trees and synthetic negatives (foreign parent, direct edge, dev-client, synthetic-assets, canary) passes.

Also carries PR #1249 exactly (`github/codeql-action` init/analyze `cdf488f5...` to `2892aa5e19bbd11bc0cff5427e3b750a04d9e3c2`
in `codeql.yml`, `merge-gate.yml`, `merge-group-gate.yml`); it supersedes #1249.

Candidate-side pins updated in `tools/repository`: `EXPECTED_MERGE_GROUP_GATE_BLOB`, the codeql fragments,
`EXPECTED_POST_MERGE_RUST_SHA256` (rust.yml is hash-pinned) and the `APPROVED` blob in
`test_validate_merge_group_pg_sim.py`.

## Findings and notes

- `workspace-boundaries.toml` is unchanged: the `oteryn-client -> oteryn-session` edge is added by N1 with the client
  code, not by this CI change (the client does not depend on the session crate yet).
- "#1083" named in ADR-0020 is not this closure change: PR #1083 is the closed tibia.com capture tool. The "game-gate
  fan-in" batch partner is unidentified; reported, not widened.
- `merge-authority-audit.yml` is not edited; it needs a protected-base stage A pinning the new `merge-gate.yml` and
  `merge-group-gate.yml` blobs.

## Validation

- `validate_repository_policy.py` PASS (23 files, 50 workflows); `validate_governance.py` PASS; `git diff --check` clean.
- `test_validate_merge_group_pg_sim.py` and `test_validate_pr_gate_pg_sim.py` need `pwsh`, absent in this container; CI runs them.
- Owner authorization: Q4a (N5), Q2a (#1249 bump), Q7c (one combined rotation), #162 comments 5897283986, 5898411039.
