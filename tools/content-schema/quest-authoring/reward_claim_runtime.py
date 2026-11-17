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
        if claim.get('definition_profile') == 'native_authored_v1':
            if native_placement_is_bound(root, index, claim):
                enabled.add(key)
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


def native_placement_is_bound(root, index, claim):
    """Read the actual region/ordinal; authored claims have no legacy selectors."""
    import sys
    sys.path.insert(0, str(root / 'tools/content-schema/world-authoring'))
    import world_region_codec as codec
    regions = {Path(r['path']).name: r for r in index['regions']}
    for p in claim['placements']:
        s = p['source_binding']; x,y,z = (s['project_position'][a] for a in 'xyz')
        region = regions.get(codec.region_name(z,x//256,y//256))
        if region is None: return False
        raw = (root / region['path']).read_bytes()
        if hashlib.sha256(raw).hexdigest() != region['sha256']:
            raise ValueError('native reward placement region digest differs')
        _,_,_,sectors = codec.decode_region(raw)
        found = False
        for _, tiles in sectors:
            for tx,ty,_,house,_,items in tiles:
                if (tx,ty) != (x,y): continue
                roots = [(i,a) for i,d,a in items if d == 0]
                ordinal = s['native_entry_ordinal']
                found = not house and ordinal < len(roots) and index['palette'][roots[ordinal][0]]['source_item_id'] == p['appearance_tibia_id']
        if not found: return False
    return True
