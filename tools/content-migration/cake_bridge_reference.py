"""Register M57's exact owner-approved map Item without native inventory admission."""
import json

from boss_conditional_item_reference import definition
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools/content-schema/native-gameplay'))
from build_spell_item_identities import historical_binding_bytes, PRE_CAKE_BINDINGS_SHA
from event_item_identity import evidence

KEY = 'oteryn:item.tibia.i48227'
PACKET = 'tools/content-schema/native-gameplay/cake-bridge-identity.json'


def extend_records(records, root):
    packet = json.loads((root / PACKET).read_bytes())
    expected = definition(KEY)
    # Crystal 201189f8 items.xml:81432-81441 omits 48227. The protobuf
    # appearance and src/items/items.cpp:171-181,219-251 define ground and
    # physical flags. The defaults come from src/items/items.hpp:276-300.
    expected['semantics'].update({
        'charges': {'state': 'NOT_APPLICABLE'},
        'classification': {'state': 'KNOWN', 'value': {
            'item_type': {'state': 'NOT_APPLICABLE'},
            'capabilities': {'state': 'UNKNOWN'},
        }},
        'container': {'state': 'NOT_APPLICABLE'},
        'physical': {'state': 'KNOWN', 'value': {
            'movable': {'state': 'KNOWN', 'value': False},
            'pickupable': {'state': 'KNOWN', 'value': False},
            'weight': {'state': 'KNOWN', 'value': 0},
        }},
        'stack': {'state': 'KNOWN', 'value': {
            'stackable': {'state': 'KNOWN', 'value': False},
            'stack_max': {'state': 'NOT_APPLICABLE'},
        }},
        'temporal': {'state': 'NOT_APPLICABLE'},
    })
    binding = {'source_key': 'oteryn:source.crystalserver',
               'source_revision': evidence()['source_revision'],
               'identity_namespace': 'ots/item_server_id', 'external_id': '48227',
               'target': expected['identity'], 'disposition': 'EXACT'}
    if packet != {**evidence(), 'definition': {k: v for k, v in expected.items() if k != 'semantics'},
                  'binding': binding}:
        raise ValueError('CAKE_BRIDGE_APPROVED_REFERENCE_CHANGED')
    raw = (root / 'imports/crystalserver/bindings/items.json').read_bytes()
    historical_binding_bytes(raw, PRE_CAKE_BINDINGS_SHA)
    if [row for row in json.loads(raw)['bindings'] if row['external_id'] == '48227'] != [binding]:
        raise ValueError('CAKE_BRIDGE_BINDING_CHANGED')
    existing = [row for row in records if row['identity']['key'] == KEY]
    if existing and existing != [expected]:
        raise ValueError('CAKE_BRIDGE_REFERENCE_CHANGED')
    if not existing:
        records.append(expected)
