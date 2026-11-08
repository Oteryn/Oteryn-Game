"""Retain source-qualified quest Item facts through tree regeneration.

Crystal201189f rookgaard/rapier_quest.lua:4-11 grants one 3272. Canonical Item
facts already prove its 15 oz weight, portability and nonstackability. Preserve
all existing equipment/weapon facts and reject contradictory core facts.
"""

import hashlib
import importlib.util
import json
from pathlib import Path

SMALL_SAPPHIRE_SOURCE_SHA256 = '229eb3979fe64d69d726104122a3d76e2c281902777ef151cc796834c38a4291'


def apply_admissions(definitions):
    matches = [d for d in definitions if d['identity']['key'] == 'oteryn:item.tibia.i3272']
    if len(matches) != 1:
        raise ValueError('Rapier canonical identity missing or duplicated')
    item = matches[0]
    physical = item['semantics']['physical']
    stack = item['semantics']['stack']
    known = lambda value: {'state': 'KNOWN', 'value': value}
    if (item['identity'] != {'family': 'Item', 'key': 'oteryn:item.tibia.i3272',
                            'revision': 'definition-r1'}
            or physical != known({'weight': known(1500), 'movable': known(True),
                                  'pickupable': known(True)})
            or stack.get('state') != 'KNOWN'
            or stack['value']['stackable'] != known(False)
            or (item['materializable'], item['stack_class']) not in
               ((False, 'Unknown'), (True, 'NonStackable'))):
        raise ValueError('Rapier core facts changed')
    item['materializable'] = True
    item['stack_class'] = 'NonStackable'
    # Crystal201189f lee_delle.lua:149 and willie.lua:272. These two
    # existing canonical equipment definitions already carry exact portable facts.
    # Small Sapphire's certificate likewise has exact 2 oz portable paper facts
    # (TibiaWiki Document (Certificate), captured in small_sapphire_source.json).
    for ident, weight in ((3362, 2600), (3426, 5900), (637, 200)):
        key = f'oteryn:item.tibia.i{ident}'
        candidates = [row for row in definitions if row['identity']['key'] == key]
        if len(candidates) != 1:
            raise ValueError(f'R–S reward missing or duplicated: {key}')
        reward = candidates[0]
        if ident == 637:
            source = Path(__file__).resolve().parents[2] / 'tools/content-schema/reward-claim-authoring/small_sapphire_source.json'
            raw = source.read_bytes()
            if hashlib.sha256(raw).hexdigest() != SMALL_SAPPHIRE_SOURCE_SHA256:
                raise ValueError('Small Sapphire source packet changed')
            proof = json.loads(raw)['certificate_stack_evidence']
            if proof['object_id'] != 637 or proof['flags_cumulative'] is not False:
                raise ValueError('Certificate source stack fact changed')
            if reward['semantics']['stack'] == {'state': 'UNKNOWN'}:
                reward['semantics']['stack'] = known({
                    'stackable': known(False), 'stack_max': {'state': 'NOT_APPLICABLE'}})
        if (reward['identity'] != {'family': 'Item', 'key': key, 'revision': 'definition-r1'}
                or reward['semantics']['physical'] != known({
                    'weight': known(weight), 'movable': known(True), 'pickupable': known(True)})
                or reward['semantics']['stack'].get('state') != 'KNOWN'
                or reward['semantics']['stack']['value']['stackable'] != known(False)
                or (reward['materializable'], reward['stack_class']) not in
                   ((False, 'Unknown'), (True, 'NonStackable'))):
            raise ValueError(f'R–S reward core facts changed: {key}')
        reward['materializable'] = True
        reward['stack_class'] = 'NonStackable'

    # Source chest.lua:1780-1786 / :1949-1955 gives full definition charges.
    # Absence of a timed row means these full values under TIMED-ITEM-0.
    for ident, weight, charges in ((3054, 500, 200), (3084, 550, 250)):
        key = f'oteryn:item.tibia.i{ident}'
        candidates = [row for row in definitions if row['identity']['key'] == key]
        if len(candidates) != 1:
            raise ValueError(f'R–S charged reward missing or duplicated: {key}')
        reward = candidates[0]
        if (reward['identity'] != {'family': 'Item', 'key': key, 'revision': 'definition-r1'}
                or reward['semantics']['physical'] != known({
                    'weight': known(weight), 'movable': known(True), 'pickupable': known(True)})
                or reward['semantics']['charges'] != known({'count': known(charges)})
                or reward['semantics']['stack'].get('state') != 'KNOWN'
                or reward['semantics']['stack']['value']['stackable'] != known(False)
                or reward['stack_class'] != 'NonStackable'):
            raise ValueError(f'R–S charged core facts changed: {key}')
        reward['materializable'] = True

    # Retained Crystal NPC sources: aurita.lua:81-88, taegen.lua:174-188,
    # valindara.lua:75-90 at tree9f5a72c6. Existing canonical physical and
    # nonstacking facts are complete; preserve all other semantics verbatim.
    # Santiago Crystal201189f santiago.lua:145-171 also requires coat3562 and grants club3270.
    # Retained Crystal9f5a72c ikassis.lua:77-85 grants the portable 2 oz talisman25295.
    for ident, weight in ((25731, 1200), (25733, 1200), (25779, 400), (3270, 2500), (3562, 2700), (25730, 200), (25732, 200), (25734, 200), (25780, 800), (25782, 10),
                          (25975, 200), (25976, 200), (25977, 200), (25295, 200), (3452, 1500)):
        key = f'oteryn:item.tibia.i{ident}'
        candidates = [row for row in definitions if row['identity']['key'] == key]
        if len(candidates) != 1:
            raise ValueError(f'Threatened reward missing or duplicated: {key}')
        reward = candidates[0]
        if (reward['identity'] != {'family': 'Item', 'key': key, 'revision': 'definition-r1'}
                or reward['semantics']['physical'] != known({
                    'weight': known(weight), 'movable': known(True), 'pickupable': known(True)})
                or reward['semantics']['stack'].get('state') != 'KNOWN'
                or reward['semantics']['stack']['value']['stackable'] != known(False)
                or (reward['materializable'], reward['stack_class']) not in
                   ((False, 'Unknown'), (True, 'NonStackable'))):
            raise ValueError(f'Threatened reward core facts changed: {key}')
        if ident == 25780 and reward['semantics']['container'] != known({'capacity': known(24)}):
            raise ValueError('Threatened blossom bag capacity changed')
        reward['materializable'] = True
        reward['stack_class'] = 'NonStackable'

    # Tutorial corpse/food facts share this retained Item materializer seam.
    spec = importlib.util.spec_from_file_location(
        'questpc3_tutorial_item_admission',
        Path(__file__).with_name('questpc3_tutorial_item_admission.py'))
    tutorial = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(tutorial)
    tutorial.apply_admissions(definitions)


    # Only these missing portable teeth: retained observation hashes qualify
    # stackability while canonical physical facts remain authoritative.
    tooth_facts = {
        10196: (100, True, 'bbbe5725f7933524d629ef908a5fc8351dc99ebe188c43db0864c688ad579f53'),
        10275: (88, True, '8f121093abd38c5642bdca74357b71e6da9bb83f02fae108195365750cd14892'),
        22052: (160, True, '87237a4b313c69127bc8351f060e5d25582fc89ce8985f8d1db32df67bd2329c'),
        22649: (200, False, '3bb14ba4447e0cbc00897bd0f256c30be60638bed08579ec6eb98f0d37be81c1'),
        5893: (500, True, 'eaaabb6611f391323ddd23141a18992e2d9abf0718e005ea28447de562500344'),
        # Retained Raven Herb page57403/rev1115118, exact1oz portable plant.
        5953: (100, True, 'bcdfc8af1914aa157665ddfa477c2e0d8bb81ed3b670bc324b2f066134692125'),
    }
    source = Path(__file__).resolve().parents[2] / 'imports/tibiawiki/facts/items-stats.json'
    observations = json.loads(source.read_bytes())['records']
    for ident, (weight, stackable, digest) in tooth_facts.items():
        key = f'oteryn:item.tibia.i{ident}'
        candidates = [row for row in definitions if row['identity']['key'] == key]
        evidence = observations[key]
        if len(candidates) != 1 or evidence['item_id'] != ident or len(evidence['observations']) != 1:
            raise ValueError(f'Threatened tooth identity changed: {key}')
        observation = evidence['observations'][0]
        if (observation['content_sha256'] != digest
                or observation['fields']['stackable'].lower() != ('yes' if stackable else 'no')
                or observation['fields']['weight'] != f'{weight / 100:.2f}'):
            raise ValueError(f'Threatened tooth source changed: {key}')
        item = candidates[0]
        target_class = 'StackCapable' if stackable else 'NonStackable'
        expected_stack = known({'stackable': known(stackable),
                                'stack_max': known(100) if stackable else {'state': 'NOT_APPLICABLE'}})
        old_stack = item['semantics']['stack']
        compatible_stack = old_stack == {'state': 'UNKNOWN'} or old_stack == expected_stack
        if old_stack.get('state') == 'KNOWN':
            compatible_stack |= old_stack == known({'stackable': known(stackable), 'stack_max': {'state': 'UNKNOWN'}})
        if (item['identity'] != {'family': 'Item', 'key': key, 'revision': 'definition-r1'}
                or item['semantics']['physical'] != known({'weight': known(weight),
                    'movable': known(True), 'pickupable': known(True)})
                or not compatible_stack
                or (item['materializable'], item['stack_class']) not in
                   ((False, 'Unknown'), (True, target_class))):
            raise ValueError(f'Threatened tooth core facts changed: {key}')
        item['semantics']['stack'] = expected_stack
        item['materializable'] = True
        item['stack_class'] = target_class


    # Four source lava trophies reuse the same canonical admission seam.
    spec = importlib.util.spec_from_file_location(
        'questpc3_first_dragon_items', Path(__file__).with_name('questpc3_first_dragon_items.py'))
    trophies = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(trophies)
    trophies.apply_admissions(definitions)

    # Tainted source talismans: custody admission is independent of crafting.
    spec = importlib.util.spec_from_file_location(
        'questpc3_tainted_items', Path(__file__).with_name('questpc3_tainted_items.py'))
    talismans = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(talismans)
    talismans.apply_admissions(definitions)


def refresh_tree(root, check=False):
    """Apply strict admissions to existing canonical shards without rebuilding World."""
    index = json.loads((root / 'content/items/index.json').read_text())
    documents = {}
    definitions = []
    for relative in index['shards']:
        document = json.loads((root / relative).read_text())
        documents[relative] = document
        definitions.extend(row['definition'] for row in document['records'])
    apply_admissions(definitions)
    changed = []
    for relative, document in documents.items():
        target = root / relative
        data = json.dumps(document, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n'
        if target.read_text() != data:
            changed.append(relative)
            if not check:
                target.write_text(data)
    if check and changed:
        raise ValueError('Quest Item admission drift: ' + ', '.join(changed))
    return changed


def main():
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['content'])
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    changed = refresh_tree(Path(__file__).resolve().parents[2], args.check)
    print(f'PASS quest Item admissions changed={len(changed)} check={args.check}')


if __name__ == '__main__':
    main()
