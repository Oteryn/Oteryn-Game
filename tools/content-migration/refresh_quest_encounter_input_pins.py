"""Refresh derived canonical-shard descriptors after exact encounter admission.

The Git base and the captured projection must reproduce every changed shard
exactly before any descriptor changes. Source hashes and native records
are never edited. This avoids recapturing unrelated donor binaries merely
because an unrelated definition shares a canonical JSON shard.
"""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path
import quest_encounter_item_semantics as encounter

ROOT = encounter.ROOT


def outputs(base='HEAD', root=ROOT, *, newhaven=False):
    changed = {}
    changed_definitions = {}
    previous_definitions = {}
    selected = set(encounter.profiles(root))
    if newhaven:
        import quest_finish_newhaven_items as newhaven_items
        selected.update(f'oteryn:item.tibia.i{i}' for i in newhaven_items.REQUIRED_REWARDS)
    for path in (root / 'content/items/definitions').glob('items-*.json'):
        raw = path.read_bytes()
        if not any(r['definition']['identity']['key'] in selected for r in json.loads(raw)['records']):
            continue
        relative = path.relative_to(root).as_posix()
        before = subprocess.check_output(['git', 'show', f'{base}:{relative}'], cwd=root)
        if before == raw:
            continue
        # Pass the whole Item collection to the exact projection, then retain
        # this shard's rows. Other definitions and metadata must remain equal.
        document = json.loads(before)
        original_definitions = {r["definition"]["identity"]["key"]: r["definition"] for r in json.loads(before)["records"]}
        rows = [r['definition'] for r in document['records']]
        captured = encounter.profiles(root)
        local = {d['identity']['key'] for d in rows}
        missing = [dict(identity={'family':'Item','key':key,'revision':'definition-r1'},
                        materializable=False,stack_class='Unknown',kind='Item',client_projection='ClientSafe')
                   for key in captured if key not in local]
        encounter.apply_admissions(rows + missing, root)
        if newhaven:
            newhaven_items.apply_admissions(rows, root)
        if document != json.loads(raw):
            raise ValueError('ENCOUNTER_INPUT_PIN_UNRELATED_CHANGE:' + relative)
        changed[relative] = (hashlib.sha256(before).hexdigest(), hashlib.sha256(raw).hexdigest())
        for row in document['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key in selected and original_definitions[key] != definition:
                previous_definitions[key] = original_definitions[key]
                encode = lambda value: json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()
                changed_definitions[key] = (hashlib.sha256(encode(original_definitions[key])).hexdigest(), hashlib.sha256(encode(definition)).hexdigest())
    taxonomy = root / 'content/items/taxonomy/items.json'
    definition_digests = None
    if taxonomy.exists() and previous_definitions:
        reference = json.loads((root / 'content/world/definitions/reference.json').read_bytes())
        definitions = {d['identity']['key']: d for d in reference['records']}
        for path in (root / 'content/items/definitions').glob('items-*.json'):
            definitions.update({r['definition']['identity']['key']: r['definition']
                                for r in json.loads(path.read_bytes())['records']})
        encode = lambda value: json.dumps(value,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode()
        current_digest = hashlib.sha256(encode(definitions)).hexdigest()
        definitions.update(previous_definitions)
        definition_digests = (hashlib.sha256(encode(definitions)).hexdigest(),current_digest)
    result = {}
    def update(value):
        if isinstance(value, dict):
            if definition_digests and 'canonical_definitions_sha256' in value:
                if value['canonical_definitions_sha256'] not in definition_digests:
                    raise ValueError('ENCOUNTER_NAVIGATION_UNRELATED_DEFINITION_DIGEST')
                value['canonical_definitions_sha256'] = definition_digests[1]
            path = value.get('path')
            if isinstance(path, str) and path in changed and value.get('sha256') == changed[path][0]:
                value['sha256'] = changed[path][1]
            key = value.get('key')
            if isinstance(key, str) and key in changed_definitions and value.get('definition_sha256') == changed_definitions[key][0]:
                value['definition_sha256'] = changed_definitions[key][1]
            for nested in value.values():
                update(nested)
        elif isinstance(value, list):
            for nested in value:
                update(nested)
    captures = list((root / 'tools/content-schema/native-gameplay').glob('*.json'))
    if taxonomy.exists():
        captures.append(taxonomy)
    for path in captures:
        before = json.loads(path.read_bytes())
        after = json.loads(path.read_bytes())
        update(after)
        raw = ((json.dumps(after,sort_keys=True,separators=(',', ':'),ensure_ascii=False)
                if path == taxonomy else json.dumps(after,indent=2,ensure_ascii=False))+'\n').encode()
        if after != before or (path == taxonomy and path.read_bytes() != raw):
            result[path] = raw
    return result

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', default='HEAD')
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--newhaven', action='store_true')
    args = parser.parse_args()
    updates = outputs(args.base, newhaven=args.newhaven)
    if args.check and updates:
        raise SystemExit('ENCOUNTER_CANONICAL_INPUT_PIN_DRIFT')
    for path, raw in updates.items():
        print(path.relative_to(ROOT))
        path.write_bytes(raw)
