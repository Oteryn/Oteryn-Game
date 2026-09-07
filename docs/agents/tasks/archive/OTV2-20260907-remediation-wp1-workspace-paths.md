# OTV2-20260907-remediation-wp1-workspace-paths

```yaml
task_id: OTV2-20260907-remediation-wp1-workspace-paths
title: Bind workspace packages to declared manifest paths
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 364
pr: 372
admission_main_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
base_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
head_sha: 40c6443db2a69ebd48f9656e4a188717ef353cbe
final_head_sha: 40c6443db2a69ebd48f9656e4a188717ef353cbe
owner: WP1_F11_WORKSPACE_PATHS
created_at: 2026-09-07
updated_at: 2026-09-07
owned_paths: []
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

## Outcome

Repair audit F11 by binding every declared workspace package to Cargo metadata's actual manifest directory rather than checking only member/path cardinality. Preserve all existing edge, cycle, role and production-closure controls.

## Source of truth

PROVEN on admission main: policy `members` and `paths` are parsed into independent BTreeSets and `validate_policy_shape` only compares their counts; `validate_workspace` compares actual package names/edges but never manifest paths. Current policy arrays are order-aligned, so a mapping can be validated without changing `workspace-boundaries.toml`. Exact allocation is #162 comment `5567281231`.

## Acceptance criteria

- [x] Preserve declaration order/pairing for `members` and `paths` while still rejecting duplicates.
- [x] Resolve each workspace package's Cargo `manifest_path` parent relative to repository root and compare exact package→declared-path mapping.
- [x] Same-cardinality wrong path fails.
- [x] Same path set paired to the wrong package fails.
- [x] Current real workspace metadata is covered by a focused positive control.
- [x] Existing member, edge, cycle, role, production-to-fixture and forbidden-fragment checks remain unchanged.
- [x] Focused unit tests, Rust 1.94 fmt/strict Clippy/workspace tests and exact-head canonical CI pass.

## Validation

- PROVEN by source inventory: the 21 checked-in policy package/path pairs match the package names in their declared Cargo manifests; neither policy array contains duplicates.
- Historical implementation-host limitation: Rust execution was initially unavailable because that workspace had no `cargo`, `rustc` or `rustfmt`; no PASS was claimed from that environment.
- Later local qualification on exact head `40c6443db2a69ebd48f9656e4a188717ef353cbe` passed Rust 1.94 formatting, strict Clippy, all 9 focused tests and the real workspace checker. Source evidence: PR #372 comment `5568395752` and lane evidence `5568359601`.
- Exact delivery head `40c6443db2a69ebd48f9656e4a188717ef353cbe`: Merge gate `34105636826`, Architecture semantic audit `34105394810` and Agent governance `34105637224` passed.
- Full Merge Queue run `34105424118` passed; PR #372 squash-merged as `6a83ab15d51be5b05adcd31baf48380172b97e7d`. The task blob `b205dcf6c72f45a69df10a31411426fee9fb6ed1` and checker blob `3e4284235bee11588b6f00afb582789f13bd6373` were read back from protected `main` (subsequently `15164c38a2775e45eaff4001fddddbabf4b63ab6`). Durable evidence: PR #372 comment `5568575851`.

## Self-review and closeout

- Exact delivery head: `40c6443db2a69ebd48f9656e4a188717ef353cbe`.
- Full changed-file and effective-diff review: PASS; zero open material findings, no unresolved review threads and no scope outside the two allocated files.
- Independent review: NOT_REQUIRED under the META-owned policy for this bounded checker repair; exact-head repository gates and Merge Queue passed.
- Ownership: released after protected-main blob readback; the implementation branch has no continuing provenance role. This closes F11 only; broader WP1, G0 and G1 remain open.
- Branch disposition: merged task branch deleted; live matching-ref readback returned no branch.

## Excluded scope

No `workspace-boundaries.toml`, Cargo manifests/lock, product/runtime/schema, workflow/protection, ruleset/MQ, production/live data or external mutation unless exact tests prove current policy bytes cannot express their existing intended pairings.

## Context checkpoint

```yaml
last_progress: PR 372 integrated through successful Merge Queue and read back from protected main; F11 task archived and ownership released
status: completed
branch: null
head_sha: 40c6443db2a69ebd48f9656e4a188717ef353cbe
final_head_sha: 40c6443db2a69ebd48f9656e4a188717ef353cbe
pr: 372
blocker: null
next_action: null
```
