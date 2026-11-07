"""Normalize bounded source-authored leaves through the existing claim builder.

These are authoring facts with immutable witnesses, not copies of donor scripts.
"""
import hashlib
import json
import re

PATH = 'tools/content-schema/quest-authoring/native-source-claims.json'


def load(root):
    path = root / PATH
    if not path.exists():
        return [], [], None
    raw = path.read_bytes()
    document = json.loads(raw)
    if set(document) != {'schema', 'claims'} or document['schema'] != 1:
        raise ValueError('native source claims: document shape')
    rows = document['claims']
    if not isinstance(rows, list) or len(rows) > 128:
        raise ValueError('native source claims: bounded source list')
    claims, bindings, seen, cells = [], [], set(), set()
    for row in rows:
        if not isinstance(row, dict) or set(row) != {'key', 'label', 'quest', 'position', 'appearance', 'unique_id',
                        'item', 'count', 'source', 'placement_source'}:
            raise ValueError('native source claims: unknown or missing facts')
        key = row['key']
        if (not isinstance(key, str) or not re.fullmatch(
                r'crystalserver:reward-claim/[a-z0-9_/-]+', key) or key in seen):
            raise ValueError('native source claims: source identity')
        position = row['position']
        if (set(position) != {'x', 'y', 'z'}
                or any(type(position[k]) is not int for k in position)
                or not 0 <= position['x'] <= 65535
                or not 0 <= position['y'] <= 65535 or not 0 <= position['z'] <= 15):
            raise ValueError('native source claims: position')
        cell = (position['x'], position['y'], position['z'])
        if cell in cells:
            raise ValueError('native source claims: duplicate placement')
        for field in ('appearance', 'unique_id', 'item', 'count'):
            if type(row[field]) is not int or not 1 <= row[field] <= 65535:
                raise ValueError('native source claims: bounded positive source number')
        sources = [row['source'], row['placement_source']]
        for source in sources:
            if (not isinstance(source, dict)
                    or set(source) != {'repository', 'revision', 'path', 'line', 'sha256', 'url'}
                    or not all(isinstance(source[k], str) for k in ('repository', 'revision', 'path', 'sha256', 'url'))
                    or not re.fullmatch('[0-9a-f]{40}', source['revision'])
                    or not re.fullmatch('[0-9a-f]{64}', source['sha256'])
                    or type(source['line']) is not int or source['line'] < 1
                    or source['repository'] != 'zimbadev/crystalserver'
                    or not source['path'].startswith('data-global/')
                    or source['url'] != 'https://github.com/' + source['repository']
                    + '/blob/' + source['revision'] + '/' + source['path']
                    + '#L' + str(source['line'])):
                raise ValueError('native source claims: immutable source witness')
        if row['source']['revision'] != row['placement_source']['revision']:
            raise ValueError('native source claims: differing source revisions')
        if not isinstance(row['label'], str) or not row['label'] or len(row['label']) > 128:
            raise ValueError('native source claims: label')
        if not re.fullmatch(r'oteryn:quest\.[a-z0-9_.-]+', row['quest']):
            raise ValueError('native source claims: owner quest')
        revision = row['source']['revision']
        ref = lambda number: {'family': 'Item', 'key': f'crystalserver:item/{number}',
                              'revision': revision}
        claims.append({'identity': {'key': key, 'revision': revision},
            'label': row['label'], 'quest': {'family': 'Quest', 'key': row['quest'],
                                           'revision': 'quest-r1'},
            'claim': {'per': 'character', 'repeat': {'kind': 'once'}},
            'placements': [{'position': position, 'appearance': ref(row['appearance']),
                'reward': {'items': [{'item': ref(row['item']), 'count': row['count']}]}}]})
        bindings.append({'destination': key, 'position': list(cell),
                         'sources': [{'source': 'crystalserver', 'uid': row['unique_id']}]})
        seen.add(key)
        cells.add(cell)
    return claims, bindings, {'path': PATH, 'sha256': hashlib.sha256(raw).hexdigest(),
                              'record_count': len(rows)}


def item_facts(root, items, claims):
    """Qualified native overlays apply only to the added source claim cohort."""
    import copy
    keys = {f"oteryn:item.tibia.i{int(r['item']['key'].rsplit('/', 1)[1])}"
            for claim in claims for p in claim['placements'] for r in p['reward']['items']}
    result = copy.deepcopy({key: items[key] for key in keys if key in items})
    manifest = json.loads((root / 'content/spells.manifest.json').read_text())
    found = set()
    for entry in (manifest.get('item_profiles'), manifest.get('starting_loadout_item_profiles')):
        if entry is None:
            continue
        raw = (root / 'content' / entry['path']).read_bytes()
        if hashlib.sha256(raw).hexdigest() != entry['sha256']:
            raise ValueError('native source claims: stale native profile registration')
        for profile in json.loads(raw)['records']:
            identity = profile['production_definition']
            key = identity['production_key']
            if key not in keys:
                continue
            admission = profile['admission']
            if (key in found or identity['family'] != 'Item'
                    or identity['revision_ref'] != 'definition-r1'
                    or not admission['materializable']
                    or 'CharacterInventory' not in admission['legal_destinations']
                    or admission['stack_class'] not in ('NonStackable', 'StackCapable')):
                raise ValueError('native source claims: native profile not admissible or duplicated')
            if key not in result:
                raise ValueError('native source claims: native profile lacks canonical Item identity')
            result[key].update(materializable=True, stack_class=admission['stack_class'],
                               semantics=copy.deepcopy(profile['semantics']))
            found.add(key)
    return result


def callback_definition(root, native, definition):
    """Reuse qualified native facts only for the independently witnessed leaves."""
    import copy
    import grizzly_shop_profiles as accepted
    claims, _, _ = load(root)
    keys = {f"oteryn:item.tibia.i{int(r['item']['key'].rsplit('/', 1)[1])}"
            for claim in claims for placement in claim['placements']
            for r in placement['reward']['items']}
    key = definition['identity']['key']
    if key not in keys:
        return definition
    expected = {'family': 'Item', 'production_key': key,
                'revision_ref': definition['identity']['revision']}
    if (native is None or native['production_definition'] != expected
            or not native['admission']['materializable']
            or 'CharacterInventory' not in native['admission']['legal_destinations']
            or native['admission']['stack_class'] not in ('NonStackable', 'StackCapable')
            or definition['stack_class'] not in ('Unknown', native['admission']['stack_class'])):
        raise ValueError('native source claims: callback native admission')
    effective = copy.deepcopy(definition)
    effective['materializable'] = True
    effective['stack_class'] = native['admission']['stack_class']
    for group in ('physical', 'stack', 'container'):
        effective['semantics'][group] = accepted.accepted_first(
            definition['semantics'][group], native['semantics'][group])
    return effective
