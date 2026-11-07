"""Reproduce Wheel item metadata and record the native admission boundary.

Reference evidence only: this does not grant item materialization, inventory use,
trade, client packaging or asset redistribution. Reads repository-owned inputs.
"""
import hashlib
import json
import sys
from pathlib import Path
from wheel_authoring import ROOT, read
from verify_item_assets import REPO, build_reference as asset_reference
from client_icons import validate_manifest

sys.path.insert(0, str(ROOT.parent / 'item-authoring'))
from engine_items import load_appearance_objects


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_reference():
    assets = asset_reference()  # verifies appearance/catalogue/atlas bytes
    keys = set(assets['items'])
    if len(keys) != 18:
        raise ValueError('ITEM_DELIVERY_COVERAGE')
    inputs = dict(assets['input_digests'])
    inputs['imports/official/client-assets/15.30/manifest.json'] = assets['manifest_sha256']
    appearances = next(path for path in inputs if path.endswith('.dat'))
    flags = load_appearance_objects((REPO / appearances).read_bytes())
    native = {}
    index_path = REPO / 'content/items/index.json'
    inputs['content/items/index.json'] = sha(index_path)
    for relative in read(index_path)['shards']:
        path = REPO / relative
        for record in read(path)['records']:
            definition = record['definition']
            key = definition['identity']['key']
            if key in keys:
                if key in native:
                    raise ValueError('ITEM_DELIVERY_DUPLICATE_KEY')
                inputs[relative] = sha(path)
                native[key] = {'definition': definition, 'path': relative}
    if set(native) != keys:
        raise ValueError('ITEM_DELIVERY_NATIVE_KEYS')
    trade = {key: [] for key in keys}
    trade_index = REPO / 'content/services/trade/index.json'
    inputs['content/services/trade/index.json'] = sha(trade_index)
    for relative in read(trade_index)['shards']:
        path = REPO / relative
        for record in read(path)['records']:
            declaration = record['declaration']
            for offer in declaration.get('offers', []):
                key = offer['item']['key']
                if key in keys:
                    inputs[relative] = sha(path)
                    trade[key].append({'service': declaration['identity']['key'],
                                       'direction': offer['direction'],
                                       'unit_price': offer['unit_price']})
    if any(not offers for offers in trade.values()):
        raise ValueError('ITEM_DELIVERY_TRADE_KEYS')
    items = {}
    selected_flags = ('flags.take', 'flags.cumulative', 'flags.market',
                      'flags.skillwheel_gem', 'flags.usable', 'flags.multiuse',
                      'flags.wearout', 'skillwheel_gem.gem_quality_id',
                      'skillwheel_gem.vocation_id', 'market.category',
                      'market.trade_as_object_id', 'market.show_as_object_id')
    for key in sorted(keys):
        identifier = assets['items'][key]['appearance_object_id']
        observation = flags[identifier]['flags']
        definition = native[key]['definition']
        semantics = definition.get('semantics', {})
        unknown = [field for field, value in semantics.items()
                   if value['state'] == 'UNKNOWN']
        physical = semantics.get('physical', {})
        if physical.get('state') == 'KNOWN':
            unknown.extend('physical.' + field for field, value in physical['value'].items()
                           if value['state'] == 'UNKNOWN')
        items[key] = {
            'appearance_object_id': identifier,
            'native_definition_path': native[key]['path'],
            'native_definition_sha256': hashlib.sha256(json.dumps(definition, sort_keys=True, allow_nan=False).encode()).hexdigest(),
            'native_materializable_observed': definition['materializable'],
            'native_stack_class_observed': definition['stack_class'],
            'native_known_semantics': {field: value for field, value in semantics.items()
                                       if value['state'] == 'KNOWN'},
            'native_unknown_fields': sorted(unknown),
            'native_trade_declarations_observed': sorted(trade[key], key=lambda x: (x['service'], x['direction'], x['unit_price'])),
            'client_flags_observed': {field: observation[field] for field in selected_flags
                                      if field in observation},
            'client_npc_price_sets_observed': {
                field: sorted(set(observation.get('npcsaledata.' + field, [])))
                for field in ('buy_price', 'sale_price')},
        }
    icon_manifest = read(ROOT / 'samples/client-icon-manifest.json')
    validate_manifest(icon_manifest)
    fallback = sum(entry['reference_fallback_verified'] for entry in icon_manifest['icons'].values())
    return {
        'schema': 'OTERYN_WHEEL_ITEM_DELIVERY_REFERENCE/v1',
        'runtime_admitted': False, 'redistribution_authorized': False,
        'scope': 'METADATA_REFERENCE_AND_OBSERVED_NATIVE_BOUNDARY',
        'input_digests': inputs, 'items': items,
        'client_icon_coverage': {
            'manifest_sha256': sha(ROOT / 'samples/client-icon-manifest.json'),
            'reference_crops': len(icon_manifest['icons']),
            'bindings': len(icon_manifest['bindings']),
            'pixel_equal_fallback_crops': fallback,
            'blocked_different_fallback_crops': len(icon_manifest['icons']) - fallback,
            'item_appearance_catalogue_provides_perk_ui_crosswalk': False,
        },
        'native_trade_coverage': {
            'item_keys_with_declarations': len(trade),
            'declared_offer_count': sum(len(offers) for offers in trade.values()),
            'interpretation': 'AUTHORING_DECLARATIONS; NOT_NATIVE_TRADE_ADMISSION',
        },
        'native_delivery_requires': [
            'Accepted item family and physical/stack/charge/use/trade semantics',
            'Atomic item consumption, fragment placement, charge decrement and capacity rules',
            'Native inventory/trade/loot integration under current catalogue revision',
            'Admitted client projections and asset packaging/redistribution authority',
        ],
    }


if __name__ == '__main__':
    path = ROOT / 'samples/item-delivery-reference.json'
    rendered = json.dumps(build_reference(), indent=2, allow_nan=False) + '\n'
    if '--check' in sys.argv:
        if path.read_text() != rendered:
            raise ValueError('ITEM_DELIVERY_REFERENCE_DRIFT')
    elif '--output' in sys.argv:
        Path(sys.argv[sys.argv.index('--output') + 1]).write_text(rendered)
    else:
        path.write_text(rendered)
    print('PASS: 18 item metadata references; native delivery and client asset admission remain separate.')
