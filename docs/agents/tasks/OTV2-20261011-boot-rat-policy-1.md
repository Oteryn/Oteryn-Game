# BOOT-RAT-POLICY-1

```yaml
task_id: BOOT-RAT-POLICY-1
title: "BOOT-RAT-POLICY-1: boot-spawned rat binds an AI policy and qualifies for melee (skeleton)"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/boot-rat-policy-1
issue: 1622
pr: 2185
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 PREREQ-1; App. 7 Archive follow-ups BOOT-RAT-POLICY-1; wave 2 (PREREQ-1 residue child)"
class: "hard (combat), combat review"
migration: no
registry: no
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/content/src/content/project/native_entry.rs # D1138: source carrier checkpoint
  - docs/agents/tasks/OTV2-20261011-boot-rat-policy-1.md
slot_only_paths:
  - content/world/pins/*, goldens and qualified digests if any entry-room fact changes (one author, one commit)
  - mod.rs (only if the boot spawn source composition moves)
public_contracts: []
depends_on:
  - "CREATURE-AI-1: MERGED #1813"
  - "ATTACK-1b: MERGED #1798"
  - "SPAWN-1b: MERGED #1981"
  - "ENTRY-SPELL-TILES-1: MERGED #2005"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

A rat spawned at boot on a no-manifest boot binds a creature AI policy and qualifies for melee in the entry room, so entry-room rats fight a player; also closes the 'bundle World monster cycle' deferral of CREATURE-MOVE-1a.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| CREATURE-AI-1 | MERGED #1813 |
| ATTACK-1b | MERGED #1798 |
| SPAWN-1b | MERGED #1981 |
| ENTRY-SPELL-TILES-1 | MERGED #2005 |

All dependencies merged; the task may start.

## Owned paths (builder edits freely)

- `crates/gameplay/content/src/content/project/native_entry.rs`
- `docs/agents/tasks/OTV2-20261011-boot-rat-policy-1.md` (this record)

Widened to exactly this checkpoint's file set (D1138). Later slices (`native_gameplay.rs` after #2173,
`node/serve.rs` after #2187, `ai/spawn.rs`, `monster_ai_cycle.rs`, boot rat tests) are not owned here
and need a further widening in their own turn.

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- content/world/pins/*, goldens and qualified digests if any entry-room fact changes (one author, one commit)
- mod.rs (only if the boot spawn source composition moves)
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `ai/spawn.rs` spawn realization -> policy binding
- `gameplay_transport/monster_ai_cycle.rs` monster think cycle
- entry-room facts `content/project/native_entry.rs` (read-only unless pins rotate)

## Inseparable bundles

content+regeneration+all pins by one author if facts change.

## Acceptance criteria

- [ ] Reproduce the current refusal first (RED).
- [ ] Boot rat attacks and is attackable in the entry room on a no-manifest boot.
- [ ] Deterministic replay unchanged for existing spawn tests.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

archive records OTV2-20261004-spawn-1a, OTV2-20261011-accept-chat-attack-death-0. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 PREREQ-1; App. 7 Archive follow-ups BOOT-RAT-POLICY-1.

## Builder instructions

- Work on branch `skeleton/boot-rat-policy-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/boot-rat-policy-1
blocker: null
next_action: builder confirms residue against main and starts owned paths
```

## Local preparation evidence (2026-10-15)

- Local-only authoring for existing draft #2185, branch `skeleton/boot-rat-policy-1`,
  inspected base `953ce2c4bf46209367679f8369716d961d8449d2`. No commit, push,
  publication, review trigger, content regeneration, pin rotation or wiring change.
- Residue confirmed against this supplied main-derived skeleton source. The bundle monster
  cycle deferral is already implemented: `monster_ai_cycle::drain_monster_melee` reads
  `rooms.gate_monster_ai()`, and `seam_rooms::gate_monster_ai` returns the combat room.
  Existing bundle cell/target tests cover that accepted path; no duplicate correction is needed.
- `cross_domain_finding: BOOT-RAT-1` (outside current owned paths): the no-manifest branch in
  `node/serve.rs::activate_content` calls `activate_native_entry_room`, producing an active
  generation without `NativeGameplayState`. Boot installs companion policies only when that
  state exists. The qualified fixture spawn still realizes two live creatures, but
  `runtime_actor_carrier::bind_spawned_creature_policy` leaves them unbound when no active
  policy table names their definition. `NativeEntrySpawnSource::activation_facts` carries only
  placement cells, timing, definition key and initial health; the authored behaviour, maximum
  health and speed do not reach policy admission through that tuple.
- The same missing binding affects both directions: `monster_ai_cycle::qualified_melee`
  requires `companion_snapshot` and an exact native creature profile; the cycle also returns
  early when the active generation has no native gameplay. `attack::target_facts` treats a
  failed companion snapshot as `alive = false`, refusing player attacks on an unbound rat.
- A complete correction needs the owning Content projection/admission seam
  (`content/project/native_entry.rs` and/or `content/native_gameplay.rs`) and boot composition
  (`node/serve.rs`) to carry the admitted authored rat policy, followed by the owned melee
  consumer. These files are outside this allocation. No policy literal, key-specific default,
  fabricated native gameplay, room-gate bypass or weaker PZ/authority check was introduced.
- RED reproduction and focused validation are recorded below. The temporary
  acceptance test is retained as an external local patch, not an ignored or failing test in the
  prepared repository diff. The production source remains unchanged pending the owning slot.

### Reproduction provenance

The local overlay is `953ce2c4bf46209367679f8369716d961d8449d2` plus
`/workspace/oteryn-prep/boot-rat-policy-1-red.patch`. It adds only
`boot_rat_policy_no_manifest_activation_binds_the_authored_policy` to the existing
`monster_ai_cycle.rs` test module. The test uses typed version-7/variant-2 identifiers,
the real no-manifest activation and consumed Channel pin, the boot owner's conditional
policy installation, and the source's actual two activation spawn points. It requires
each live rat's admitted policy to match the authored definition key, revision, maximum
health and speed, without injecting a policy or manifest.

SHA-256 bindings:

- Baseline/restored `monster_ai_cycle.rs`:
  `d354951eeb6bdc474ba5da19cdd20d75a3a30d9f452c666ef693d978f795177c`.
- Temporary corrected test source:
  `d805dcc58e47505997765230b35c86f980e92841402396cc810c24397b25c0f5`.
- External RED patch:
  `2afd282a961a4c1df2b2eae28d5fea01544c79566414d8c4a55c6f60fde681c5`.
- Private executable, copied and hashed while holding the Rust lock:
  `/workspace/oteryn-prep/boot-rat-policy-1-evidence/boot-rat-policy-1-tests.gz`,
  retained with gzip level 1 after both positive-control runs. Decompressed ELF SHA-256:
  `5d9a4f8b091875011df84924a493ae313e181a0940e16072d05a7353a737fc36`.
  Its saved `--list` contains 3,180 tests, including the added RED and the seven existing
  `foundation::runtime_actor_carrier::spawn::tests` cases. The shared Cargo target is only
  a cache; the retained executable is a separate read-only compressed copy. Its decompressed
  stream hash was verified against the recorded ELF before removing the uncompressed copy.

### Local validation and remaining acceptance

- Valid RED: `cargo test --locked -p oteryn-game-server --lib
  boot_rat_policy_no_manifest_activation_binds_the_authored_policy --quiet`:
  **0 passed, 1 failed, 0 ignored**. Actual activation and two-creature realization succeed;
  the authored-policy expectation fails at `companion_snapshot` with `NotCreature`.
  Log: `/workspace/oteryn-prep/boot-rat-policy-1-red-corrected.log`.
- Positive controls from the same private overlay executable: filter
  `gameplay_transport::monster_ai_cycle::tests::`, skipping only the added RED:
  **13 passed, 0 failed, 0 ignored**. These cover existing fixture respawn, accepted-room chase
  and cell eligibility, bundle respawn/cell eligibility, and target exclusion. They establish
  existing behavior, not boot melee acceptance.
  Log: `/workspace/oteryn-prep/boot-rat-policy-1-green-corrected.log`.
- Existing deterministic spawn schedule controls from that same executable, filter
  `foundation::runtime_actor_carrier::spawn::tests::`: **7 passed, 0 failed, 0 ignored**.
  Log: `/workspace/oteryn-prep/boot-rat-policy-1-spawn-green.log`.
- Prepared docs-only diff: governance validator passed (22 policy documents, 9 lanes);
  governance unit tests **124 passed, 0 failed**; `git diff --check` passed.
- All Rust builds/tests above use `flock /tmp/oteryn-prep-rust.lock`,
  `CARGO_TARGET_DIR=/workspace/oteryn-prep/cargo-target`, `CARGO_BUILD_JOBS=2` and Rust 1.94.0.
  The temporary Rust overlay was reversed; the restored source hash matches the baseline above.
  The prepared repository diff contains only this task record. No failing or ignored RED test
  remains in the repository, and the overlay tests are not final-head implementation validation.
- Boot-rat melee and player attackability remain unaccepted pending the owning Content/boot
  slot. Independent exact-head review, final wiring, full affected-package validation and any
  publication remain future control-plane work; no trigger or external mutation was made here.


## D1138 source-carrier checkpoint

CP2131 comment6224930839 grants `content/project/native_entry.rs` now.
The qualified native spawn now retains the complete original
`ProjectV2AuthoringProfile` for its exact Creature definition, including its
authored details; the existing health/speed/placement activation tuple remains
compatible. The read accessor returns admitted source data, not a compiled policy
or current runtime authority. No flags, defense values or defaults are invented.
A regression case follows the actual project parser and qualified-room boundary
and compares the carried profile with the complete original admitted profile.

NOT_FULLY_QUALIFIED: the new regression runtime and strict all-target Cargo checks
are NOT_RUN. The earlier genuine boot RED (`NotCreature`) remains OPEN; this
source carrier does not fix or qualify melee/attackability by itself.
`native_gameplay.rs` (and activation only if necessary) follows merge #2173;
`node/serve.rs` follows merge #2187 under D1138. No boot caller, companion-policy
installation, content pin or spawn determinism change is included here.
Checkpoint publication follows D1122; full qualification is required before
final review.
