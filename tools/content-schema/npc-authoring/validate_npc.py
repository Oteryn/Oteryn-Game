"""Validate candidate NPC authoring bundles against npc.schema.json plus semantic rules.

Semantic rules (beyond JSON Schema):
- status is RESOLVED exactly when `unresolved` is empty;
- the key namespace matches the source repository and the key name matches the source file stem;
- a text reference never carries `text` unless --allow-text (local review output only);
- every gated, effectful or scripted keyword node, scripted handler, gated service row and missing
  placement has its unresolved row.

Usage: python validate_npc.py [--allow-text] <bundle.json | directory> ...
"""
import argparse
import json
import sys
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parent
NAMESPACES = {'opentibiabr/canary': 'canary', 'zimbadev/crystalserver': 'crystal'}


def texts(value):
    if isinstance(value, dict):
        if 'text' in value and 'sha256' in value:
            yield value
        for item in value.values():
            yield from texts(item)
    elif isinstance(value, list):
        for item in value:
            yield from texts(item)


def gated_nodes(nodes, path='dialogue.keywords'):
    for index, node in enumerate(nodes):
        node_path = f'{path}[{index}]'
        yield node_path, node
        yield from gated_nodes(node['children'], node_path + '.children')


def semantic_errors(bundle, allow_text):
    errors = []
    namespace = NAMESPACES[bundle['source']['repository']]
    if not bundle['key'].startswith(namespace + ':npc/'):
        errors.append(f"key {bundle['key']} does not match source {bundle['source']['repository']}")
    if bundle['key'].split('/', 1)[1] != Path(bundle['source']['path']).stem:
        errors.append('key name does not match the source file stem')
    if not allow_text:
        for ref in texts(bundle):
            errors.append(f"text reference {ref['sha256'][:12]} carries text; committed bundles must not")
    if bundle['status'] not in ('RESOLVED', 'PARTIAL'):
        return errors
    unresolved = bundle['unresolved']
    if (bundle['status'] == 'RESOLVED') != (not unresolved):
        errors.append('status must be RESOLVED exactly when unresolved is empty')
    paths = {(row['path'], row['reason']) for row in unresolved}
    for node_path, node in gated_nodes(bundle['dialogue']['keywords']):
        if node['gate'] != 'NONE' and (node_path + '.gate', 'LUA_PREDICATE') not in paths:
            errors.append(f'{node_path}: gate without unresolved row')
        if node['effect'] != 'NONE' and (node_path + '.effect', 'LUA_ACTION') not in paths:
            errors.append(f'{node_path}: effect without unresolved row')
        if node['kind'] == 'script' and (node_path, 'LUA_CALLBACK') not in paths:
            errors.append(f'{node_path}: script callback without unresolved row')
    for handler in bundle['dialogue']['scripted_handlers']:
        if ('dialogue.scripted_handlers', 'LUA_CALLBACK') not in paths:
            errors.append(f'scripted handler {handler} without unresolved row')
    for kind in ('travel', 'spells', 'blessings', 'promotion'):
        for row in bundle['services'][kind]:
            if row.get('gate', 'NONE') != 'NONE' and (row['dialogue_path'] + '.gate', 'LUA_PREDICATE') not in paths:
                errors.append(f"services.{kind} {row['dialogue_path']}: gate without unresolved row")
    if not bundle['placements'] and ('placements', 'NO_PLACEMENT') not in paths:
        errors.append('no placement and no NO_PLACEMENT row')
    return errors


def bundle_paths(targets):
    for target in targets:
        path = Path(target)
        if path.is_dir():
            yield from sorted(p for p in path.rglob('*.json') if p.name != 'index.json')
        else:
            yield path


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('targets', nargs='+')
    parser.add_argument('--allow-text', action='store_true')
    args = parser.parse_args()
    validator = Draft202012Validator(json.loads((ROOT / 'npc.schema.json').read_text(encoding='utf-8')))
    checked = failed = 0
    for path in bundle_paths(args.targets):
        bundle = json.loads(path.read_text(encoding='utf-8'))
        errors = [f'{"/".join(map(str, e.absolute_path))}: {e.message}' for e in validator.iter_errors(bundle)]
        if not errors:
            errors = semantic_errors(bundle, args.allow_text)
        checked += 1
        if errors:
            failed += 1
            print(f'{path}:')
            for error in errors[:10]:
                print(f'  {error[:300]}')
    print(f'{checked - failed}/{checked} bundles valid')
    return 1 if failed or not checked else 0


if __name__ == '__main__':
    sys.exit(main())
