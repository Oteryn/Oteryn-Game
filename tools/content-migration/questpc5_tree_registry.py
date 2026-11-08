"""Reconcile selected tree-first family counts after merging independent admissions."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FAMILIES = {'Item': 'content/items/index.json',
            'RewardClaim': 'content/interactions/reward_claims/index.json'}


def outputs(root=ROOT, generated=None):
    generated = generated or {}
    read = lambda path: json.loads(generated[path] if path in generated else (root / path).read_bytes())
    paths = ('content/manifest.json', 'content/content.lock.json')
    manifest, lock = [read(path) for path in paths]
    for family, path in FAMILIES.items():
        index = read(path)
        count = index.get('record_count')
        if (index.get('family') != family or type(count) is not int or count < 0
                or manifest['families'][family]['index'] != path):
            raise ValueError('TREE_REGISTRY_INDEX:' + family)
        manifest['families'][family]['records'] = count
        lock['family_counts'][family] = count
        if family == 'Item':
            bindings = index.get('attached_source_bindings')
            if type(bindings) is not int or bindings < 0:
                raise ValueError('TREE_REGISTRY_INDEX:Item bindings')
            lock['source_binding_counts'][family] = bindings
    return {path: (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2)+'\n').encode()
            for path, value in zip(paths, (manifest, lock))}


if __name__ == '__main__':
    import sys
    for name, raw in outputs().items():
        path = ROOT / name
        if '--check' in sys.argv:
            if path.read_bytes() != raw:
                raise SystemExit('stale tree registry: ' + name)
        elif path.read_bytes() != raw:
            path.write_bytes(raw)
