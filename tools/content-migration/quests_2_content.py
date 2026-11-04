#!/usr/bin/env python3
"""Compose reviewed batch-2 callbacks without rewriting other quests."""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CALLBACK_INPUTS = 'tools/content-schema/quest-authoring/quests-2/world-callbacks'


def read(root, path):
    return json.loads((root / path).read_bytes())


def outputs(root):
    callback_document = read(root, 'content/quests/world-callbacks.json')
    original = {entry['key']: entry for entry in callback_document['callbacks']}
    curated = {}
    for path in sorted((root / CALLBACK_INPUTS).glob('*.json')):
        packet = json.loads(path.read_bytes())
        if packet['schema'] != 1:
            raise ValueError('QUESTS_2_CALLBACK_SCHEMA')
        for entry in packet['callbacks']:
            if entry['key'] in curated or not entry['sources']:
                raise ValueError('QUESTS_2_CALLBACK_DUPLICATE_OR_SOURCE')
            curated[entry['key']] = entry
    original.update(curated)
    callback_document['callbacks'] = sorted(original.values(), key=lambda entry: entry['key'])
    return {'content/quests/world-callbacks.json': callback_document}



if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for path, value in outputs(args.root).items():
        text = json.dumps(value, ensure_ascii=False, indent=2) + '\n'
        target = args.root / path
        if args.check:
            if target.read_text() != text:
                raise SystemExit('QUESTS_2_CONTENT_DRIFT:' + path)
        else:
            target.write_text(text)
