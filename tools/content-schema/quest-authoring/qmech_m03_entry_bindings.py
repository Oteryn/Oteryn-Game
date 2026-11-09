"""M03: source-position entrances with chosen live Character prerequisites.

Crystal 201189f: data-global/scripts/quests/25_years_of _tibia_quest/
actions_time_machine.lua:1-11,26-49 supplies both machine areas and destinations.
Chosen requirements: definitions/quests-00200-00299.json:1 record67 and
quests-00000-00099.json:1 record89. These prerequisites are not claimed as Lua rules.
PoI entry anchor: https://www.tibiawiki.com.br/wiki/Pits_of_Inferno_Quest
(the explicit 32795,32335,8 entrance to Goshnar's tomb).
"""
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
PIN = 'crystal:201189f871d23597b82caf88fd611454a2ddefb8:'
MACHINE = 'data-global/scripts/quests/25_years_of _tibia_quest/actions_time_machine.lua:1-11,26-49'


def callbacks(root=ROOT):
    root = Path(root)
    sys.path.insert(0, str(root / 'tools/content-schema/world-authoring'))
    import world_region_codec as codec
    index = json.loads((root / 'content/world/placements/index.json').read_text())
    regions = {}

    def native(x, y, z, appearance):
        path = root / 'content/world/placements' / codec.region_name(z, x // 256, y // 256)
        if path not in regions:
            regions[path] = {(a, b): (house, entries)
                             for _, tiles in codec.decode_region(path.read_bytes())[3]
                             for a, b, _, house, _, entries in tiles}
        house, entries = regions[path].get((x, y), (None, []))
        bare = [(index['palette'][palette]['source_item_id'], attrs)
                for palette, depth, attrs in entries if depth == 0]
        matches = [i for i, (source, attrs) in enumerate(bare)
                   if source == appearance and not attrs]
        if house != 0 or len(matches) != 1:
            raise ValueError('M03 needs one retained bare native object at ' + str((x, y, z, appearance)))
        return dict(position=dict(x=x, y=y, floor=-z), ordinal=matches[0], appearance=appearance)

    def entry(name, quest, binding, trigger, *, destination=None, requirements=None, level=None, source):
        row = dict(key='oteryn:quest-callback/m03/' + name, quest='oteryn:quest.' + quest,
                   trigger=trigger, selector=dict(position=binding['position'], native=binding,
                   unique=None, action=None, appearance=binding['appearance']),
                   lever=None, reversible=False, passages=[], teleport=destination,
                   storage=[], offerings=[], return_on_failure=requirements is not None,
                   failure_message=None, sources=source)
        if requirements:
            row.update(minimum_level=level, character_requirements=requirements,
                       requires_world_admission=True,
                       storage=[dict(track='oteryn:quest-progress/chosen-source/' + quest + '/s1',
                                     minimum=0, maximum=1, set=1, at_least=True)])
        return row

    blessing_catalogue = json.loads((root / 'rulesets/character/blessings/catalogue.json').read_text())
    blessings = sorted(b['storage_key'] for b in blessing_catalogue['blessings'] if b['price_family'] == 'regular')
    if len(blessings) != 5:
        raise ValueError('M03 requires the admitted five regular blessings')
    for file, record, quest, level, prerequisites in [
        ('quests-00200-00299.json', 67, '25_years_of_tibia_quest', 20,
         ['All regular blessings', 'Vocation promotion']),
        ('quests-00000-00099.json', 89, 'the_pits_of_inferno_quest', 80,
         ['Rope', 'Shovel']),
    ]:
        definition = json.loads((root / 'content/quests/definitions' / file).read_text())['records'][record]['definition']
        recipe = definition.get('recipe') or definition['oteryn_recipe']['payload']['recipe']
        if (definition['identity']['key'] != 'oteryn:quest.' + quest
                or recipe['requirements']['min_level'] != level
                or recipe['requirements']['premium'] is not True
                or recipe['requirements']['prerequisites'] != prerequisites
                or recipe['stages'][0]['kind'] != 'explore'):
            raise ValueError('M03 chosen entry recipe changed: ' + quest)
    guards = dict(premium=True, promoted=True, blessings=blessings)
    rows = []
    for n in range(9):
        rows.append(entry('time-machine-entry-' + str(n), '25_years_of_tibia_quest',
                          native(32429 + n % 3, 32164 + n // 3, 8, 25091 + n), 'use',
                          destination=dict(x=33453, y=31030, floor=-8), requirements=guards,
                          level=20, source=[PIN + MACHINE,
                          'content/quests/definitions/quests-00200-00299.json#/records/67',
                          'OTERYN_CHOSEN_ENTRY_REQUIREMENTS:25-years;first-stage-counter']))
        rows.append(entry('time-machine-return-' + str(n), '25_years_of_tibia_quest',
                          native(33452 + n % 3, 31027 + n // 3, 8, 25091 + n), 'use',
                          destination=dict(x=32430, y=32167, floor=-8), source=[PIN + MACHINE]))
    rows.append(entry('pits-of-inferno-entry', 'the_pits_of_inferno_quest',
                      native(32795, 32335, 8, 354), 'step_in', requirements=dict(premium=True),
                      level=80, source=[
                      'content/quests/definitions/quests-00000-00099.json#/records/89',
                      'https://www.tibiawiki.com.br/wiki/Pits_of_Inferno_Quest',
                      'OTERYN_CHOSEN_ENTRY_REQUIREMENTS:pits-of-inferno;first-stage-counter']))
    return rows
