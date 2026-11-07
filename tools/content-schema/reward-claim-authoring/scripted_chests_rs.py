"""Exact R–S script rewards omitted by the historical chest-table-only pilot.

Rapier: Crystal201189f data-global/scripts/actions/rookgaard/rapier_quest.lua:4-11
(SHA256 6420408ad37b8bbbaecffe6ddfc04f445028a330c35109261b46f1a89ff45635).
The UID and position are independently retained by samples/chests/manifest.json;
the current chest.lua:3454 supplies appearance2473. Wiki3334/rev1043841 records
one Rapier and no quest-log entry. These are existing once-claim semantics.
"""
import copy
import json

PATH = 'content/interactions/reward_claims/reward-claims-scripted-rs.json'
REVISION = 'crystal-201189f-scripted-rs-v1'


def source_claims():
    return [{
        'identity': {'key': 'canary:reward-claim/kv/rapier', 'revision': REVISION},
        'quest': {'family': 'Quest', 'key': 'canary:quest/rapier_quest',
                  'revision': 'canary-04b83b51+crystalserver-9f5a72c6'},
        'claim': {'per': 'character', 'repeat': {'kind': 'once'}},
        'placements': [{
            'position': {'x': 32099, 'y': 32198, 'z': 9},
            'appearance': {'family': 'Item', 'key': 'crystalserver:item/2473',
                           'revision': REVISION},
            'reward': {'items': [{'count': 1, 'item': {
                'family': 'Item', 'key': 'crystalserver:item/3272', 'revision': REVISION}}]},
        }],
    }] + [beginning_source_claim()]


def beginning_source_claim():
    """One conservative coat: source NPC expects3562, map table names UID50080.

    Crystal201189f startup/tables/chest.lua:3540-3543 binds7757at32032/32276/5
    (SHA2567c3ac1280196923e0bf5ca6f2ea932c19065bb9ac22d1559150a184f3cf06df4).
    quest_system1.lua:166-171 sets SantiagoGreet3, never the journal. The pilot
    omitted the chest because its table has no explicit reward; source map
    children are not retained there. Author exactly one coat, no extra assets.
    """
    return {
        'identity': {'key': 'canary:reward-claim/kv/beginning_coat', 'revision': REVISION},
        'quest': {'family': 'Quest', 'key': 'canary:quest/the_beginning_quest',
                  'revision': 'canary-04b83b51+crystalserver-9f5a72c6'},
        'claim': {'per': 'character', 'repeat': {'kind': 'once'}},
        'placements': [{
            'position': {'x': 32032, 'y': 32276, 'z': 5},
            'appearance': {'family': 'Item', 'key': 'crystalserver:item/7757', 'revision': REVISION},
            'reward': {'items': [{'count': 1, 'item': {
                'family': 'Item', 'key': 'crystalserver:item/3562', 'revision': REVISION}}]},
        }],
    }


def native_chest_transitions():
    """Closed counterparts of the source chest bindings in the native catalogue."""
    return {
        'oteryn:quest-transition/kv/beginning_coat/chest',
        'oteryn:quest-transition/quest/u11_40/threatened_dreams/mission01/poacher_chest/chest',
        'oteryn:quest-transition/quest/u11_40/threatened_dreams/mission02/dark_moon_mirror/chest',
    }


def derive(manifest, items, build_records):
    claims = source_claims()
    unique = {'canary:reward-claim/kv/rapier': 14042,
              'canary:reward-claim/kv/beginning_coat': 50080}
    entries = []
    for claim in claims:
        position = claim['placements'][0]['position']
        xyz = [position[axis] for axis in ('x', 'y', 'z')]
        witnesses = [row for row in manifest['entries'] if row.get('position') == xyz]
        if len(witnesses) != 1 or not any(
                row['source'] == 'crystalserver' and row['uid'] == unique[claim['identity']['key']]
                for row in witnesses[0]['sources']):
            raise ValueError('script chest UID/position witness changed')
        entry = copy.deepcopy(witnesses[0])
        entry['destination'] = claim['identity']['key']
        entry['sources'] = [row for row in entry['sources'] if row['source'] == 'crystalserver']
        entries.append(entry)
    records, checks = build_records(claims, {'entries': entries}, items,
                                    charge_evidence=[], transitions=native_chest_transitions())
    if checks or len(records) != len(claims) or any(
            row['definition']['readiness'] != 'ready' for row in records):
        raise ValueError('script reward Item is not admitted')
    return records + [small_sapphire(items)]


def extend_outputs(outputs, records, compact):
    index_path = 'content/interactions/reward_claims/index.json'
    index = json.loads(outputs[index_path])
    existing = [row for path in index['shards'] for row in json.loads(outputs[path])['records']]
    keys = [row['definition']['identity']['key'] for row in existing + records]
    positions = [tuple(p['source_binding']['project_position'][axis] for axis in ('x', 'y', 'z'))
                 for row in existing + records for p in row['definition']['placements']]
    if len(keys) != len(set(keys)) or len(positions) != len(set(positions)):
        raise ValueError('scripted R–S claim identity/placement collision')
    outputs[PATH] = compact({'family': 'RewardClaim', 'schema': 'OTERYN_REWARD_CLAIM_SHARD/v1',
                            'shard': {'index': 0, 'start': 0, 'end': len(records) - 1,
                                      'count': len(records)}, 'records': records})
    index['shards'].append(PATH)
    index['record_count'] += len(records)
    index['scripted_rs_record_count'] = len(records)
    index['readiness']['ready'] += len(records)
    outputs[index_path] = compact(index)


SMALL_SAPPHIRE_PROFILE = 'scripted_rs_v1'
SMALL_SAPPHIRE_SOURCE_SHA256 = '229eb3979fe64d69d726104122a3d76e2c281902777ef151cc796834c38a4291'


def small_sapphire(items):
    """Personal wiki recipe with a separately declared native import marker."""
    proof = small_sapphire_source()
    ref = lambda ident: {'family': 'Item', 'key': f'oteryn:item.tibia.i{ident}',
                         'revision': 'definition-r1'}
    root = ref(proof['root'])
    contents = [{'item': ref(row['item']), 'count': row['count'],
                 **({'text': row['text']} if 'text' in row else {})}
                for row in proof['rewards']]
    for entry in [{'item': root, 'count': 1}, *contents]:
        item = items.get(entry['item']['key'])
        if (not item or item['identity'] != entry['item'] or not item['materializable']
                or item['stack_class'] not in ('StackCapable', 'NonStackable')):
            raise ValueError('Small Sapphire Item is not admitted: ' + entry['item']['key'])
    if items[root['key']]['semantics']['container']['value']['capacity']['value'] != proof['capacity']:
        raise ValueError('Small Sapphire bag capacity changed')
    binding = proof['map_binding']
    recipe = {'unique_id': None, 'root': root, 'capacity': proof['capacity'],
              'contents': contents, 'completion': {'kind': 'quest_kv', 'key': 'smallSapphire'}}
    return {'definition': {
        'identity': {'key': 'oteryn:reward-claim.kv.smallsapphire', 'revision': 'reward-claim-r1'},
        'definition_profile': SMALL_SAPPHIRE_PROFILE,
        'claim': {'per': 'character', 'repeat': {'kind': 'once'}},
        'quest': {'family': 'Quest', 'key': 'oteryn:quest.authored.small_sapphire_quest',
                  'revision': 'authored-r1'},
        'readiness': 'ready',
        'provenance': {'source_packet': 'tools/content-schema/reward-claim-authoring/small_sapphire_source.json',
                       'source_sha256': SMALL_SAPPHIRE_SOURCE_SHA256},
        'placements': [{'appearance_tibia_id': binding['appearance'],
                        'source_binding': {'project_position': binding['position'],
                                           'legacy_unique_ids': binding['legacy_unique_ids']},
                        'reward': {'container': root, 'items': [
                            {'item': row['item'], 'count': row['count']} for row in contents]}}],
        'native_contents': {'schema': 'OTERYN_NATIVE_CONTAINER_REWARD/v1',
                            'revision': 'tibiawiki-small-sapphire-v1',
                            'native_unique_id': binding['native_unique_id'],
                            'source_sha256': SMALL_SAPPHIRE_SOURCE_SHA256,
                            'placements': [recipe]},
    }}



def small_sapphire_source():
    import hashlib
    from pathlib import Path
    raw = Path(__file__).with_name('small_sapphire_source.json').read_bytes()
    if hashlib.sha256(raw).hexdigest() != SMALL_SAPPHIRE_SOURCE_SHA256:
        raise ValueError('Small Sapphire source packet changed')
    return json.loads(raw)


def record_count():
    """Pinned script leaves and one independently authored wiki container recipe."""
    return len(source_claims()) + 1
