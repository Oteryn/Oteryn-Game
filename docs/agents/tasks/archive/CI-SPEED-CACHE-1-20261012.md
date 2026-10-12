# CI-SPEED-CACHE-1-20261012

```yaml
task_id: CI-SPEED-CACHE-1
title: "CI-SPEED-CACHE-1: restore the protected-main Cargo cache in PR Rust jobs"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ci-speed-cache-20261012
pr: 2021
head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
final_head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
owner: claude-code-session_01CwP6d84eCPvpgoEuyci8Tx (implementation worker)
created_at: 2026-10-12
updated_at: 2026-10-12
owned_paths:
  - .github/workflows/merge-gate.yml
  - .github/workflows/g4-canonical-worldproject-package-seed.yml
  - docs/agents/tasks/archive/CI-SPEED-CACHE-1-20261012.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The PR changed-crate Rust job (`merge-gate.yml` `rust_fast`) already restored the protected-main Cargo cache. It now also normalizes source mtimes, so Cargo rebuilds only the crates whose files differ from the cached commit.

The G4 WorldProject `qualify` job gets the same restore-only cache and normalization. `rust.yml` on protected main stays the only cache writer, and every step still builds and tests the exact head. Normalization fails open.

`merge-group-gate.yml` is unchanged. sccache is not adopted: per `docs/agents/evidence/OTV2-20261003-rust-cache-pilot.md`, a warm target dir rebuilds in 0.3 s against 30 s for warm sccache, and no shared backend exists.

## Evidence

| Job | Before this change | Why |
|---|---|---|
| PR `rust_fast` | about 20 min: restore 2.4 min, clippy 3.5 min, unit tests 11.3 min | built without normalization |
| G4 `qualify` | 17-18 min | no cache |
| MQ (run 38283735377) | Durability PostgreSQL about 16 min is the critical path | normalization kept fresh mtimes on only 4 files |

## Validation

python tools/repository/validate_repository_policy.py: pass
python tools/repository/test_*.py (13 files, PG-simulation tests with local pwsh 7.4.6): pass
actionlint 1.7.12 -shellcheck= -pyflakes= on both workflows: pass
git diff --check: pass
python tools/agents/validate_governance.py: pass
python -m unittest discover -s tools/agents/tests: OK

## Review and closeout

Governance: `merge-authority-audit.yml` pins the approved merge-gate blob `55bc76801a86ec2104b280b09ce39d7ea4be6d45`. This change's blob is `bfcc6438a71d806f6cb757ab14dfaf5a67154fde` and needs an owner-authorized pin rotation before the audit passes.

Follow-ups: node boot and Server Seam caching, if they become the MQ critical path.

Merge result: squash merge of #2021, pending CI, review and Merge Queue at authoring.
