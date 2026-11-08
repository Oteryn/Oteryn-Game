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
LOOT = 'tools/content-schema/native-gameplay/loot-tables.json'
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



def read_project_document(root, locator):
    """Read a generated package member through its current, hash-bound manifest.

    Merge regeneration legitimately changes the package around the unchanged
    Cake packet. qualify() still checks every admitted Cake row against that
    complete, HEAD-pinned donor packet; the generic immutable reader stays intact.
    """
    schemas = {
        'definitions/declarations.json': 'OTERYN_WORLD_PROJECT_DECLARATIONS/v2',
        'definitions/reference.json': 'OTERYN_WORLD_PROJECT_REFERENCE_RECORDS/v1',
    }
    if locator not in schemas:
        raise ValueError('CAKE_NATIVE_PROJECT_DOCUMENT_LOCATOR')
    world = root / 'content/world'
    project = json.loads((world / 'project.json').read_bytes())
    raw_manifest = (world / 'manifest.json').read_bytes()
    if (project.get('schema') != 'OTERYN_WORLD_PROJECT_ROOT/v2'
            or project.get('manifest_locator') != 'manifest.json'
            or project.get('manifest_sha256') != hashlib.sha256(raw_manifest).hexdigest()):
        raise ValueError('CAKE_NATIVE_PROJECT_MANIFEST_PIN')
    manifest = json.loads(raw_manifest)
    if (manifest.get('schema') != 'OTERYN_WORLD_PROJECT_MANIFEST/v2'
            or manifest.get('package_key') != 'oteryn:content.world-project'
            or manifest.get('package_revision') != project.get('project_revision')):
        raise ValueError('CAKE_NATIVE_PROJECT_MANIFEST_SCHEMA')
    members = [row for row in manifest['documents'] if row.get('locator') == locator]
    if len(members) != 1:
        raise ValueError('CAKE_NATIVE_PROJECT_DOCUMENT_MEMBERSHIP')
    member = members[0]
    size = member.get('byte_length')
    # Existing canonical ProjectEvidenceLimits permit 96MB per document.
    if type(size) is not int or not 0 < size <= 96_000_000:
        raise ValueError('CAKE_NATIVE_PROJECT_DOCUMENT_BOUND')
    path = world / locator
    if path.stat().st_size != size:
        raise ValueError('CAKE_NATIVE_PROJECT_DOCUMENT_PIN')
    raw = path.read_bytes()
    if (len(raw) != size or member.get('sha256') != hashlib.sha256(raw).hexdigest()
            or member.get('schema') != schemas[locator]):
        raise ValueError('CAKE_NATIVE_PROJECT_DOCUMENT_PIN')
    document = json.loads(raw)
    if document.get('schema') != schemas[locator]:
        raise ValueError('CAKE_NATIVE_PROJECT_DOCUMENT_SCHEMA')
    return document


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
    if len(records) != len(reference['records']) or len(profiles) != len(declarations['authoring_profiles']):
        raise ValueError('CAKE_NATIVE_CANONICAL_IDENTITY_DUPLICATE')
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


def loot_successor(root, creatures, native_item_profiles=None):
    # Crystal cake_golem.lua:73 binds its real source loot, never an empty
    # stand-in. The existing loot producer copies exact canonical table entries.
    packet, _ = read_canonical(root / PACKET)
    expected = next(r for r in packet['staged']['records']
                    if r['identity']['family'] == 'Loot')
    tables = []
    for path in sorted((root / 'content/loot').glob('loot-*.json')):
        document, _ = read_canonical(path)
        tables.extend(r['definition'] for r in document['records'])
    if [t for t in tables if t['identity'] == expected['identity']] != [expected]:
        raise ValueError('CAKE_NATIVE_CANONICAL_LOOT_MISMATCH')
    from build_full_spell_artifact import loot_tables, LIMITS
    raw = loot_tables(creatures, root / 'content/creatures/definitions',
                      root / 'content/loot', root / 'content/items/definitions', native_item_profiles)
    if len(raw) > LIMITS['loot_tables']:
        raise ValueError('CAKE_NATIVE_LOOT_BYTE_BOUND')
    return raw


def admit(root, generated):
    """Use the normal canonical union, restricted to this lane's one Creature."""
    packet, _ = read_canonical(root / PACKET)
    declarations = read_project_document(root, 'definitions/declarations.json')
    reference = read_project_document(root, 'definitions/reference.json')
    rows = []
    for path in sorted((root / 'content/creatures/definitions').glob('creatures-*.json')):
        # Unrelated generated shards may be repartitioned by another admission.
        # Only Cake's input shard participates here and still must match HEAD.
        candidate = json.loads(path.read_bytes())
        if not any(identity(r['definition']['identity']) == CAKE for r in candidate['records']):
            continue
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
    result[LOOT] = loot_successor(root, creatures)
    manifest = json.loads(result['content/spells.manifest.json'])
    for field, path in [('creature_profiles', CREATURES), ('presentation_profiles', PRESENTATIONS)]:
        manifest[field]['sha256'] = hashlib.sha256(result[path]).hexdigest()
    result['content/spells.manifest.json'] = canonical_bytes(manifest)
    return result
