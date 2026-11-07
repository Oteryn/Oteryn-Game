"""Export qualified authoring data into the server's data-only ruleset catalogue."""
import hashlib
import json
import sys
from pathlib import Path
from wheel_authoring import ROOT, read, validate, validate_evidence

REPO = ROOT.parents[2]
DIRECTORY = REPO / 'rulesets/progression/wheel-of-destiny'
GEM_VOCATION_FIELDS = ('gem_family', 'gem_names', 'basic_mods_position_1',
                       'basic_mods_position_2', 'supreme_mods')


def encoded(value):
    return json.dumps(value, indent=2, allow_nan=False) + '\n'


def exports():
    source = ROOT / 'samples/wheel-candidate.json'
    data = read(source)
    validate(data)
    validate_evidence(data, source.read_bytes())
    digest = hashlib.sha256(source.read_bytes()).hexdigest()
    header = {'revision': data['revision'], 'runtime_admitted': False,
              'source_candidate_sha256': digest}
    wheel = {'schema': 'OTERYN_WHEEL_DATA_IMPORT/v1', **header, 'data': {
        'progression': data['progression'], 'topology': data['topology'],
        'vocations': {v: {k: row for k, row in fields.items()
                         if k not in GEM_VOCATION_FIELDS}
                      for v, fields in data['vocations'].items()}}}
    gems = {'schema': 'OTERYN_GEM_DATA_IMPORT/v1', **header, 'data': {
        'vocations': {v: {k: fields[k] for k in GEM_VOCATION_FIELDS}
                      for v, fields in data['vocations'].items()},
        'catalogue': data['gems']}}
    # Lossless projection: every prepared gameplay datum is exported exactly once.
    restored = {v: {**wheel['data']['vocations'][v], **gems['data']['vocations'][v]}
                for v in data['vocations']}
    if restored != data['vocations'] or gems['data']['catalogue'] != data['gems']:
        raise ValueError('SERVER_IMPORT_PROJECTION')
    result = {'wheel.json': encoded(wheel), 'gems.json': encoded(gems)}
    manifest = {'schema': 'OTERYN_WHEEL_GEM_DATA_MANIFEST/v1', **header,
        'source': source.relative_to(REPO).as_posix(),
        'files': {name: hashlib.sha256(value.encode()).hexdigest()
                  for name, value in result.items()},
        'coverage': {'vocations': 5, 'slots': 180, 'revelations': 20,
                     'basic_mods': 46, 'supreme_mods': 94},
        'pending_runtime': ['W-R/GEM-R admission and native effect bindings',
            'Character allocation, gems, grades, vessels and initial grant',
            'Atomic Atelier economy/items, crusher/drop/trade',
            'Protocol, client UI/icons and gameplay E2E',
            'Guiding Presence arithmetic and declared reveal-scope difference',
            'Architect decision synchronization: owner-confirmed Supreme III12M/15']}
    result['import-manifest.json'] = encoded(manifest)
    return result


def check_or_write(check=False):
    for name, value in exports().items():
        path = DIRECTORY / name
        if check:
            if not path.is_file() or path.read_bytes() != value.encode('utf-8'):
                raise ValueError('SERVER_IMPORT_DRIFT: ' + name)
        else:
            path.write_text(value, encoding='utf-8', newline='\n')


if __name__ == '__main__':
    check_or_write('--check' in sys.argv)
    print('PASS: server import, 5 vocations / 180 slots / 46 Basic / 94 Supreme; runtime inactive.')
