"""Source-only conjure helper branch facts; never chooses a rendered effect."""
import argparse
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

import native_world_items

PINS = {'canary': '04b83b512114bfd888000d6e1433ed8ecaec7c5b',
        'crystal': '00ce02a57ca5a12e48f32a3476e37471167e4c3f'}
HELPER = 'data/scripts/lib/register_spells.lua'
EXPECTED_BODY = ('if not conjureCount and conjureId ~= 0 then local itemType = ItemType(conjureId) '
    'if itemType:getId() == 0 then return false end local charges = itemType:getCharges() '
    'if charges ~= 0 then conjureCount = charges end end '
    'if reagentId ~= 0 and not self:removeItem(reagentId, 1, -1) then '
    'self:sendCancelMessage(RETURNVALUE_YOUNEEDAMAGICITEMTOCASTSPELL) '
    'self:getPosition():sendMagicEffect(CONST_ME_POFF) return false end '
    'local item = self:addItem(conjureId, conjureCount) if not item then '
    'self:sendCancelMessage(RETURNVALUE_NOTPOSSIBLE) '
    'self:getPosition():sendMagicEffect(CONST_ME_POFF) return false end '
    'if item:hasAttribute(ITEM_ATTRIBUTE_DURATION) then item:decay() end '
    'self:getPosition():sendMagicEffect(item:getType():isRune() and CONST_ME_MAGIC_RED or effect) return true')


def sha(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2) + '\n').encode()


def qualify_helper(data):
    body = native_world_items._body(data.decode(), 'Player:conjureItem')
    if body != EXPECTED_BODY:
        raise ValueError('conjure helper changed; branch recipe not qualified')
    return sha(body.encode())


def success_effect(is_rune, supplied_effect):
    """Qualified Lua and/or rule, with nil/false distinct from numeric zero."""
    return 'CONST_ME_MAGIC_RED' if is_rune else supplied_effect


def recipe():
    return {'count_default': 'when_count_nil_or_false_and_result_id_nonzero_read_ItemType_charges',
            'invalid_result_type_returns': False,
            'reagent_operation': {'quantity': 1, 'subtype': -1, 'skip_if_id_zero': True,
                                  'failure_message': 'RETURNVALUE_YOUNEEDAMAGICITEMTOCASTSPELL'},
            'creation_failure_message': 'RETURNVALUE_NOTPOSSIBLE',
            'failure_effect': 'CONST_ME_POFF',
            'duration_attribute_action': 'decay_created_item_before_success_effect',
            'success_effect': {'predicate': 'created_item_getType_isRune',
                               'when_true': 'CONST_ME_MAGIC_RED',
                               'when_false': 'supplied_fourth_argument_including_nil'},
            'success_returns': True,
            'operation_order': ['count_resolution', 'reagent_removal', 'item_creation',
                                'conditional_decay', 'conditional_success_effect', 'return_true']}


def closed(properties):
    return {'type': 'object', 'additionalProperties': False,
            'properties': properties, 'required': list(properties)}


def schema():
    hex64 = {'type': 'string', 'pattern': '^[a-f0-9]{64}$'}
    effect_arg = {'oneOf': [closed({'kind': {'const': 'omitted_nil'}}),
                            closed({'kind': {'const': 'source_symbol'},
                                    'symbol': {'type': 'string', 'pattern': '^CONST_ME_[A-Z0-9_]+$'}})]}
    return {'$schema': 'https://json-schema.org/draft/2020-12/schema',
            'allOf': [{'if': {'properties': {'source': {'const': source}}},
                       'then': {'properties': {'revision': {'const': pin}}}} for source, pin in PINS.items()],
            **closed({'registration_key': {'type': 'string', 'minLength': 1},
                      'source': {'enum': sorted(PINS)}, 'revision': {'enum': list(PINS.values())},
                      'source_sha256': hex64, 'callback_record_sha256': hex64,
                      'helper_sha256': hex64, 'helper_body_sha256': hex64,
                      'result_item_id': {'type': 'integer', 'minimum': 1},
                      'reagent_item_id': {'type': 'integer', 'minimum': 0},
                      'count': {'type': 'integer', 'minimum': 1},
                      'fourth_argument': effect_arg,
                      'base_status': {'enum': ['BLOCKED', 'CANDIDATE_SCHEMA_VALID']},
                      'helper_recipe': {'const': recipe()},
                      'result_item_type_qualified': {'const': False},
                      'effect_enum_value_qualified': {'const': False},
                      'effect_asset_binding_qualified': {'const': False},
                      'runtime_activation': {'const': False},
                      'external_sources_used': {'const': False}})}


def generate(out, base, source_root):
    from jsonschema import Draft202012Validator
    manifest = json.loads((base / 'package-manifest.json').read_bytes())
    paths = ['source-callback-facts.jsonl.gz', 'import-summary.json']
    for path in paths:
        if sha((base / path).read_bytes()) != manifest['files'][path]:
            raise ValueError('base input checksum mismatch: ' + path)
    statuses = {r['registration_key']: r['status'] for r in json.loads((base / paths[1]).read_bytes())['records_index']}
    callbacks = [json.loads(line) for line in gzip.decompress((base / paths[0]).read_bytes()).splitlines()]
    helpers = {}
    for source, pin in PINS.items():
        data = subprocess.check_output(['git', '-C', str(source_root / source), 'show', pin + ':' + HELPER])
        helpers[source] = {'path': HELPER, 'revision': pin, 'sha256': sha(data),
                           'body_sha256': qualify_helper(data)}
    records = []
    validator = Draft202012Validator(schema())
    for callback in sorted(callbacks, key=lambda c: c['registration_key']):
        raw = callback['source_callback_facts']
        if raw['cast'].get('tier') != 'conjure':
            continue
        source = raw['source']
        if callback['source_revision'] != PINS[source]:
            raise ValueError('source revision mismatch')
        data = subprocess.check_output(['git', '-C', str(source_root / source), 'show', PINS[source] + ':' + raw['file']])
        if sha(data) != callback['source_sha256']:
            raise ValueError('source callback checksum mismatch')
        conjure = raw['cast']['conjure']
        helper = helpers[source]
        record = {'registration_key': callback['registration_key'], 'source': source,
                  'revision': PINS[source], 'source_sha256': callback['source_sha256'],
                  'callback_record_sha256': sha(encoded(callback)),
                  'helper_sha256': helper['sha256'], 'helper_body_sha256': helper['body_sha256'],
                  'result_item_id': conjure['result_item_id'], 'reagent_item_id': conjure['reagent_item_id'],
                  'count': conjure['count'],
                  'fourth_argument': {'kind': 'source_symbol', 'symbol': conjure['effect']} if 'effect' in conjure else {'kind': 'omitted_nil'},
                  'base_status': statuses[callback['registration_key']], 'helper_recipe': recipe(),
                  'result_item_type_qualified': False, 'effect_enum_value_qualified': False,
                  'effect_asset_binding_qualified': False, 'runtime_activation': False,
                  'external_sources_used': False}
        validator.validate(record)
        records.append(record)
    data = encoded(records)
    compressed = gzip.compress(data, mtime=0)
    out.mkdir(parents=True, exist_ok=False)
    (out / 'source-conjure-helper.json.gz').write_bytes(compressed)
    (out / 'source-conjure-helper.schema.json').write_bytes(encoded(schema()))
    receipt = {'schema': 'OTERYN_SOURCE_CONJURE_HELPER/v1', 'records': len(records),
               'status_counts': dict(Counter(r['base_status'] for r in records)),
               'fourth_argument_counts': dict(Counter(r['fourth_argument']['kind'] for r in records)),
               'helper_proofs': helpers, 'input_proofs': {p: sha((base / p).read_bytes()) for p in paths},
               'data_sha256': sha(compressed), 'payload_sha256': sha(data),
               'schema_sha256': sha(encoded(schema())), 'exporter_sha256': sha(Path(__file__).read_bytes()),
               'runtime_activation': False, 'external_sources_used': False,
               'limits': ['Result Item type, enum numeric binding, renderer binding and native helper execution remain unqualified.',
                          'These are helper branch facts, not complete Spell candidates or callback control-flow qualification.',
                          'Disabled fixtures retain their blocked status. Missing effect is nil, not a universal red default.']}
    (out / 'source-conjure-helper-receipt.json').write_bytes(encoded(receipt))
    return receipt


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--base', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(generate(args.out, args.base, args.source_root)))
