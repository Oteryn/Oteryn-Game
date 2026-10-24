# LOOP-TRAIN-1 — offline training statue and exercise weapons (consolidated task card)

```yaml
task_id: LOOP-TRAIN-1
title: "LOOP-TRAIN-1: offline training statue and exercise weapons"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 1 row LOOP-TRAIN-1 (rows 34, 35); App. 7 LOOP-VITALS-1, LOOP-TRAIN-1, house beds; §5 Wave 5"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_OFFLINE0_STAMINA_AND_OFFLINE_TRAINING_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_EXERCISE0_EXERCISE_WEAPONS_AND_DUMMIES_DECISION_2026-10-03.md
base_branch: main
branch: claude/happy-davinci-ijz3wy-srv-looptrain
pr: null
class: hard
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Local implementation, 2026-10-23: **basic server path implemented and locally qualified;
not frozen, reviewed, published or merged**. `status: implementing` remains until the control
plane completes the ordinary lifecycle. No Git mutation or external review was requested by
this worker. Base HEAD: `581fc2b43eb3541f79be40c34c73c1bfd9463ced`; all changes remain in this worktree.
Client UI and beds are outside this allocation.

Historical CARDS-AUDIT-1 at `origin/main@ac3f7fbc6` found no retained marker/settlement owner.
The pre-ADR-0022 draft #2174 is reference evidence only; none of its private validation
qualifies this implementation. This implementation reuses the current native build,
progression, stamina, item-timed and admission owners.

## Goal

Offline training (Premium statue) and exercise weapons/dummies.

## Server wiring gap list

Local audit, 2026-10-23 at `581fc2b43eb3541f79be40c34c73c1bfd9463ced`
(before implementation; `MECHANICS_STATUS.md` E2E audit and this worktree inspected).
Client UI is outside this allocation.

| Layer | Residue confirmed before implementation | Oteryn now |
|---|---|---|
| L3 | Native relevel and stamina helpers; no offline/exercise rules | Data-loaded gated rates, bounded offline planner and native gain arithmetic in progression `loop_training.rs` |
| L4 | Fenced build and timed-item writers; no composed training owner | Idempotent markers, same-TX initialization/settlement and charge/build/retirement receipts; current session-generation fences |
| Schema | Build/receipts (0030), stamina column (0141), timed state | Migration **0180** extends existing build/death chain; DB-stamped immutable session clock and explicit predecessor; deferred integrity/provenance guards |
| Protocol | No training messages | Commands **66–67**, capability **54**, bounded codecs and append-only registry fragments; limits **0840–0843** |
| Dispatch | No training callers | Real wire adapters, statue logout participant, admission settlement, exercise cadence/stop/checkpoint and actor-end flush |
| Content/offer | Native curves and staged item evidence; exercise profiles absent | 23 exercise profiles generated from existing exact item facts; capability offered only with gate + native weapon/statue/dummy/formula facts in a bundle World |
| Qualification | No real training dispatch PG cases | Three auto-mounted dispatch `--lib` PostgreSQL cases: wire → durable receipt → result bytes, last charge, replay/fence/reload, real successor admission and D88 rollback |

### Reference parity table

| Visible behavior | Reference evidence | Oteryn now |
|---|---|---|
| Only Current Premium may select a statue skill and log out | Accepted OFFLINE0 §6; `training-content-1/facts.json`, `training.offline` | Real command 66 resolves a pinned same-floor reachable PZ statue, refuses houses/PvP/PvE locks/cooldown and commits selection with logout; Free and NotActivated refused in PG |
| Next login settles once after at least 600 seconds offline | Accepted OFFLINE0 §§3–5 | Immutable DB session clock + explicit predecessor, latest build-carrying logout check, capped pool/time, clearing selection, shielding/native curves and stamina; crash/missing predecessor grants no offline gain |
| Offline gain coefficients and percent scale | OFFLINE0 candidate formula; Reference `gameplay-source-reconciliation.json` withdrawal of runtime selection | **PROVISIONAL / GATED OFF**; all coefficients loaded from `rulesets/training/provisional-v1.json`; native formula used for progress/percent |
| Exercise starts on a public dummy, ticks every two seconds and retires on the last charge | Accepted EXERCISE0 §§2–5; staged item charges PROVEN | Real command 67 starts/status/stops; cadence catches up at most 30 ticks, checkpoints at 60 seconds/level/stop, and composes charge/build/last-charge retirement in one TX; 500-charge PG scenario passes |
| Exercise gain per charge | Retained Canary `52550d5` training/vocation sources; reconciliation records melee/bow conflicts and unqualified final magic gain | **PROVISIONAL / GATED OFF**; data values 600 mana / 7 skill tries are test-enabled candidates, never claimed Reference-qualified |
| Inventory sources and item selection | EXERCISE0 ordinary carried charge weapons | Basic path accepts direct main-backpack entries and public non-house dummies; nested/equipment sources refuse. 23 known item profiles admitted; regular wraps 50293 remain unavailable because source movable fact is UNKNOWN |
| Training interruptions and durability | EXERCISE0 §§3–5 | Activity other than chat/look/status stops training; actor-end saves flush; old connection generation cannot stop the current run; unknown saves block control-loss detachment; exact pending checkpoint survives an unknown outcome in-process; restart reloads committed gain/charges without restarting training |

### Owner-selected implementation gate (2026-10-23)

The owner-delegated decision selects option A **with a gate**. Implement the basic
server path using data-loaded rates. OFFLINE0 formula coefficients and retained
Canary exercise gains remain **PROVISIONAL / GATED OFF** in shipped configuration.
Tests explicitly enable provisional training. The Reference reconciliation record
is unchanged; this task does not claim Reference-qualified rates. The previous
rate-selection owner question is resolved. No paid Reference measurement or
external review is triggered by this local worker.

Shipped `rulesets/training/provisional-v1.json` has `enabled: false`. The server's only
operator opt-in is **`OTERYN_ENABLE_PROVISIONAL_TRAINING=1`**; absence or any other value
keeps capability 54 unoffered and both admission settlement and training commands disabled.
Tests inject enabled data explicitly. Native item policies, pinned statue/dummy definitions
and a native training formula are still required after opt-in. No Reference reconciliation
record was edited. Runtime uses `protocol-oteryn`, distinct World/Channel scope, and existing
session-generation fencing.

## Implementation locations

- L3: `crates/gameplay/progression/src/loop_training.rs` and child tests; data in
  `rulesets/training/provisional-v1.json`.
- L4: `crates/gameplay/durability/src/durability/{loop_training,exercise_training,offline_training_state}.rs`;
  existing D88 initializer extracted to `progression_initialization.rs` for same-TX use.
  Small integration changes in build, recovery, revision sequencer, terminal release and timed-item owners.
  Control-loss cleanup waits on unknown training saves before detaching the session.
- Schema: `apps/game-server/migrations/0180_loop_training.sql`. Existing stamina column reused;
  ordinary build and death writers carry the offline fields and recovery verifies their chain.
- Protocol: `crates/protocol-oteryn/src/loop_training.rs` and child tests;
  registry capability `0054-training-v1.json`, commands `0066-offline-training.json` and
  `0067-exercise-training.json`; resource-limit entries 0840–0843.
- Dispatch: `crates/gameplay/dispatch/src/gameplay_transport/loop_training/{mod,exercise,wire}.rs`;
  production connection/admission/cadence/logout callers registered in existing owners.
- PG: auto-mounted `postgres_tests/loop_training.rs` and `loop_training/{exercise,world}.rs`;
  existing fresh-admission fixture extended for an independently refreshed successor publication.
- Content: `tools/content-migration/loop_training_profiles.py` called by the existing
  `register_spell_families.py`; generated spell-native profiles/index/manifest/lock refreshed.
  Pin-refresh ran; the World pin digest did not change.

The historical dependency audit (2026-10-22) is superseded for this local candidate by the
implemented ports above. Publication, independent review and full CI remain control-plane work.

## Accepted design (OFFLINE0 §§3–6, EXERCISE0, as built in the draft)

Settlement arithmetic:

- Latest build-carrying receipt must be a logout marker of the independently resolved immediate
  predecessor lease/session (same Character and World); never inferred as `lease - 1`. Missing/
  older/non-logout latest receipts (death counts) → crash `d=0`; selected skill still cleared.
- Offline time = lease acquisition − logout commit, floored to seconds, ≥ 0, capped 21 days.
  Later settlement `as_of` adds nothing.
- Selected skill with `d<600` consumes/restores no pool; else `t=min(d,pool,43200)` trains and
  restores `d-t` (cap 43200). `t<60` grants no tries. All settlement plans clear the skill.
  No skill → pool restores `d`.
- Rates: top-vocation base attack speed ×2 fist/melee, ×4 distance; magic uses current-vocation
  mana amount and top-vocation ticks with 1-second zero-tick fallback. Level change adds
  `floor(t/4)` shielding via native relevel; same-level percent changes come from a typed native
  Content port (no guessed rounding). Missing source facts refuse before mutation and preserve
  selected skill/pool.
- Stamina regen via the VITALS helper (39h/42h boundaries) inside the same transaction; before
  and u16 after values recorded in the marker receipt.
- Logout restores online pool only if this lease has a settlement marker and only from the
  current GameSession creation clock.

Writer obligations:

1. Validate recovery and gameplay fences; serialize (Character, lease, cause); resolve
   first-marker replay BEFORE fresh intent/time/formula checks; read DB clock once after the
   root lock; one root + progression advance; whole-TX rollback incl. D88 initialization.
2. Settlement waits for pending respawn and initializes progression in the same TX when absent;
   logout waits for actor removal before its marker. Generic Instance scopes are not houses.
3. `CommittedOfflineMarker` acknowledgement only after COMMIT (fresh or replay); it is
   historical completion, never current authority. Reader refuses zero/mismatched marker leases
   and non-successor revisions.
4. Clocks: DB-stamped integer microseconds from FND; checked ms→µs conversion of
   `committed_at`; reconnect `predecessor_generation` is not a predecessor-lease link.
5. Schema: extend the same build row, build/death before/after receipts (death all-NULL/
   all-non-NULL OFFLINE fields), marker version/cause/no-op constraints, once-per-
   (Character, lease, cause) uniqueness, deferred chain guards, `verify_character_integrity`;
   bump the build binding version. Ordinary v1 bindings/replays unchanged.
6. Statue: only `PremiumStatus::Current` passes (no NotActivated bypass); refuse logout/PZ
   locks; activation proposal binds Character/session/connection generation/lease/scope and
   persists only with the final logout marker. WorldUse reach/PZ/cooldown admission required.
7. Exercise charges, tries and last-charge share one composed transaction.

## Acceptance evidence

- [x] Once-per-lease offline settlement from DB clocks; real successor admission covers gain and replay.
- [x] Exercise charges/gain commit exactly once; same occurrence replay and changed-intent refusal covered.
- [x] Current Premium required; Free and NotActivated refused.
- [x] Marker replay retains its original revision/pool; no live authority is inferred from receipts.
- [x] Root/receipt locks, current generation checks and per-character sequencer serialize writers;
      stale fresh writes refuse and a stale connection cannot stop a current trainer.
      A missing training context makes control-loss reconciliation wait; the original run remains intact.
- [x] Committed values reload through a reopened recovery authority; absent D88 initialization rolls
      back with a failed settlement and succeeds atomically with a valid one.
- [x] PG output contains `POSTGRES_STARTED LOOP-TRAIN-1` and `POSTGRES_STARTED LOOP-TRAIN-1 exercise`.
- [x] Mechanic-owned modules and auto-mounted test children; no new PostgreSQL target or client path.
- [x] Shared integration edits are bounded wiring/owner-participant changes; no large shared file grows
      by 150 lines. Existing tests were preserved; no new ignore/skip was introduced.

## Unfinished / deliberately gated

- Rate reconciliation remains **PROVISIONAL / GATED OFF**, as directed. Production activation needs
  the separate accepted Reference rate decision; this work does not grant it.
- Full EXERCISE0 inventory parity: nested/equipment sources, movement/trade of an already checkpointed
  timed item (existing ITEM-LOOP guard), regular wraps 50293 with UNKNOWN movable evidence,
  house/expert dummies, NPC sales and additional world placements are not implemented here.
  Unsupported sources/targets fail closed. Known statue/dummy definitions and public placements can
  be used with the opt-in; the PG mini World proves both target types through real dispatch.
- No client UI, publication, frozen candidate, independent exact-head review or GitHub CI evidence.
  The owner prohibited commits/pushes; the standalone worker produced the reviewable local diff only.
- No load-contention failure is deferred to CI. All three own filtered PG cases passed together.

## Inseparable bundles

- migration + marker writer + durability (build row, receipts, chain guards)
- exercise charge consumption + tries + last-charge in one transaction
- content + regeneration + pins by one author

## Validation (local worktree, 2026-10-23)

All cargo commands use the queued wrapper, pinned toolchain, `--locked --offline`; no dependency
download. PG was run with only the mechanic's filter, real wrapper-provided PostgreSQL,
`--nocapture --test-threads=1` (not a route-less early return).

```bash
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib loop_training_postgres -- --nocapture --test-threads=1
# PASS: 3 passed, 0 failed, 997 filtered; 56.83s; database-start markers present.
# loop_training_postgres_admission_marker_replay_and_restart
# loop_training_postgres_missing_progression_is_atomic_with_settlement
# loop_training_postgres_exercise_real_wire_atomic_stop_replay_fence_and_reload
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-protocol-oteryn -p oteryn-gameplay-progression --lib loop_training
# PASS: protocol 2, progression 4.
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-durability --lib character_build::tests
# PASS: 4.
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-content --lib native_gameplay::tests
# PASS: 25, existing 1 ignored (unchanged).
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-protocol-oteryn -p oteryn-gameplay-progression -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch --all-targets -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-game-server --lib -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check
python3 tools/content-migration/register_spell_families.py --check
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
python3 tools/repository/validate_repository_policy.py
git diff --check
```

Clippy and server lib compilation passed; upstream vendor warnings remain unchanged.
Governance passed; its 129 unit tests passed; repository policy passed. Content regeneration
check passed (36 files); all 432 pre-existing native item profiles remain semantically unchanged,
with 23 exercise profiles added. `pin-refresh .` succeeded via `oteryn-world-bundle-compiler`.
Direct rustfmt also covers the modules reached through generated mounts. Whole-diff checklist
review and workspace-wide enum/field/trait constructor/implementation searches completed;
all new public functions have production callers exercised by the dispatch PG path.
No bound-policy paid review was emitted by this direct worker.

## Changed files (local diff)

- `apps/game-server/migrations/0180_loop_training.sql`
- `content/content.lock.json`
- `content/items/definitions/spell-native-profiles.json`
- `content/items/index.json`
- `content/manifest.json`
- `content/spells.manifest.json`
- `crates/gameplay/dispatch/src/gameplay_transport/channel_states.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/control_loss_reconciliation.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/loop_training/exercise.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/loop_training/mod.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/loop_training/wire.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/mod.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/monk_save_fresh_fixture.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/loop_training.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/loop_training/exercise.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/loop_training/world.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/training_save.rs`
- `crates/gameplay/durability/src/durability/character_authority.rs`
- `crates/gameplay/durability/src/durability/character_build.rs`
- `crates/gameplay/durability/src/durability/character_progression.rs`
- `crates/gameplay/durability/src/durability/character_revision_sequencer.rs`
- `crates/gameplay/durability/src/durability/exercise_training.rs`
- `crates/gameplay/durability/src/durability/fresh_admission.rs`
- `crates/gameplay/durability/src/durability/fresh_admission/terminal_release.rs`
- `crates/gameplay/durability/src/durability/item_timed_state.rs`
- `crates/gameplay/durability/src/durability/loop_training.rs`
- `crates/gameplay/durability/src/durability/mod.rs`
- `crates/gameplay/durability/src/durability/offline_training_state.rs`
- `crates/gameplay/durability/src/durability/progression_initialization.rs`
- `crates/gameplay/progression/src/lib.rs`
- `crates/gameplay/progression/src/loop_training.rs`
- `crates/gameplay/progression/src/loop_training_tests.rs`
- `crates/protocol-oteryn/src/lib.rs`
- `crates/protocol-oteryn/src/loop_training.rs`
- `crates/protocol-oteryn/src/loop_training_tests.rs`
- `docs/agents/tasks/LOOP-TRAIN-1.md`
- `docs/contracts/protocol-oteryn/v1/registry/capabilities/0054-training-v1.json`
- `docs/contracts/protocol-oteryn/v1/registry/command_types/0066-offline-training.json`
- `docs/contracts/protocol-oteryn/v1/registry/command_types/0067-exercise-training.json`
- `docs/contracts/resource-limits/entries/0840-LOOPTRAIN1-RL-REQUEST-BYTES.json`
- `docs/contracts/resource-limits/entries/0841-LOOPTRAIN1-RL-RESULT-BYTES.json`
- `docs/contracts/resource-limits/entries/0842-LOOPTRAIN1-RL-CONFIG-BYTES.json`
- `docs/contracts/resource-limits/entries/0843-LOOPTRAIN1-RL-CATCHUP-TICKS.json`
- `rulesets/training/provisional-v1.json`
- `tools/content-migration/loop_training_profiles.py`
- `tools/content-migration/register_spell_families.py`

## Reference evidence

- OFFLINE0 §§3–6 (§30–34); EXERCISE0 §33–35; SKILLS0 (CANDIDATE)
- Completion plan §4, §5 (Wave 5), §7; coordination #2131 (D1108, D1122, D1160, D1220, D1222)


## Merge repair checkpoint (2026-10-24, #2359)

Local AUTHORING only, branch `claude/happy-davinci-ijz3wy-srv-looptrain`.
No staging, commit, merge, checkout, reset, stash, push or remote mutation by this worker.
The existing merge index remains unmerged until the operator stages its resolutions.

- Resolved the five conflict files, retaining main's generated capability tables and every
  content overlay (training, timed items, crafting and mining).
- Retained allocated capability 54 and commands 66/67, which do not collide with main;
  added runtime fragment `capabilities/0054-training-v1.json` and its JSON entry in main's
  actual golden fixture `src/tests/capabilities_golden.rs` (there is no golden directory).
- Renumbered the training migration from 0173 to **0180**, above main's maximum 0179.
  The embedded migrator discovers migration files automatically; no explicit ledger list exists.
- Training and offline-state modules now live in `core_modules.rs`, since the core build,
  recovery and fresh-admission owners depend on them. The training context imports canonical
  Content types directly so all existing standalone test targets compile too.
- Runtime still has no INSERT/UPDATE privilege on `stamina_minutes`. Ordinary build upserts
  omit it; offline settlement calls migration-owned `game_training_set_stamina` after its
  receipt and build row are written, in the same physical transaction.
- The function locks root then build, requires the current build's same-transaction offline
  receipt, checks character/occurrence/target, and independently derives the allowed target
  from immutable predecessor/logout clocks using OFFLINE0 stamina recovery (600s delay,
  180s/min to 39h, 360s/min to 42h, 21-day interval cap). PUBLIC execute is revoked and the
  installed schema is pinned in search_path. Runtime receives EXECUTE only.
- Pre-training 0141 stamina remains a legacy baseline: projection/continuity checks for stamina
  start at the first training marker, while pool/skill checks cover the whole chain. Historical
  receipt defaults do not override a character's existing stamina. This fixes unchanged party
  XP exhaustion/modifier fixtures and existing low-stamina characters without broadening grants.
- The exercise case seeds 2339 minutes before its first marker and drives real statue logout
  and successor admission across the 39h boundary to 2341 minutes, with committed reload/replay.
- Main's `party_xp.rs` and `character_build_postgres_cases.rs` are byte-identical to main.
  The exercise dispatch PG case now also qualifies direct stamina UPDATE denial, receipt
  replay rejection, valid recovery/boundaries/cap, and invalid target/identity rejection.
- `register_spell_families.py` regenerated the native profiles and all referring manifests/lock.
  All profiles from both sides remain: branch 455, main 461, combined 484. `pin-refresh .`
  regenerated the World pin using the repository compiler; the compiled bundle digest is
  unchanged. Its input digest currently describes the existing index: the operator must
  rerun pin-refresh after staging the resolved/generated inputs, then stage that pin.

Known baseline check failure: `tools/contracts/tests/test_registry_fragments.py` /
`test_split_of_the_aggregate_is_the_committed_tree` fails on resource-limit ordinal filenames
(first mismatch `0613-GUILDCHAT-RL-01.json` vs `0620-GUILDCHAT-RL-01.json`). Reproduced using
only `git show origin/main:<path>` for the base and every fragment; unrelated to training.
No registry test or unrelated resource-limit filename was changed.


Files edited in this repair (incoming main's already-merged files are excluded):

- `apps/game-server/migrations/0173_loop_training.sql` renamed to `0180_loop_training.sql`.
- `crates/gameplay/durability/src/durability/{character_build,offline_training_state,loop_training,core_modules,mod}.rs`.
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/loop_training.rs` and
  `loop_training/{exercise,stamina}.rs`.
- `crates/protocol-oteryn/src/{capability_tables,lib}.rs`,
  `src/tests/capabilities_golden.rs`, `capabilities/0054-training-v1.json`.
- `tools/content-migration/register_spell_families.py`.
- Generated `content/{content.lock,manifest,spells.manifest}.json`,
  `content/items/{index,definitions/spell-native-profiles}.json`, `content/world/pins/oteryn.json`.
- This task record.


### Final validation for the merge repair

Rust toolchain: repository-pinned **1.99.1**. Every command below used
`/home/mole/oteryn-cache/codex-bin/cargo`, `--locked --offline --quiet`.
The three PG commands also used `-- --nocapture --test-threads=1`.
Real PostgreSQL starts and assertions were observed; no route-less early return qualified.

| Exact cargo test package / target / filter | Final result |
|---|---|
| `-p oteryn-gameplay-dispatch --lib loop_training_postgres` | 3 passed, including the extended stamina seam and legacy baseline case |
| `-p oteryn-gameplay-dispatch --lib party_xp` | 13 passed |
| `-p oteryn-game-server --test character_authority_postgres character_build_postgres_cases` | 6 passed |
| `-p oteryn-protocol-oteryn --lib` (all unit tests, no filter) | 414 passed |
| `-p oteryn-gameplay-progression --lib loop_training` | 4 passed |
| `-p oteryn-gameplay-durability --lib character_build::tests` | 4 passed |
| `-p oteryn-gameplay-durability --lib migration_ledger_inspection_is_bounded_before_checksum_materialization` | 1 passed |
| `-p oteryn-gameplay-dispatch --lib all_capability_offer_sets_and_gates_match_fragments` | 1 passed |
| `-p oteryn-gameplay-content --lib native_gameplay::tests` | 25 passed; 1 existing ignored test unchanged |

- PASS `cargo fmt --all --check` and `git diff --check`.
- PASS `cargo clippy -p oteryn-protocol-oteryn -p oteryn-gameplay-progression
  -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server
  --all-targets --locked --offline --quiet -- -D warnings` (exit 0).
  Existing warnings in vendored dependencies were not edited or suppressed.
- PASS governance validator (22 documents, 9 lanes) and all 129 governance unit tests.
- PASS spell registration freshness, 9 spell import tests and 3 runtime-profile tests.
- PASS `oteryn-world-bundle-compiler pin-refresh .` and `pin-check . <local-output>`;
  the compiler built/validated the 25,089,845-byte bundle.
- Contract fragment suite: 4 passed, 1 pre-existing resource-limit roundtrip failure,
  reproduced on untouched `origin/main` as documented above. Runtime capability fragment
  generation, exact registry coverage, golden facts and dispatch gate/offer parity all pass.
- D1462: whole-diff/checklist review, constructor/enum/trait implementation search across
  `apps/`, `crates/`, `tools/`, and production caller/real dispatch test trace completed.
  No test was weakened, deleted or newly ignored. No forbidden CI/policy path was edited.

Parity: all main and branch native profiles retained (484); capability 54 / commands 66–67
preserved; protected stamina tests unchanged; training rates still PROVISIONAL / GATED OFF.

### Unfinished for the operator / CI

1. The working files have no conflict markers, but the existing Git merge index remains
   unmerged, as staging was prohibited. Stage resolved/generated inputs, rerun the repository
   compiler's `pin-refresh .` against that index, then stage the regenerated pin and commit.
   The local pin check passed against the current pre-staging index; it cannot qualify a
   different index produced by the operator's later staging.
2. The unrelated resource-limit roundtrip test on main remains failing; no bypass was added.
3. Local wrapper PostgreSQL reports **17.6** (`170006`); canonical repository CI uses **18.6**.
   CI's canonical-version/full-suite qualification and independent exact-head review remain
   control-plane work. No PostgreSQL version check was changed. No load-contention failure
   is deferred to CI; every requested filtered PG test passed locally.
