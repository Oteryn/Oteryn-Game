"""Reproduce structural coverage from receipt-verified immutable Git blobs.

Only the observed typed wrapper shapes are normalized. Domain admission state
is ordinary data; this is not inference of arbitrary future schema shapes.
"""
import collections
import csv
import hashlib
import io
import json
import pathlib
import re
import subprocess
import xml.etree.ElementTree as ET

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
COLUMNS = ['corpus', 'path', 'record_count', 'records_present', 'records_absent',
           'KNOWN', 'UNKNOWN', 'NOT_APPLICABLE', 'records_empty_string',
           'records_empty_list', 'records_zero', 'records_null',
           'records_absent_with_unknown_ancestor',
           'records_absent_with_not_applicable_ancestor']


def is_wrapper(value):
    if not isinstance(value, dict):
        return False
    keys = set(value)
    if value.get('state') == 'KNOWN':
        return {'state', 'value'} <= keys <= {'state', 'value', 'source', 'reason'}
    return (value.get('state') in ('UNKNOWN', 'NOT_APPLICABLE')
            and 'value' not in keys and keys <= {'state', 'source', 'reason'})


def census(records, label):
    fields = {}
    for index, record in enumerate(records):
        bit = 1 << index

        def walk(value, path):
            row = fields.setdefault(path, collections.defaultdict(int))
            row['present'] |= bit
            if is_wrapper(value):
                row[value['state']] |= bit
                if value['state'] == 'KNOWN':
                    walk(value['value'], path)
                for key in sorted(set(value) - {'state', 'value'}):
                    walk(value[key], path + '.' + key if path else key)
                return
            if value is None:
                row['null'] |= bit
            elif not isinstance(value, bool) and isinstance(value, (int, float)) and value == 0:
                row['zero'] |= bit
            elif isinstance(value, str) and not value:
                row['empty_string'] |= bit
            if isinstance(value, dict):
                for key, child in value.items():
                    walk(child, path + '.' + key if path else key)
            elif isinstance(value, list):
                if not value:
                    row['empty_list'] |= bit
                for child in value:
                    walk(child, path + '[]')

        walk(record, '')
    total = (1 << len(records)) - 1
    result = []
    for path, field in sorted(fields.items()):
        absent = total ^ field['present']
        ancestors = []
        collection_parent = path
        while collection_parent.endswith('[]'):
            collection_parent = collection_parent[:-2]
            ancestors.append(collection_parent)
        parts = path.split('.')
        for index in range(1, len(parts)):
            parent = '.'.join(parts[:index])
            ancestors.append(parent)
            if parent.endswith('[]'):
                ancestors.append(parent[:-2])
        row = {'corpus': label, 'path': path or '<record>', 'record_count': len(records),
               'records_present': field['present'].bit_count(), 'records_absent': absent.bit_count()}
        for state in ('UNKNOWN', 'NOT_APPLICABLE'):
            blocked = 0
            for parent in ancestors:
                blocked |= fields.get(parent, {}).get(state, 0)
            row['records_absent_with_' + state.lower() + '_ancestor'] = (absent & blocked).bit_count()
        for state in ('KNOWN', 'UNKNOWN', 'NOT_APPLICABLE'):
            row[state] = field.get(state, 0).bit_count()
        for kind in ('empty_string', 'empty_list', 'zero', 'null'):
            row['records_' + kind] = field.get(kind, 0).bit_count()
        result.append(row)
    return result


def source_groups():
    receipts = json.loads((HERE / 'source-receipts.json').read_text())['files']
    blobs = {}
    for receipt in receipts:
        raw = subprocess.check_output(['git', 'cat-file', 'blob', receipt['git_blob_sha1']], cwd=REPO)
        assert len(raw) == receipt['bytes'] and hashlib.sha256(raw).hexdigest() == receipt['sha256'], receipt['repository_path']
        assert hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest() == receipt['git_blob_sha1']
        blobs[receipt['repository_path']] = raw

    def read(path):
        return json.loads(blobs[path])

    def key(record):
        return record['production_definition']['production_key']

    canonical = []
    for path in sorted(blobs):
        if re.fullmatch(r'content/items/definitions/items-\d+-\d+\.json', path):
            canonical += read(path)['records']
    base = read('content/items/definitions/spell-native-profiles.json')['records']
    updates = read('content/items/definitions/srx1-item-updates.json')['records']
    starting = read('content/items/definitions/starting-loadout-profiles.json')['records']
    merged = {key(record): record for record in base}
    for record in updates + starting:
        merged[key(record)] = record
    groups = {'canonical': canonical, 'manifest-base': base, 'manifest-updates': updates,
              'manifest-starting-loadout': starting, 'manifest-merged-offline': list(merged.values())}
    supplemental = []
    for path in sorted(blobs):
        if re.fullmatch(r'tools/content-schema/native-gameplay/[^/]*item[^/]*profiles[^/]*\.json', path):
            supplemental += read(path).get('records', [])
    groups['supplemental-profile-records'] = supplemental
    for label, path in [('taxonomy', 'content/items/taxonomy/items.json'),
                        ('relations', 'content/items/relations/items.json'),
                        ('trade-services', 'content/services/trade/trade-00000-00323.json')]:
        groups[label] = read(path)['records']
    offers, held = [], []
    for service in groups['trade-services']:
        declaration = service['declaration']
        service_key = declaration['identity']['key']
        offers += [{'service': service_key, **offer} for offer in declaration.get('offers', [])]
        for field in declaration.get('fields', []):
            if 'held_' in field['field_path']:
                raw = field.get('value', {}).get('value')
                try:
                    value = json.loads(raw)
                except (TypeError, ValueError):
                    value = None
                held.append({'service': service_key, 'field_path': field['field_path'], 'value': value,
                             'raw_sha256': hashlib.sha256(str(raw).encode()).hexdigest()})
    groups['trade-offers'], groups['held-trade-facts'] = offers, held
    groups['aliases'] = read('content/items/aliases.json')['entries']
    groups['identity-bindings'] = read('imports/crystalserver/bindings/items.json')['bindings']
    groups['market-metadata'] = [{'item_id': row[0], 'category': row[1], 'classification': row[2] if len(row) > 2 else None}
                                 for row in read('crates/gameplay/content/src/content/native_gameplay/market_filter_catalogue.json')['rows']]
    for label in ('ranged-profiles', 'readable-profiles', 'writable-profiles', 'authored-static-admissions'):
        groups[label] = read('content/items/' + label + '.json')['records']
    groups['drown-protection-overrides'] = read('content/items/drown-protection.json')['overrides']
    groups['canary-reference-xml'] = [{'header': dict(element.attrib),
                                     'attributes': [dict(attribute.attrib) for attribute in element.iter('attribute')]}
                                    for element in ET.fromstring(blobs['imports/canary/items-xml/items.xml'])]
    return groups


def generate():
    return [row for label, records in source_groups().items() for row in census(records, label)]


def csv_text(rows):
    output = io.StringIO(newline='')
    writer = csv.DictWriter(output, fieldnames=COLUMNS, lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)
    return output.getvalue()


if __name__ == '__main__':
    import sys
    sys.stdout.write(csv_text(generate()))
