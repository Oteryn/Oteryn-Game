# OTV2-20261003-ci-mq-docs-fastpath

```yaml
task_id: OTV2-20261003-ci-mq-docs-fastpath
title: Skip product lanes for documentation-only candidates
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ci-mq-docs-fastpath-20261003
pr: null
issue: 1622
base_sha: 852dfca07f2d39eaf28d649d547d37876232cd8c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Claude worker, sole writer allocated by the control plane (D307, owner answer 2a)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - tools/repository/classify_pr_test_lanes.py
  - tools/repository/test_classify_pr_test_lanes.py
  - docs/agents/tasks/archive/OTV2-20261003-ci-mq-docs-fastpath.md
public_contracts: []
depends_on: [OTV2-20261003-ci-stack-applicability]
blocks: []
external_repositories: []
```

## Outcome and evidence

PROVEN: documentation-only Merge Queue entries ran the full product suite. #1636
(one decision Markdown file and one task record) classified as
`windows-consumer-affected` and took about 20 minutes (Rust Linux, Windows and
PostgreSQL). The reference scan counted a bounded directory literal as a consumer.
Rust doc comments cite `docs/architecture/` and `docs/architecture/reviews/`
(`apps/game-server/src/content/encounter_map_item.rs`,
`crates/protocol-oteryn/src/bestiary.rs`). So every new decision document made
`oteryn-game-server` and `oteryn-protocol-oteryn` consumers.

Repair, in the existing classifier only:

- `documentation_path()`: Markdown under `docs/` and the root README, CHANGELOG
  and CONTRIBUTING (`neutral()`), excluding `AGENTS*`, `docs/migration/` and
  `docs/agents/evidence/`. Paths under `.github/**` are never documentation.
- For Cargo-package consumers, a directory literal counts for a documentation
  target only when it ends a quoted string. Doc-comment prose is not a consumer.
  Exact-file references, canonical workflow scans and every fail-closed path are
  unchanged.

The PR gate, the merge-group gate and post-merge `rust.yml` all load this
classifier from the protected base. No workflow, pinned blob or `game-gate`
aggregation changes. `game-gate` stays required. It accepts a skipped product
job only when the classifier explicitly selected `rust=false`/`windows=false`.

Replayed on real queue history: #1636, #1631, #1633 and #1623 (docs only) now
give `rust=false windows=false`, and #1625 (code) still selects the server lanes.

Not changed: CodeQL python (about 2 minutes) still runs on every queue entry.
Skipping it needs a preapproved merge-group-gate blob. A non-canonical
`.github/workflows/*.yml` change keeps its existing pinned-contract routing.

## Validation and review

`test_classify_pr_test_lanes.py` adds `test_documentation_fast_path` on a real Git
fixture:

- documentation only skips the product lanes;
- documentation plus code runs full;
- canonical workflow, composite action or classifier changes run full;
- an unknown root runs full;
- quoted-directory and exact-file documentation consumers keep their lanes.

The following pass:

- every `tools/repository/test_*.py` except the two PowerShell canaries, which
  need `pwsh` and are not available locally;
- `validate_repository_policy.py`;
- `validate_pr_routing_contract.py --protected-main`;
- `validate_governance.py`;
- `git diff --check`.

Independent review is required on the frozen head through the control plane.

## Final authoring and closeout

The freeze waits until #1634 has merged and `main` is merged into this branch
(merge commit). The exact frozen SHA is recorded in the FREEZE_SHA message.
Merge result: squash merge of this PR; it merges individually, not in a stack.
