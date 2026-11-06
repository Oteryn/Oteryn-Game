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
            declined = {'success_say'} & row.keys()
            storage_extensions = any(
                set(state) - {'track', 'minimum', 'maximum', 'set', 'read_only', 'set_if'}
                for state in row.get('storage', [])
            )
            failure = row.get('failure_teleport')
            if failure is not None and (
                row.get('trigger') != 'step_in' or row.get('return_on_failure') is not True
                or not isinstance(failure, dict) or set(failure) != {'x', 'y', 'floor'}
                or any(type(failure[axis]) is not int or not 0 <= failure[axis] <= 65535
                       for axis in ('x', 'y'))
                or type(failure['floor']) is not int or not -15 <= failure['floor'] <= 0
            ):
                raise ValueError(f'held callback contract extension: {row["key"]}')
            conditional_extensions = any(
                state.get('set_if') is not None and (
                    type(state['set_if']) is not int
                    or not -(2 ** 63) <= state['set_if'] <= 2 ** 63 - 1
                    or type(state.get('minimum')) is not int
                    or type(state.get('maximum')) is not int
                    or not state['minimum'] <= state['set_if'] <= state['maximum']
                    or state.get('preserve', False) or state.get('read_only', False)
                    or row.get('native_rule') is not None
                ) for state in row.get('storage', [])
            )
            if declined or storage_extensions or conditional_extensions:
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
    if len(callbacks) >= 512:
        raise ValueError('callback budget including existing Rotten Blood altar')
    return {'schema': 1, 'callbacks': sorted(callbacks, key=lambda row: row['key'])}, {'schema': SCHEMA, 'sources': sources}


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
    # QUESTPC: exact literal USEs and genuine read-only Stag guards now have
    # production owners. Historical held records remain source evidence.
    choices = {
        'content/quests/world-callbacks-held/batch3-a-the_way_of_the_monk.json': None,
        'content/quests/world-callbacks-held/batch3-b-a_pirates_tail_quest.json': {'oteryn:quest-callback/pirates-tail-cheesy-exit'},
        'tools/content-schema/quest-authoring/batch3_c/held-world/the_order_of_the_stag.json': None,
    }
    for name, keys in choices.items():
        callbacks = [row for row in read(root / name)['callbacks']
                     if keys is None or row['key'] in keys]
        expected = 1 if keys else 4
        if len(callbacks) != expected:
            raise ValueError(f'QUESTPC callback source changed: {name}')
        shards[name] = {'schema': 1, 'callbacks': callbacks}
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
