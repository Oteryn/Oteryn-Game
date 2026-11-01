# OTV2-20261031-monsterai

```yaml
task_id: OTV2-20261031-monsterai
title: Data-driven monster targeting, fleeing and attacks
type: mechanic
mode: IMPLEMENT
status: blocked
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/monsterai
pr: 2766
base_sha: 38641fecd4558a4a14e6b15b3874110ffa2dff19
head_sha: null
final_head_sha: null
owner: monsterai-task-owner
created_at: 2026-10-31T11:11:44Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/monster-ai/
  - crates/gameplay/combat/
  - crates/gameplay/spell/
  - crates/gameplay/dispatch/src/gameplay_transport/
  - crates/gameplay/dispatch/src/lifted_tests/
  - crates/gameplay/dispatch/src/lib.rs
  - crates/gameplay/domain/src/domain/melee_definition.rs
  - crates/gameplay/entities/src/carrier/
  - crates/gameplay/entities/src/carrier.rs
  - crates/gameplay/durability/src/durability/creature_source_items.rs
  - crates/gameplay/durability/src/durability/native_world_read.rs
  - crates/gameplay/content/src/content/native_gameplay.rs
  - crates/gameplay/content/src/content/native_gameplay/test_support/activation_fixture.rs
  - tools/content-migration/bind_monster_source_definitions.py
  - tools/content-migration/register_spell_families.py
  - tools/content-migration/test_bind_monster_source_definitions.py
  - content/
  - docs/agents/tasks/OTV2-20261031-monsterai.md
  - docs/agents/evidence/OTV2-20261031-monsterai/
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

## Takeover checkpoint (2026-11-01)

The local merge of `38641fecd4558a4a14e6b15b3874110ffa2dff19` into
`8a0d937779bbc10a45f9cbffd8364780b1159442` has resolved file contents and a clean
`git diff --check`. The Git index is untouched, so its nine unmerged entries
remain for the owner to stage. No commit, push, reset, checkout, stash, external
write, workflow edit, migration edit, credential access or Docker action occurred.

**BLOCKER: PostgreSQL qualification.** Every database case returned early even
through `/home/mole/oteryn-cache/codex-bin/cargo`. The installed wrapper has no
PostgreSQL environment assignment and the cases report an unset
`OTERYN_TEST_POSTGRES_ADMIN_URL`. The owner was asked to have the coordinator
restore that injection outside this worktree; no database URL was requested or
printed. A second absolute-wrapper Rat-only probe also returned early. These
harness successes are not database passes. The task remains active and is not
archived while that required qualification is missing.

The next step is to rerun the three exact filtered PostgreSQL commands below,
require seven `MONSTER_AI POSTGRES_CASE_COMPLETED` markers for `monster_attacks::`
and no configuration skips, and repair any actual failures. The future committed
candidate still needs its own CI/game-gate, independent review and the
coordinator-owned node-boot result (owner answer 6547138645). No frozen SHA or
mergeability is claimed for this uncommitted worktree.

## Outcome

W01a/W01c ordinary admitted monsters use their source targeting, flee threshold,
shared attack clock, all melee indices and existing current combat/condition/Item
owners. Rat 0–8 at 2000 ms, Hunter distance, Fire Devil area, Fire Elemental and
Firestarter field creation and The Monster wall removal reach actual dispatch and PostgreSQL read-back.

Deliver one draft PR. The user explicitly excludes review requests, heavy-tests,
merge and deployment. No new wire capability, command, migration or mechanic ID.

## Architecture and source of truth

- PROVEN primary donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
- PROVEN secondary: Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`.
- Evidence class: OTS_HYPOTHESIS_ONLY; the task explicitly chooses Crystal parity.
- PROVEN complete native projection: 1870 records, including 1863 canonical records
  with exact attack-definition closures and seven preserved legacy identities.
- PROVEN attack census: 1689 melee, 4295 ordinary spell/Item/composed entries,
  and 18 explicit Encounter refusals. Of the ordinary entries, 16 are composed
  and 82 are source Item operations (62 creation, 20 removal).
- Existing Channel position, damage ledger, spell/timer, condition, character
  vitals and durable Item owners remain authoritative; no parallel effect engine.
- The one-field shared position extension is explicitly authorized by PR answer
  6543529998. It reuses existing position revisions and preserves receipt Debug
  bytes. Actual Slot is 184 bytes within the unchanged existing budget.

## High-risk authority/recovery qualification

Applicable: current-owner HP, conditions, timers and durable source Item writes.
Current authority comes from runtime owner/work stamps, exact actor generations,
live player session/control, independently read Map/Item facts and existing
database scope/character fencing. Prepared source proof defines expected identity
and content; it never substitutes for current authority.

Negative cases cover stale owner/actor/session, content/source substitution,
target changes, wrong floor/context, protection, superseded occurrence, cooldown,
clock/refusal and replay. Selected native melee binds exact source index and
per-creature sequence; normal N commits before movement N+1. Timer readers are
specific to each source/pulse/session and validate the shared transaction after
their final awaited read. A lost transaction preserves pending timer owners.

The source Item transaction uses its existing occurrence receipt and writes item,
ground placement and audit rows atomically. Its source reference is the existing
validated native occurrence, within the schema's 1..128-byte bound. No merged
migration, SQL guard or permission is weakened.

## Acceptance criteria

- [x] Source-backed fleeing and targeting consume current player/creature facts.
- [x] All native melee slots, source intervals and extra/1500 ms semantics execute.
- [x] Distance/area attacks and existing owned conditions use current combat owners.
- [x] Full native projection and melee census execute against actual source data.
- [ ] Seven canonical dispatch/PostgreSQL cases qualify on the takeover tree, including Rat movement, invisible area recipients and removal.
- [ ] Final filtered unit, summon/Map PostgreSQL, content/pin and repository qualification completes on the takeover tree.

## Reference parity

| Behaviour | Reference value/source | Oteryn | Test |
|---|---|---|---|
| Fleeing | Crystal monster.cpp:52–54,1001–1003,1204–1205; admitted 1x, melee-only suppression | monster-ai/summon_plan, profile_schedule | native_fleeing_keeps_ranged_spells_and_returns_to_fighting_after_heal |
| Targeting | Crystal monster.cpp:771–789,853–895,1295–1349; current HP/session damage and source strategy rules | targeting, existing damage_ledger, monster_ai_cycle | native_percentile_strategy_retains_crystal_remainder_and_damage_quirk; live-health/current-damage tests |
| Melee | monster.cpp:1188–1292; shared clock, all indices, last-extra 1500 ms | profile_schedule, ai_monster_melee, creature_bite | native_melee_clock; full 1689-entry census; Rat PostgreSQL |
| Movement retry | creature.cpp:488–495; game.cpp:6996–7000; onAttacking(0) | existing position stamp and movement_attack | issuer/target/out-back/turn/foreign-context cases |
| Source spells | combat.cpp:1666–1743,2337–2375; source geometry, elements and current protections | creature_damage_spell and monster_profile_cycle | 67 focused current-owner/condition/composed/geometry cases |
| Owned conditions | combat.cpp:944–946,1077–1080 | existing atomic player damage/condition and appearance owners | current_source_conditions_share_atomic_damage_and_replay_then_refuse_retirement; appearance execution suite |
| Source ground | monster.cpp:1288–1292; source range zero is uncapped | existing creature_source_items durable writer | Fire Elemental, Firestarter range-zero and The Monster removal PostgreSQL |
| Damage distribution | Crystal combat.cpp:211–219, monster.cpp:2457–2464, tools.cpp:442–450; explicit Q7A difference | Existing uniform_draw project policy retained | Source bounds and replay tests; no truncated-normal parity claim |
| Encounter boundary | exact eighteen source callbacks, answer 6543276153 | explicit unsupported-shape refusal | monster_actual_encounter_attacks_remain_eighteen_explicit_refusals |

Full donor links, per-rule names and the exact eighteen-entry table are in
[implementation evidence](../evidence/OTV2-20261031-monsterai/README.md).
Crystal's first-candidate exclusion in MostDamage differs from the pinned Canary
implementation and is deliberately retained.

## Excluded scope

All seven PR questions have explicit ANSWER A decisions:

- Configurable monster/boss health rates: W14a; keep admitted 1x.
- Arbitrary type/CreatureEvent Think scripts: typed Interaction/Encounter admission,
  retaining the two distinct override/ordering contracts.
- Eighteen Encounter-bound source attack entries: typed Encounter admission and
  composition, individually listed with donor lines and pinned by a refusal test.
- Automatic ordinary Defence effects: separate production integration. Wild
  monster self-healing/haste/invisibility are not newly playable.
- Translation observation: included through the authorized existing owner field.
- SPELL-PRESENT-0: separate cross-client activation. No monster producer into an
  undrained queue, capability, protocol envelope or client FX claim.

- Damage sampling: Q7A (6544219482) retains the existing uniform project policy.
  Crystal truncated-normal parity remains a separate shared combat-randomness
  mechanic (combat.cpp:211–219, monster.cpp:2457–2464, tools.cpp:442–450).
  Source bounds, schedules and replay remain required; no distribution parity claim.

W01b respawn is outside scope. Existing bounded recipient/timer/replay policies
remain. Ambiguous top-occupant callbacks require an existing arrival-order proof;
composed creature secondary effects without an atomic owner receipt refuse before
writes. Cardinal chain traces are a documented project presentation projection.

## Implementation / findings

The complete closure generator binds canonical Ability/Effect/Formula dependencies
after accepted creature overlays and refuses duplicate/missing/oversized closures.
The creature-profile input limit is a separate measured 32 MiB bound; other input
limits remain unchanged. Generated hashes/pins are produced by repository tooling.

Material findings repaired: missing canonical attack bodies, losing later source
slots, native duplicate summon/defence-melee projection, synthetic movement test
contexts, zero-range field refusal, empty source-ground reference, missing Drown
protection mapping and current Map invalidation after timer-bank preparation.
The actual removal case also exposed known static Source/Bundle walls being
misclassified as missing dynamic projectile metadata; the existing creature
reader now retains those qualified flags, matching the player reader. Synthetic
EQMOD and summon fixtures now update their exact copied Creature/Behavior roots
when applying an explicit test overlay; production validation is unchanged.
No new dead-code suppression, model identifier, secret or audit-ledger edit.

## Validation

These results describe the uncommitted takeover tree, not the old HEAD alone.
Historical implementation results remain in the evidence README. All Cargo
commands below used the absolute queued wrapper, locked dependencies and offline
operation. Actual-manifest tests set `OTERYN_FULL_SPELL_TEST_MANIFEST` to this
worktree's `content/spells.manifest.json`. Default opt-in Content, melee census,
Encounter refusal and six appearance checks were also run explicitly with
`--ignored`; the existing RL-17 host measurement remains outside this task.

| Command | Result |
|---|---|
| `/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check` | PASS, exit 0 |
| `/home/mole/oteryn-cache/codex-bin/cargo check --locked --workspace --all-targets --all-features` | PASS, exit 0 |
| `/home/mole/oteryn-cache/codex-bin/cargo clippy --locked -p oteryn-game-server -p oteryn-gameplay-combat -p oteryn-gameplay-content -p oteryn-gameplay-dispatch -p oteryn-gameplay-domain -p oteryn-gameplay-durability -p oteryn-gameplay-entities -p oteryn-gameplay-monster-ai -p oteryn-gameplay-spell --all-targets --all-features -- -D warnings` | PASS, exit 0 |
| `python3 tools/content-migration/test_bind_monster_source_definitions.py` | PASS, exit 0 |
| `python3 tools/content-migration/regenerate_content.py --check` | PASS: 5 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo run --locked -p oteryn-architecture-check -- workspace .` | PASS, exit 0 |
| `/home/mole/oteryn-cache/codex-bin/cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-refresh .` | PASS, exit 0 |
| `/home/mole/oteryn-cache/codex-bin/cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-check . /home/mole/oteryn-cache/rs-wt/tk-monsterai/tmp/monsterai-takeover/pin-check` | PASS, exit 0 |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-domain --all-features --lib native_zero_melee -- --nocapture --test-threads=1` | PASS: 1 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-entities --all-features --lib translation -- --nocapture --test-threads=1` | PASS: 5 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-entities --all-features --lib current_session -- --nocapture --test-threads=1` | PASS: 4 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-entities --all-features --lib activity_transfer -- --nocapture --test-threads=1` | PASS: 4 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-entities --all-features --lib echo_warden:: -- --nocapture --test-threads=1` | PASS: 8 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-monster-ai --all-features --lib ai_think:: -- --nocapture --test-threads=1` | PASS: 86 passed, 0 failed, 1 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-monster-ai --all-features --lib ai_monster_melee:: -- --nocapture --test-threads=1` | PASS: 22 passed, 0 failed, 1 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-monster-ai --all-features --lib actual_full_admitted_melee_sources_resolve_every_authored_index -- --nocapture --test-threads=1 --ignored` | PASS: 1 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-combat --all-features --lib creature_ -- --nocapture --test-threads=1` | PASS: 9 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-spell --all-features --lib spell::actor_conditions:: -- --nocapture --test-threads=1` | PASS: 24 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-durability --all-features --lib creature_source_items:: -- --nocapture --test-threads=1` | PASS: 2 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-content --all-features --lib native_gameplay:: -- --nocapture --test-threads=1` | PASS: 58 passed, 0 failed, 1 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-content --all-features --lib actual_full_manifest_qualifies_source_world_and_all_owner_profiles -- --nocapture --test-threads=1 --ignored` | PASS: 1 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib native_composite_tests:: -- --nocapture --test-threads=1` | PASS: 6 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib creature_damage_spell -- --nocapture --test-threads=1` | PASS: 70 passed, 0 failed, 6 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib monster_ai_cycle:: -- --nocapture --test-threads=1` | PASS: 13 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib boss_reward_contribution:: -- --nocapture --test-threads=1` | PASS: 8 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib creature_damage_spell_appearance_execution_tests -- --nocapture --test-threads=1 --ignored` | PASS: 6 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib monster_actual_encounter_attacks_remain_eighteen_explicit_refusals -- --nocapture --test-threads=1 --ignored` | PASS: 1 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib monster_attacks:: -- --nocapture --test-threads=1` | NOT QUALIFIED: 0 database cases executed, 7 returned early (harness exit 0) |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib equipment_mods:: -- --nocapture --test-threads=1` | NOT QUALIFIED: 1 Content-only case passed, 1 database case returned early (harness exit 0) |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib postgres_tests::loop_monster:: -- --nocapture --test-threads=1` | NOT QUALIFIED: 0 database cases executed, 4 returned early (harness exit 0) |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --all-features --test ability_engine '' -- --nocapture --test-threads=1` | PASS: 95 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --all-features --test integration_rest content_ -- --nocapture --test-threads=1` | PASS: 123 passed, 0 failed, 0 ignored |
| `/home/mole/oteryn-cache/codex-bin/cargo run --locked -p oteryn-game-server -- --smoke` | PASS, exit 0 |

- `python3 tools/content-migration/regenerate_content.py --skip-checks`: PASS,
  non-staging regeneration of the full World/content/authoring chain. The
  registered `--check` and separate pin refresh/check passed afterward.
- `bash tools/merge-driver/regen.sh --check`: invocation refused (exit 2), because
  the wrapper always adds `--resolve`. The direct generator route above avoided
  its prohibited Git checkout/add operations and ran every registered check.
- The first direct freshness check hit a read-only sibling temporary directory in
  the historical Item checker. A local `tempfile.TemporaryDirectory` routing shim
  under `tmp/monsterai-takeover/temp-routing` redirected that one outside parent
  to this worktree's scratch directory. No checker, source bytes or assertions
  changed. The full freshness rerun passed, including 599 Item engine checks,
  the 33,971-entry Crystal binding check and five Rust World inventory tests.
- `python3 tools/content-migration/test_bind_monster_source_definitions.py`: PASS,
  six tests. Regenerated providers retain 1,870 Creature and 2,210 Item records,
  with matching manifest digests. World pin refresh retained digest
  `8bf53203d1f8159695f2872937135578903c4ad89067eb19344515de1c904e6c`.
- The typed composite regression was proved RED by injecting lost element
  forwarding (0 passed, 1 failed: 101 damage instead of 99), then GREEN with the
  original production bytes restored (1 passed, 0 failed). Its explicit fixture
  has a currently promoted Premium level-500 player and 1.01% Death resistance;
  damage and source conditions share one revision, replay is identical, and a
  retired owner refuses. An initially ineligible Wheel fixture was corrected;
  the expected damage assertion stayed 99.
- The native Content run first reported 57 passed, 1 failed, 1 ignored because
  main's reward-boss fixture changed the Rat root without refreshing its copied
  attack root. The existing regression passed after the fixture repair
  (1 passed, 0 failed), then the full selected Content run passed 58/0/1.
- `bash .claude/skills/oteryn-dev/scripts/dup-migrations.sh`: PASS, no duplicates.
- `bash .claude/skills/oteryn-dev/scripts/dup-migrations.sh 38641fecd`: PASS,
  no duplicates. A separate working-tree prefix census also found none.
- Migration renumbering: **none needed; this branch adds no migrations**.
- `python tools/agents/validate_governance.py`: PASS, 22 required policy documents and nine project lanes.
- `python -m unittest discover -s tools/agents/tests`: PASS, 129 tests, zero failures.
- Final `git diff --check`: PASS; no conflict markers remain in the nine resolved working files.

## Self-review

The takeover delta was read in full against both the repository checklist and
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. Workspace searches covered added
fields/types, constructors, trait implementations and production callers across
`apps/`, `crates/` and `tools/`. The activity-transfer position constructor now
initializes the translation marker. Main's typed damage, Channel/Instance scope,
boss participation and Echo Warden flee scaling survive alongside the branch's
current-owner and native-summon behavior; movement retries use the same scaled
threshold as full thinks. Main's reward-boss classification fixture now refreshes
its exact copied roots using the same helper as the summon fixtures. Production
binding validation remains strict.

An unused heal accessor was removed, and the timer/fingerprint convenience APIs
that have only test callers are confined to test/test-support builds. The new
current-owner typed bridge has a production spell caller and the real spell-owner
regression above. No assertion or test was removed or weakened. No remaining
self-review finding is claimed; PostgreSQL behavior remains unqualified.
Independent frozen-head review was not requested by this worker. The public PR
review and inline-comment reads were empty; the earlier standalone Ability
fixture report is covered by the 95 passing `ability_engine` tests.

## PR and closeout

PR: https://github.com/Oteryn/Oteryn-Game/pull/2766. The owner commits after review.
The task has not reached FREEZE_SHA, CI qualification, merge or archive. No new
number reservations were consumed. Node boot remains coordinator-owned and
unqualified under answer 6547138645; the earlier excluded scopes remain unchanged.

## Takeover changed files

This is the delta from the prepared merge's index, including resolved unmerged
files. Incoming main changes already staged by the owner are preserved. Other
regenerated outputs matched their prepared merged bytes; the serving manifest
conflict was regenerated rather than hand-edited.

- `content/spells.manifest.json`
- `crates/gameplay/combat/src/ability/player_lethal.rs`
- `crates/gameplay/combat/src/creature_chain_attack.rs`
- `crates/gameplay/content/src/content/native_gameplay/test_support/activation_fixture.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/actor_conditions.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/actor_spell.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/monster_ai_cycle.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/monster_profile_cycle/physical_spell.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/monster_profile_cycle/physical_spell/composed.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/monster_profile_cycle/physical_spell/timers.rs`
- `crates/gameplay/dispatch/src/lib.rs`
- `crates/gameplay/dispatch/src/lifted_tests/creature_damage_spell_appearance_execution_tests.rs`
- `crates/gameplay/dispatch/src/lifted_tests/creature_damage_spell_tests.rs`
- `crates/gameplay/entities/src/carrier/activity_transfer.rs`
- `crates/gameplay/monster-ai/src/ai_think/movement_attack.rs`
- `crates/gameplay/monster-ai/src/ai_think/summon_plan.rs`
- `crates/gameplay/spell/src/creature_damage_spell/composed.rs`
- `crates/gameplay/spell/src/creature_damage_spell/mod.rs`
- `docs/agents/tasks/OTV2-20261031-monsterai.md`
