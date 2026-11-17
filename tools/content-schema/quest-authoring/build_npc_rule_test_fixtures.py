"""Retain two exact source-branch fixtures for the shared pure NPC planner tests.

These fixtures are independent of the registered quest programs and are never
mounted as gameplay dialogue. Their generator selects existing reviewed source
branches; it does not synthesize a new quest or alter a donor rule.
"""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CASES = (
    ('hero_greeting', 'batch3_b/npc/hero_of_rathleton.json',
     'oteryn:quest-npc/hero_of_rathleton_quest/strange_pipe/greet-access'),
    ('father_acceptance', 'batch3_a/npc/fathers_burden.json',
     'oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-accept'),
)


def outputs():
    result = {}
    for name, path, key in CASES:
        document = json.loads((Path(__file__).parent / path).read_text())
        rules = [rule for rule in document['rules'] if rule['key'] == key]
        if len(rules) != 1:
            raise ValueError(f'NPC test source branch missing or duplicated: {key}')
        references = {entry['track'] for rule in rules
                      for entry in rule['guards'] + rule['effects'] if 'track' in entry}
        tracks = [track for track in document['tracks'] if track['key'] in references]
        if len(tracks) != len(references):
            raise ValueError(f'NPC test source track missing: {key}')
        fixture = {'schema': 1, 'quest': document['quest'], 'tracks': tracks, 'rules': rules}
        result[ROOT / 'crates/gameplay/quests/src/quest/npc_rules/test-fixtures' / f'{name}.json'] = (
            json.dumps(fixture, ensure_ascii=False, indent=2) + '\n')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for path, payload in outputs().items():
        if args.check:
            if not path.exists() or path.read_text() != payload:
                raise ValueError(f'Stale NPC source test fixture: {path}')
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(payload)
    print('2 pinned NPC source test fixtures')


if __name__ == '__main__':
    main()
