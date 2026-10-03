"""Reproduce snapshot selection and fluid evidence from immutable public Git sources."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import urllib.parse
import urllib.request


def fetch_blob(witness):
    repository, revision, path = (witness[k] for k in ('repository', 'revision', 'path'))
    url = f'https://api.github.com/repos/{repository}/contents/{urllib.parse.quote(path)}?ref={revision}'
    headers = {'Accept': 'application/vnd.github.raw+json', 'User-Agent': 'oteryn-source-container-check'}
    if repository == 'Oteryn/Oteryn-Game' and os.getenv('GITHUB_TOKEN'):
        headers['Authorization'] = 'Bearer ' + os.environ['GITHUB_TOKEN']
    with urllib.request.urlopen(urllib.request.Request(url, headers=headers), timeout=30) as response:
        return response.read()


def verify_sources(snapshot, fetch=fetch_blob, parser_path=None):
    provenance, evidence = snapshot['provenance'], snapshot['source_fluid_evidence']
    cache = {}
    def read(w):
        identity = (w['repository'], w['revision'], w['path'])
        raw = cache.setdefault(identity, None)
        if raw is None: raw = cache[identity] = fetch(w)
        git_hash = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
        if git_hash != w['blob_sha1'] or hashlib.sha256(raw).hexdigest() != w.get('blob_sha256', w.get('sha256')):
            raise ValueError('immutable source blob differs: ' + w['path'])
        if 'bytes' in w and w['bytes'] != len(raw): raise ValueError('source byte count differs')
        for proof in w.get('line_proofs', []):
            actual = '\n'.join(raw.decode('utf-8').splitlines()[proof['start_line'] - 1:proof['end_line']])
            if actual != proof['text']: raise ValueError('source line witness differs')
        return raw
    def owned(w):
        return {**w, 'repository': provenance['repository'], 'revision': provenance['revision']}
    packet = json.loads(read(owned(provenance['variant_packet'])))
    selected = [r for r in packet['records'] if any('container' in p['reward'] for p in r['source_claim']['placements'])]
    if selected != snapshot['records'] or packet['source_refs'] != provenance['donor_sources']:
        raise ValueError('complete container record selection differs')
    definitions = []
    for shard in provenance['item_shards']:
        keys = {i['key'] for i in shard['definitions']}
        chosen = [row['definition'] for row in json.loads(read(owned(shard)))['records'] if row['definition']['identity']['key'] in keys]
        if sorted((d['identity'] for d in chosen), key=lambda i: i['key']) != sorted(shard['definitions'], key=lambda i: i['key']):
            raise ValueError('source Item witness inventory differs')
        definitions.extend(chosen)
    if sorted(definitions, key=lambda d: d['identity']['key']) != snapshot['items']:
        raise ValueError('complete source Item definitions differ')
    for group in ('code_witnesses', 'source_helper_witnesses', 'schema_witnesses', 'xml_witnesses'):
        for witness in evidence[group]: read(witness)
    read(evidence['variant_input']); read(evidence['item_classification_input'])
    path = parser_path or Path(__file__).resolve().parents[2] / 'game-atlas-appearances/export.py'
    if hashlib.sha256(path.read_bytes()).hexdigest() != evidence['parser']['sha256']:
        raise ValueError('generic wire parser source differs')
    spec = importlib.util.spec_from_file_location('_source_container_wire', path)
    parser = importlib.util.module_from_spec(spec); sys.modules[spec.name] = parser; spec.loader.exec_module(parser)
    for witness in evidence['donor_binary_witnesses']:
        wanted = {m['item_id']: m for m in witness['members']}; found = set()
        for field, wire, payload in parser._fields(read(witness)):
            if field != 1 or wire != 2: continue
            values = parser._values(payload); identity = parser._first_int(values, 1)
            if identity not in wanted: continue
            if identity in found: raise ValueError('duplicate donor appearance identity')
            found.add(identity); member = wanted[identity]
            flags = parser._first_bytes(values, 3); decoded = parser._values(flags)
            if (hashlib.sha256(payload).hexdigest() != member['object_payload_sha256'] or hashlib.sha256(flags).hexdigest() != member['flags_payload_sha256'] or decoded.get(19) != [1] or decoded.get(5, []) != member['container_raw_values'] or decoded.get(6, []) != member['cumulative_raw_values'] or bool(decoded.get(1)) != member['bank_present']):
                raise ValueError('direct donor fluid classification differs')
        if found != set(wanted): raise ValueError('missing donor fluid appearance')
    for receipt in snapshot['source_quantity_wiki_evidence']['receipts']:
        wiki = receipt['wiki']
        for proof in (wiki, *([wiki['identity_line']] if 'identity_line' in wiki else [])):
            if hashlib.sha256(proof['raw_line'].encode()).hexdigest() != proof['line_sha256']:
                raise ValueError('captured wiki source line differs')
