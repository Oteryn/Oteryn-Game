"""Bind the native arena's durable goals to its canonical chosen recipe."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
QUEST = 'oteryn:quest.barbarian_arena_quest'
PREFIX = 'oteryn:quest-progress/chosen-source/barbarian_arena_quest/'
OUTPUT = 'content/quests/arena-completion.json'


def expected(root=ROOT):
    definitions = [(path, index, row['definition'])
                   for path in sorted((Path(root) / 'content/quests/definitions').glob('quests-*.json'))
                   for index, row in enumerate(json.loads(path.read_text())['records'])]
    matches = [(path, index, definition) for path, index, definition in definitions
               if definition['identity']['key'] == QUEST]
    if len(matches) != 1:
        raise ValueError('arena completion needs one canonical Quest')
    path, index, definition = matches[0]
    recipe = definition['oteryn_recipe']['payload']['recipe']
    shape = [(stage['key'], stage['kind'], stage['count']) for stage in recipe['stages']]
    if shape != [('s1', 'talk', 1), ('s2', 'kill', 10), ('s3', 'kill', 10),
                 ('s4', 'kill', 10), ('s5', 'complete', 3)]:
        raise ValueError('arena native goals differ from the canonical recipe')
    requirements = recipe['requirements']
    if requirements['min_level'] != 0 or not requirements['premium'] or requirements['prerequisites'] != ['Barbarian Test']:
        raise ValueError('arena native admission differs from the canonical recipe')
    prerequisite = [d['identity']['key'] for _, _, d in definitions
                    if d['identity']['key'] == 'oteryn:quest.barbarian_test']
    if prerequisite != ['oteryn:quest.barbarian_test']:
        raise ValueError('Barbarian Test needs one canonical prerequisite')
    encoded = json.dumps(recipe, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
    return dict(schema=1, quest=QUEST,
                transition='oteryn:quest-transition/chosen-source/barbarian_arena_quest/completion',
                stages=[dict(track=PREFIX + stage['key'], goal=stage['count']) for stage in recipe['stages']],
                completed_rank=3,
                trophy_claims=[f'oteryn:reward.svargrond-arena.{rank}.4' for rank in range(1, 4)],
                requires_premium=True, prerequisite=prerequisite[0],
                recipe_sha256=hashlib.sha256(encoded).hexdigest(),
                sources=[path.relative_to(root).as_posix() + f'#/records/{index}',
                         'OTS_HYPOTHESIS_ONLY:Crystal ff7ede59 data-global/lib/quests/svargrond_arena.lua'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    output = ROOT / OUTPUT
    content = json.dumps(expected(), ensure_ascii=False, indent=2) + '\n'
    if args.check:
        if not output.exists() or output.read_text() != content:
            raise SystemExit('arena completion binding is stale: ' + OUTPUT)
    else:
        output.write_text(content)


if __name__ == '__main__':
    main()
