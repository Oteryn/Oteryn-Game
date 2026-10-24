# CARRIER-SPLIT-1 — separate carrier mechanic implementation modules

```yaml
task_id: CARRIER-SPLIT-1
mode: IMPLEMENT
status: published-for-review
pr: null
repository: Oteryn/Oteryn-Game
branch: claude/happy-davinci-ijz3wy-carriersplit
base: fcd048da6
publication: pushed to the task branch by the coordinator for PR review
```

## Summary

Pure move refactor of the 12,015-line carrier into private mechanic modules.
`carrier.rs` retains sealed carrier/runtime types, construction, slot allocation,
generic actor admission/removal, owner borrows and main activation/respawn step
entry points. It is now **1,956 lines**, approximately the requested 2,000-line core.
The header directs new mechanics to `carrier/<topic>.rs` and module registrations
are sorted by name. `lib.rs` is unchanged.

Owner-authorized local work only; no Git mutation or publication. The bound META
organization policy 3.1.0 was read from the immutable raw source at commit
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb` in `Oteryn/Oteryn`.
No credentials, external writes, dependencies, manifests, features, vendor files,
Soul War files or CI/repository policy files were changed.

## Measured reason and split boundaries

Measurement command (the full bounded patch is retained in local evidence):

```sh
git log --since=60.days -p --format=%H -- crates/gameplay/entities/src/carrier.rs
```

Five commits touched this path in the 60-day window: four mechanic PRs and the
ADR-0022 layout move. Ranges below are the old-side hunk regions in each commit,
rather than a claim that every line in a region changed.

| Commit / PR | Task | Edited regions / items | Split decision |
| --- | --- | --- | --- |
| `bc88d5914`, #2360 | LOOP-MAP-1 | 1653–1802: spawn point Cyclopedia state, constructors and timing; 3501–3507: `apply_cyclopedia_respawn` insertion in the runtime impl. | Spawn definitions, timing and Cyclopedia changes belong to `spawn_realization.rs`. |
| `04ed1930c`, #2322 | PARTY-XP-1 | 681: `RewardDamageShare`; 1455: reward-ledger documentation; 5288: `party_reward_occurrence_inner`; 11419: owner reward occurrence/shares; 11905: test mount. | Damage/reward data and algorithms belong to `damage_ledger.rs`; retain only carrier-owned storage and value payloads in the core. |
| `778a4b1bc`, #2323 | LOOP-MONSTER-1 | 10708: native summon realization removed from the large runtime impl; 11108: summon-spec realization removed; 12097: new `monster_summon_admission` mount. | Preserve that existing admission sibling and separate the remaining native link/death/movement implementation into `native_summon.rs`. |
| `3e9509f0c`, #2305 | CREATURE-MOVE-1 | 67: `creature_relocation` registration. The movement implementation already went to a sibling, rather than editing the large impl bodies. | Preserve `creature_relocation.rs`; move remaining ordinary position implementation into `positions.rs`; use a sorted registration block. |
| `d25623a36`, #2280 | ADR-0022 | Entire 12,098-line file added at its new layered path. | Layout-only migration, excluded from mechanic overlap counts. |

The history supports distinct spawn, reward and summon ownership boundaries.
It also shows that CREATURE-MOVE-1 and LOOP-MONSTER-1 already used sibling files;
this refactor continues those actual boundaries rather than inventing additional
body overlaps. No compile-time or runtime performance improvement is claimed.

## Scope and module map

Owned production paths are `crates/gameplay/entities/src/carrier.rs` and the new
files below under `crates/gameplay/entities/src/carrier/`. The only tracked-path
exception is this task card, explicitly requested by the owner. Local review and
validation artifacts live in ignored `target/CARRIER-SPLIT-1/`. No other existing
source file needed an edit.

| Destination | Items / implementation responsibility | Lines |
| --- | --- | ---: |
| `carrier.rs` | Core types and value payloads, constructors, slot allocation/admission/removal, owner borrows, owner-cycle entry points and main activation/respawn steps. | 1,956 |
| `carrier/actor_memos.rs` | `ActorMemoTable`, `ActorMemos` and ordered replay-memo operations. | 110 |
| `carrier/area_heal_batch_native_tests.rs` | Body of the existing `area_heal_batch_native_tests` module; same cfg, allow and path. | 92 |
| `carrier/continuity.rs` | `PreProductionContinuityGrant`, `NamespaceContinuityGuard` and assignment-binding impls; `CommittedAssignmentOwnerCycle` trait impls. Their sealed type declarations remain in the core. | 159 |
| `carrier/corpse_views.rs` | `RuntimeCorpseItem`, `VisibleRuntimeCorpse`, `CorpseItemBinding`; corpse binding, pin facade and visible-corpse methods. | 187 |
| `carrier/creature_heal.rs` | `CreatureSelfHealRegistration`, `CreatureSelfHealLedger`, `CreatureSelfHealEntry`; live-health/identity reads, self-heal binding/commits and source heal batches. | 333 |
| `carrier/damage_ledger.rs` | `DamageOrigin`, `HighWater`, `DamageReceipts`, `DamageContributor`, `DamageContributors`, `RewardDamageShare`, damage ceilings; contributor ordering, damage preparation/commit, lethal receipts/projections, reward occurrences/shares, `CommittedLethalReceipt` impl and `project_fixed_one_creature_death`. | 1,205 |
| `carrier/fixtures.rs` | `MovementActorFixture`, `CombatDeathFixture`, allocation-failure sentinel; existing test-support carrier/runtime helpers and lab writes. Entire private module is test/test-support gated. | 794 |
| `carrier/native_encounter.rs` | `NativeEncounterSpawnSpec`, `EncounterSpawnError`; native masterless admission and subject removal. | 175 |
| `carrier/native_summon.rs` | `NativeSummonLink`, `NativeSummonLinks`, actor-key helpers, `NativeSummonDeath`, `NativeSummonAdmissionSpec`, `NativeSummonMovementOutcome`, `NativeSummonOrigin`; link bookkeeping, death cascades, admission qualification, reconciliation and summon movement. | 700 |
| `carrier/native_summon_reconciliation_tests.rs` | Body of the existing `native_summon_reconciliation_tests` module; same cfg, allow and path. | 261 |
| `carrier/owner_views.rs` | `CurrentOwnerExactActorCommit`, `CurrentOwnerCombatDeath` and `CurrentOwnerExactActorLookup` impls; visible entities and live-creature census helpers. Borrow entry points remain in the core. | 441 |
| `carrier/player_control.rs` | Player control facts, lease/fence/session transitions and slot-level reservation, commit, rollback, removal and reconciliation methods. Runtime fresh-session entry points remain in the core. | 590 |
| `carrier/positions.rs` | Position snapshot/context impls, `CurrentOwnerMovementPosition` impl, runtime occupancy/census/first-entry helpers and carrier exact position initialization/read/compare-commit methods. | 877 |
| `carrier/spawn_realization.rs` | `SpawnSourceId`, `SpawnDefinition`, point/build/realization state, activation fact aliases/report/progress, respawn family/occurrence/resolution, activation/window state and impls; activation preparation, realization, policy binding, respawn scan/timing/cell and Cyclopedia improvement methods. Main activation/respawn step entry points remain in the core. | 1,213 |
| `carrier/tests.rs` | Body of the existing `tests` module, with an explicit import of moved spawn helpers; same module name and all original tests/assertions. | 2,965 |

All existing public names remain available at `carrier::<name>` through one-item
re-exports where their declarations moved. No new public function, struct field,
enum variant or trait method was introduced. Private implementation seams needed
by the parent, existing sibling modules or existing tests use `pub(super)`,
preserving the original `carrier` privacy boundary. The grant, continuity guard,
owner-borrow fields and runtime construction authority remain private in the core.
Local-only helpers retain private visibility.

## Test parity

Both inventories used the same pre-existing `test-support` feature. No test was
renamed, deleted, ignored or weakened; no module prefix mapping is needed.

| Library module prefix (before → same after) | Before | After |
| --- | ---: | ---: |
| `carrier::area_heal_batch_native_tests::*` | 2 | 2 |
| `carrier::channel_actor_position_tests::*` | 4 | 4 |
| `carrier::damage_contributors_tests::*` | 12 | 12 |
| `carrier::native_summon_reconciliation_tests::*` | 3 | 3 |
| `carrier::npc_actor::tests::*` | 13 | 13 |
| `carrier::party_reward_occurrence_tests::*` | 1 | 1 |
| `carrier::runtime_actor_companion::tests::*` | 16 | 16 |
| `carrier::runtime_actor_conditions::tests::*` | 9 | 9 |
| `carrier::runtime_actor_death::channel_owner_player_death_tests::*` | 10 | 10 |
| `carrier::runtime_actor_periodic::tests::*` | 4 | 4 |
| `carrier::runtime_actor_reentry::tests::*` | 1 | 1 |
| `carrier::runtime_actor_source_reservation::tests::*` | 1 | 1 |
| `carrier::runtime_actor_spell::tests::*` | 26 | 26 |
| `carrier::spawn::tests::*` | 7 | 7 |
| `carrier::tests::*` | 59 | 59 |
| **Total** | **168** | **168** |

`lib-before.txt` and `lib-after.txt` are byte-identical. The pre-move library run
passed 168/168, as did the post-move run (0 failed, 0 ignored).

A workspace scan of `#[path]` and `include!` uses found no other crate directly
including the entities source/carrier. Other crates consume it as a dependency
or facade re-export, so there is no second path-included test inventory to map.
The combat attacker-authority gate reads carrier source text recursively and is
validated separately. The entities library has no PostgreSQL tests; no unrelated
PostgreSQL suite is run for this pure move.

## Move-only review

`target/CARRIER-SPLIT-1/review_moves.py` compares the original source from `HEAD`
with the retained core and every extracted file. It lexes Rust while preserving
literal contents and normalizes both sides through rustfmt in the same context.

- All **451** function signatures/bodies match after canonical rustfmt, including
  nested fixtures, tests and macro entry points.
- **441** bodies are byte-identical modulo indentation. The remaining **10** only
  differ because dedenting the existing test modules changes rustfmt wrapping:
  `child`, `separated_summon_late_reservation_failure_restores_occupied_index_and_visibility`,
  `corpse_projection_pins_are_bounded_and_name_retained_projections`,
  `spawn_points_per_channel_ceiling_is_exact`,
  `spawn1b_mixed_source_blocks_only_its_blockable_points`,
  `activation_realizes_at_most_rl_14_points_per_owner_window`,
  `creature_admission_refuses_past_the_creature_actor_ceiling`,
  `a_131_072_death_backlog_reads_bounded_spawn_records_per_owner_window`,
  `carrier_loss_does_not_release_same_generation_claim`, and
  `strictly_newer_advance_permits_one_fresh_namespace_and_keeps_old_fenced`.
- All **121** struct/enum/trait/constant/type declarations match, allowing only
  `pub(super)` for the moved privacy boundary. Every existing field/variant and
  trait signature is therefore preserved; there are no added constructors or
  exhaustive matches to wire.
- All **87** original public top-level declaration names remain at their original
  public paths. Workspace `rg` over `apps/`, `crates/` and `tools/` recorded **4,483**
  declaration/use sites in `public-consumers.txt`.
- The whole change is reviewed against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`:
  unchanged fence/admission/error ordering, receipt replay and projection semantics;
  no SQL, wire, persistence, production trust, schema or dependency change. No new
  public API requires a new dispatch caller/test. Existing test cfg and assertions
  are preserved. Full diagnostics are retained in `structural-review.txt`.

## Validation

All Cargo build/test commands use the absolute queued wrapper, `--locked`,
`--offline` and:

```sh
export CARGO_TARGET_DIR=/home/mole/oteryn-cache/target-codex/lanes/carriersplit
```

No dependency downloads occurred. The initial exact default-feature inventory
command failed before any source edits with 20 existing errors: content test
helpers (`ChannelContentPin::test` / its test-only `Clone`) are unavailable.
The existing `--features test-support` is required for isolated entities tests;
no manifest or feature definition was changed. Both parity inventories and all
qualifying entities test/Clippy commands enable that same existing feature.

The owner hard rule forbids `--workspace` unless Cargo manifests/lockfiles change.
Accordingly the task-card workspace check is replaced by explicit package-scoped
`--all-targets` checks of entities and all ten direct consumers (including the
real server/dispatch). Full workspace checks remain for repository CI.

| Validation | Result / evidence |
| --- | --- |
| Workspace rustfmt check | Passed; `fmt.log` is empty. |
| Entities Clippy, all targets, `-D warnings`, existing test-support feature | Passed; `clippy.log` is empty. |
| Entities default-feature production check | Passed; `production-check.log` is empty. |
| Entities library inventories before/after | 168 / 168; byte-identical names, unchanged module prefixes. |
| Entities library tests before/after | 168 / 168 passed, 0 ignored/failed. |
| Entities doctests | All 6 passed (2 + 4 rustdoc groups), including private-field compile-fail checks. |
| Combat attacker-authority source gate | 7 passed, 29 unrelated tests filtered out. |
| Explicit all-target checks: entities + all 10 direct consumers | Passed; server, dispatch, durability, encounters, monster-ai, world, spell, combat, NPC and social compile. Existing vendored Tokio/SQLx warnings are outside this diff. |
| Move-only comparison | Passed: 451 functions, 121 data/type declarations, 87 public names; formatting-only body exceptions listed above. |
| Governance validator | Passed: 22 required policy documents, 9 lanes. |
| Governance unit tests | 129 passed. |
| `git diff --check` | Passed. |

Exact successful commands (the listing and full library command ran before and
after the move; the wrapper target directory above applies to every build):

```sh
/home/mole/.cargo/bin/cargo fmt -p oteryn-gameplay-entities
/home/mole/.cargo/bin/cargo fmt --all -- --check
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-entities --features test-support --all-targets --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo check --locked --offline -p oteryn-gameplay-entities --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-entities --features test-support --lib --quiet -- --list
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-entities --features test-support --lib --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-entities --features test-support --doc --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-combat --lib --quiet -- ability::charm_desc_fence_gate_tests::
/home/mole/oteryn-cache/codex-bin/cargo check --locked --offline -p oteryn-gameplay-entities -p oteryn-gameplay-combat -p oteryn-gameplay-spell -p oteryn-gameplay-world -p oteryn-gameplay-monster-ai -p oteryn-gameplay-encounters -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-gameplay-npc -p oteryn-gameplay-social -p oteryn-game-server --all-targets --quiet
python3 target/CARRIER-SPLIT-1/review_moves.py
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
rg -n '(#\[path|include!)' apps crates tools --glob '*.rs'
git diff --stat
git diff --check
```

The exact failed baseline command, before any source move, was:

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-entities --lib --quiet -- --list
```

The whole uncommitted diff, including all new files, is retained as
`whole-diff.patch` for self-review. Plain `git diff --stat` shows only the tracked
`carrier.rs` (372 insertions, 10,431 deletions); it excludes the 15 new source
files and this task card because the owner forbids staging. The line-count map
above includes all source files: 12,058 lines total versus 12,015 originally.
All other existing carrier siblings and `lib.rs` are unchanged.

## Unfinished

No implementation or permitted local-validation work remains. All changes are
uncommitted. Publication and repository CI are outside this local-only task.
The full workspace check is left to CI under the owner hard rule; the explicit
11-package all-target check passed locally. Isolated default-feature entities
test compilation retains its pre-existing missing-content-test-support problem;
qualifying local tests and Clippy use the existing `test-support` feature,
without changing a manifest or feature definition. No PostgreSQL case is left
unqualified: this crate has none and this task adds no mechanic or DB path.
