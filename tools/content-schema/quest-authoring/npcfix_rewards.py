"""Merge NPCFIX's owned reward inputs without rewriting other lanes' bindings."""
import argparse
import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[3]
INPUT = Path(__file__).with_suffix('.json')


def encoded(value):
    return json.dumps(value, ensure_ascii=False, indent=2) + '\n'


def outputs(root=ROOT):
    source = json.loads(INPUT.read_text())
    if source['schema'] != 1:
        raise ValueError('NPCFIX_REWARD_SCHEMA')
    owners = set(source['owns'])
    callbacks = root / 'content/quests/shared-reward-callbacks.json'
    document = json.loads(callbacks.read_text()) if callbacks.is_file() else {'schema': 1, 'callbacks': []}
    document['callbacks'] = [row for row in document['callbacks']
                             if row['npc'] not in owners] + source['callbacks']
    declared = {row['key'] for row in source['tracks']}
    document['tracks'] = [row for row in document.get('tracks', [])
                          if row['key'] not in declared] + source['tracks']
    schema = json.loads((root / 'tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json').read_text())
    Draft202012Validator(schema).validate(document)
    for callback in document['callbacks']:
        if callback.get('repeatable') and (not callback.get('give') or callback.get('outfit')
                or not (callback['take'] or callback.get('cooldown'))):
            raise ValueError('NPCFIX_UNBOUNDED_REPEAT')
    result = {callbacks: encoded(document)}
    for relative, field in [('content/quests/shared-reward-offers.json', 'offers'),
                            ('content/dialogues/quests/shared-reward-greetings.json', 'npcs')]:
        path = root / relative
        current = json.loads(path.read_text()) if path.is_file() else {'schema': 1, field: []}
        current[field] = [row for row in current[field] if row['npc'] not in owners] + source[field]
        result[path] = encoded(current)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for path, value in outputs().items():
        if path.is_file() and path.read_text() == value:
            continue
        if args.check:
            raise SystemExit('NPCFIX_REWARD_DRIFT: ' + str(path.relative_to(ROOT)))
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(value)


if __name__ == '__main__':
    main()
