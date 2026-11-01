"""Admit exactly the Crystal operands reached by the FIXM09 gathering dispatcher."""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
from build_spell_item_identities import binding_index

PATH = 'tools/content-schema/native-gameplay/gathering-item-profiles.json'
IDS = {3580, 4012, 4026, 4063, 4098, 4138, 4325, 5876, 5877, 5878, 5905, 5906,
       5908, 5942, 5948, 5969, 5973, 5984, 5995, 6006, 6040, 7158, 7159, 7200,
       7236, 7237, 7441, 7442, 9306}
MECHANICAL_KEY = 'oteryn:item.tibia.i9306'
MECHANICAL_CANONICAL = 'content/items/definitions/items-33000-33499.json'
WIKI = 'imports/tibiawiki/facts/items-stats.json'


def mechanical_rod_policy(root):
    """Retain the already qualified 9.20 oz stat; the donor's raw XML says 919."""
    raw = (root / MECHANICAL_CANONICAL).read_bytes()
    selected = [r['definition'] for r in json.loads(raw)['records']
                if r['definition']['identity']['key'] == MECHANICAL_KEY]
    binding = binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())['9306']
    known = lambda value: {'state': 'KNOWN', 'value': value}
    physical = known({'weight': known(920), 'movable': known(True), 'pickupable': known(True)})
    if (len(selected) != 1 or selected[0]['identity'] != binding['target']
            or selected[0]['semantics']['physical'] != physical
            or selected[0]['semantics']['stack']['value']['stackable'] != known(False)):
        raise ValueError('GATHERING_MECHANICAL_CANONICAL_POLICY')
    wiki_raw = (root / WIKI).read_bytes()
    source = json.loads(wiki_raw)['records'][MECHANICAL_KEY]
    observations = source['observations']
    if source['item_id'] != 9306 or len(observations) != 1:
        raise ValueError('GATHERING_MECHANICAL_WIKI_POLICY')
    observation = observations[0]
    if (observation['page_id'] != 36707 or observation['revision_id'] != 1114794
            or observation['content_sha256'] != '8fcdf203860b17899ed7f300fc89863dfd8d4146f2f56839c507ab9968f5e2f3'
            or observation['fields']['weight'] != '9.20'
            or observation['fields']['stackable'] != 'no'):
        raise ValueError('GATHERING_MECHANICAL_WIKI_POLICY')
    proof = {
        'item': binding['target'], 'field': 'physical.weight',
        'donor_value': 919, 'qualified_value': 920,
        'policy': 'tools/content-schema/item-authoring/lower_wiki_stats_packet.py:4-8,22-24,584',
        'page_id': observation['page_id'], 'revision_id': observation['revision_id'],
        'content_sha256': observation['content_sha256'],
        'inputs': [{'path': MECHANICAL_CANONICAL, 'sha256': hashlib.sha256(raw).hexdigest()},
                   {'path': WIKI, 'sha256': hashlib.sha256(wiki_raw).hexdigest()}],
    }
    return binding, physical, proof


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = [row['production_definition']['production_key'] for row in records]
    if (overlay['schema'] != 'OTERYN_NATIVE_GATHERING_ITEM_PROFILES/v1'
            or overlay['source_revision'] != '201189f871d23597b82caf88fd611454a2ddefb8'
            or len(keys) != len(IDS) or set(keys) != {f'oteryn:item.tibia.i{i}' for i in IDS}):
        raise ValueError('GATHERING_ITEM_PROFILE_SET')
    binding, physical, proof = mechanical_rod_policy(root)
    mechanical, = [r for r in records if r['production_definition']['production_key'] == MECHANICAL_KEY]
    if (overlay.get('qualified_overrides') != [proof]
            or mechanical['production_binding'] != binding
            or mechanical['production_definition'] != {'family': 'Item', 'production_key': MECHANICAL_KEY,
                                                        'revision_ref': binding['target']['revision']}
            or mechanical['production_binding_qualification'] is not None
            or mechanical['semantics']['physical'] != physical
            or mechanical['semantics']['stack'] != {'state': 'KNOWN', 'value': {
                'stackable': {'state': 'KNOWN', 'value': False}, 'stack_max': {'state': 'NOT_APPLICABLE'}}}
            or mechanical['semantics']['container'] != {'state': 'NOT_APPLICABLE'}
            or mechanical['admission'] != {'materializable': True, 'stack_class': 'NonStackable',
                                           'legal_destinations': ['CharacterInventory', 'Ground']}):
        raise ValueError('GATHERING_MECHANICAL_PROFILE_QUALIFICATION')
    document = json.loads(generated[profile_path])
    if {row['production_definition']['production_key'] for row in document['records']} & set(keys):
        raise ValueError('GATHERING_ITEM_PROFILE_DUPLICATE')
    document['records'] = sorted(document['records'] + records, key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
