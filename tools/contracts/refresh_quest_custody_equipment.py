"""Regenerate the exact merged quest-custody addition to the equipment census."""
import hashlib
import json
import posixpath
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = '1ae39c7fb9bef09c3cd50d16410374f4134b1676'
OUTPUT = 'tools/content-schema/native-gameplay/questpc8-equipment-custody-census.json'


def composition(revision):
    def read(path):
        return (subprocess.check_output(['git', 'show', revision + ':' + path], cwd=ROOT)
                if revision else (ROOT / path).read_bytes())
    manifest = json.loads(read('content/spells.manifest.json'))
    keys, inputs = set(), []
    for field in ('item_profiles', 'item_profile_updates', 'starting_loadout_item_profiles'):
        pin = manifest[field]
        path = posixpath.normpath('content/' + pin['path'])
        raw = read(path)
        digest = hashlib.sha256(raw).hexdigest()
        if digest != pin['sha256']:
            raise ValueError('EQUIPMENT_CENSUS_INPUT_PIN:' + path)
        keys.update(row['production_definition']['production_key']
                    for row in json.loads(raw)['records'])
        inputs.append({'path': path, 'sha256': digest})
    return keys, inputs


if __name__ == '__main__':
    baseline, _ = composition(BASE)
    current, inputs = composition(None)
    if len(baseline) != 3663 or baseline - current:
        raise ValueError('EQUIPMENT_CENSUS_BASELINE_LOSS')
    packet = {'schema': 1, 'base': BASE, 'base_count': len(baseline),
              'inputs': inputs, 'added_keys': sorted(current - baseline)}
    (ROOT / OUTPUT).write_text(json.dumps(packet, indent=2) + '\n')
    print('PASS equipment custody additions=' + str(len(packet['added_keys'])))
