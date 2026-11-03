"""Compile the owned batch-3 callback shards into the existing bounded registry.

The generated index records only this batch's keys. Rebuilding never removes or
reauthors a callback belonging to another batch, including the ten qualified
mainland Monk shrines. Held source branches are deliberately outside world/.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

WORLD = 'content/quests/world-callbacks.json'
INDEX = 'content/quests/world-callbacks/batch3-index.json'
SCHEMA = 'OTERYN_QUESTS3_WORLD_INDEX/v1'
PIN = 'crystal:201189f871d23597b82caf88fd611454a2ddefb8:'


def read(path):
    return json.loads(path.read_text(encoding='utf-8'))


def compile_callbacks(existing, previous, shards):
    if existing.get('schema') != 1 or previous.get('schema') != SCHEMA:
        raise ValueError('callback/index schema')
    old_keys = {key for source in previous['sources'] for key in source['keys']}
    callbacks = [row for row in existing['callbacks'] if row['key'] not in old_keys]
    keys = {row['key'] for row in callbacks}
    sources = []
    for path, document in sorted(shards.items()):
        if document.get('schema') != 1:
            raise ValueError(f'callback source schema: {path}')
        additions = document['callbacks']
        source_keys = []
        for row in additions:
            # ANSWER R-CALLBACK-SCHEMA=B: do not silently lower a held field
            # into a different operation accepted by the current contract.
            # https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6590300355
            declined = {'failure_teleport', 'success_message', 'success_say'} & row.keys()
            storage_extensions = any(
                set(state) - {'track', 'minimum', 'maximum', 'set'}
                for state in row.get('storage', [])
            )
            if declined or storage_extensions:
                raise ValueError(f'held callback contract extension: {row["key"]}')
            if row['key'] in keys:
                raise ValueError(f'duplicate callback key: {row["key"]}')
            if not any(ref.startswith(PIN) for ref in row.get('sources', [])):
                raise ValueError(f'unpinned Crystal callback: {row["key"]}')
            keys.add(row['key'])
            source_keys.append(row['key'])
            callbacks.append(row)
        sources.append({'path': path, 'keys': sorted(source_keys)})
    # Runtime adds one existing Rotten Blood altar. Keep its reserved slot.
    if len(callbacks) >= 128:
        raise ValueError('callback budget including existing Rotten Blood altar')
    return {'schema': 1, 'callbacks': callbacks}, {'schema': SCHEMA, 'sources': sources}


def outputs(root):
    index = read(root / INDEX) if (root / INDEX).exists() else {'schema': SCHEMA, 'sources': []}
    # Each quest compiler emits only enabled rows here. Its separate authoring
    # source may also retain held branches, which must never enter the runtime.
    base = root / 'content/quests/world-callbacks'
    paths = sorted(path for group in ('a', 'b')
                   for path in base.glob(f'batch3-{group}-*.json'))
    # C's authored world/ is itself the enabled projection; its held-world/
    # sibling is outside this closed input set.
    paths += sorted((root / 'tools/content-schema/quest-authoring/batch3_c/world').glob('*.json'))
    shards = {path.relative_to(root).as_posix(): read(path) for path in paths}
    world, index = compile_callbacks(read(root / WORLD), index, shards)
    encode = lambda value: json.dumps(value, ensure_ascii=False, indent=2) + '\n'
    return {WORLD: encode(world), INDEX: encode(index)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[3]
    for name, text in outputs(root).items():
        path = root / name
        if args.check:
            if not path.exists() or path.read_text(encoding='utf-8') != text:
                raise ValueError(f'stale batch-3 callback projection: {name}')
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding='utf-8')
    print('batch-3 callback registry verified' if args.check else 'batch-3 callback registry generated')


if __name__ == '__main__':
    main()
