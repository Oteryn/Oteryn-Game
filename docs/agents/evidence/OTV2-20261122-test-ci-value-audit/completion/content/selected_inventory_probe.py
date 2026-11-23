#!/usr/bin/env python3
"""Execute repaired modules through the real current inventory and subprocess pool."""
import json
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[6]
sys.path.insert(0, str(ROOT / 'tools/content-migration'))
import test_world_project_v2_to_tree as inventory

SELECTED = {
    'test_boss_corpse_terminal_reference', 'test_cake_bridge_reference',
    'test_coin_converter_profiles', 'test_creature_level_rating',
    'test_forge_sql_admission', 'test_forge_sql_admission_forward',
    'test_harvest_item_profiles', 'test_itemhold_q', 'test_monster_reference_level',
    'test_official_creature_balance', 'test_qengine_reward_item_profiles',
    'test_questpc4_treasure_reward_profiles',
    'test_quick_loot_item_profiles',
}
checks = [check for check in inventory.check_inventory(workflow_checks=True) if check.name in SELECTED]
assert len(checks) == len(SELECTED) and {check.name for check in checks} == SELECTED
report = {'selection_contract': 'Exact repaired module identities each occur once in actual workflow inventory. Commands are executed by parallel_checks.run_checks without substitution.',
          'inventory': [{'name': c.name, 'command': list(c.command), 'cwd': str(c.cwd)} for c in checks]}
code = 'import sys; sys.path.insert(0, "tools/content-migration"); import parallel_checks, test_world_project_v2_to_tree as i; selected=' + repr(SELECTED) + '; c=[x for x in i.check_inventory(workflow_checks=True) if x.name in selected]; assert len(c)==len(selected) and {x.name for x in c}==selected; parallel_checks.run_checks(c,jobs=2)'
for name, command, cwd in (
    ('selected_inventory', [sys.executable, '-c', code], ROOT),
    ('runner_regressions', [sys.executable, '-m', 'unittest', '-v', 'test_parallel_checks'], ROOT / 'tools/content-migration'),
):
    start = time.monotonic()
    result = subprocess.run(command, cwd=cwd, capture_output=True, text=True)
    report[name] = {'command': command, 'cwd': str(cwd), 'exit_code': result.returncode, 'seconds': round(time.monotonic()-start, 3), 'stdout': result.stdout, 'stderr': result.stderr}
    print(name, result.returncode, flush=True)
report['pass'] = all(report[n]['exit_code']==0 for n in ('selected_inventory','runner_regressions'))
Path(__file__).with_name('selected_inventory_final.json').write_text(json.dumps(report, indent=2)+'\n')
raise SystemExit(0 if report['pass'] else 1)
