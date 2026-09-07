# OTV2-20260907-remediation-wp1-workspace-paths

```yaml
task_id: OTV2-20260907-remediation-wp1-workspace-paths
title: Bind workspace packages to declared manifest paths
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: fix/remediation-wp1-workspace-paths-364
issue: 364
pr: null
admission_main_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
base_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
head_sha: null
final_head_sha: null
owner: WP1_F11_WORKSPACE_PATHS
created_at: 2026-09-07
updated_at: 2026-09-07
owned_paths:
  - tools/architecture-check/src/lib.rs
  - docs/agents/tasks/active/OTV2-20260907-remediation-wp1-workspace-paths.md
public_contracts: []
depends_on: []
blocks: [WP1_verification_credibility]
external_repositories: []
```

## Outcome

Repair audit F11 by binding every declared workspace package to Cargo metadata's actual manifest directory rather than checking only member/path cardinality. Preserve all existing edge, cycle, role and production-closure controls.

## Source of truth

PROVEN on admission main: policy `members` and `paths` are parsed into independent BTreeSets and `validate_policy_shape` only compares their counts; `validate_workspace` compares actual package names/edges but never manifest paths. Current policy arrays are order-aligned, so a mapping can be validated without changing `workspace-boundaries.toml`. Exact allocation is #162 comment `5567281231`.

## Acceptance criteria

- [ ] Preserve declaration order/pairing for `members` and `paths` while still rejecting duplicates.
- [ ] Resolve each workspace package's Cargo `manifest_path` parent relative to repository root and compare exact package→declared-path mapping.
- [ ] Same-cardinality wrong path fails.
- [ ] Same path set paired to the wrong package fails.
- [ ] Current real workspace metadata passes.
- [ ] Existing member, edge, cycle, role, production-to-fixture and forbidden-fragment checks remain unchanged.
- [ ] Focused unit tests, Rust 1.94 fmt/strict Clippy/workspace tests and exact-head canonical CI pass.

## Excluded scope

No `workspace-boundaries.toml`, Cargo manifests/lock, product/runtime/schema, workflow/protection, ruleset/MQ, production/live data or external mutation unless exact tests prove current policy bytes cannot express their existing intended pairings.

## Context checkpoint

```yaml
last_progress: two-path F11 allocation created from protected main
status: implementing
branch: fix/remediation-wp1-workspace-paths-364
pr: null
blocker: null
next_action: implement exact package-to-manifest-directory validation with synthetic wrong-path/pairing regressions
```
