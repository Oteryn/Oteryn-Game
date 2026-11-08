"""Pin L2's native Item identity union against the immutable collector baseline."""
import hashlib
import json
from pathlib import Path
import subprocess
import re
import sys
from questl2_equipment_inputs import keys as composed_keys

ROOT = Path(__file__).resolve().parents[2]
BASE = '78294c081a099c96a57948573d5dbf0caf398abf'
L2_SOURCE = 'dbe5060463120ea78ba3edf2fb79d43c5b157085'
MAIN_SOURCE = 'c77d436d8be39f348924e66d97928ffd69e61a8c'
PROFILE = 'content/items/definitions/spell-native-profiles.json'
PROOF = 'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json'
DEST = 'tools/content-schema/native-gameplay/questl2-equipment-cohort.json'



def reviewed_main_overlap(engine_keys):
    read = lambda path: subprocess.check_output(['git', 'show', MAIN_SOURCE + ':' + path], cwd=ROOT)
    prior_engine = {row['production_definition']['production_key'] for row in json.loads(read(PROOF))['records']}
    return (engine_keys & composed_keys(read)) - prior_engine


def checked_source_union(published, source_keys, proof_keys, actual):
    if source_keys != proof_keys:
        raise ValueError('L2_EQUIPMENT_UNREVIEWED_SOURCE_PROOF')
    expected = published | source_keys
    if actual != expected:
        raise ValueError('L2_EQUIPMENT_REVIEWED_SOURCE_UNION_DRIFT')
    return expected


def main():
    baseline = subprocess.check_output(['git', 'show', BASE + ':' + PROFILE], cwd=ROOT)
    old = composed_keys(lambda path: subprocess.check_output(['git', 'show', BASE + ':' + path], cwd=ROOT))
    keys = composed_keys(lambda path: (ROOT / path).read_bytes())
    snapshot = lambda ref: composed_keys(lambda path: subprocess.check_output(['git', 'show', ref + ':' + path], cwd=ROOT))
    # New source-backed quest Items are admitted only through the pinned producer;
    # qengine_equipment_audit has independently checked exact IDs and XML pins.
    import qengine_reward_item_profiles as source
    source_keys = {f'oteryn:item.tibia.i{item}' for item in source.IDS}
    proof_keys = {r['production_definition']['production_key']
                  for r in json.loads((ROOT / PROOF).read_bytes())['records']}
    expected = checked_source_union(snapshot(L2_SOURCE) | snapshot(MAIN_SOURCE),
                                    source_keys, proof_keys, keys)
    if not old <= keys:
        raise ValueError('L2_EQUIPMENT_LOST_OR_DUPLICATE_SOURCE_KEY')
    old_proof = json.loads(subprocess.check_output(['git', 'show', BASE + ':' + PROOF], cwd=ROOT))
    test_path = 'crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs'
    test = subprocess.check_output(['git', 'show', BASE + ':' + test_path], cwd=ROOT, text=True)
    initial = int(re.search(r'const ORIGINAL_PROFILES: usize = ([0-9_]+);', test)[1].replace('_', ''))
    table = test.split('const PROFILE_ADDITIONS:', 1)[1].split('\n];', 1)[0]
    baseline_cohorts = {name: int(count.replace('_', '')) for name, count in re.findall(r'\("([^"\n]+)", ([0-9_]+)\)', table)}
    declared = initial + sum(int(n.replace('_', '')) for n in re.findall(r'\("[^"\n]+", ([0-9_]+)\)', table)) + len(old_proof['records'])
    if declared < len(old):
        raise ValueError('L2_EQUIPMENT_BASELINE_COHORT_UNDERCOUNT')
    qengine = {row['production_definition']['production_key'] for row in json.loads((ROOT / PROOF).read_bytes())['records']}
    independent_l2 = expected - snapshot(MAIN_SOURCE) - qengine
    test_file = ROOT / test_path
    test_source = test_file.read_text()
    entry = '    // Published L2 source keys outside the main and QENGINE2 cohorts.\n    ("QUESTL2", ' + str(len(independent_l2)) + '),\n'
    if '("QUESTL2",' in test_source:
        test_source = re.sub(r'    // Published L2 source keys outside the main and QENGINE2 cohorts.\n    \("QUESTL2", [0-9]+\),\n', entry, test_source)
    else:
        end = test_source.index('\n];', test_source.index('const PROFILE_ADDITIONS:'))
        test_source = test_source[:end] + '\n' + entry.rstrip('\n') + test_source[end:]
    if '--check' in sys.argv:
        if test_file.read_text() != test_source:
            raise ValueError('L2_EQUIPMENT_DECLARED_COHORT_STALE')
    else:
        test_file.write_text(test_source)
    packet = {'source_cohort_overlap_keys': sorted(reviewed_main_overlap(qengine)), 'baseline_cohorts': baseline_cohorts, 'reviewed_sources': [L2_SOURCE, MAIN_SOURCE], 'baseline_commit': BASE, 'baseline_sha256': hashlib.sha256(baseline).hexdigest(),
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
