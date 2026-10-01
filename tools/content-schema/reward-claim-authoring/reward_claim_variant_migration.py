"""Complete source data for RewardClaim variants without widening canonical admission."""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
from collections import Counter
from pathlib import Path

SOURCE = 'tools/content-schema/quest-authoring/samples/chests/claims.json'
MANIFEST = 'tools/content-schema/quest-authoring/samples/chests/manifest.json'
SOURCE_SCHEMA = 'tools/content-schema/quest-authoring/quest_content.schema.json'
CANONICAL_INDEX = 'content/interactions/reward_claims/index.json'
EVIDENCE = 'tools/content-schema/reward-claim-authoring/samples/migration/reward-claim-charge-evidence.json'
RULINGS = 'tools/content-schema/reward-claim-authoring/samples/migration/reward-claim-architect-rulings.json'
INTERACTION_SCHEMA = 'tools/content-schema/quest-authoring/interaction.schema.json'
ACHIEVEMENT_INDEX = 'content/achievements/index.json'
OUTPUT = 'tools/content-schema/reward-claim-authoring/samples/migration/reward-claim-variants.json'
CONTRACTS = {
    'container': 'CONTAINER_REWARD_NATIVE_LOWERING_NOT_IMPLEMENTED',
    'key_binding': 'KEY_REWARD_NATIVE_LOWERING_NOT_IMPLEMENTED',
    'written_text': 'WRITTEN_REWARD_NATIVE_LOWERING_NOT_IMPLEMENTED',
    'random_one_of': 'RANDOM_CLAIM_NATIVE_LOWERING_NOT_IMPLEMENTED',
}


def read(root, relative):
    return json.loads((root / relative).read_text(encoding='utf-8'))


def sha(root, relative):
    return hashlib.sha256((root / relative).read_bytes()).hexdigest()


def identity(ref):
    return ref['key'], ref['revision']


def plain_once(claim):
    return claim['claim']['repeat']['kind'] == 'once' and all(set(p['reward']) == {'items'} and 'achievement' not in p for p in claim['placements'])


def achievement_grant(source, placement_index, achievements, ruling):
    observations = [b for b in ruling['achievement_source_bindings'] if b['source_key'] == source['key']]
    matches = []
    if len(observations) == 1:
        observation = observations[0]
        matches = [a for a in achievements if a.get('provenance', {}).get('staticdata', {}).get('source_id') == observation['source_id']
                   and a.get('name') == observation['name'] and a['identity'].get('family') == 'Achievement' and not a.get('retired')]
    grant = {'state': 'UNKNOWN'}
    if len(matches) == 1:
        grant = {'state': 'KNOWN', 'value': {'owner': 'Achievement', 'request': 'grant', 'achievement': copy.deepcopy(matches[0]['identity'])}}
    return {'placement_index': placement_index, 'source_achievement': copy.deepcopy(source),
            'binding_basis': 'unique source achievement numeric ID and exact name in indexed catalogue' if len(matches) == 1 else 'UNKNOWN: no unique canonical achievement identity',
            'grant_request': grant, 'native_status': 'WAITING_ARCHITECTURE',
            'architecture_item': 'ACHIEVEMENT_OWNER_ARCHITECTURE_AFTER_CLOSEOUT'}


def charge_disposition(pi, field, qi, raw, item):
    count = (item or {}).get('semantics', {}).get('charges', {})
    known = count.get('value', {}).get('count', {}) if count.get('state') == 'KNOWN' else {}
    value = known.get('value') if known.get('state') == 'KNOWN' else None
    fact = {'state': 'KNOWN', 'value': value} if type(value) is int and value > 0 else {'state': 'UNKNOWN'}
    equal = fact['state'] == 'KNOWN' and raw == value
    # Physical quantity does not depend on source/default charge equality.
    one_instance = ((item or {}).get('stack_class') == 'NonStackable'
                    and fact['state'] == 'KNOWN' and type(raw) is int and raw > 0)
    return {'placement_index': pi, 'reward_field': field, 'reward_index': qi,
            'source_raw_argument': raw, 'definition_charges': fact,
            'normalized_quantity': {'state': 'KNOWN', 'value': 1} if one_instance else {'state': 'UNKNOWN'},
            'data_status': 'AUTHORED' if equal else ('CONFLICT' if fact['state'] == 'KNOWN' else 'WAITING_DATA'),
            'reason': 'DEFINITION_CHARGES_EQUAL_SOURCE_EVIDENCE' if equal else ('SOURCE_CHARGE_MISMATCH' if fact['state'] == 'KNOWN' else 'CHARGES_DEFINITION_FACT_UNKNOWN'),
            'native_status': 'WAITING_IMPLEMENTATION'}


def dispositions(features, claim, charge_checks, ruling):
    out = []
    for feature in sorted(features):
        status, reasons = 'AUTHORED', []
        if feature == 'container': status, reasons = 'WAITING_DATA', ['CONTENTS_FIELD_AUTHORING_CHILD_AFTER_CLOSEOUT']
        if feature == 'written_text' and any(p['reward'].get('written_text', {}).get('item', True) is None for p in claim['placements']):
            status, reasons = 'CONFLICT', ['D25_SOURCE_WRITTEN_TEXT_CARRIER_CONFLICT']
        if feature == 'charges':
            reasons = sorted({c['reason'] for c in charge_checks if c['data_status'] != 'AUTHORED'})
            status = 'CONFLICT' if any(c['data_status'] == 'CONFLICT' for c in charge_checks) else ('WAITING_DATA' if reasons else 'AUTHORED')
        out.append({'feature': feature, 'data_format': 'COVERED', 'data_status': status, 'reasons': reasons,
                    'closed_contract_refs': copy.deepcopy(ruling['closed_contracts'][feature]),
                    'ruling_id': ruling['comment_id']})
    return out


def derive_packet(root, evidence):
    """Derive all facts from source inputs without calling validation."""
    ruling = json.loads(metadata_path(root, RULINGS).read_text())
    achievement_paths = sorted(p.relative_to(root).as_posix() for p in (root / 'content/achievements').glob('achievements-*.json'))
    if (root / ACHIEVEMENT_INDEX).is_file() and read(root, ACHIEVEMENT_INDEX).get('owner') != 'Achievement':
        raise ValueError('Achievement directory index owner mismatch')
    achievements = [r for path in achievement_paths for r in read(root, path)['records']]
    claims = read(root, SOURCE)['claims']
    manifest = read(root, MANIFEST)
    index = read(root, CANONICAL_INDEX)
    canonical = [r['definition'] for p in index['shards'] for r in read(root, p)['records']]
    canonical_by_source = {}
    for c in canonical:
        key = (c['provenance']['pilot_key'], c['provenance']['pilot_revision'])
        if key in canonical_by_source:
            raise ValueError('duplicate canonical RewardClaim source binding')
        canonical_by_source[key] = c
    item_paths = sorted(p.relative_to(root).as_posix() for p in (root / 'content/items/definitions').glob('items-*.json'))
    items = {r['definition']['identity']['key']: r['definition'] for p in item_paths for r in read(root, p)['records']}
    charged = {i['tibia_id']: i['default_charges'] for i in evidence['charged_items']}
    records, eligible_missing = [], []
    for claim in sorted(claims, key=lambda c: identity(c['identity'])):
        if plain_once(claim):
            if identity(claim['identity']) not in canonical_by_source:
                eligible_missing.append(copy.deepcopy(claim['identity']))
            continue
        reasons, gaps, bindings, positions = set(), [], [], []
        features, grants, charge_checks = set(), [], []
        if claim['claim']['repeat']['kind'] == 'cooldown':
            reasons.add('COOLDOWN_CLAIM_NATIVE_LOWERING_NOT_IMPLEMENTED')
            features.add('cooldown')
        for pi, placement in enumerate(claim['placements']):
            matched = [entry for entry in manifest['entries'] if entry.get('destination') == claim['identity']['key'] and entry['position'] == [placement['position'][k] for k in ('x','y','z')]]
            positions.append({'placement_index': pi, 'manifest_entries': copy.deepcopy(matched)})
            if not matched:
                gaps.append({'code': 'SOURCE_PLACEMENT_BINDING_MISSING', 'placement_index': pi})
            reward = placement['reward']
            for field, code in CONTRACTS.items():
                if field in reward:
                    reasons.add(code)
                    features.add(field)
            if 'achievement' in placement:
                reasons.add('ACHIEVEMENT_OWNER_ARCHITECTURE_AFTER_CLOSEOUT')
                features.add('achievement')
                grant = achievement_grant(placement['achievement'], pi, achievements, ruling)
                grants.append(grant)
                if grant['grant_request']['state'] == 'UNKNOWN':
                    gaps.append({'code': 'ACHIEVEMENT_IDENTITY_BINDING_UNRESOLVED', 'placement_index': pi, 'source_key': placement['achievement']['key']})
            if reward.get('written_text', {}).get('item', True) is None:
                gaps.append({'code': 'SOURCE_WRITTEN_TEXT_CARRIER_UNRESOLVED', 'placement_index': pi})
            refs = [(field, qi, quantity['item'], quantity['count']) for field in ('items','random_one_of') for qi, quantity in enumerate(reward.get(field, []))]
            if 'container' in reward:
                refs.append(('container', None, reward['container'], None))
            if reward.get('written_text', {}).get('item'):
                refs.append(('written_text', None, reward['written_text']['item'], None))
            for field, qi, source, count in refs:
                tibia_id = int(source['key'].split(':item/', 1)[1])
                key = f'oteryn:item.tibia.i{tibia_id}'
                item = items.get(key)
                canonical_ref = {'family': 'Item', **copy.deepcopy(item['identity'])} if item else None
                if not item:
                    gaps.append({'code': 'ITEM_IDENTITY_BINDING_MISSING', 'placement_index': pi, 'source_key': source['key']})
                argument = None
                if count is not None:
                    argument = {'raw_count_argument': count, 'kind': 'COUNT_OR_SUBTYPE_RETAINED', 'source_quantity': None, 'source_charges': None}
                    if tibia_id in charged:
                        argument.update(kind='CHARGES_SUBTYPE', source_quantity=1, source_charges=count)
                        reasons.update(('NATIVE_INSTANCE_LOWERING_NOT_IMPLEMENTED', 'TIMED_ITEM_BEHAVIOUR_NOT_IMPLEMENTED'))
                        features.add('charges')
                        charge_checks.append(charge_disposition(pi, field, qi, count, item))
                bindings.append({'placement_index': pi, 'reward_field': field, 'reward_index': qi, 'source_item': copy.deepcopy(source), 'canonical_item': canonical_ref, 'argument': argument})
        if not reasons:
            raise ValueError('a source variant has no explicit implementation disposition')
        records.append({'source_claim': copy.deepcopy(claim), 'item_bindings': bindings, 'placement_bindings': positions,
                        'source_data': {'status': 'WAITING_SOURCE' if gaps else 'COMPLETE', 'missing': gaps},
                        'authoring_dispositions': dispositions(features, claim, charge_checks, ruling),
                        'achievement_grants': grants, 'charge_dispositions': charge_checks,
                        'native_lowering': {'status': 'WAITING_IMPLEMENTATION', 'pending_implementation': sorted(reasons)}})
    inputs = [SOURCE, MANIFEST, SOURCE_SCHEMA, INTERACTION_SCHEMA, CANONICAL_INDEX, *index['shards'], *item_paths, *achievement_paths]
    if (root / ACHIEVEMENT_INDEX).is_file(): inputs.append(ACHIEVEMENT_INDEX)
    packet = {'schema': 'OTERYN_REWARD_CLAIM_SOURCE_MIGRATION_PACKET/v1', 'classification': 'OTS_HYPOTHESIS_ONLY',
              'source_completeness_scope': 'SOURCE_FIELDS_AND_IDENTITY_BINDINGS',
              'source_claim_count': len(claims), 'record_count': len(records), 'canonical_eligible': sum(map(plain_once, claims)),
              'canonical_eligible_missing': eligible_missing, 'native_lowering': 'WAITING_IMPLEMENTATION',
              'source_schema_sha256': sha(root, SOURCE_SCHEMA), 'charge_evidence': copy.deepcopy(evidence),
              'architect_rulings': [copy.deepcopy(ruling)],
              'authoring_sources': [{'path': p, 'sha256': sha(root, p)} for p in inputs]
                  + [{'path': path, 'sha256': hashlib.sha256(metadata_path(root, path).read_bytes()).hexdigest()} for path in (EVIDENCE, RULINGS)],
              'source_refs': copy.deepcopy(manifest['sources']), 'records': records}
    return packet


def metadata_path(root, relative):
    """Committed evidence is authoritative; the local copy supports staging only."""
    committed = root / relative
    return committed if committed.is_file() else Path(__file__).parent / "samples/migration" / Path(relative).name


def build(root, evidence):
    trusted = json.loads(metadata_path(root, EVIDENCE).read_text())
    if evidence != trusted:
        raise ValueError("charge evidence differs from the authoritative input")
    packet = derive_packet(root, trusted)
    validate(packet, root)
    return packet


def validate(packet, root):
    import jsonschema
    from referencing import Registry, Resource
    schema = json.loads(Path(__file__).with_name('reward_claim_variant_packet.schema.json').read_text())
    vocabulary = read(root, SOURCE_SCHEMA)
    registry = Registry().with_resource(vocabulary['$id'], Resource.from_contents(vocabulary))
    interaction = read(root, INTERACTION_SCHEMA)
    registry = registry.with_resource(interaction['$id'], Resource.from_contents(interaction))
    jsonschema.Draft202012Validator(schema, registry=registry).validate(packet)
    if packet['record_count'] != len(packet['records']):
        raise ValueError('variant record_count mismatch')
    if packet['canonical_eligible'] + packet['record_count'] != packet['source_claim_count']:
        raise ValueError('source coverage counts mismatch')
    if packet['source_schema_sha256'] != sha(root, SOURCE_SCHEMA):
        raise ValueError('source schema digest mismatch')
    trusted = json.loads(metadata_path(root, EVIDENCE).read_text())
    expected_packet = derive_packet(root, trusted)
    # Whole descriptors are derived again from authoritative input files. No
    # packet-provided evidence, nullable binding, status or blocker inventory
    # is allowed to define its own truth, and derivation never calls validate.
    for field in ('authoring_sources', 'source_refs', 'charge_evidence', 'architect_rulings'):
        if packet[field] != expected_packet[field]:
            raise ValueError(f'{field} differs from authoritative source inputs')
    for field in ('source_claim_count', 'record_count', 'canonical_eligible',
                  'canonical_eligible_missing', 'source_schema_sha256'):
        if packet[field] != expected_packet[field]:
            raise ValueError(f'{field}: source coverage inventory mismatch')
    expected_records = expected_packet['records']
    actual = [identity(r['source_claim']['identity']) for r in packet['records']]
    expected = [identity(r['source_claim']['identity']) for r in expected_records]
    if actual != expected:
        raise ValueError('variant source identities mismatch')
    for record, expected_record in zip(packet['records'], expected_records):
        if record['source_claim'] != expected_record['source_claim']:
            raise ValueError('variant source claim changed')
        if record['item_bindings'] != expected_record['item_bindings']:
            raise ValueError('variant Item references, argument kind or canonical binding/revision changed')
        if record['placement_bindings'] != expected_record['placement_bindings']:
            raise ValueError('source placement bindings / UID witnesses changed')
        if record['source_data'] != expected_record['source_data']:
            raise ValueError('source completeness / missing text carrier fields changed')
        for field in ('authoring_dispositions', 'achievement_grants', 'charge_dispositions'):
            if record[field] != expected_record[field]:
                raise ValueError(f'{field} differs from authoritative source disposition')
        if record['native_lowering'] != expected_record['native_lowering']:
            raise ValueError('native implementation blocker inventory changed')
    if packet != expected_packet:
        raise ValueError('variant packet differs from authoritative source derivation')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--charge-evidence', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    evidence = json.loads((args.charge_evidence or args.root / EVIDENCE).read_text())
    packet = build(args.root, evidence)
    text = json.dumps(packet, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n'
    output = args.output or args.root / OUTPUT
    if args.check:
        if not output.is_file() or output.read_text() != text:
            parser.exit(1, 'variant migration packet is stale\n')
    else:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(text)
    print(f"RewardClaim variants: {packet['record_count']}; eligible plain missing={len(packet['canonical_eligible_missing'])}; native lowering waits for implementation")


if __name__ == '__main__':
    main()
