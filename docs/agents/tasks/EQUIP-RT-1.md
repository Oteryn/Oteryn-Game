# EQUIP-RT-1

```yaml
task_id: EQUIP-RT-1
title: "EQUIP-RT-1: Equipment active set, SPEED and SUPPRESS (bounded checkpoint)"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/equip-rt-1
issue: 1622
pr: null
base_sha: 030262523
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md § LOOP-DAMAGE-1, MAGIC-COND-1: EQUIP-RT-1 (EQUIP0); bounded checkpoint, SKILLS-0 consumer excluded"
class: "hard (combat); combat and determinism review"
migration: no
registry: no
owner: Codex lane 1 (2026-10-22)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-22
updated_at: 2026-10-22
execution_policy: continuous_progress
merge_authority: CP_ONLY
owned_paths:
  - crates/gameplay/items/src/equipment_effects.rs (new)
  - crates/gameplay/items/src/lib.rs (one pub mod line)
  - crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner.rs (new)
  - crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests.rs (new)
  - crates/gameplay/dispatch/src/gameplay_transport/equipment_move.rs (module line, post-commit refresh only)
  - crates/gameplay/dispatch/src/gameplay_transport/movement_equipment.rs (speed term only)
  - crates/gameplay/dispatch/src/gameplay_transport/familiar_cast.rs (speed argument only)
  - crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast.rs (speed argument only)
  - crates/gameplay/spell/src/spell/owned_cast_facts.rs (equipment_speed_bonus removal only)
  - crates/gameplay/foundation/src/foundation/condition.rs (suppressed set only)
  - crates/gameplay/foundation/src/foundation/condition_tests.rs (suppression cases only)
  - crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs (test-only fixture module registration)
  - crates/gameplay/dispatch/src/gameplay_transport/mod.rs (initialize_first_entry wiring only)
  - crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/*.rs
  - crates/gameplay/items/src/equipment_effects_tests.rs
  - crates/gameplay/foundation/src/foundation/condition_suppression.rs
  - crates/gameplay/foundation/src/foundation/condition_suppression_tests.rs
  - docs/agents/tasks/EQUIP-RT-1.md (task packet/notes)
slot_only_paths: []
public_contracts: []
depends_on:
  - "ITEM-MOVE-2a: merged in #2190 (ITEM-LOOP-1, CP D1438)"
  - "SPEED-1, COND-1, CHAR-BUILD-1, EQUIP-CONTENT-1: merged"
  - "EQUIP0 ACCEPTED 2026-10-13; Extra slot LIGHT-only by the 2026-10-14 amendment"
  - "SKILLS-0 live skill read: not on main; the SKILL_BOOST consumer is excluded from this checkpoint"
blocks:
  - EQUIP-PARITY-1
  - TIMED-FX-1
  - IMBUE-RT-1
cross_repository_coordination_id: null
external_repositories: []
```

Bounded checkpoint; this does not close the EQUIP-RT-1 plan row. Run 1h continues
WIP `7a26092e4` in AUTHORING under the integrator's explicit local-worktree grant.
Changes stay uncommitted; independent review and integration belong to the control plane.

## Module layout (run 1h, planned before edits)

- Items: pure equipment derivation in `equipment_effects.rs`, its tests in
  `equipment_effects_tests.rs` (one registration, 180-line module).
- Dispatch: admission reads/installation and shared suppression mapping in
  `equipment_effects_owner.rs`; `mod.rs` holds only calls, permit and lock comments (+17).
- Owner tests split ability errors, first entry, moves and shared PG setup into children.
  `monk_scenario.rs` reuses the private monk fixture, registered once from its owner (+3).
- Foundation: suppression setter in `condition_suppression.rs`, tests in
  `condition_suppression_tests.rs`, reusing the existing condition test helpers.
  The large condition parent retains field/default/admission wiring only.

## Earlier runs

Previous WIP task-card revisions retain historical implementation/validation. All 413 profiles
match ability/raw speed (zero); exact reductions commute and sorting fixes overflow order.
Run 1f narrowed the admission lane to final revalidation/read/install, outside runtime;
monk save uses separate roots/databases and never takes that lane.

## Run 1g diagnostics

The single-holder `release_monk_unknown` PgPool races its own observation: save #2 increments
SAVES before `current_monk_fence`; `drive_until` sees 2 and cancels that pending read. SQLx
returns its holder asynchronously; the fixture revision query is no readiness barrier.
Failing trace: cancellation 09:40:52.123620; idle=0 at .125756, RootUnavailable. Focused pass
returned the holder within 1.818ms. New equipment tests overlapped the failing full run, but
the uninstrumented focused boundary also failed (0 passed/1 failed, 5.28s), without equipment.
Full diagnostic run: 902 passed/1 failed/14 ignored, 118.24s; equipment-skipped and temporarily
base-admission measurements passed 897 each. No exact clean-base failure was claimed here.
Integrator subsequently reproduced 2/10 on clean origin/main. Proposed existing readiness
barrier belongs to MONK-SAVE-FLAKE-1; no boundary, drive_until or fixture change in this lane.
All temporary diagnostics were removed; retained `.equip-run1g-*.log` files are local evidence.

## Run 1h restructure and reuse

Admission refusal outcomes and lane -> runtime -> spell-state ordering are preserved. Equipment
is read before runtime is locked. Installation and committed moves reuse one suppression
mapping; the setter still removes through `ConditionStore::dispel_key` and retains the set
on death. No new durable state, dependency, trait method or condition kind.
Moved each fixture/test body once; no second `#[path]` load or duplicate registration.
The only access wrapper is `equipment_effects_owner_tests/monk_scenario.rs`, mounted as a
child of the existing monk test owner, so Harness, prepare_current_session, Dependencies::owner,
fresh::Source, run, recovery, root and pool are reused without changed visibility or copies.
Also reused: item_ability_profile, read_current_equipment, content activation, item mint/transfer,
condition test root/facts/dot/def/apply, spell state initialization and movement equipment reader.
No changes to the monk boundary test, drive_until, fixture logic or duplicate-source allowlist.

## Validation (run 1h)

Earlier login-shell runs selected rustup and left PG unset; discarded for PG qualification.
Results below use the queued wrapper with `login=false`, PostgreSQL 17.6, and `pg-go` present.
No missing-PG marker in the qualified logs. Timings are test time, not compile/queue time.

- PASS `cargo +1.95.0 fmt --all --check`.
- PASS `cargo +1.95.0 clippy --locked -p oteryn-gameplay-items -p oteryn-gameplay-foundation -p oteryn-gameplay-spell -p oteryn-gameplay-dispatch --all-targets --quiet -- -D warnings`.
- PASS `cargo +1.95.0 test --locked -p oteryn-gameplay-items --lib --quiet -- --nocapture`: 30, 0.00s.
- PASS `cargo +1.95.0 test --locked -p oteryn-gameplay-foundation --lib --quiet -- --nocapture`: 334, 1.83s.
- PASS `cargo +1.95.0 test --locked -p oteryn-gameplay-spell --lib --quiet -- --nocapture`: 221, 5.54s.
- PASS `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib equipment_effects_owner --quiet -- --nocapture`: 6/0 failed/916 filtered, 71.78s.
- PASS x3 `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib --quiet -- --skip boundary_unknown_monk_save_keeps_lease_until_store_recovers --nocapture`:
  907 passed/0 failed/14 existing ignores/1 filtered each; 183.94s, 141.09s, 181.66s, consecutively.
- OBSERVED x3 `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib --quiet -- --nocapture`:
  PASS 908/0 failed, 199.64s; FAIL 907/1, 186.87s; PASS 908/0, 142.65s; 14 existing ignores each.
  The only failure was the unchanged monk boundary, RootUnavailable: MONK-SAVE-FLAKE-1.
  Boundary observations: PASS / FAIL / PASS. Accepted three-green exclusion criterion MET.
- PASS `cargo +1.95.0 test --locked -p oteryn-game-server --lib --quiet -- --nocapture`: 306/0 failed/3 existing ignores, 31.24s.
- PASS `cargo +1.95.0 test --locked -p oteryn-game-server --test item_ref_production --quiet -- equipment --nocapture`: 7/0 failed/414 filtered, 8.78s; PG version 170006 logged.
- PASS `cargo +1.95.0 run --locked -p oteryn-architecture-check --quiet -- workspace .`: workspace-boundaries PASS.
- PASS `python3 tools/agents/validate_governance.py`: 22 documents/9 lanes.
- PASS `python3 -m unittest discover -s tools/agents/tests`: 124 tests (includes lifecycle/META).
- PASS `python3 tools/repository/validate_repository_policy.py`: 23 files/67 workflows.
- PASS `git diff --check` and `git diff d9e21624b --check`.

Workspace clippy excluded (HARD rule: manifests unchanged); inherited-prompt HTTP 403 excluded.
Unchanged vendor/tokio missing-doc warning is non-fatal. CI: Merge Queue Rust Linux `lib-rest`,
`not test(content::)`, includes these PG tests. Logs/exact commands: `.equip-run1h-pg-*.log`.
D1462: whole WIP/working diff/new files reviewed; workspace constructors/impls/callers checked;
registrations retained once (5 pure/6 dispatch/2 suppression). No new variant, field or trait
method in 1h. Production helpers have real dispatch callers; deferred apply_stat/apply_protection
retain their explicit no-runtime-caller grant. No test semantics, fixture logic, timeout or assertion changed.

## File changes (cumulative WIP + run 1h)

Against immutable task fork `d9e21624b`; origin/main advanced separately during this run.
Prefixes under `crates/gameplay/`: D=`dispatch/src/gameplay_transport/`,
F=`foundation/src/foundation/`, I=`items/src/`, S=`spell/src/spell/`.

| File | + | - |
| --- | ---: | ---: |
| `D/equipment_effects_owner.rs` | 137 | 0 |
| `D/equipment_effects_owner_tests.rs` | 127 | 0 |
| `D/equipment_effects_owner_tests/ability_tests.rs` | 88 | 0 |
| `D/equipment_effects_owner_tests/committed_moves_tests.rs` | 134 | 0 |
| `D/equipment_effects_owner_tests/first_entry_tests.rs` | 148 | 0 |
| `D/equipment_effects_owner_tests/monk_scenario.rs` | 59 | 0 |
| `D/equipment_effects_owner_tests/postgres_scenario.rs` | 300 | 0 |
| `D/equipment_move.rs` | 21 | 1 |
| `D/familiar_cast.rs` | 11 | 2 |
| `D/mod.rs` | 17 | 0 |
| `D/monk_save_postgres_tests.rs` | 3 | 0 |
| `D/movement_equipment.rs` | 1 | 12 |
| `D/native_combat_cast.rs` | 4 | 1 |
| `F/condition.rs` | 8 | 1 |
| `F/condition_suppression.rs` | 18 | 0 |
| `F/condition_suppression_tests.rs` | 94 | 0 |
| `F/condition_tests.rs` | 3 | 0 |
| `I/equipment_effects.rs` | 180 | 0 |
| `I/equipment_effects_tests.rs` | 256 | 0 |
| `I/lib.rs` | 1 | 0 |
| `S/owned_cast_facts.rs` | 0 | 23 |
| `docs/agents/tasks/EQUIP-RT-1.md` | 194 | 0 |

Files growing >200: I/equipment_effects_tests.rs (+256), pure L3 arithmetic/permutations;
D/equipment_effects_owner_tests/postgres_scenario.rs (+300), shared L5 composition of the
existing durable/runtime/content owners for both mechanics. No copied fixture or durable rule.
Existing files grow <=54 lines in 1h; large parents retain wiring only.

## Residue / handoff

- Monk boundary ownership is MONK-SAVE-FLAKE-1. The integrator accepts three green full
  dispatch suites with that boundary excluded only on the command line, plus three full
  observations without exclusion. No source skip, assertion change or timeout increase.
- PROTECTION awaits incoming element/armor/defence; SKILL_BOOST awaits SKILLS-0. STAT_BOOST
  maxima/LIGHT have no v1 rows; Premium has no content field. TIMED-FX-1, IMBUE-RT-1 and
  EQUIP-PARITY-1 remain excluded. apply_stat/apply_protection retain the deferred-consumer grant.

## Planned commits (integrator only)

1. `feat(items): derive deterministic equipment effects` —
   `I/{equipment_effects.rs,equipment_effects_tests.rs,lib.rs}`.
2. `feat(gameplay): consume equipment speed and suppression on move and admission` —
   all D/, F/ and S/ files in the file table above (dispatch, foundation and spell).
3. `docs: record EQUIP-RT-1 bounded checkpoint` — `docs/agents/tasks/EQUIP-RT-1.md`.
