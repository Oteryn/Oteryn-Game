"""Record only the final cohort's actually completed PG source-claim proofs."""
import ast
import json
import re
from pathlib import Path

log = Path('.questpc2/phase-a-pg-final.log').read_text()
assert 'test result: ok. 5 passed; 0 failed;' in log
assert 'not configured' not in log
capture = json.loads(Path('tools/content-schema/world-authoring/samples/quest-chest-bindings/questpc2-phase-a.json').read_text())
keys = {r['claim'] for r in capture['bindings']}
completed = set(re.findall(r'POSTGRES_CASE_COMPLETED questpc2 (oteryn:reward-claim\S+) elapsed_ms=[1-9][0-9]*', log))
assert keys == completed, (keys - completed, completed - keys)
p = Path('tools/content-schema/quest-authoring/quest_callback_coverage.py')
s = p.read_text()
node = next(n for n in ast.parse(s).body if isinstance(n, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'QUALIFIED' for t in n.targets))
qualified = ast.literal_eval(node.value)
for key in keys:
    if key.endswith('parchment_room.bag'):
        test = 'key_container'
    elif key.endswith('powerbolt2'):
        test = 'book'
    elif key.rsplit('.', 1)[-1] in ('orcshaman', 'panpipe', 'poisondaggers', 'powerbolt1'):
        test = 'container'
    else:
        test = 'leaf'
    qualified[key] = 'questpc2_rewards_phase_a_' + test + '_cohort'
lines = s.splitlines(keepends=True)
lines[node.lineno - 1:node.end_lineno] = ['QUALIFIED = {\n'] + [f'    "{key}": "{qualified[key]}",\n' for key in sorted(qualified)] + ['}\n']
s = ''.join(lines)
needle = '            if native and any(p[\'reward\'].get(\'written_text\') for p in definition[\'placements\']):'
if 'Target reward-count parity remains open' not in s:
    assert needle in s
    s = s.replace(needle, '''            if key == "oteryn:reward-claim.kv.poisondaggers":
                inventory["qualification_limit"] = (
                    "PG qualifies the existing canonical/source recipe: one Poison Dagger. "
                    "The current Wiki lists two. Target reward-count parity remains open."
                )
''' + needle)
p.write_text(s)
print(f'Qualified {len(keys)} exact source claims; no full-quest completion asserted')
