"""Admit quest rewards from their exact existing portable Item facts.

Crystal201189f rookgaard/rapier_quest.lua:4-11 grants one 3272. Canonical Item
facts already prove its 15 oz weight, portability and nonstackability. Preserve
all existing equipment/weapon facts and reject contradictory core facts.
"""

import hashlib
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
    for ident, weight in ((25731, 1200), (25733, 1200), (25779, 400)):
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
        reward['materializable'] = True
        reward['stack_class'] = 'NonStackable'


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
