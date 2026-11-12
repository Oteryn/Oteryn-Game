"""Recognize applied, source-pinned startup bindings; never assert playability."""
import hashlib
import json
from pathlib import Path
from questpc2_reward_capture import reward_rows

PLACEMENTS = 'content/world/placements/index.json'


def enabled_claims(root: Path, claims):
    index = json.loads((root / PLACEMENTS).read_bytes())
    captures = [pin for name, value in index.get('source', {}).items()
                if name.startswith('quest_chests')
                for pin in value.get('captures', [])]
    bound = set()
    for pin in captures:
        raw = (root / pin['path']).read_bytes()
        if hashlib.sha256(raw).hexdigest() != pin['sha256']:
            raise ValueError('reward runtime capture digest differs from applied World input')
        for row in reward_rows(pin['path'], raw):
            bound.add((row['claim'], tuple(row['position']), row['appearance'], row['unique']))
    enabled = set()
    for key, claim in claims.items():
        placements = claim.get('placements', [])
        if claim.get('readiness') != 'ready' or not placements:
            continue
        expected = set()
        for placement in placements:
            source = placement['source_binding']
            uniques = [u['unique_id'] for u in source['legacy_unique_ids']
                       if u['server'] == 'crystalserver']
            if len(uniques) != 1:
                break
            expected.add((key, tuple(source['project_position'][a] for a in 'xyz'),
                          placement['appearance_tibia_id'], uniques[0]))
        else:
            if expected <= bound:
                enabled.add(key)
    return enabled
