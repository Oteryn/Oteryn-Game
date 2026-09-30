# OTV2-20260930-map-bundle-1b

```yaml
task_id: OTV2-20260930-map-bundle-1b
title: "MAP-BUNDLE-1b World Bundle compiler: families, teleport split, parity, key resolution"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/map-bundle-1b-2   # 1b-1 was claude/map-bundle-1b
issue: 162
pr: "1382 (1b-1, merged); 1b-2 in the #162 FREEZE_SHA entry"
base_sha: a6a054e6
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
merge_commit: "squash merge of the 1b-2 PR (git log --grep)"
owner: claude-code task worker, allocated by the #162 control plane (5916023254)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - tools/world-bundle-compiler/**
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json (MAP-BUNDLE rows only)
  - docs/agents/tasks/active/OTV2-20260930-map-bundle-1b.md (moved)
  - docs/agents/tasks/archive/OTV2-20260930-map-bundle-1b.md
public_contracts:
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md
depends_on:
  - MAP-BUNDLE-1a (#1338, merged)
  - "#1160 and #1170 (open; read-only evidence for the parity and compile runs)"
blocks:
  - MAP-LOAD-1
cross_repository_coordination_id: null
external_repositories: []
```

## Split

- **1b-1 (#1382, merged).** Families (`project.rs`), the Q3 teleport split with
  `dropped_teleports`, the `parity` command, OPEN-3 counts, OPEN-4 closure, real-map limits.
- **1b-2 (this PR).** Key resolution under ruling 5915258560 (b) (`resolve.rs`): Item keys with
  the route from the catalogue `item_pointer`, one-to-one, checked against `routed_to` when
  present; Terrain keys only without an Item record; revision-scoped compact ids. The
  `compile` command with a tile-by-tile source-to-bundle equivalence proof (`equivalence`).
  The review carry of #1382 (5915956893): the MEDIUM (a record decides before the (0,0,0)
  guard, now a mismatch, tested), the LOW format-doc wording of the six orphans, and the LOW
  `dropped_teleports` lookup (binary search by `(y, x)` in the sector).

## Review round 1 (Codex review 5371325268 of `0d38d046`)

Repair candidate after returning to AUTHORING; all four findings fixed:

- P1 `compile.rs`: `equivalence` now checks every manifest palette entry against the resolver's
  `(family, id)`, not only the key.
- P1 `compile.rs`: the provisional skips come from the resolver (the placements index flags),
  and the manifest's `skipped_provisional_keys` must equal the set met, never trusted.
- P1 `resolve.rs`: an `item_pointer` must match the Item's whole typed reference, revision
  included.
- P2 `resolve.rs`: Terrain and WorldObject identities are unique across shards.

Real-map rerun on #1170 `2ffba017` (scratch strip of the six orphans): same digest `fa65ffb1…`,
19,373,519 tiles proven, 40 s.

## Review round 2 (Codex review 5371689985 of `434fca26`)

Repair candidate after returning to AUTHORING; both P1 findings fixed in `compile.rs`:

- `equivalence` derives the placement keys of the dropped (0,0,0) teleports from the source and
  requires `manifest.dropped_teleports` to equal them exactly.
- `equivalence` takes the families and applies the Transition rule again, independently of the
  compiler: a mismatch, a record on a (0,0,0) tile, a real destination without a record, or an
  unmet record fails the proof.

Real-map rerun: same digest `fa65ffb1…`, 1,577 dropped keys matched, 41 s.

Architect ruling 5918085982 confirmed the stated assumptions 1 and 2 below, and noted 5,995
provisional keys as superseding "five" in ADR-0021 §4.5.

## Stated assumptions (reversible; for the control plane)

- An Item key that no catalogue record points at is a plain Item and resolves to `item`
  (4,714 such palette keys on #1170, 51 materializable). Ruling 5915258560's "when no pointer
  exists, the entry fails closed" is read as: a catalogue key used directly, or a `routed_to`
  that no pointer confirms, fails.
- `family` stays the key's own family: an Item routed to Terrain stays `item`. A WorldObject key
  never resolves directly (all 12,782 records point at an Item).
- A compact id is the key's index among all keys of its family in the revision, ascending.
- A registry key flagged `provisional` in the index resolves; the flag never hides a real key.

## Evidence

- `compile` on #1170 head `2ffba017` (with #1160 `ee19179e`): stops at the first of the six
  real-destination orphans, as specified. In a scratch copy with only those six attributes
  removed: non-production bundle, 37 s, peak RSS 3.8 GB; 19,373,519 tiles and 24,168,528
  entries proven equivalent; 814,803 entries skipped under 5,995 provisional donor keys (none
  has an Item record); 1,577 teleports dropped; 19,989 Item keys resolved; 23,721,569 bytes,
  identical on a second run, digest `fa65ffb1…`. Production build fails on the first
  provisional key.
- `parity` on the same head reproduces the 1b-1 counts (2,455 / 872 / 1,577 / 6 / 0 / 0).

## Validation

- `cargo fmt --check`; `cargo clippy --locked -p oteryn-world-bundle-compiler --all-targets --
  -D warnings`; `cargo test --locked -p oteryn-world-bundle-compiler` (18 tests, round 1
  assertions added).
- `python3 tools/agents/validate_governance.py`; `python3
  tools/repository/validate_repository_policy.py`; `git diff --check`.

## Closeout

```yaml
status: completed
review: "independent exact-head review by the control plane (pending at freeze)"
follow_ups:
  - "content train: mark the four outside-World orphan teleports as not teleports; records or marks for the other two"
  - "#1170 regeneration with canonical A12 keys (OPEN-2); 5,995 provisional donor keys block production today"
  - "MAP-LOAD-1"
```
