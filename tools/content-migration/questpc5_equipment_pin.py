"""Regenerate the exact questpc5 zero-speed admission inventory and equipment count."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
PROFILE = 'content/items/definitions/spell-native-profiles.json'
PROOF = ROOT / 'tools/content-schema/native-gameplay/questpc5-equipment-profile-delta.json'
TEST = ROOT / 'crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (ROOT / PROFILE).read_bytes()
    current = {r['production_definition']['production_key']: r for r in json.loads(raw)['records']}
    if args.check:
        proof = json.loads(PROOF.read_bytes())
    else:
        if not args.baseline:
            parser.error('regeneration requires an immutable --baseline commit')
        revision = subprocess.check_output(['git', 'rev-parse', args.baseline], cwd=ROOT, text=True).strip()
        baseline = subprocess.check_output(['git', 'show', revision + ':' + PROFILE], cwd=ROOT)
        old = {r['production_definition']['production_key']: r for r in json.loads(baseline)['records']}
        if old.keys() - current.keys():
            raise ValueError('existing native profiles removed')
        if any(old[k]['attributes'].get('speed_bonus') != current[k]['attributes'].get('speed_bonus') for k in old):
            raise ValueError('existing native speed changed')
        overlays = ['content/items/definitions/starting-loadout-profiles.json',
                    'content/items/definitions/srx1-item-updates.json']
        baseline_keys = set(old)
        overlay_sources = []
        for path in overlays:
            overlay = subprocess.check_output(['git', 'show', revision + ':' + path], cwd=ROOT)
            baseline_keys.update(r['production_definition']['production_key']
                                 for r in json.loads(overlay)['records'])
            overlay_sources.append(dict(path=path, sha256=hashlib.sha256(overlay).hexdigest()))
        delta = current.keys() - old.keys()
        shared_raw = (ROOT / 'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json').read_bytes()
        shared_keys = {row['production_definition']['production_key']
                       for row in json.loads(shared_raw)['records']}
        proof = dict(schema=1, baseline_commit=revision,
                     baseline_native_count=len(baseline_keys),
                     baseline_overlay_sources=overlay_sources,
                     baseline_overlay_keys=sorted(delta & baseline_keys),
                     shared_cohort_keys=sorted((delta & shared_keys) - baseline_keys),
                     shared_cohort_sha256=hashlib.sha256(shared_raw).hexdigest(),
                     baseline_sha256=hashlib.sha256(baseline).hexdigest(), baseline_count=len(old),
                     profile_sha256=hashlib.sha256(raw).hexdigest(),
                     keys=sorted(current.keys() - old.keys()))
    keys = proof['keys']
    shared = proof['shared_cohort_keys']
    counted = sorted(set(keys) - set(proof['baseline_overlay_keys']) - set(shared))
    overlap = proof['baseline_overlay_keys']
    if (set(counted) & set(overlap) or set(shared) & set(overlap)
            or set(counted) | set(overlap) | set(shared) != set(keys)
            or len(counted) + len(overlap) + len(shared) != len(keys)):
        raise ValueError('questpc5 native union partition drift')
    shared_raw = (ROOT / 'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json').read_bytes()
    shared_keys = {row['production_definition']['production_key']
                   for row in json.loads(shared_raw)['records']}
    if (proof['shared_cohort_sha256'] != hashlib.sha256(shared_raw).hexdigest()
            or set(shared) != (set(keys) & shared_keys) - set(overlap)):
        raise ValueError('questpc5 shared source cohort drift')
    native_keys = set(current)
    for path in ['content/items/definitions/starting-loadout-profiles.json',
                 'content/items/definitions/srx1-item-updates.json']:
        native_keys.update(r['production_definition']['production_key']
                           for r in json.loads((ROOT / path).read_bytes())['records'])
    if len(native_keys) != proof['baseline_native_count'] + len(counted) + len(shared):
        raise ValueError('questpc5 native union count drift')
    if (len(keys) != len(set(keys)) or not keys or proof['profile_sha256'] != hashlib.sha256(raw).hexdigest()
            or len(current) != proof['baseline_count'] + len(keys)
            or any(not current[k]['admission']['materializable'] or current[k]['attributes'].get('speed_bonus') != 0 for k in keys)):
        raise ValueError('questpc5 exact zero-speed inventory drift')
    text = TEST.read_text()
    row = f'    ("QUESTPC5", {len(counted)}),'
    shared_row = f'    assert_eq!(added_keys.len(), {len(shared_keys)});'
    if args.check:
        if row not in text or shared_row not in text:
            raise ValueError('stale equipment cohort count')
    else:
        if re.search(r'    \("QUESTPC5", \d+\),', text):
            text = re.sub(r'    \("QUESTPC5", \d+\),', row, text)
        else:
            text = text.replace('    ("QNPCFX", 1),', '    ("QNPCFX", 1),\n    // Exact source-backed branch delta; regenerated by questpc5_equipment_pin.py.\n' + row)
        text = re.sub(r'    assert_eq!\(added_keys\.len\(\), \d+\);', shared_row, text)
        TEST.write_text(text)
        PROOF.write_text(json.dumps(proof, indent=2) + '\n')
    print(f'PASS questpc5 equipment checked={len(keys)} new_union={len(counted)} baseline={proof["baseline_count"]} total={len(current)}')


if __name__ == '__main__':
    main()
