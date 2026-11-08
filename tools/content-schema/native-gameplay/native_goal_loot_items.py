"""Source-bound portable loot closure for the selected native goal creatures.

Reuse the accepted encounter loot/source importer. Canonical physical and combat
facts win; unknown custody, motion, charges and temporal fields require the exact
pinned XML/client/binding proof. Fresh nonstackable loot uses the existing full
native definition charge rule; no partial source subtype is rewritten.
"""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[3]


def importer(root):
    sys.path.insert(0, str(root / 'tools/content-migration'))
    import encounter_loot_item_admissions
    return encounter_loot_item_admissions


def selected(root=ROOT, creatures=None):
    if creatures is None:
        goals = json.loads((root / 'tools/content-schema/quest-authoring/samples/native-kill-goals.json').read_bytes())
        creatures = {o['creature'] for g in goals['goals'] for o in g['origins']}
    document = json.loads((root / 'tools/content-schema/native-gameplay/loot-tables.json').read_bytes())
    tables = {r['identity']['key']: r for r in document['tables']}
    keys, found = set(), set()
    def visit(entries):
        for entry in entries:
            item = entry['item']
            if item['family'] != 'Item' or item['revision'] != 'definition-r1' or not item['key'].startswith('oteryn:item.tibia.i'):
                raise ValueError('NATIVE_GOAL_LOOT_ITEM_IDENTITY')
            keys.add(int(item['key'].rsplit('.i', 1)[1]))
            visit(entry.get('children', []))
    for row in document['creature_loot']:
        if row['creature']['key'] not in creatures:
            continue
        if row['creature']['key'] in found or row['creature']['family'] != 'Creature':
            raise ValueError('NATIVE_GOAL_LOOT_CREATURE_BINDING')
        found.add(row['creature']['key'])
        if row['loot'] is not None:
            table = tables[row['loot']['key']]
            if table['identity'] != row['loot']:
                raise ValueError('NATIVE_GOAL_LOOT_TABLE_BINDING')
            visit(table['entries'])
    if found != creatures or not 1 <= len(keys) <= 512:
        raise ValueError('NATIVE_GOAL_LOOT_BOUND')
    return keys


def alternate_xml(root):
    with tarfile.open(root / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as archive:
        manifest = json.load(archive.extractfile('corpus-manifest.json'))
        rows = [r for r in manifest['files'] if r['source'] == 'crystal-summer'
                and r['revision'] == '00ce02a57ca5a12e48f32a3476e37471167e4c3f'
                and r['path'] == 'data/items/items.xml']
        if len(rows) != 1 or rows[0]['sha256'] != '13a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece':
            raise ValueError('NATIVE_GOAL_TROPHY_SOURCE')
        row = rows[0]
        raw = archive.extractfile(row.get('blob_path', row.get('cache_path'))).read()
        if hashlib.sha256(raw).hexdigest() != row['sha256']:
            raise ValueError('NATIVE_GOAL_TROPHY_SOURCE_BYTES')
        return raw


def profiles(root, ids):
    source = importer(root)
    regular = ids - {44158}
    result = source.facts(root, regular, allow_containers=True) if regular else {}
    if 44158 in ids:
        result.update(source.facts(root, {44158}, allow_containers=True, xml_override=alternate_xml(root)))
    return result


def apply_admissions(definitions, root=ROOT):
    definitions = list(definitions)
    selected_keys = {f'oteryn:item.tibia.i{i}' for i in selected(root)}
    # Existing qualified Items retain every field. Only the missing closure is authored.
    pending = [d for d in definitions if d['identity']['key'] in selected_keys and not d.get('materializable', False)]
    if pending:
        planned = copy.deepcopy(pending)
        source = importer(root)
        import grizzly_shop_profiles as shop
        # Shop custody is already source-qualified. Reuse its admission without
        # rewriting the separate frozen charge and temporal SOURCE witnesses.
        shop_keys = {r['production_definition']['production_key'] for r in
                     json.loads((root / shop.PATH).read_bytes())['records']}
        for alternate in (False, True):
            for source_core in (False, True):
                group = [d for d in planned if d['identity']['key'].endswith('.i44158') == alternate
                         and (d['identity']['key'] not in shop_keys) == source_core]
                if group:
                    ids = {int(d['identity']['key'].rsplit('.i', 1)[1]) for d in group}
                    source.apply_admissions(group, root, ids, source_core=source_core,
                        xml_override=alternate_xml(root) if alternate else None)
        for definition, replacement in zip(pending, planned):
            definition.clear()
            definition.update(replacement)


def admit_native(root, generated, profile_path, manifest_path, order, canonical, *, item_ids=None, qualified_profiles=None):
    keys = {f'oteryn:item.tibia.i{i}' for i in (selected(root) if item_ids is None else item_ids)}
    definitions = {}
    for path in (root / 'content/items/definitions').glob('items-*.json'):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            if definition['identity']['key'] in keys:
                definitions[definition['identity']['key']] = definition
    if set(definitions) != keys or any(not d['materializable'] for d in definitions.values()):
        raise ValueError('NATIVE_GOAL_LOOT_CANONICAL_CLOSURE')
    source = importer(root)
    observed_profiles = profiles(root, {int(key.rsplit('.i', 1)[1]) for key in keys}) if qualified_profiles is None else qualified_profiles
    if set(observed_profiles) != keys:
        raise ValueError('NATIVE_GOAL_LOOT_SOURCE_PROFILE_SET')
    if any(profile['production_definition'] != dict(family='Item', production_key=key, revision_ref='definition-r1')
           for key, profile in observed_profiles.items()):
        raise ValueError('NATIVE_GOAL_LOOT_SOURCE_PROFILE_IDENTITY')
    document = json.loads(generated[profile_path])
    for key in sorted(keys):
        definition, observed = definitions[key], observed_profiles[key]
        present = [r for r in document['records'] if r['production_definition']['production_key'] == key]
        if len(present) > 1:
            raise ValueError('NATIVE_GOAL_LOOT_DUPLICATE_PROFILE')
        if present:
            record = present[0]
            if record['production_binding'] != observed['production_binding']:
                raise ValueError('NATIVE_GOAL_LOOT_BINDING_CONFLICT')
        else:
            record = copy.deepcopy(observed)
            document['records'].append(record)
        existing_admitted = bool(present) and record['admission']['materializable']
        import questpc2_loot_speed_facts
        questpc2_loot_speed_facts.retain(record, observed)
        charge_fields = [d.get('charges', {'state': 'UNKNOWN'}) for d in
                         (definition['semantics'], record['semantics'], observed['semantics'])]
        if any(field.get('state') == 'KNOWN' for field in charge_fields) and not (
                charge_fields[0] == charge_fields[1] == charge_fields[2]):
            raise ValueError('NATIVE_GOAL_LOOT_EXACT_CANONICAL_NATIVE_SOURCE_CHARGES')
        for group in ('physical', 'stack', 'container', 'charges', 'temporal'):
            if group in ('charges', 'temporal') and source.accepted.known_conflict(
                    definition['semantics'][group], record['semantics'][group]):
                raise ValueError('NATIVE_GOAL_LOOT_EXACT_CHARGE_OR_TIME_REQUIRED')
            # Existing admitted physical facts belong to their original owner.
            # Canonical facts fill unknowns; this loot closure cannot retune them.
            primary, fallback = (record['semantics'][group], definition['semantics'][group]) \
                if group == 'physical' and existing_admitted else \
                (definition['semantics'][group], record['semantics'][group])
            record['semantics'][group] = source.accepted.accepted_first(primary, fallback)
        record['admission'] = dict(materializable=True, stack_class=definition['stack_class'],
                                  legal_destinations=['CharacterInventory', 'Ground'])
    document['records'].sort(key=order)
    raw = canonical(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
