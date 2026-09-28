# OTV2-20260928-world-base-map

```yaml
task_id: OTV2-20260928-world-base-map
title: Populate the WorldPlacement.Base full map (tiles and items) from CrystalServer as B3 region files
mode: MIGRATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/blissful-turing-oca168-world-map
issue: 162
pr: 1170
base_sha: 4e65a5e4
head_sha: null
owner: owner-launched Claude Code session
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-28T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/world-authoring/
  - content/world/placements/
  - .gitattributes
  - .github/workflows/world-metadata-authoring.yml
  - apps/game-server/tests/content_world_project_repository.rs
  - docs/agents/tasks/active/OTV2-20260928-world-base-map.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: KAN-16
```

## Outcome

Step 3 of the owner's world-map plan: the whole base map of `world.otbm` in
`zimbadev/crystalserver@00ce02a5` (`summer-update`) becomes `WorldPlacement.Base`:
19,325,129 tiles and 24,925,845 items on floors 0-15, in 1,208 region files
(`OTERYN_WORLD_REGION_B3/v1`, about 21.6 MB) plus a 2.5 MB `index.json`. The source is
sha256-pinned and `OtsHypothesisOnly`.

## Architecture and source of truth

- **Palette indirection (owner decision):** region files store a palette index per item.
  `index.json` holds the palette, one entry per distinct map server id in ascending id
  order: `{key, source_item_id, provisional}`.
  - A bound id (`imports/crystalserver/bindings/items.json`, `ots/item_server_id`) takes its
    binding target key, registry or named, with `provisional: false`.
  - Any other id takes `donor:crystalserver@00ce02a5:item/<id>` with `provisional: true`
    (5,943 appearance-only ids and 44 items that `items.xml` declares).
  - Item identity work later rewrites palette entries only. Region files do not change.
- **Format:** measured before selection (about 22 MB versus 3.25 GB as JSON sectors). Spec
  in the `world_region_codec.py` docstring.
- **Coexistence:** `content/world/placements/` held only the `READY_UNPOPULATED` marker. The
  legacy WorldProject seed reproduction removes the successor files before its exact diff.
- **Not authoritative for runtime:** the base is migration evidence. Runtime loading, the
  per-channel overlay and Oteryn edits are separate work.

## Acceptance criteria

- [x] `world_region_codec.py` encodes and decodes region files deterministically and rejects
      damaged input.
- [x] `convert_world_base.py` is sha256-pinned, deterministic, has a `--check` mode, and
      fails closed on unknown OTBM attributes and node types.
- [x] The palette follows the owner rule, and the capture summary records its size and the
      provisional entries and occurrences, split by presence in `items.xml`.
- [x] `validate_world_base.py` checks per-region sha256, decode, sorted tiles, bounds,
      counts, palette rules, palette use and summary totals.
- [x] `test_world_base.py` covers codec round trips, palette variants and validator
      negatives.
- [x] The world-metadata workflow validates and tests the base family.
- [ ] Required checks pass on the frozen PR head.

## Excluded scope

- Admitting provisional items (item B1b) and a Terrain identity path.
- A patch layer for authored Oteryn edits over a regenerated base.
- Runtime consumption, the per-channel overlay and the native `WorldTilePosition` mapping.
- The `15.30/` fragment maps.

## Validation

- `convert_world_base.py --check` against the pinned checkout is byte-identical.
- `validate_world_base.py` passes; `test_world_base.py` and `test_world_authoring.py` pass.
- `ruff check` and `ruff format --check` pass from the repository root.
- `validate_materialized_game_tree.py`, `validate_governance.py` and
  `validate_repository_policy.py` pass.
- `cargo test --locked -p oteryn-game-server --test content_world_project_repository` and
  `cargo fmt --all -- --check` pass; the legacy seed reproduction diff is empty.

## Independent review

- required: pending owner/control-plane decision
- exact head: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`
