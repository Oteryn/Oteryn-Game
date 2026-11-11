import json
import re
import subprocess
from pathlib import Path

root = Path.cwd()
base = 'crates/gameplay/dispatch/src/gameplay_transport/'
def ref(path, needle):
    lines = Path(path).read_text().splitlines()
    number = next(i for i, line in enumerate(lines, 1) if needle in line)
    return f'`{path}:{number}`'
def dispatch(path, needle):
    return ref(base + path, needle)

rows = [
('mount_taming::mount_taming_postgres_construction_hotkeys_use_production_catalogue', 'Glooth Glider now uses a seven-part construction recipe, rather than the one-item taming path. Keep all four constructions covered: three one-item cases plus existing stock-glider success/replay/refusal scenarios.', dispatch('postgres_tests/mount_taming.rs', 'fn mount_taming_postgres_construction_hotkeys'), 'Static repair; PostgreSQL rerun reserved for CI.'),
('prey::prey_postgres_real_dispatch_draw_select_reroll_expiry_and_replay', 'Dragon now has an admitted loot row. The empty-loot prey scenario must use Hydra, which still has no loot row; the exact selected creature and expiry activity were updated together.', dispatch('postgres_tests/prey.rs', 'assert!(native.reward_table().row("oteryn:creature.hydra")'), 'Static repair; PostgreSQL rerun reserved for CI.'),
('quest_catalogue_chests::quest_catalogue_chests_a_m_unqualified_source_placements_use_replay_and_refusal', 'An untagged positional world callback could claim an explicitly UID/AID-bound reward placement before its reward owner. Reject that fallback on a source-bound placement, retaining explicit native bindings.', dispatch('quest_callbacks.rs', '&& map.facts.quest_callback_source_bound(key)'), 'Static repair; PostgreSQL rerun reserved for CI.'),
('quest_catalogue_chests::quest_catalogue_chests_phase_a_09_use_replay_and_refusal', 'Same positional-callback/source-bound chest ownership collision; exact acceptance, replay, refusal and item assertions remain.', dispatch('quest_callbacks.rs', '&& map.facts.quest_callback_source_bound(key)'), 'Static repair; PostgreSQL rerun reserved for CI.'),
('quest_catalogue_chests::standalone_claims_goblin_inukaya_iron_helmet_katana_lion', 'Same positional-callback/source-bound chest ownership collision. The added assertion context identifies the exact claim/placement without changing expected outcomes.', dispatch('quest_callbacks.rs', '&& map.facts.quest_callback_source_bound(key)'), 'Static repair; prior D standalone pass is historical evidence only.'),
('quest_l7_asymmetric::quest_l7_giant_smithhammer_four_source_states', 'The generic required-state rejection ran before the source asymmetric lever no-op. Check the handled no-op first, preserving both objects and the lever revision.', dispatch('quest_callbacks/execute.rs', 'Crystal giant_smithhammer/lever.lua'), 'Static repair; PostgreSQL rerun reserved for CI.'),
('questpc7_feaster::questpc7_feaster_native_sampling_prefix', 'Three traced issues: direct NPC helper calls reused later wire USE CommandRefs (reservation primary-key collision); bounded sample increments shared one fixed reward claim; twenty separate probe instances plus tools exhausted the main bag. NPC_TALK now uses the real wire sequence; world additive rewards use command-scoped claims with an additional exact same-transaction increment proof; probes use ordinary moves into a real nested backpack. Replay, 20-probe totals and Premium refusal remain exact.', dispatch('postgres_tests/questpc7_feaster.rs', 'NPC grants and world USE') + '; ' + ref('crates/gameplay/durability/src/durability/reward_claim_mint/quest.rs', 'let claim_key = if callback.has_repeating_clock()') + '; `apps/game-server/migrations/2601_world_counter_reward_occurrences.sql:1`', 'New E repair; PostgreSQL qualification required in CI.'),
('questpc7_hydra::questpc7_hydra_requires_personal_native_nest_pickup', 'No deterministic failure established: the previous lane reran the unchanged case alone and recorded one pass. No speculative Hydra patch.', 'No Hydra test/code change justified by the available panic evidence.', 'CI-only per owner contention rule; D log diagnose-questpc7_hydra_requires_personal_native_nest_pickup.log records 1 passed.'),
('questpc8_magic_sword::questpc8_magic_sword_two_and_four_native_demons_both_caches_once', 'Callback-opened/removed doors remained solid in the raw DoorBook used by movement. BundleFacts now excludes only the exact projected door; collision and production movement consumers use that view, while another closed door still blocks.', ref('crates/gameplay/world/src/map/facts/quest_callbacks.rs', 'impl crate::map::floor::CellBlocker for BundleFacts') + '; ' + dispatch('paced_movement.rs', 'BundleDoors(&map.facts)'), 'Static repair; non-PG map tests pass, PG qualification reserved for CI.'),
('questpc8_npc_program_regressions::catalogue_npc_scoped_a_m_starts_are_source_ordered_and_atomic', 'The test assumed Xelvar plain-item grants were unsupported, but the branch composes their atomic owner. It now proves the exact track plus four crystals, exactly-once replay and the additional single-track receipt.', dispatch('postgres_tests/questpc8_npc_program_regressions.rs', 'The composed plain-item owner'), 'Static fixture update with stronger success/replay assertions; current PG qualification reserved for CI.'),
('questpc8_remaining_routes::questpc8_remaining_trader_complete_native_journey', 'Initial command 8 mission rejection: a previous failed confirmation retained its dialogue topic across a new greeting. Greeting now restarts the topic unless a writer is pending. The later command 32 yes rejection remains unresolved: the captured output has no specific fee/MINT refusal. No speculative relaxation or further writer change.', dispatch('npc_command.rs', 'state.quest_dialogue.restart_topic()') + '; ' + dispatch('npc_command/quest_dialogue.rs', 'fn restart_topic'), 'BLOCKER: later deterministic 32:yes refusal not root-caused; exact panic below.'),
('questpc8_smith_cache::questpc8_smith_cache_native_guardian_and_atomic_outputs', 'The fixture and USE targeted the old chest coordinates after consolidation onto the native source cache at (32777,32254,-8). Both fixture geometry and interaction now agree with the source.', dispatch('postgres_tests/questpc8_smith_cache.rs', 'x: 32777') + '; ' + dispatch('postgres_tests/quest_world_callbacks/scenario.rs', '(32777, 32254, -8)'), 'Static repair; PostgreSQL rerun reserved for CI.'),
('questpc8_winterlight::questpc8_winterlight_native_disguise_four_fires_queen_and_base_outfit', 'Guardian activation and lethal eligibility read raw stored tracks, omitting declared initial zero counters. Both now read the catalogue-resolved bounded view. E completed the sibling sweep for native objective phases and disguise and removed duplicate guardian refresh work.', dispatch('grizzly_kills/guardians.rs', 'copy.read_tracks') + '; ' + dispatch('grizzly_kills.rs', 'let tracks = copy.read_tracks') + '; ' + dispatch('quest_kill_objectives.rs', 'let tracks = copy.read_tracks') + '; ' + dispatch('quest_disguise.rs', 'copy.read_tracks'), 'Static repair; PostgreSQL qualification required in CI.'),
('rotten_blood_progression::qengine_l13_decaying_defender_parent_and_accessories', 'Bakragore now admits its empty physical corpse while its unavailable private loot still refuses admission. Keep the loot refusal, require the exact physical corpse and assert one body with zero children.', dispatch('postgres_tests/rotten_blood_progression.rs', 'let empty_bakragore'), 'Static fixture correction; current PostgreSQL qualification reserved for CI.'),
('rotten_blood_progression::rotten_blood_postgres_native_chain_final_reward_replay_and_cleanse', 'Same shared Bakragore qualification helper. Other unqualified Rotten Blood bodies still require their original refusal; durable progression/replay assertions remain.', dispatch('postgres_tests/rotten_blood_progression.rs', 'let empty_bakragore'), 'Static fixture correction; current PostgreSQL qualification reserved for CI.'),
('s17_client_watch::s17b_native_observer_map_creatures_effects_and_public_speech_dispatch', 'The Stop rejection was not reproduced by the previous isolated rerun, which passed without a cast-path change. The branch does not change cast.rs or the social cast owner; no guessed change to authority or revision checks.', 'No cast-path repair justified by the available evidence.', 'CI-only per owner contention rule; D log diagnose-s17b_native_observer_map_creatures_effects_and_public_speech_dispatch.log records 1 passed.'),
]

lines = [
'# FIX3042E result', '',
'PR #3042; branch `claude/happy-davinci-ijz3wy-wpkg2`; base worktree HEAD `7de810ae4`. AUTHORING only: all existing work was preserved; no commit, add, push, reset, checkout, stash, workflow/policy edit, credential access, dependency download, Docker action or PostgreSQL test was performed.', '',
'## Unfinished / hard blocker', '',
'BLOCKER: `questpc8_remaining_trader_complete_native_journey` still has an unresolved later deterministic rejection in the inherited log. The available output does not identify the rejecting fee, MINT or actor/quest predicate; PostgreSQL execution is explicitly prohibited on this PC. A fresh CI run with the final candidate is needed before claiming all sixteen failures resolved. No test was weakened to turn this into a pass.', '',
'Verbatim captured assertion (`.fix3042d/after-questpc8_remaining_trader_complete_native_journey.log`):', '',
'```text', 'assertion `left == right` failed: 32:yes; None', '  left: Rejected', ' right: Ok', '```', '',
'All PostgreSQL results on this exact uncommitted tree remain **not run**, by the owner rule. Historical D passes are explicitly not current-tree qualification. The SQL migration is likewise not executed locally.', '',
'## Per-test root cause and fix', '',
'All test filters below have the prefix `gameplay_transport::monk_save::postgres_tests::` and belong to `oteryn-gameplay-dispatch`. For CI, use each exact listed filter with `cargo test --locked -p oteryn-gameplay-dispatch --lib <prefix><filter>`; do not run them on this PC.', '',
'| Exact filter suffix | Root cause / treatment | Fix location | Qualification |', '| --- | --- | --- | --- |',
]
for name, cause, location, result in rows:
    lines.append(f'| `{name}` | {cause} | {location} | {result} |')
lines += ['', '## Node boot', '',
'The full-manifest seam failure matches the existing main failure documented in [PR #3057](https://github.com/Oteryn/Oteryn-Game/pull/3057): the disposable database lacks an admitted World epoch, so Cyclopedia/daily-boost warm-up cannot make admissions ready. Its body explicitly names `corpse_decay_read_deferred` and `server_save_countdown_read_deferred`. Read-only inspection of PR head `a96d1f944e14f7e5c7fe4392d234bcf4dd8a8076` corroborates that diagnosis. `git diff origin/main...HEAD -- tools/qualification/node_boot crates/gameplay/dispatch/src/gameplay_transport/composed_admission/admission.rs crates/gameplay/dispatch/src/gameplay_transport/cyclopedia/respawn.rs crates/gameplay/dispatch/src/gameplay_transport/daily_boost.rs` is empty. The branch\'s `boot_bundle_items` change extracts the same item-definition lookup without changing its behavior. This is evidence for the existing harness issue, not a claim of a successful node-boot run. The separate MAINRED-NB work owns its fix; no duplicate harness patch was made here. Related non-PG game-server catalogue/boot tests were run.', '',
'## Checks and exact local filters', '',
'All commands used `/home/mole/oteryn-cache/codex-bin/cargo`; PostgreSQL-capable tests were excluded by explicit non-PG module filters. `--locked` was supplied for checks, clippy and tests. Evidence logs are under `.fix3042d/` in this worktree.', '',
'| Command | Exit code | Result / evidence |', '| --- | --- | --- |',
]
checks = json.loads(Path('.fix3042d/fix3042e-tests.json').read_text()) + json.loads(Path('.fix3042d/fix3042e-final-checks.json').read_text())
for item in checks:
    log = Path(item['log']).read_text()
    matches = re.findall(r'test result:.*', log)
    summary = matches[-1] if matches else item['log']
    command = ' '.join(item['command']).replace('/home/mole/oteryn-cache/codex-bin/cargo', 'cargo')
    lines.append(f"| `{command}` | {item['exit_code']} | {summary} |")
lines += ['', 'The map filter reports one pre-existing ignored test; this task added no ignore or skip. Initial fmt/workspace-check/requested-clippy runs also exited 0 before the final edits; the final reruns above supersede them. No content/import/pinned input was changed in the inherited uncommitted delta or this continuation, so regen.sh and pin-refresh/pin-check were not selected.', '',
'## Self-review', '',
'Read the complete uncommitted code/migration diff against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. Checked the relevant branch-vs-main slices behind the reported failures. New additive world rewards retain exact command identity, same-transaction quest receipt/increment proof, original bounded storage constraints and unchanged claim/MINT guards. No production trust, session-generation fence, authorization rule or test acceptance was loosened. Source-bound callback selection has a production caller in `quest_callbacks::matches` and dispatch PostgreSQL coverage in the three catalogue chest filters above. All `BundleDoors` constructor and `CellBlocker` implementation sites in `apps/`, `crates/`, `tools/` were searched: production composed paths use BundleFacts; native door-displacement and explicit DoorBook test consumers retain their isolated door view. No enum variant, named struct field or trait method was added. Temporary D diagnostic edits were removed while the existing baseline test diagnostic was retained. Independent exact-candidate review and runtime PostgreSQL/node-boot qualification belong to the coordinator/CI before publication.', '',
'## Changed files', '',
'The full preserved uncommitted result contains these source/migration paths. `.fix3042d/` additionally contains inherited scratch data and this task\'s local evidence/scripts.', '',
]
paths = subprocess.check_output(['git', 'diff', '--name-only'], text=True).splitlines()
paths += ['apps/game-server/migrations/2601_world_counter_reward_occurrences.sql']
for path in sorted(set(paths)):
    lines.append(f'- `{path}`')
lines += ['', 'Unfinished: the Trader command-32 root cause and fresh CI qualification of all PostgreSQL cases and node boot. No completion/mergeability claim is made.', '']
output = Path('/home/mole/oteryn-cache/target-codex/FIX3042E-RESULT.md')
output.write_text('\n'.join(lines))
print(output)
print(f'{len(set(paths))} changed source/migration files; {len(checks)} recorded checks')
