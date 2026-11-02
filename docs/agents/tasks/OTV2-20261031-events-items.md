# OTV2-20261031-events-items

## Task and authority

- Status: AUTHORING; implementation and validation remain in progress.
- Branch: `chatgpt/events-items`; one task lead owns commits and publication.
- Base: `ae4188a9c8d461acb152044814ea23075de02ed4` (verified protected main at task start).
- Primary donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
- Accepted request: W19a-e item actions, W05b/d world and seasonal events, W05e Autumn Vintage, W16 Summer 2026 encounters.
- Bound organization policy: version 3.1.0, `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.
- Deliver one draft PR against main. No merge, rebase, force-push, review request, production activation or heavy-tests label.

## Owned implementation slices

| Slice | Existing owner to extend | Allocated migration, only if needed |
|---|---|---|
| W19a-e | item actions, reward bags, consume/transform, appearance ownership | 0712 |
| W05b/d | world-change draws, map/spawn projections, scheduled events | 0713 |
| W05e | scheduled events, loot, NPC services, potion restoration | 0714 |
| W16 | shared boss encounter and creature condition owners | 0715 |

Capability IDs 256-257, command IDs 482-485 and mechanic ordinals 622-625 are reserved by the task. Reuse existing client commands whenever possible. Check current main and open PRs before using or publishing any reserved number.

## Acceptance

1. Every sourced action reaches an existing live command or server consumer; no test-only implementation or duplicate owner.
2. Every value has donor file:line or accepted content evidence. Unsourced choices are raised on this PR as individual `QUESTION:` comments with alternatives and a recommendation.
3. Durable item, unlock, event and encounter transitions use the existing session-generation fence and atomic receipt/transaction patterns. Replays return recorded results.
4. World changes project their recorded state into the map or spawn owner. Seasonal effects are bounded by their sourced windows and success conditions.
5. Autumn Vintage has collection, NPC delivery, treading and world-success consumers only after the required rules are sourced or answered by the owner.
6. Summer encounters use the shared encounter owner; absent donor admissions or transitions are explicit questions, never guessed encounters.

## Verification plan

- Read the required repository skills and nearest applicable instructions before edits.
- Unit coverage for rules, boundaries and refusals; route/dispatch PostgreSQL coverage for each durable transition in the existing mounted directory, with completion markers and route-less handling.
- Format, clippy and changed-package tests; focused PostgreSQL cases on the local test database where available.
- Regenerate content and world pins through repository tooling; run content-loader, pin and repository integration checks for affected inputs.
- Review full diff and exact changed-file list, run `git diff --check`, recheck number allocation and read PR answers before publication and closeout.

## Initial evidence

- Main includes rust remover and lottery admission through PR #2761, and the world-change/schedule owners through PR #2757.
- Crystal provides source-backed item tables and world-change scripts; Summer monster statistics alone are not evidence for encounter admission or phase rules.
- Official Autumn Vintage news confirms October 1-8 and October 17-24, with +33% health/mana potion effect for seven days after world success; thresholds, exact monster pool and treading rules require further source evidence or owner answers.

## Outstanding work

Implementation, source parity tables and test results are pending. This initial record establishes scope and the early draft question channel; it makes no completion claim.

## Local takeover checkpoint (2026-11-01)

- Current supplied head: `4c33b1409af987aba430bcd3e466c16fa9ca1f0e`, with
  `38641fecd4558a4a14e6b15b3874110ffa2dff19` merged without a commit. Still
  AUTHORING; no frozen candidate or publication. The owner retains all Git writes.
- PR #2781's current description and comments were read. The supplied tree contains
  the source packets and Phosphorus curse implementation, but lacks the described
  W19/Cake/Autumn runtime files, mounted cases, and migrations 0712–0715. The PR's
  handoff comment 6550615315 identifies uncommitted runtime work in another
  worktree. That preserved patch must be supplied here before this task can finish;
  the takeover is authorized to work only in this directory.
- The registration conflict retains main's enchanting and Task Board cohorts plus
  event admission. Event admission must run after established owners: running it
  before Task Board loses accepted equipment, elemental damage and trade facts for
  items 21178–21180. A new regression reproduces that loss.
- Main's typed damage kind is passed through the extracted condition helper. The
  initial workspace check reproduced three undefined-`kind` errors after the merge.
- Generated JSON conflicts are rebuilt by the repository generators. The Git index
  remains unmerged because staging is forbidden. `regen.sh --no-stage` refuses an
  unmerged index; individual generation steps can run without changing it. Final
  pin refresh/check requiring a resolved index remain the owner's follow-up.
- This supplied branch adds no migration relative to the merged main, so there is
  nothing to renumber to 0735. The repository duplicate-number script and a
  working-tree duplicate-number scan both found no duplicate migration numbers.
- All answered scope boundaries on PR #2781 remain binding, especially Cake Golem
  admission answer 6556536661: production must refuse before writes; the explicit
  test-only policy and production no-write PG case belong to the missing runtime
  patch. No native Creature admission or Item reference suffix is authorized here.

The new auto-mounted `postgres_tests/summer_phosphorus.rs` case drives the actual
scheduled monster dispatcher and checks all 21 source ticks. It uses the existing
64 MiB scenario runner and exact admitted final form, without introducing encounter
admission or phase transitions. Its PostgreSQL body remains unqualified below.

### Files edited during the takeover

- `content/content.lock.json`
- `content/items/definitions/spell-native-profiles.json`
- `content/items/index.json`
- `content/manifest.json`
- `content/spells.manifest.json`
- `crates/gameplay/dispatch/src/gameplay_transport/actor_conditions.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/summer_phosphorus.rs` (new, unstaged)
- `docs/agents/tasks/OTV2-20261031-events-items.md`
- `tools/content-migration/register_spell_families.py`
- `tools/content-migration/test_event_item_profiles.py`

### Actual local validation

All Cargo invocations below use this executable and `CARGO_NET_OFFLINE=true`:

```sh
task_cargo=/home/mole/oteryn-cache/codex-bin/cargo
```

Generators used the same wrapper on `PATH` and `TMPDIR` inside this worktree.
Counts are per command, not a sum of unique tests. Existing dependency warnings
in vendored Tokio/SQLx were left unchanged.

| Command | Result |
|---|---|
| `$task_cargo fmt --all -- --check` | PASS; also checked the auto-mounted new file with `rustfmt --check --edition 2024` |
| `$task_cargo check --locked --workspace --all-targets --all-features --quiet` | PASS on final Rust bytes |
| `$task_cargo clippy --locked -p oteryn-gameplay-combat -p oteryn-gameplay-dispatch -p oteryn-gameplay-encounters -p oteryn-gameplay-spell --all-targets --all-features --quiet -- -D warnings` | PASS for all four changed Rust packages on final Rust bytes |
| `$task_cargo test --locked -p oteryn-gameplay-combat --lib creature_auto_attack -- --nocapture` | 5 passed, 0 failed |
| `$task_cargo test --locked -p oteryn-gameplay-spell --lib spell::actor_conditions:: -- --nocapture` | 24 passed, 0 failed |
| `$task_cargo test --locked -p oteryn-gameplay-encounters --lib smelly_cheese -- --nocapture` | 2 passed, 0 failed |
| `$task_cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib creature_damage_spell -- --nocapture` | 56 passed, 0 failed; 6 pre-existing appearance tests require `OTERYN_FULL_SPELL_TEST_MANIFEST` and remained ignored |
| `$task_cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib native_composite_tests:: -- --nocapture` | 4 passed, 0 failed; includes typed hit/replay, secondary-only fencing and lethal rollback |
| `$task_cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib -- postgres_tests::summer_population:: --nocapture --test-threads=1` | Reported 6 passed, but only 5 are qualified non-PG tests; the NPC PG case returned before connecting. Repeated with the absolute wrapper; still unconfigured. **0 PG passes claimed.** |
| `$task_cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib summer_phosphorus -- --nocapture --test-threads=1` | 2 unit tests passed; new PG case returned before connecting. **0 PG passes claimed.** |
| `$task_cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib -- summer_phosphorus_curse_postgres_real_dispatch --nocapture --test-threads=1` | Repeated with the absolute wrapper: early return, no database configured, no completion marker; **unqualified** |
| `$task_cargo test --locked -p oteryn-game-server --test integration_rest content_ -- --nocapture` | 123 passed, 0 failed |
| `$task_cargo test --locked -p oteryn-game-server --test content_world_project_repository -- --nocapture` | 5 passed, 0 failed |
| `$task_cargo run --locked -p oteryn-game-server -- --smoke` | PASS, exit 0 |
| `$task_cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-refresh .` | BLOCKED: Git index has an unmerged native Item profile path; exit 1, no pin refreshed |
| `$task_cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-check . tmp/tk-events-validation/pin-check` | BLOCKED by the same unmerged index; exit 1 |
| `python3 tools/content-migration/test_event_item_profiles.py` | 7 passed, 0 failed; new precedence regression first failed on item 21178 before the fix |
| `python3 tools/content-migration/test_task_board_item_profiles.py` | 6 passed, 0 failed |
| `python3 tools/content-migration/register_spell_families.py --check` | PASS, 38 registered outputs |
| `python3 tools/content-migration/regenerate_content.py --check` | 13 registered commands passed, then Item-authoring historical fixture failed trying to create a sibling directory outside this writable worktree. The last 3 commands were run separately below: 16/17 registered checks qualified overall, 1 blocked |
| `python3 tools/content-schema/item-authoring/item_weapon_proficiency.py --check` | PASS, up to date |
| `python3 tools/content-census/g4_item_crystal_binding_generator.py --check` | PASS, 33,971 bindings; 9,456,726 bytes |
| `python3 tools/contracts/registry_fragments.py` | PASS, no further diff |
| `python3 tools/agents/validate_governance.py` | PASS, 22 required documents / 9 lanes |
| `python3 -m unittest discover -s tools/agents/tests` | 129 passed, 0 failed |
| `bash .claude/skills/oteryn-dev/scripts/dup-migrations.sh` | PASS, no duplicate numbers; independent working-tree scan also clean |
| `git diff --check` | PASS; zero conflict markers in all 3 conflicted working files |

The initial workspace check failed on three undefined-`kind` sites; the forwarding
repair above cleared them. Initial over-specific actor-condition/encounter filters
matched zero tests; those runs are not coverage and were replaced with the matching
filters in the table. No tests were weakened, disabled or deleted.

### Generation and preservation evidence

- `bash tools/merge-driver/regen.sh --no-stage --skip-checks` was attempted and
  correctly refused the unmerged index. No Git mutation was used to bypass it.
- The same generator's individual `materialize_world()` and `repin_rust_inventory()`
  steps completed, followed by `world_project_v2_to_tree.py` and all six registered
  craft/imbuement/charm/proficiency/reward-claim/starter-kit authoring generators.
  The first materializer attempt failed on an incremental-cache error; the full
  queued-wrapper retry completed successfully.
- `python3 tools/content-schema/native-gameplay/build_full_spell_artifact.py --refresh-loot-tables`
  completed; `register_spell_families.py` regenerated the conflicted providers and
  manifest, and its final `--check` passed. No JSON was hand-merged.
- Compared every generated main Item profile against `38641fecd`: all 2,210 records
  are unchanged, 316 event profiles are added, none removed. The 3 working-file
  conflicts have no remaining markers; their index entries deliberately remain U.
- The full handwritten delta was reviewed against the owner checklist. All
  `PlayerLethalVitals` uses/implementations and both new public spell/qualification
  entrypoints were searched across `apps/`, `crates/`, `tools/`. Production callers
  exist; the new real-dispatch case is auto-mounted into the CI dispatch lib target.
  No new enum variant, struct field, protocol code or migration was introduced by
  this takeover. The existing conservative trait defaults still refuse unsupported
  condition writes. PostgreSQL qualification remains outstanding.

### Unfinished / next step

1. Owner must supply the preserved W19/Cake/Autumn implementation, migrations and
   tests in this directory. Complete the PR's pending House/seed repairs and Cake
   Golem test-only-policy / production-no-write cases against that restored patch.
   Source artifacts and the curse alone do not satisfy this task's acceptance.
2. Restore PostgreSQL provisioning in `/home/mole/oteryn-cache/codex-bin/cargo`.
   Both absolute-wrapper PG attempts reported `OTERYN_TEST_POSTGRES_ADMIN_URL is not
   configured`; the installed wrapper contains no PostgreSQL setup/export. Do not
   count these early returns as PG qualification. Rerun the exact filtered commands
   after provisioning, without exposing the connection string.
3. Owner stages the resolved merge files, then reruns pin refresh and pin check.
   This worker is explicitly forbidden to stage or otherwise mutate Git state.
4. Finish `test_engine_items.py` / the complete content check in an authorized
   environment: its historical replay helper explicitly uses `dir=root.parent`,
   which is read-only here. The helper and its checks were not modified or bypassed.
5. Node boot remains unrun: it requires the exact external Platform checkout and
   Docker/topology prerequisites, outside this worker's directory/Docker authority.
   No frozen-head review, publication, `game-gate` or merge readiness is claimed.

No prior chat's reported validation qualifies this tree. Logs are local under the
ignored `tmp/tk-events-validation/` directory. No Git state-changing command,
credential mutation, workflow/repository-policy edit or external write was used.
