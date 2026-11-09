"""Pinned physical Item qualification for the finite batch-three NPC/action inputs.

Accepted native cohorts keep authority. New plain portable Items reuse the source
importer and its exact bindings; fluid, charge, decay and custom use policies are
not inferred. The retained client bank supplies flags, Crystal 201189f the XML.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
from pathlib import Path
from xml.parsers import expat

import grizzly_shop_profiles as grizzly
import quest_reward_item_semantics as reward

ROOT = Path(__file__).resolve().parents[2]
PATH = 'tools/content-schema/native-gameplay/quest-batch3-item-profiles.json'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
XML_SHA256 = '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509'
SCHEMA = 'OTERYN_NATIVE_QUEST_BATCH3_ITEM_PROFILES/v1'
SOURCE = f'zimbadev/crystalserver@{REVISION}:data/items/items.xml:'
NATIVE = 'content/items/definitions/spell-native-profiles.json'
# Closed publication scope. Working-tree experiments are not source inputs;
# notably batch3_c/boss_rooms.json belongs to the withdrawn owner candidate.
# Individual quest commits may add their approved file later; an absent file
# does not create a dependency on another quest's unpublished work.
PUBLISHED_AUTHORING = tuple(
    f'tools/content-schema/quest-authoring/{group}/{relative}'
    for group, paths in (
        ('batch3_a', (
            'native/hot_cuisine.json',
            'npc/bigfoot_burden.json', 'npc/fathers_burden.json',
            'npc/hot_cuisine.json', 'npc/in_service_of_yalahar.json',
            'npc/the_cursed_crystal.json', 'npc/the_hidden_city_of_beregar.json',
            'npc/the_way_of_the_monk.json',
            'world/bigfoot_burden.json', 'world/in_service_of_yalahar.json',
            'world/the_cursed_crystal.json', 'world/the_hidden_city_of_beregar.json',
            'world/the_way_of_the_monk.json',
        )),
        ('batch3_b', (
            'held/a_pirates_tail_quest-world.json',
            'held/hero_of_rathleton-presentation.json', 'held/hero_of_rathleton-world.json',
            'held/marapur-world.json', 'held/the_shattered_isles_quest-presentation.json',
            'held/the_shattered_isles_quest-world.json',
            'npc/a_pirates_tail_quest.json', 'npc/hero_of_rathleton.json',
            'npc/oramond.json', 'npc/the_shattered_isles_quest.json',
            'world/a_pirates_tail_quest.json', 'world/hero_of_rathleton.json',
            'world/marapur.json', 'world/the_shattered_isles_quest.json',
        )),
        ('batch3_c', (
            'deaths/heart_of_destruction.json', 'deaths/the_order_of_the_stag.json',
            'deaths/the_primal_ordeal.json', 'deaths/tower_defence_quest.json',
            'gnomadness_native.json', 'heart_of_destruction_deaths.json',
            'held-world/cradle_of_monsters.json', 'held-world/heart_of_destruction.json',
            'held-world/the_order_of_the_stag.json',
            'held-world/the_primal_ordeal.json',
            'npc/the_order_of_the_stag.json', 'npc/tower_defence_quest.json',
            'stag_death.json', 'world/the_primal_ordeal.json',
        )),
    )
    for relative in paths
)
EXTRA_INPUTS = (
    'content/dialogues/quest-source/quest-batch3-a-bigfoot-burden.json',
    'content/dialogues/quest-source/quest-batch3-a-the_cursed_crystal.json',
    'content/dialogues/quest-source/quest-batch3-a-the_way_of_the_monk.json',
)
ANSWER = 'https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589517205'
# The root-owned quest chest cohort, read at the pinned donor lines. These are
# literal Item identities; charged tuple counts retain their existing holds.
EXTRA_ITEM_LINES = {
    3554: (802,), 3387: (809,), 3420: (816,),
    3052: (1599,), 3085: (1599,), 3083: (1837,), 3029: (1837,),
    3097: (1837,), 2853: (1597, 1835),
}


def qualify(item_id, node, appearance, physical, bindings, definition, *, hold_reasons=()):
    """Qualify physical custody only, preserving every existing instance hold."""
    if hold_reasons:
        raise ValueError('ACCEPTED_SEMANTIC_HOLD:' + ','.join(sorted(hold_reasons)))
    flags = appearance['flags']
    if flags.get('flags.liquidcontainer') or flags.get('flags.liquidpool'):
        raise ValueError('FLUID_SUBTYPE_OWNER_REQUIRED')
    attributes = grizzly.source.attrs(node)
    if {'charges', 'duration', 'decayto', 'stopduration'} & attributes.keys():
        raise ValueError('INSTANCE_STATE_OWNER_REQUIRED')
    return grizzly.qualify(item_id, node, appearance, physical, bindings, definition)


def descriptor(root, path):
    return {'path': path.relative_to(root).as_posix(),
            'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}


def item_references(value, pointer='', sources=()):
    """Read explicit canonical Item identities, never names or arbitrary numbers."""
    if isinstance(value, dict):
        sources = value.get('sources', sources)
        if value.get('kind') == 'uniform_choice':
            for index, item_id in enumerate(value.get('items_by_choice', [])):
                if not isinstance(item_id, int) or not 0 < item_id <= 65535:
                    raise ValueError('SOURCE_CHOICE_ITEM_ID')
                yield f'oteryn:item.tibia.i{item_id}', pointer + f'/items_by_choice/{index}', list(sources)
        for key, child in value.items():
            yield from item_references(child, pointer + '/' + str(key), sources)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from item_references(child, pointer + '/' + str(index), sources)
    elif isinstance(value, str) and re.fullmatch(r'oteryn:item\.tibia\.i[1-9][0-9]*', value):
        yield value, pointer, list(sources)


def literal_offer_items(document):
    """Gnomally's retained table is source evidence, not an admitted shop owner."""
    for entry in document.get('gnomally_offers_literal_table', {}).get('fields', []):
        for field in entry['value']['fields']:
            if field['key'] == 'itemid':
                yield field['value'], field['line']
            elif field['key'] == 'token':
                for token_field in field['value']['fields']:
                    if token_field['key'] == 'id':
                        yield token_field['value'], token_field['line']


def selected(root):
    inputs = sorted(root / path for path in PUBLISHED_AUTHORING if (root / path).exists())
    inputs.extend(root / path for path in EXTRA_INPUTS if (root / path).exists())
    references, descriptors = {}, []
    for path in inputs:
        document = json.loads(path.read_bytes())
        descriptors.append(descriptor(root, path))
        for key, pointer, sources in item_references(document):
            references.setdefault(key, []).append({
                'path': path.relative_to(root).as_posix(), 'pointer': pointer, 'sources': sources})
        for item_id, line in literal_offer_items(document):
            if not isinstance(item_id, int) or not 0 < item_id <= 65535:
                raise ValueError('GNOMALLY_LITERAL_ITEM_ID')
            references.setdefault(f'oteryn:item.tibia.i{item_id}', []).append({
                'path': path.relative_to(root).as_posix(),
                'pointer': '/gnomally_offers_literal_table',
                'sources': [f'crystal:{REVISION}:data-global/npc/gnomally.lua:{line}']})
    for item_id, lines in sorted(EXTRA_ITEM_LINES.items()):
        references.setdefault(f'oteryn:item.tibia.i{item_id}', []).append({
            'path': 'tools/content-migration/quest_batch3_item_profiles.py',
            'pointer': f'/EXTRA_ITEM_LINES/{item_id}',
            'sources': [f'crystal:{REVISION}:data-global/startup/tables/chest.lua:{line}' for line in lines]})
    descriptors.append(descriptor(root, Path(__file__).resolve()))
    if not references:
        raise ValueError('QUEST_BATCH3_EMPTY_ITEM_SET')
    return references, descriptors


def xml_lines(raw):
    result = {}
    parser = expat.ParserCreate()

    def start(name, attributes):
        if name != 'item':
            return
        low = int(attributes.get('id', attributes.get('fromid', 0)))
        high = int(attributes.get('id', attributes.get('toid', 0)))
        for item_id in range(low, high + 1):
            if item_id in result:
                raise ValueError('DUPLICATE_XML_LINE_BINDING')
            result[item_id] = parser.CurrentLineNumber

    parser.StartElementHandler = start
    parser.Parse(raw, True)
    return result


def capture(source_root, root=ROOT, *, additional_inputs=()):
    references, inputs = selected(root)
    for relative in additional_inputs:
        path = root / relative
        document = json.loads(path.read_bytes())
        inputs.append(descriptor(root, path))
        for key, pointer, sources in item_references(document):
            references.setdefault(key, []).append({
                'path': relative, 'pointer': pointer, 'sources': sources})
    selected_keys = set(references)
    xml_raw = (source_root / 'items.xml').read_bytes()
    client_raw = (root / reward.CLIENT).read_bytes()
    if (hashlib.sha256(xml_raw).hexdigest() != XML_SHA256
            or hashlib.sha256(client_raw).hexdigest() != reward.CLIENT_SHA
            or len(client_raw) != reward.CLIENT_BYTES):
        raise ValueError('QUEST_BATCH3_SOURCE_PIN')
    source = grizzly.source
    xml = source.xml_index(xml_raw)
    lines = xml_lines(xml_raw)
    appearances = source.engine_items.load_appearance_objects(client_raw)
    item_ids = {int(key.rsplit('i', 1)[1]) for key in selected_keys}
    available_ids = item_ids & xml.keys() & appearances.keys()
    physical = {row['server_item_id']: row
                for row in source.policies(client_raw, xml_raw, available_ids)}
    binding_path = root / 'imports/crystalserver/bindings/items.json'
    bindings = source.binding_index(binding_path.read_bytes())
    definitions, canonical_inputs = {}, []
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        matches = {row['definition']['identity']['key']: row['definition']
                   for row in json.loads(path.read_bytes())['records']
                   if row['definition']['identity']['key'] in selected_keys}
        if matches:
            if definitions.keys() & matches.keys():
                raise ValueError('QUEST_BATCH3_CANONICAL_DUPLICATE')
            definitions.update(matches)
            canonical_inputs.append(descriptor(root, path))
    # The existing admission tool verifies its pinned snapshots and retained facts.
    # Accepted holds are constraints, including when an Item already has a profile.
    previous_admissions = reward.load_admissions(root)
    native_document = json.loads((root / NATIVE).read_bytes())
    native = {row['production_definition']['production_key']: row
              for row in native_document['records']}
    records, classification = [], []
    for item_id in sorted(item_ids):
        key = f'oteryn:item.tibia.i{item_id}'
        definition = definitions.get(key)
        row = {'key': key, 'source': SOURCE + str(lines.get(item_id, 'NIEZNANE')),
               'uses': references[key],
               'canonical_materializable': definition.get('materializable') if definition else None,
               'existing_native_profile': key in native,
               'holds': previous_admissions.get(key, ({},))[0].get('holds', [])}
        try:
            if definition is None or item_id not in available_ids:
                raise ValueError('MISSING_SOURCE_OR_CANONICAL_ITEM')
            profile = qualify(item_id, xml[item_id], appearances[item_id], physical[item_id],
                              bindings, definition, hold_reasons=row['holds'])
            existing = native.get(key)
            if existing is not None and (
                any(existing[field] != profile[field] for field in
                    ('production_definition', 'production_binding', 'admission'))
                or any(grizzly.known_conflict(existing['semantics'][field], profile['semantics'][field])
                       for field in ('physical', 'stack', 'container'))
            ):
                raise ValueError('ACCEPTED_NATIVE_PROFILE_CONFLICT')
            records.append(profile)
            row['classification'] = 'EXISTING_NATIVE' if existing else 'QUALIFIED_PLAIN_PORTABLE'
            row['reason'] = None
        except ValueError as error:
            row['classification'] = 'HELD'
            row['reason'] = str(error)
        classification.append(row)
    return {
        'schema': SCHEMA, 'producer': 'tools/content-migration/quest_batch3_item_profiles.py',
        'runtime_enabled': False,
        'purpose': 'Physical qualification only; no owner extension, canonical mutation or use-effect inference.',
        'decision': ANSWER, 'source_revision': REVISION,
        'sources': [
            {'repository': 'zimbadev/crystalserver', 'revision': REVISION,
             'path': 'data/items/items.xml', 'sha256': XML_SHA256},
            {'path': reward.CLIENT, 'sha256': reward.CLIENT_SHA, 'bytes': reward.CLIENT_BYTES},
        ],
        'authoring_inputs': inputs, 'canonical_inputs': canonical_inputs,
        'binding_input': descriptor(root, binding_path),
        'accepted_inputs': [descriptor(root, root / path) for path in
                            (NATIVE, reward.PACKET, reward.EXTRA_PACKET)],
        'records': records, 'classification': classification,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, required=True,
                        help='Read-only directory containing pinned Crystal items.xml.')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    result = capture(args.source_root)
    raw = grizzly.encode(result)
    target = ROOT / PATH
    if args.check:
        if not target.exists() or target.read_bytes() != raw:
            raise SystemExit('QUEST_BATCH3_ITEM_PROFILE_DRIFT')
    else:
        target.write_bytes(raw)
    counts = {kind: sum(row['classification'] == kind for row in result['classification'])
              for kind in ('QUALIFIED_PLAIN_PORTABLE', 'EXISTING_NATIVE', 'HELD')}
    print(json.dumps(counts, sort_keys=True))


def admit_recipes(root, generated, profile_path, manifest_path, order, canonical):
    """Admit source-qualified plain cooking operands, preserving all state holds.

    The quest owner authorizes native custody for these recipe operands. The
    captured packet remains qualification evidence, not blanket admission of
    other batch-three quests, timed corpses or fluid/charge semantics.
    """
    packet = json.loads((root / PATH).read_bytes())
    recipe_bytes = (root / 'content/quests/hot-cuisine-batch3.json').read_bytes()
    recipe_source = 'tools/content-schema/quest-authoring/batch3_a/native/hot_cuisine.json'
    recipe_pin = next(row for row in packet['authoring_inputs'] if row['path'] == recipe_source)
    if (recipe_bytes != (root / recipe_source).read_bytes()
            or hashlib.sha256(recipe_bytes).hexdigest() != recipe_pin['sha256']):
        raise ValueError('NPC_RECIPE_SOURCE_DRIFT')
    recipes = json.loads(recipe_bytes)
    selected_keys = {
        effect['item'] for recipe in recipes['recipes']
        for effect in [*recipe['ingredients'], recipe['reward']]
    }
    classifications = {row['key']: row for row in packet['classification']}
    profiles = {row['production_definition']['production_key']: row for row in packet['records']}
    if packet['schema'] != SCHEMA or packet['source_revision'] != REVISION:
        raise ValueError('NPC_RECIPE_PROFILE_SOURCE')
    # Other quests share captured shards. Keep their evidence unchanged while
    # proving every consumed cooking definition still matches its pinned capture.
    import m42_cooking_canonical
    for descriptor in packet['canonical_inputs']:
        m42_cooking_canonical.verify(root, descriptor, selected_keys)
    descriptor = packet['binding_input']
    if hashlib.sha256((root / descriptor['path']).read_bytes()).hexdigest() != descriptor['sha256']:
        raise ValueError('NPC_RECIPE_BINDING_DRIFT')
    document = json.loads(generated[profile_path])
    present = {row['production_definition']['production_key']: row for row in document['records']}
    for key in sorted(selected_keys):
        classification = classifications.get(key)
        if classification is None:
            raise ValueError('NPC_RECIPE_UNQUALIFIED_ITEM:' + key)
        if classification['classification'] == 'HELD':
            continue  # State-dependent Items require their own exact native owner.
        record = profiles.get(key)
        if record is None or classification['holds']:
            raise ValueError('NPC_RECIPE_PROFILE_CUSTODY:' + key)
        existing = present.get(key)
        if existing is None:
            document['records'].append(copy.deepcopy(record))
        elif (any(existing[field] != record[field] for field in
                  ('production_definition', 'production_binding', 'admission'))
              or any(grizzly.known_conflict(existing['semantics'][field], record['semantics'][field])
                     for field in ('physical', 'stack', 'container'))):
            raise ValueError('NPC_RECIPE_EXISTING_PROFILE_CONFLICT:' + key)
    document['records'].sort(key=order)
    data = grizzly.encode(document)
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    main()
