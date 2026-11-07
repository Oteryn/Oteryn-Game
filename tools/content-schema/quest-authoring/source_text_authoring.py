#!/usr/bin/env python3
"""Capture exact SOURCE Lua prose once; rebuild/check its registry offline from committed capture."""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path

import jsonschema
import lua_tables
import source_texts as texts

LOCAL = Path(__file__).resolve().parent
INPUTS = {'quests': 'questlog/quests.json', 'progress': 'questlog/progress.json',
          'interactions': 'interactions/interactions.json', 'gates': 'doors/gates.json',
          'claims': 'chests/claims.json', 'wiki_catalogue': 'catalogue/catalogue.json',
          'interactions_manifest': 'interactions/manifest.json'}
SOURCES = {'canary': {'repository': 'opentibiabr/canary', 'revision': '04b83b512114bfd888000d6e1433ed8ecaec7c5b'},
           'crystalserver': {'repository': 'zimbadev/crystalserver', 'revision': '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'}}


def read(path):
    return json.loads(path.read_text())


def encoded(value):
    return json.dumps(value, indent=2, ensure_ascii=False, sort_keys=True) + '\n'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inputs(samples):
    docs = {role: read(samples / rel) for role, rel in INPUTS.items()}
    data = {role: docs[role][role] for role in ('quests', 'progress', 'interactions', 'gates', 'claims')}
    data['wiki_catalogue'] = docs['wiki_catalogue']
    data['interaction_source_conflicts'] = [
        {'interaction': e['destination'], 'alternatives': e['conflict_alternatives']}
        for e in docs['interactions_manifest']['entries'] if e.get('conflict_alternatives')]
    return data


def capture(data, repos):
    authoring=Path(lua_tables.__file__).resolve().parent/'ots_chests.py'
    authoring_lines=authoring.read_text().splitlines()
    expression="text_ref(text['text'].strip('\\n'))"
    transform_basis={'path':'tools/content-schema/quest-authoring/ots_chests.py',
                     'line':next(i+1 for i,line in enumerate(authoring_lines) if expression in line),
                     'blob_sha256':digest(authoring),'expression':expression}
    wanted = {texts.identity(row['reference']) for row in texts.wanted(data)}
    found, checks, scanned = {}, [], 0
    for provider, repo in sorted(repos.items()):
        pin = SOURCES[provider]
        head = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], text=True).strip()
        if head != pin['revision']:
            raise ValueError('source checkout pin mismatch: ' + provider)
        for path in sorted(repo.rglob('*.lua')):
            if path.relative_to(repo).parts[0] not in ('data','data-otservbr-global','data-global','data-crystal'):
                continue
            rel = path.relative_to(repo).as_posix()
            raw = path.read_bytes()
            blob = hashlib.sha256(raw).hexdigest()
            try:
                tokens = lua_tables.tokenize(raw.decode('utf-8', errors='replace'))
            except lua_tables.LuaError as error:
                checks.append({'source': provider, 'path': rel, 'blob_sha256': blob, 'reason': str(error)})
                continue
            scanned += 1
            written_fields={}
            if rel.endswith('/scripts/actions/system/quest_reward_common.lua'):
                table=lua_tables.assignments(raw.decode('utf-8',errors='replace').replace('\r\n','\n').replace('\r','\n'),{'AttributeTable'})['AttributeTable']
                for entry in table['fields']:
                    for field in entry['value']['fields']:
                        if field['key']=='text' and isinstance(field['value'],str):
                            written_fields[field['line']]={'table':'AttributeTable','uid':entry['key'],'field':'text'}
            for ordinal,token in enumerate(tokens):
                if token[0] not in ('string', 'lstring'):
                    continue
                if not texts.supported_literal(token[0],token[1]):
                    continue
                modes=[('raw_utf8',token[1])]
                normalized=token[1].replace('\r\n','\n').replace('\r','\n')
                if normalized!=token[1]:
                    modes.append(('universal_newlines',normalized))
                for mode,literal in modes:
                    decoded = lua_tables._string((token[0],literal,token[2]))
                    variants=[('identity',decoded)]
                    if (token[2] in written_fields and ordinal>=2
                            and tokens[ordinal-2][:2]==('name','text') and tokens[ordinal-1][1]=='='):
                        variants.append(('chest_written_text_strip_lf',decoded.strip('\n')))
                    for transform,text in variants:
                        ref = texts.reference(text)
                        key = texts.identity(ref)
                        if key not in wanted:
                            continue
                        entry = found.setdefault(key, {'reference': ref, 'text': text, 'sources': []})
                        if entry['text'] != text:
                            raise ValueError('conflicting exact text hash')
                        witness = {'source': provider, **pin, 'path': rel, 'line': token[2],
                                   'blob_sha256': blob, 'token_kind': token[0], 'raw_literal': token[1],
                                   'literal_sha256': hashlib.sha256(token[1].encode('utf-8')).hexdigest(),
                                   'decoder': 'lua_tables._string/v1','source_read_mode':mode,'text_transform':transform}
                        if transform=='chest_written_text_strip_lf':
                            witness['transform_provenance']=transform_basis
                            witness['source_binding']=written_fields[token[2]]
                        entry['sources'].append(witness)
    for entry in found.values():
        entry['sources'] = [json.loads(v) for v in sorted({texts.identity(s) for s in entry['sources']})]
    result = {'schema': 'OTERYN_SOURCE_TEXT_CAPTURE/v1', 'classification': 'OTS_HYPOTHESIS_ONLY',
              'scope': 'source_reference_prose_only; no native or runtime admission',
              'texts': [found[k] for k in sorted(found)], 'source_checks': checks,
              'summary': {'captured_texts': len(found), 'parsed_lua_files': scanned, 'unparsed_lua_files': len(checks)}}
    texts.validate_capture(result)
    return result


def validate(value, schema_root, kind):
    schema = read(schema_root / 'source_text.schema.json')
    schema = {**schema, '$ref': '#/$defs/' + kind}
    jsonschema.Draft202012Validator.check_schema(schema)
    errors = list(jsonschema.Draft202012Validator(schema).iter_errors(value))
    if errors:
        raise ValueError(str(list(errors[0].absolute_path)) + ': ' + errors[0].message)
    if kind == 'capture':
        texts.validate_capture(value)
        if value['summary']['captured_texts']!=len(value['texts']) or value['summary']['unparsed_lua_files']!=len(value['source_checks']):
            raise ValueError('source capture summary mismatch')
    else:
        if len({texts.identity(r['reference']) for r in value['texts']})!=len(value['texts']):
            raise ValueError('duplicate exact source text reference')
        known = [{'reference': r['reference'], 'text': r['text'], 'sources': r['sources']}
                 for r in value['texts'] if r['classification'] == 'KNOWN']
        texts.validate_capture({'texts': known})
        expected = {'wanted': len(value['texts']), 'known': len(known), 'unknown': len(value['texts']) - len(known)}
        if value['summary'] != expected:
            raise ValueError('source text summary mismatch')


def build(samples_root, capture_path, schema_root):
    cap = read(capture_path)
    validate(cap, schema_root, 'capture')
    value = texts.registry(inputs(samples_root), cap)
    value['capture_provenance'] = {'path': 'tools/content-schema/quest-authoring/source_text_capture.json',
                                 'sha256': digest(capture_path)}
    value['input_provenance'] = [{'role': role, 'path': 'tools/content-schema/quest-authoring/samples/' + rel,
                                 'sha256': digest(samples_root / rel)} for role, rel in sorted(INPUTS.items())]
    validate(value, schema_root, 'registry')
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--samples-root', type=Path, default=LOCAL / 'samples')
    parser.add_argument('--capture-path', type=Path, default=LOCAL / 'source_text_capture.json')
    parser.add_argument('--schema-root', type=Path, default=LOCAL)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--capture-source', action='store_true')
    parser.add_argument('--canary', type=Path)
    parser.add_argument('--crystal', type=Path)
    args = parser.parse_args()
    if args.capture_source and (args.check or not args.canary or not args.crystal):
        parser.error('--capture-source requires --canary and --crystal and cannot use --check')
    if not args.capture_source and (args.canary or args.crystal):
        parser.error('donor checkouts are only used by --capture-source')
    try:
        if args.capture_source:
            value = capture(inputs(args.samples_root), {'canary': args.canary, 'crystalserver': args.crystal})
            validate(value, args.schema_root, 'capture')
            args.capture_path.write_text(encoded(value))
        else:
            value = build(args.samples_root, args.capture_path, args.schema_root)
            output = args.out or args.samples_root / 'source_texts/source_texts.json'
            if args.check:
                if not output.is_file() or output.read_text() != encoded(value):
                    raise ValueError('stale or missing exact source text registry')
            else:
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_text(encoded(value))
        print(json.dumps({'valid': True, 'scope': value['scope'], 'summary': value['summary']}))
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        print(json.dumps({'valid': False, 'error': str(error)}))
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
