#!/usr/bin/env python3
"""Refresh whole-shard pins only when every selected native Item row is unchanged.

The committed profile, its original shard digests and all selected definitions
are independent witnesses. No source terms or native facts are regenerated here.
The profile's normal producer/admission checks remain authoritative afterwards.
"""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SHARD = re.compile(r'content/items/definitions/items-[0-9]+-[0-9]+\.json\Z')
PROFILE = re.compile(r'tools/content-schema/native-gameplay/[a-z0-9_-]+\.json\Z')


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical_value(value):
    # Python's equality conflates False/0 and integer/float values; source
    # witnesses must retain their JSON types as well as their contents.
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False)


def refreshed(root, profile_path, base_reader):
    if not PROFILE.fullmatch(profile_path):
        raise ValueError('NATIVE_PROFILE_PATH')
    path = (root / profile_path).resolve()
    if not path.is_relative_to(root.resolve()):
        raise ValueError('NATIVE_PROFILE_PATH')
    base = json.loads(base_reader(profile_path))
    current = json.loads(path.read_bytes())
    base_terms, current_terms = copy.deepcopy(base), copy.deepcopy(current)
    base_inputs = base_terms.pop('canonical_inputs')
    current_terms.pop('canonical_inputs')
    if canonical_value(base_terms) != canonical_value(current_terms):
        raise ValueError('NATIVE_PROFILE_TERMS_CHANGED')
    references = {}
    for row in base['records']:
        reference = row['production_definition']
        key = reference['production_key']
        if reference['family'] != 'Item' or key in references:
            raise ValueError('NATIVE_PROFILE_ITEM_REFERENCES')
        references[key] = {'family': 'Item', 'key': key, 'revision': reference['revision_ref']}
    if not references or not base_inputs:
        raise ValueError('NATIVE_PROFILE_EMPTY_INPUTS')
    rejected = [row['key'] for row in base.get('rejected', [])]
    if len(set(rejected)) != len(rejected) or references.keys() & set(rejected):
        raise ValueError('NATIVE_PROFILE_ITEM_REFERENCES')
    selected_keys = references.keys() | set(rejected)
    old_definitions, seen_paths = {}, set()
    for entry in base_inputs:
        name = entry['path']
        if not SHARD.fullmatch(name) or name in seen_paths:
            raise ValueError('NATIVE_PROFILE_CANONICAL_PATH')
        seen_paths.add(name)
        raw = base_reader(name)
        if digest(raw) != entry['sha256']:
            raise ValueError('NATIVE_PROFILE_BASE_SHARD_PIN')
        for row in json.loads(raw)['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key in selected_keys:
                if key in old_definitions:
                    raise ValueError('NATIVE_PROFILE_DUPLICATE_BASE_ITEM')
                old_definitions[key] = definition
    if set(old_definitions) != selected_keys:
        raise ValueError('NATIVE_PROFILE_CANONICAL_CLOSURE')
    for key in rejected:
        reference = old_definitions[key]['identity']
        if reference.get('family') != 'Item' or reference.get('key') != key:
            raise ValueError('NATIVE_PROFILE_ITEM_REFERENCES')
        references[key] = reference
    definitions, inputs = {}, []
    for shard in sorted((root / 'content/items/definitions').glob('items-*.json')):
        if not shard.resolve().is_relative_to(root.resolve()):
            raise ValueError('NATIVE_PROFILE_CANONICAL_PATH')
        raw, selected = shard.read_bytes(), False
        for row in json.loads(raw)['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key in references:
                if key in definitions:
                    raise ValueError('NATIVE_PROFILE_DUPLICATE_CURRENT_ITEM')
                definitions[key] = definition
                selected = True
        if selected:
            inputs.append({'path': shard.relative_to(root).as_posix(), 'sha256': digest(raw)})
    if set(old_definitions) != references.keys() or set(definitions) != references.keys():
        raise ValueError('NATIVE_PROFILE_CANONICAL_CLOSURE')
    for key, reference in references.items():
        if (old_definitions[key]['identity'] != reference
                or canonical_value(definitions[key]) != canonical_value(old_definitions[key])):
            raise ValueError('NATIVE_PROFILE_CANONICAL_ITEM_CHANGED: ' + key)
    result = copy.deepcopy(base)
    result['canonical_inputs'] = inputs
    return (json.dumps(result, indent=2, ensure_ascii=False) + '\n').encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('profile', help='Registered native profile JSON relative to the checkout')
    parser.add_argument('--base', default='HEAD', help='Committed witnesses; no history mutation')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    commit = subprocess.check_output(
        ['git', 'rev-parse', '--verify', '--end-of-options', args.base + '^{commit}'],
        cwd=ROOT, text=True).strip()
    def read_base(path):
        return subprocess.check_output(['git', 'show', commit + ':' + path], cwd=ROOT)
    raw = refreshed(ROOT, args.profile, read_base)
    destination = ROOT / args.profile
    if args.check:
        if destination.read_bytes() != raw:
            raise SystemExit('NATIVE_PROFILE_CANONICAL_SHARD_DRIFT')
    else:
        destination.write_bytes(raw)
    print('PASS native profile shard witnesses; selected Item rows and terms unchanged; base=' + commit)


if __name__ == '__main__':
    main()
