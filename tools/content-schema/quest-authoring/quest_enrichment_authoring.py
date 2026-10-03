"""SPDX-License-Identifier: MPL-2.0; portable scoped evidence, never Source closure."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

from jsonschema import Draft202012Validator

DIRECTORY = 'tools/content-schema/quest-authoring/samples/enrichment242'
SELECTION = 'tools/content-schema/quest-authoring/samples/completion242/selection.json'
PROFILE = 'scoped_source_enrichment_v1'
INPUT_SHA = '9245069963196f5b310f1dd03070343af2f1467211ee6fb837fb507fc66fdb25'
FLAGS = {'source_holds_preserved': True, 'source_hold_resolved': False,
         'runtime_enabled': False, 'raw_body_rechecked': False,
         'line_spans_rechecked': False, 'source_revision_verified': False}
PROSE = {'statement', 'raw_value', 'recorded_value', 'declared_value', 'text',
         'literal', 'raw_literal', 'source_text', 'dialogue', 'journal_text', 'description'}


def encoded(value):
    return (json.dumps(value, sort_keys=True, ensure_ascii=False, indent=2) + '\n').encode()


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def read(path):
    return json.loads(path.read_text(encoding='utf-8'))


def reduce_text(value, reduced, key=''):
    if isinstance(value, dict):
        return {k: reduce_text(v, reduced, k) for k, v in value.items()}
    if isinstance(value, list):
        return [reduce_text(v, reduced, key) for v in value]
    if isinstance(value, str) and (key in PROSE or len(value) > 120 or '\n' in value
            or '[[' in value or '<br' in value or '/tmp/' in value or '/workspace/' in value):
        from source_texts import reference
        reduced.append(len(value))
        return {'text_reference': reference(value), 'disposition': 'DIGEST_ONLY_NOT_REDISTRIBUTED'}
    return value


def build(capture):
    result = copy.deepcopy(capture)
    result['schema'] = 'OTERYN_QUEST_SOURCE_ENRICHMENT/v1'
    return result


def capture_import(assembled, qualification, acquisition, root):
    raw = assembled.read_bytes(); original = json.loads(raw)
    if sha(raw) != INPUT_SHA:
        raise ValueError('unaccepted enrichment input digest')
    proof = read(qualification)
    if sha(qualification.read_bytes()) != original['qualification_sha256'] or proof['status'] != 'PASS':
        raise ValueError('unbound original qualification')
    if proof['records'] != 242 or proof['facts'] != 5418 or proof['errors']:
        raise ValueError('unexpected qualified scope')
    capture = {'schema': 'OTERYN_QUEST_ENRICHMENT_CAPTURE/v1', 'baseline_head': original['baseline_head'],
               'profile': PROFILE, **FLAGS, 'semantic_review': original['semantic_review'], 'records': []}
    inventory = []; total_reduced = 0
    for before in original['records']:
        reduced = []; row = copy.deepcopy(before)
        for fact in row['facts']:
            del fact['evidence']['path']
            fact['value'] = reduce_text(fact['value'], reduced)
            fact['scope'] = reduce_text(fact['scope'], reduced)
        row['resolutions'] = reduce_text(row['resolutions'], reduced)
        row['remaining_holds'] = reduce_text(row['remaining_holds'], reduced)
        for field in ('enrichment_note', 'recipe_corrections'):
            if field in row:
                row[field] = reduce_text(row[field], reduced)
        capture['records'].append(row); total_reduced += len(reduced)
        inventory.append({'canonical_key': row['canonical_key'], 'original_record_sha256': sha(encoded(before)),
                          'record_sha256': sha(encoded(row)), 'facts_count': len(row['facts']),
                          'candidate_count': sum(f['novelty'] == 'candidate_new' for f in row['facts']),
                          'confirmation_count': sum(f['novelty'] == 'confirmation' for f in row['facts']),
                          'reduced_text_count': len(reduced), 'reduced_text_length': sum(reduced)})
    receipt = {'schema': 'OTERYN_QUEST_ENRICHMENT_RECEIPT/v1', 'original_input_sha256': sha(raw),
               'qualification_sha256': sha(qualification.read_bytes()), 'qualification_status': 'PASS',
               'capture_sha256': sha(encoded(capture)), 'output_sha256': sha(encoded(build(capture))),
               'baseline_head': original['baseline_head'], **FLAGS, 'records': inventory,
               'fact_count': sum(r['facts_count'] for r in inventory), 'reduced_text_count': total_reduced,
               'removed_local_evidence_paths': sum(r['facts_count'] for r in inventory),
               'historical_acquisition': acquisition}
    directory = root / DIRECTORY; directory.mkdir(parents=True, exist_ok=True)
    for name, value in [('capture', capture), ('receipt', receipt), ('enrichment', build(capture))]:
        (directory / (name + '.json')).write_bytes(encoded(value))
    return receipt


def load_enrichment(root):
    root = Path(root); directory = root / DIRECTORY
    capture = read(directory / 'capture.json'); receipt = read(directory / 'receipt.json')
    packet = build(capture); schema = read(Path(__file__).with_name('quest_enrichment.schema.json'))
    validator = Draft202012Validator(schema)
    validator.validate(packet)
    Draft202012Validator(schema['$defs']['receipt']).validate(receipt)
    if sha((directory / 'capture.json').read_bytes()) != receipt['capture_sha256']:
        raise ValueError('capture digest mismatch')
    if encoded(packet) != (directory / 'enrichment.json').read_bytes() or sha(encoded(packet)) != receipt['output_sha256']:
        raise ValueError('portable enrichment regeneration mismatch')
    selected = {r['identity']['key'] for r in read(root / SELECTION)['records']}
    rows = {r['canonical_key']: r for r in packet['records']}
    inventory = {r['canonical_key']: r for r in receipt['records']}
    if len(rows) != 242 or len(packet['records']) != 242 or set(rows) != selected or set(inventory) != selected:
        raise ValueError('duplicate, missing or foreign enrichment quest')
    if receipt['baseline_head'] != packet['baseline_head'] or receipt['fact_count'] != 5418:
        raise ValueError('scope differs from original proof')
    if sum(r['facts_count'] for r in inventory.values()) != receipt['fact_count']:
        raise ValueError('fact inventory differs')
    if len(receipt['records']) != 242 or sum(r['reduced_text_count'] for r in inventory.values()) != receipt['reduced_text_count']:
        raise ValueError('reduction inventory differs')
    result = {}
    for key, row in rows.items():
        counts = inventory[key]
        if sha(encoded(row)) != counts['record_sha256'] or len(row['facts']) != counts['facts_count']:
            raise ValueError('record proof mismatch: ' + key)
        if counts['candidate_count'] != sum(f['novelty'] == 'candidate_new' for f in row['facts']) or counts['confirmation_count'] != sum(f['novelty'] == 'confirmation' for f in row['facts']):
            raise ValueError('novelty inventory differs')
        if any(r['source_hold_resolved'] is not False for r in row['resolutions']):
            raise ValueError('evidence cannot clear Source holds')
        result[key] = {'profile': PROFILE, 'classification': 'SCOPED_EVIDENCE_ONLY_NOT_SOURCE_CLOSURE',
                       'path': DIRECTORY + '/enrichment.json', 'packet_sha256': receipt['output_sha256'],
                       'receipt_sha256': sha((directory / 'receipt.json').read_bytes()),
                       **{k: counts[k] for k in ('record_sha256', 'facts_count', 'candidate_count',
                                                'confirmation_count', 'reduced_text_count')}, **FLAGS}
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--check', action='store_true', help='check portable reconstruction without acquisition')
    parser.add_argument('--capture', type=Path)
    parser.add_argument('--qualification', type=Path)
    parser.add_argument('--acquisition', type=Path)
    args = parser.parse_args()
    if args.capture:
        capture_import(args.capture, args.qualification, read(args.acquisition), args.root)
    print(json.dumps({'records': len(load_enrichment(args.root)), **FLAGS}, sort_keys=True))
