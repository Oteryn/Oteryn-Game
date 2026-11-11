"""Rehash whole Item shards only after proving selected definitions unchanged.

The Git baseline binds both the original shards and the complete source overlay.
An explicit custody mode qualifies only core facts from the unchanged captured
profile; every other selected fact, profile, source pin and refusal must match.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
from grizzly_shop_profiles import accepted_first, known_conflict

ROOT = Path(__file__).resolve().parents[2]


def qualify_custody(definition, profile):
    """Only the unchanged captured profile can qualify additive custody facts."""
    result = copy.deepcopy(definition)
    for group in ('stack', 'container'):
        if known_conflict(result['semantics'][group], profile['semantics'][group]):
            raise ValueError('ITEM_OVERLAY_CUSTODY_CONFLICT')
        result['semantics'][group] = accepted_first(
            result['semantics'][group], profile['semantics'][group])
    admission = profile['admission']
    if (not admission['materializable']
            or admission['legal_destinations'] != ['CharacterInventory', 'Ground']):
        raise ValueError('ITEM_OVERLAY_PORTABLE_CUSTODY_REQUIRED')
    if result['stack_class'] not in ('Unknown', admission['stack_class']):
        raise ValueError('ITEM_OVERLAY_CUSTODY_CONFLICT')
    result['stack_class'] = admission['stack_class']
    result['materializable'] = admission['materializable']
    return result


def qualify_native_core(definition, profile):
    """Add only core facts already present in the unchanged, Git-bound source profile."""
    result = qualify_custody(definition, profile)
    for group in ('physical', 'charges', 'temporal'):
        if known_conflict(result['semantics'][group], profile['semantics'][group]):
            raise ValueError('ITEM_OVERLAY_NATIVE_CORE_CONFLICT')
        result['semantics'][group] = accepted_first(
            result['semantics'][group], profile['semantics'][group])
    return result


def refresh(relative, root=ROOT, base='HEAD', custody=False, native_core=False):
    if (Path(relative).parts[:3] != ('tools', 'content-schema', 'native-gameplay')
            or len(Path(relative).parts) != 4 or not relative.endswith('.json')
            or not (root / relative).resolve().is_relative_to(root.resolve())):
        raise ValueError('ITEM_OVERLAY_PATH')
    base = subprocess.run(['git', '-C', str(root), 'rev-parse', '--verify',
                           '--end-of-options', base + '^{commit}'], check=True,
                          capture_output=True, text=True).stdout.strip()
    def blob(path):
        return subprocess.run(['git', '-C', str(root), 'show', f'{base}:{path}'],
                              check=True, capture_output=True).stdout
    path = root / relative
    current = json.loads(path.read_bytes())
    baseline = json.loads(blob(relative))
    if ({k: v for k, v in current.items() if k != 'canonical_inputs'}
            != {k: v for k, v in baseline.items() if k != 'canonical_inputs'}):
        raise ValueError('ITEM_OVERLAY_SOURCE_CAPTURE_DRIFT')
    keys = {r['production_definition']['production_key'] for r in baseline['records']}
    profiles = {r['production_definition']['production_key']: r for r in baseline['records']}
    if len(keys) != len(baseline['records']):
        raise ValueError('ITEM_OVERLAY_DUPLICATE_IDENTITY')
    inputs = current['canonical_inputs']
    originals = baseline['canonical_inputs']
    if len(inputs) != len(originals):
        raise ValueError('ITEM_OVERLAY_INPUT_SET')
    planned = []
    for descriptor, original in zip(inputs, originals):
        item_path = descriptor['path']
        parts = Path(item_path).parts
        if (parts[:3] != ('content', 'items', 'definitions') or len(parts) != 4
                or not item_path.endswith('.json') or item_path != original['path']
                or set(descriptor) not in ({'path', 'sha256'}, {'path', 'key', 'definition_sha256'})
                or set(descriptor) != set(original)
                or descriptor.get('key') != original.get('key')):
            raise ValueError('ITEM_OVERLAY_INPUT_PATH')
        before = blob(item_path)
        if 'sha256' in original and hashlib.sha256(before).hexdigest() != original['sha256']:
            raise ValueError('ITEM_OVERLAY_BASE_HASH')
        if not (root / item_path).resolve().is_relative_to(root.resolve()):
            raise ValueError('ITEM_OVERLAY_INPUT_PATH')
        after = (root / item_path).read_bytes()
        def selected(raw):
            rows = [r['definition'] for r in json.loads(raw)['records']
                    if r['definition']['identity']['key'] in keys]
            indexed = {d['identity']['key']: d for d in rows}
            if len(indexed) != len(rows):
                raise ValueError('ITEM_OVERLAY_DUPLICATE_SELECTED_IDENTITY')
            return indexed
        selected_before, selected_after = selected(before), selected(after)
        if 'key' in original:
            key = original['key']
            selected_before = {key: selected_before[key]}
            selected_after = {key: selected_after[key]}
            encoded = json.dumps(selected_before[key], sort_keys=True, ensure_ascii=False,
                                 separators=(',', ':')).encode()
            if hashlib.sha256(encoded).hexdigest() != original['definition_sha256']:
                raise ValueError('ITEM_OVERLAY_BASE_HASH')
        if custody or native_core:
            qualify = qualify_native_core if native_core else qualify_custody
            selected_before = {key: (qualify(value, profiles[key])
                                    if value != selected_after.get(key) else value)
                               for key, value in selected_before.items()}
        if selected_before != selected_after:
            raise ValueError('ITEM_OVERLAY_SELECTED_FACT_DRIFT')
        if 'key' in original:
            encoded = json.dumps(selected_after[original['key']], sort_keys=True,
                                 ensure_ascii=False, separators=(',', ':')).encode()
            planned.append({'key': original['key'], 'path': item_path,
                            'definition_sha256': hashlib.sha256(encoded).hexdigest()})
        else:
            planned.append({'path': item_path, 'sha256': hashlib.sha256(after).hexdigest()})
    current['canonical_inputs'] = planned
    return (json.dumps(current, indent=2, ensure_ascii=False) + '\n').encode()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('overlays', nargs='+')
    parser.add_argument('--base', default='HEAD')
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--qualify-custody-inputs', action='store_true')
    mode.add_argument('--qualify-native-core-inputs', action='store_true')
    args = parser.parse_args()
    outputs = {path: refresh(path, base=args.base, custody=args.qualify_custody_inputs, native_core=args.qualify_native_core_inputs)
               for path in args.overlays}
    for path, raw in outputs.items():
        (ROOT / path).write_bytes(raw)
        print('verified selected Item definitions against Git/source capture:', path)
