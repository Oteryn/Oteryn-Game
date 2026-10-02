#!/usr/bin/env python3
"""Materialize pinned normalized r23 test inputs; this does not qualify or activate them."""
import argparse
import hashlib
import json
from pathlib import Path


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def build(recipe_path, output):
    recipe = json.loads(recipe_path.read_bytes())
    manifest = recipe['native_manifest']
    provider_root = recipe_path.resolve(strict=True).parent.parent
    payloads = {}
    for key, binding in recipe['providers'].items():
        relative = Path(binding['source'])
        if relative.is_absolute():
            raise ValueError('Provider must be repository-relative')
        source = (recipe_path.parent / relative).resolve(strict=True)
        if not source.is_relative_to(provider_root):
            raise ValueError('Provider escapes the pinned spell evidence directory')
        raw = source.read_bytes()
        if len(raw) > 8 * 1024 * 1024:
            raise ValueError('Provider exceeds native input bound')
        if digest(raw) != binding['sha256'] or manifest[key]['sha256'] != binding['sha256']:
            raise ValueError(f'Exact normalized provider pin differs: {key}')
        name = binding['output']
        if Path(name).name != name or name in payloads or manifest[key]['path'] != name:
            raise ValueError(f'Unsafe/duplicate or substituted output path: {key}')
        payloads[name] = raw
    raw_manifest = (json.dumps(manifest, indent=2) + '\n').encode()
    if digest(raw_manifest) != recipe['expected_generated_manifest_sha256']:
        raise ValueError('Exact generated candidate manifest differs')
    output.mkdir(parents=True, exist_ok=False)
    for name, raw in payloads.items():
        (output / name).write_bytes(raw)
    (output / 'manifest.json').write_bytes(raw_manifest)
    return digest(raw_manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps({'manifest_sha256': build(Path(__file__).with_name('manifest-recipe.json'), args.out),
                      'runtime_activation': False, 'final_qualification': False}))
