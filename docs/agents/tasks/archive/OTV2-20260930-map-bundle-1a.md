# OTV2-20260930-map-bundle-1a

```yaml
task_id: OTV2-20260930-map-bundle-1a
title: "MAP-BUNDLE-1a World Bundle format and compiler skeleton"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/map-bundle-1a
pr: "exact PR in the #162 READY_FOR_REVIEW handback"
base_sha: a795d5fe
head_sha: "exact frozen head in the #162 READY_FOR_REVIEW handback"
final_head_sha: "exact frozen head in the #162 READY_FOR_REVIEW handback"
final_head_frozen_at: null
owner: claude-code-session-01EfiFA9LMuUuzoNkizLfR2R (control-plane task session)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - tools/world-bundle-compiler/**
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml
  - docs/agents/tasks/archive/OTV2-20260930-map-bundle-1a.md
public_contracts:
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
depends_on: []
blocks:
  - MAP-BUNDLE-1b
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The first MAP-BUNDLE-1 subset (ADR-0021 §5), allocated on #162 (5909595542) with a partial
waiver of the "#1160 and #1170 merged" dependency for the parts that need no real keys.

- **Format document** `docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md` (CANDIDATE): byte layout,
  canonical JSON manifest, 50-byte sector table, B3 sector grammar reused, per-frame SHA-256,
  bundle digest, derived `placement_key`, `build_class`, versioning, reader order and limits.
  OPEN-1 to OPEN-4 wait for the architect's answers on #162 and MAP-BUNDLE-1b.
- **Crate** `tools/world-bundle-compiler` (`oteryn-world-bundle-compiler`, category `tool`, no
  internal edges): a Rust reader for `OTERYN_WORLD_REGION_B3/v1`, the bundle writer and fail-closed
  reader, and a compiler skeleton (frame mapping `floor = -z`, a `KeyResolver` trait, provisional
  skip in non-production builds, bounds and limit checks). Tests use synthetic fixtures only; one
  fixture region was written by the #1170 Python codec (head `fde85fa8`) to prove the reader
  matches it byte for byte.
- **Dependency.** `zstd =0.13.3` (default features off; bundled libzstd 1.5.7) is added to the
  workspace: the B3 source and the bundle both use zstd frames (ADR-0021 §4.2). Lockfile adds
  `zstd`, `zstd-safe`, `zstd-sys` only; `cc` and `pkg-config` were already locked.
- **Registry.** Ten rows `MAP01-BUNDLE-*`, `MAP01-TILE-*` and `MAP01-ITEM-TEXT-BYTES`, appended
  without changing existing rows.

## Architecture and source of truth

- `PROVEN`: ADR-0021 §4.2-§4.6, §4.8 (merged); ADR-0005 §3; the B3 codec on #1170
  (`world_region_codec.py`, head `fde85fa8`), read as reference for the grammar.
- `OPEN` (not decided here): palette key families incl. WorldObject; appearance-only terrain
  keys; orphan teleports without Transition; per-tile draft marker.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline tool and a candidate format; no runtime, persistence, protocol or
production path. The ADR-0021 §4.8 security review of the bundle reader applies to MAP-LOAD-1.

## Acceptance criteria

- [x] Format document with OPEN items marked.
- [x] Compiler skeleton writing and reading the bundle with manifest, checksums, digest and
  limits, tested with synthetic fixtures.
- [x] Registry rows added without widening existing rows.
- [x] Read-only self-review findings fixed before freeze.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Real-map key resolution, the World, FloorChange, Transition, House and area families, draft
  marking, real-map tests (MAP-BUNDLE-1b); the runtime loader (MAP-LOAD-1); overlay and
  persistence; content and world data; #1160, #1170 and #1319 paths; `.github/workflows/`;
  `tools/repository/`.

## Validation

- `cargo fmt --all --check`: PASS.
- `cargo clippy --locked -p oteryn-world-bundle-compiler --all-targets --quiet -- -D warnings`: PASS.
- `cargo test --locked -p oteryn-world-bundle-compiler --quiet`: 11 passed (after review round 1).
- `cargo run --locked -p oteryn-architecture-check -- workspace .`: PASS.
- `python3 tools/agents/validate_governance.py` and the `tools/agents/tests` suite: PASS.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #162 READY_FOR_REVIEW handback. Merge commit/result: its squash merge.
- Self-review (`oteryn-hard-worker`, read-only) on the first authoring head `bb5b9309`: 6
  material and 2 minor findings, all fixed in the final authoring commit:
  - a per-bundle decoded tile and entry budget (`MAP01-BUNDLE-TILES`, `MAP01-BUNDLE-ENTRIES`), because
    raw-byte limits alone let a small file decode to far more memory;
  - no empty sectors, and every row floor checked against the World;
  - the writer decodes what it wrote and requires identical tiles;
  - skipped provisional containers still resolve their contents;
  - `compiler_version` derived from the crate and zstd versions;
  - canonical varints and exactly one checksummed, size-declaring zstd frame per sector;
  - sorted manifest key lists;
  - bounded B3 decompression capacity and a checked `placement_key`.
- Hand-written code is about 1,000 lines of Rust plus tests, above the ~500-line guide: the
  format writer, the fail-closed reader and the B3 reader are one reviewable unit with the format
  document (ADR-0021 §5 reviews the format with the compiler).
- Review round 1 on `5e34fac9` (comments 5910607238 and 5910692846), all answered in one repair
  push (a new frozen head; the head is named in the #162 handback):
  - `build_class` and `family` deserialize from a JSON string only (test added);
  - the manifest field order is a writer-only rule (format §3);
  - "exactly one byte encoding" now applies to the raw payload only; `compiler_version` fixes the
    frame bytes (§5);
  - boundary tests: reduced `ReadCaps` maxima for file, sector-count, sector-raw and total-raw,
    exact and one over; exact bundle tile and entry budgets. Rows that need a real maximum
    (a 1 GiB bundle, 1,048,576 sectors, 16 MiB payload, 1 GiB total accepted) are covered by the
    reduced-cap tests; rewording those `boundary_tests` rows is left to the registry owner
    (`RESOURCE_LIMITS_REGISTRY.json` is outside this repair's owned paths);
  - a deterministic reseal-and-flip reader test; "stacking order" is now "payload order";
  - OPEN-1 to OPEN-3 updated to the architect's answers (#162 5910173902); OPEN-4 stays open.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

## Context checkpoint

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/map-bundle-1a
owner_action_required: null
blocker: null
next_action: null
```
