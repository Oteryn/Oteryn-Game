# OTV2-20261011-game-ci-shared-cache

```yaml
task_id: OTV2-20261011-game-ci-shared-cache
title: Publish complete protected-main Cargo cache for Game CI consumers
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: 94ec74346eec7976a4b8dd109b3dc35fd9877a8f
branch: codex/game-ci-shared-cache-20261011
pr: 1988
head_sha: "final frozen head in PR #1988 metadata"
final_head_sha: "final frozen head in PR #1988 metadata"
owner: Codex task under direct owner instruction, sole task-branch writer
created_at: 2026-10-11
updated_at: 2026-10-11
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/rust.yml
  - tools/repository/validate_repository_policy_core.py
  - tools/repository/test_classify_post_merge_lanes.py
  - docs/agents/tasks/archive/OTV2-20261011-game-ci-shared-cache.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

## Outcome

Implementation complete for #1988; hosted qualification, independent review and
integration remain pending. This record accompanies the final authoring commit
as required by the archive convention and reaches main only if the PR merges.

The protected-main Linux producer now trims and publishes its Cargo cache after
successful build, strict Clippy, workspace tests, synthetic harness and server
smoke. Its timeout rises from 45 to 60 minutes to accommodate cold qualification
before saving. All six existing canonical consumers retain the same namespace:
post-merge Linux/PostgreSQL, PR fast/full Linux, Merge Queue Linux/PostgreSQL.
Only the existing post-merge workflow hash is rotated.

## Evidence and limits

Run 38154056379 succeeded in 49m11s. Linux took 47m55s; its cold build took
15m33s and Clippy 8m16s. The Rust 1.95 cache miss followed toolchain migration;
one miss does not establish a persistent restore defect. The prior producer
published before Clippy/tests, excluding their later artifacts.

No hosted speedup is claimed. After reviewed integration, verify a successful
protected-main producer save and compare a warm consumer on a comparable input,
including cache transfer overhead. Tests still execute normally; cached outputs
are not test-result evidence. Integration-test source/test duplication, Windows
caching and runtime changes remain separate work. No required check, gate,
protection, permission, action pin or test command is weakened.

## Validation

- `python tools/repository/validate_repository_policy.py`: pass (23 files, 63 workflows)
- `python tools/repository/test_classify_post_merge_lanes.py`: pass (routing, six consumer keys, five rejected workflow mutations)
- `python tools/repository/test_normalize_cargo_mtimes.py`: OK (3 tests)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK (60 tests)
- `/tmp/game-ci-actionlint/actionlint -shellcheck= -pyflakes= .github/workflows/rust.yml`: pass (official 1.7.12 release, checksum verified)
- `git diff --check`: pass

Hosted exact-head checks and protected-main cache publication remain pending.
Local Rust execution is not claimed: only workflow ordering, timeout and the
matching policy/regressions change; hosted CI owns runtime qualification.

## Self-review and independent review

Full owned delta reviewed. Cache writes retain protected-main ownership and
implicit successful-step gating; source marker and incremental trimming remain.
Canonical PR/MQ workflows remain byte-identical to the admitted base. The final
API publication uses an atomic expected-head commit on the exclusive task branch.
Independent deep review is required for workflow/control-plane changes under
bound META policy and remains pending; no review result or integration authority
is inferred from local checks.

## Lifecycle

Final remote SHA is frozen externally after the task-record authoring write;
the commit cannot contain its own SHA. Keep #1988 draft until required exact-head
validation/review and normal integration authorization are established.
No enqueue, merge, protected environment or production action is performed.
Merge commit/result: pending (if integrated, resolve squash merge of #1988).
