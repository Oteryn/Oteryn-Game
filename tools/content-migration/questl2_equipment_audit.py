"""Pin L2's native Item identity union against the immutable collector baseline."""
import hashlib
import json
from pathlib import Path
import subprocess
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
BASE = '78294c081a099c96a57948573d5dbf0caf398abf'
PROFILE = 'content/items/definitions/spell-native-profiles.json'
PROOF = 'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json'
DEST = 'tools/content-schema/native-gameplay/questl2-equipment-cohort.json'


def main():
    baseline = subprocess.check_output(['git', 'show', BASE + ':' + PROFILE], cwd=ROOT)
    old = {r['production_definition']['production_key'] for r in json.loads(baseline)['records']}
    current = json.loads((ROOT / PROFILE).read_bytes())['records']
    keys = [r['production_definition']['production_key'] for r in current]
    if len(keys) != len(set(keys)) or not old <= set(keys):
        raise ValueError('L2_EQUIPMENT_LOST_OR_DUPLICATE_SOURCE_KEY')
    old_proof = json.loads(subprocess.check_output(['git', 'show', BASE + ':' + PROOF], cwd=ROOT))
    test_path = 'crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs'
    test = subprocess.check_output(['git', 'show', BASE + ':' + test_path], cwd=ROOT, text=True)
    initial = int(re.search(r'const ORIGINAL_PROFILES: usize = ([0-9_]+);', test)[1].replace('_', ''))
    table = test.split('const PROFILE_ADDITIONS:', 1)[1].split('\n];', 1)[0]
    declared = initial + sum(int(n.replace('_', '')) for n in re.findall(r'\("[^"\n]+", ([0-9_]+)\)', table)) + len(old_proof['records'])
    if declared < len(old):
        raise ValueError('L2_EQUIPMENT_BASELINE_COHORT_UNDERCOUNT')
    packet = {'baseline_commit': BASE, 'baseline_sha256': hashlib.sha256(baseline).hexdigest(),
              'baseline_count': len(old), 'baseline_declared_count': declared,
              'baseline_cohort_overlap': declared - len(old), 'baseline_qengine_count': len(old_proof['records']),
              'additional_keys': sorted(set(keys) - old),
              'identity_sha256': hashlib.sha256(('\n'.join(sorted(keys)) + '\n').encode()).hexdigest()}
    raw = json.dumps(packet, indent=2) + '\n'
    if '--check' in sys.argv:
        if (ROOT / DEST).read_text() != raw:
            raise ValueError('L2_EQUIPMENT_UNION_PIN_STALE')
    else:
        (ROOT / DEST).write_text(raw)
    print('L2_EQUIPMENT_UNION_GENERATED', len(old), len(packet['additional_keys']), len(keys))


if __name__ == '__main__':
    main()
