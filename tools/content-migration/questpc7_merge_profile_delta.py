"""Verify the closed native profile union introduced by the reviewed main merge."""
import json
import subprocess

PARENT = '5c39e6be3c934499b27ff01f6c3cc63b8af46581'
PATHS = ('content/items/definitions/spell-native-profiles.json',
         'content/items/definitions/srx1-item-updates.json',
         'content/items/definitions/starting-loadout-profiles.json')
ADDITIONS = {430, 431, 1772, 1791, 1947, 1948, 2772, 2773,
             *range(3222, 3231), *range(3235, 3242), 3537, 3543,
             3589, 3590, 3593, 5102, 5103, 9125, 9126, 11544, 52824}


def verify(root, actual):
    previous = set()
    for path in PATHS:
        document = json.loads(subprocess.check_output(
            ['git', 'show', f'{PARENT}:{path}'], cwd=root))
        previous.update(row['production_definition']['production_key']
                        for row in document['records'])
    additions = {f'oteryn:item.tibia.i{number}' for number in ADDITIONS}
    if len(additions) != 35 or actual != previous | additions or previous & additions:
        raise ValueError('questpc7 main merge must preserve all profiles and add exactly 35')
