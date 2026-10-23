# MAP-CUTOVER-1c — map cutover final stage (consolidated task card)

```yaml
task_id: MAP-CUTOVER-1c
title: "MAP-CUTOVER-1c: map cutover final stage"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
class: "hard (persistence and review of the first durable-Ground cutover)"
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row MAP-CUTOVER-1c; §7 map cutover chain"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_TRACK_PACKETS_2026-10-05.md   # §2.5
  - docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md       # §2.8 (old MAP-CUTOVER-1)
base_branch: main
branch: null
pr: null
needs_migration: false
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. Bundle boot exists, but the legacy-removal/first-reset cutover scoped by MAP-CUTOVER-1c has not landed. Evidence: `crates/gameplay/world/src/map/boot.rs`.

Remaining: MAP-OVERLAY-1c reset integration, legacy loader removal, retirement and crash-safe final cutover remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2169, branch skeleton/map-cutover-1c
(the draft carries a skeleton task record only; it has no code)

## Goal

Finish the map cutover: the runtime boots solely from the new map pipeline with the overlay reset in
place, and the legacy path is removed (MAP_TRACK §2.5, SOCIAL_MAP §2.8). It is the third of
MAP-CUTOVER-1a -> 1b -> 1c and the first cutover with durable Ground items: the ADR-0021 §4.7 reset,
the overlay rebuild and the position fallback. Acceptance tests carried from SOCIAL_MAP §2.8:

- the first boot of a World from a bundle runs the §4.7 reset: every pre-cutover Ground item with the
  legacy map revision is retired, then the bundle activates;
- a restart outside a reset reuses the active bundle and never switches revisions;
- a durable Character position the active bundle does not hold falls back at admission.

## Dependencies (re-verified against origin/main 864beba6d, 2026-10-22)

| Dependency | State |
|---|---|
| MAP-CUTOVER-1a (boot a Channel from a pinned bundle) | MERGED; `crates/gameplay/world/src/map/boot.rs` |
| MAP-CUTOVER-1b (domain 17 bundle World) | MERGED (#1916) |
| CHAR-POSITION-1 (position fallback) | MERGED (#2006); archive `OTV2-20261012-char-position-1.md` |
| MAP-OVERLAY-1c (World reset RETIRING -> drain -> Ground retirement -> atomic epoch N+1 ACTIVATED) | **BLOCKING**: NOT_STARTED re-implementation card on main (draft #2167 closing); `docs/agents/tasks/MAP-OVERLAY-1c.md`. MAP-OVERLAY-1c "releases MAP-CUTOVER-1c": this task starts only after it merges and consumes its reset port, active-revision reader and epoch history |

Rows marked BLOCKING must merge (or be accepted/decided) before the slot commit; owned code may be
drafted earlier but must not assume unmerged schemas. MAP-CUTOVER-1c follows MAP-OVERLAY-1c on its own
track (MAP_TRACK order); no child of a CANDIDATE brief is allocated before the brief is ACCEPTED.

## Accepted decisions

- SOCIAL_MAP §2.8 packet: decision ADR-0021 §4.7 (first cutover) and §5; MAP-LOAD-PACKET-1 §2.2 (the
  pin); `candidate_bases: [CHAR-POSITION-0]` (now merged as CHAR-POSITION-1); depends on MAP-OVERLAY-1c;
  review hard and persistence (final frozen head).
- MAP_TRACK §2.5 renames the old SOCIAL-MAP §2.8 to MAP-CUTOVER-1c and leaves it unchanged; 1a/1b serve
  a bundle World on testing and preproduction only, with no Ground writes.
- Not in scope: the CI artifact job and the production pin; running a cutover on a protected or
  production World.
- Inseparable bundle: legacy path removal + boot switch + architecture-check update (never split).

## Reference behaviour from the draft (re-implement, do not copy)

None: the draft has no code. The task record only restated the packet's goal, owned paths and
inseparable bundle above. The reset sequence, the census, the epoch writer and the active-revision
reader are owned by MAP-OVERLAY-1c and are consumed here, not duplicated.

## Owned paths (ADR-0022 layout)

Old `apps/game-server/src/world_runtime.rs`, `src/map/boot.rs` and `src/map/mod.rs` live in
`crates/gameplay/world/src/world_runtime.rs`, `crates/gameplay/world/src/map/boot.rs` and
`crates/gameplay/world/src/map/mod.rs` (all exist; L4 `world`). The node composition root stays
`apps/game-server/src/node/serve.rs`.

- `crates/gameplay/world/src/map/cutover.rs` (new, one mechanic: first-boot decision "reset vs reuse active bundle" over the MAP-OVERLAY-1c port)
- `crates/gameplay/world/src/map/cutover_tests.rs` (new)
- `apps/game-server/tests/map_cutover_postgres.rs` (new, PostgreSQL 17.6 harness; use `apps/game-server/tests/map_load_base.rs` as the neighbour)
- `docs/agents/evidence/MAP-CUTOVER-1c-*.md`
- `docs/agents/tasks/MAP-CUTOVER-1c.md`

Slot-only (final commit at the queue turn, after merging main): `crates/gameplay/world/src/world_runtime.rs`
boot switch; `crates/gameplay/world/src/map/boot.rs` first-boot reset call; `crates/gameplay/world/src/map/mod.rs`
(the `mod cutover;` line only); `apps/game-server/src/node/serve.rs` wiring of the boot path and the
legacy-loader removal; `workspace-boundaries.toml` / `tools/architecture-check` rule update; no migration,
no registry unless MAP-OVERLAY-1c leaves one open.

## Wiring points (verified on origin/main 864beba6d)

- `crates/gameplay/world/src/map/boot.rs` (`check`, `CheckedBundle::boot`, `BundleWorld`): boots a Channel from the pinned bundle; legacy `z` frame constants stay as the bundle frame
- `crates/gameplay/world/src/world_runtime.rs` boot path
- `crates/gameplay/world/src/map/overlay.rs` and `map/overlay/`: Channel overlay, `map_revision`
- `crates/gameplay/world/src/map/mod.rs` (`BundlePins`, `LoadError`, `WorldBase`)
- `apps/game-server/src/node/serve.rs` and `serve/bundle_npcs.rs`: node composition of the bundle World
- `tools/architecture-check` (`cargo +1.95.0 run -p oteryn-architecture-check -- workspace .`): rules that name the legacy loader
- CHAR-POSITION-1 position fallback at admission (merged)

## Acceptance criteria

- [ ] The server boots only from the new pipeline; the legacy loader is removed.
- [ ] First boot of a World from a bundle runs the reset and retires every pre-cutover legacy-revision Ground item before activation.
- [ ] A restart outside a reset reuses the active bundle and never switches revisions.
- [ ] Character positions fall back correctly when the active bundle does not hold them (CHAR-POSITION-1).
- [ ] `oteryn-architecture-check` passes with the legacy-path rules removed.
- [ ] A crash during the first-boot reset resumes (MAP-OVERLAY-1c step 2) and never boots the old bundle over partially retired Ground.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests in their own child files, not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`serve.rs` are wiring-only; no file over ~1000 lines grows by more than ~150 lines.

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-world map_cutover
cargo test -p oteryn-gameplay-world map
cargo test -p oteryn-game-server --test map_cutover_postgres   # PostgreSQL 17.6 harness
cargo +1.95.0 run -p oteryn-architecture-check -- workspace .
python3 tools/agents/validate_governance.py
```

## Reference evidence

- `docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_TRACK_PACKETS_2026-10-05.md` §2.5
- `docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md` §2.8
- PR #1916 (MAP-CUTOVER-1b); `docs/agents/tasks/MAP-OVERLAY-1c.md`
- `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4, §5 (Wave 3), §7
