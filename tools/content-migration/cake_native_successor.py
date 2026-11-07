"""Source-qualified canonical Cake successor; adopted overlays stay immutable."""
import copy
import hashlib
import json
import sys
from pathlib import Path

from bind_monster_source_definitions import CREATURES
from import_spell_families import canonical_bytes
import cake_creature_admission as admission

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
from build_canonical_monster_profiles import read_canonical, union_profiles, identity

PRESENTATIONS = 'content/presentations/definitions/spell-native-profiles.json'
PACKET = 'tools/content-schema/native-gameplay/cake-creature-admission.json'
CAKE = ('Creature', 'oteryn:creature.cake_golem', 'definition-r1')


def source_profile(row):
    """Match the canonical serializer's empty/null Creature field omission."""
    result = copy.deepcopy(row)
    if result['target']['family'] == 'Creature':
        profile = result['data']['profile']
        if profile.get('immunities') == []:
            del profile['immunities']
        if profile['details'].get('reference_level') is None:
            profile['details'].pop('reference_level', None)
    return result


def qualify(rows, declarations, reference, packet, item_bindings):
    # Crystal cake_golem.lua:79 retains the previously omitted source attack;
    # monsters.hpp:244 defaults the omitted attack family to Undefined.
    # The normal admission capture validates the full pinned donor blobs, then
    # the normal stage/materializer supplies these exact canonical identities.
    if packet.get('schema') != 'OTERYN_SEASONAL_CREATURE_ADMISSION/v1':
        raise ValueError('CAKE_NATIVE_SOURCE_PACKET_SCHEMA')
    accepted_items = {identity(r['identity']) for r in reference['records']
                      if r['identity']['family'] == 'Item'}
    item_map = {int(b['external_id']): b['target']['key'] for b in item_bindings['bindings']
                if b['disposition'] == 'EXACT' and b['external_id'].isdecimal()
                and b['target']['family'] == 'Item' and identity(b['target']) in accepted_items}
    bundle = admission.complete_bundle()
    expected = admission.stage_bundle(bundle, item_map)
    if (packet.get('crystal_commit') != admission.PIN or packet.get('bundle') != bundle
            or packet.get('staged') != expected
            or packet.get('sources') != [{'path': path, 'sha256': sha}
                                         for path, sha in admission.SOURCE_HASHES.items()]):
        raise ValueError('CAKE_NATIVE_PARTIAL_OR_SUBSTITUTED_SOURCE')
    staged = packet['staged']
    records = {identity(r['identity']): r for r in reference['records']}
    profiles = {identity(r['target']): r for r in declarations['authoring_profiles']}
    for row in staged['records']:
        if records.get(identity(row['identity'])) != row:
            raise ValueError('CAKE_NATIVE_CANONICAL_SOURCE_RECORD_MISMATCH')
    for row in staged['authoring_profiles']:
        if profiles.get(identity(row['target'])) != source_profile(row):
            raise ValueError('CAKE_NATIVE_CANONICAL_SOURCE_PROFILE_MISMATCH')
    cake, = [r for r in rows if identity(r['definition']['identity']) == CAKE]
    expected = records[CAKE]
    if cake['definition'] != expected or source_profile({'target': cake['definition']['identity'], 'data': cake['authoring']})['data'] != profiles[CAKE]['data']:
        raise ValueError('CAKE_NATIVE_CREATURE_ROW_MISMATCH')
    bindings = [b for b in staged['source_identity_bindings'] if identity(b['target']) == CAKE]
    if cake['source_bindings'] != bindings or len(bindings) != 1:
        raise ValueError('CAKE_NATIVE_SOURCE_BINDING_MISMATCH')
    return cake


def admit(root, generated):
    """Use the normal canonical union, restricted to this lane's one Creature."""
    packet, _ = read_canonical(root / PACKET)
    declarations, _ = read_canonical(root / 'content/world/definitions/declarations.json')
    reference, _ = read_canonical(root / 'content/world/definitions/reference.json')
    rows = []
    for path in sorted((root / 'content/creatures/definitions').glob('creatures-*.json')):
        document, _ = read_canonical(path)
        rows.extend(document['records'])
    item_bindings, _ = read_canonical(root / 'imports/crystalserver/bindings/items.json')
    cake = qualify(rows, declarations, reference, packet, item_bindings)
    active = json.loads(generated[CREATURES])
    presentations = json.loads(generated[PRESENTATIONS])
    creatures, presentations, appended = union_profiles(
        active, presentations, [cake], declarations['authoring_profiles'],
        preserve_existing_display_aliases=True)
    prior = [r for r in active['records'] if identity(r['profile']['target']) == CAKE]
    selected = [r for r in creatures['records'] if identity(r['profile']['target']) == CAKE]
    if len(selected) != 1 or (not prior and appended != [cake['definition']['identity']]):
        raise ValueError('CAKE_NATIVE_IDENTITY_OR_DISPLAY_NAME_CONFLICT')
    if prior and (selected[0]['profile']['data'] != cake['authoring']
                  or selected[0]['behavior'] != next(r for r in declarations['authoring_profiles']
                      if r['target'] == cake['definition']['behavior'])):
        raise ValueError('CAKE_NATIVE_EXISTING_POLICY_CONFLICT')
    result = dict(generated)
    result[CREATURES] = canonical_bytes(creatures)
    result[PRESENTATIONS] = canonical_bytes(presentations)
    manifest = json.loads(result['content/spells.manifest.json'])
    for field, path in [('creature_profiles', CREATURES), ('presentation_profiles', PRESENTATIONS)]:
        manifest[field]['sha256'] = hashlib.sha256(result[path]).hexdigest()
    result['content/spells.manifest.json'] = canonical_bytes(manifest)
    return result
