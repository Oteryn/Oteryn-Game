"""Generate the separately registered Canary Primal variants, never alias base actors."""
import argparse
import copy
import hashlib
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

import complete_remaining_monster_mechanics as core

HELPER = 'data-otservbr-global/lib/quests/the_primal_ordeal.lua'
BOSS = 'data-otservbr-global/monster/quests/primal_ordeal_quest/the_primal_menace.lua'


def proof(canary, path):
    data = subprocess.check_output(['git', '-C', str(canary), 'show', core.PIN + ':' + path])
    local = canary / path
    if local.exists() and local.read_bytes() != data:
        raise ValueError('dirty donor input ' + path)
    return data, {'kind': 'git', 'repository': 'opentibiabr/canary', 'revision': core.PIN, 'path': path,
        'blob_sha1': hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest(),
        'sha256': hashlib.sha256(data).hexdigest()}


def replace_keys(value, mapping):
    if isinstance(value, dict):
        return {k: mapping.get(v, v) if k == 'key' and isinstance(v, str) else replace_keys(v, mapping)
                for k, v in value.items()}
    if isinstance(value, list):
        return [replace_keys(v, mapping) for v in value]
    return value


def prepare(repo, baseline, current_population, previous, canary, variants, output):
    if variants.exists() or output.exists():
        raise ValueError('candidate outputs must not exist')
    sys.path[:0] = [str(repo / 'tools/content-schema/monster-authoring'),
                    str(repo / 'tools/content-schema/encounter-authoring')]
    import canary_batch as cb
    import validate_monster as vm
    import validate_encounter as ve
    helper, helper_proof = proof(canary, HELPER)
    expected = ['primalMonster.experience = 0', 'primalMonster.loot = {}',
        'primalMonster.name = "Primal Pack Beast"', 'primalMonster.description = "a primal pack beast"',
        'primalMonster.maxHealth = primalMonster.maxHealth * 0.7',
        'primalMonster.health = primalMonster.maxHealth', 'primalMonster.raceId = nil',
        'primalMonster.Bestiary = nil', 'primalMonster.corpse = 0',
        'local primalMonster = table.copy(template)', 'name .. " (Primal)"']
    if not all(s in helper.decode() for s in expected) or len(helper.decode().splitlines()) != 15:
        raise ValueError('derived registration helper changed')
    boss, boss_proof = proof(canary, BOSS)
    pool = re.search(r'MonsterPool\s*=\s*\{([^}]+)\}', boss.decode())
    names = re.findall(r'"([^"]+)"', pool[1])
    if len(names) != 11 or any(not n.endswith(' (Primal)') for n in names):
        raise ValueError('Primal source pool changed')
    registered = {}
    for path in (canary / cb.MONSTER_DIR).rglob('*.lua'):
        text = path.read_text()
        match = re.search(r'Game\.createMonsterType\("([^"]+)"', text)
        if match and 'RegisterPrimalPackBeast(monster)' in text:
            registered[match[1]] = path.relative_to(canary).as_posix()
    objects = cb.load_appearance_objects(canary / 'data/items/appearances.dat')
    items = cb.load_items_xml(canary / 'data/items/items.xml')
    appearance_names, appearance_index = cb.name_index(objects, items)
    converter = cb.Converter(canary, objects, items, appearance_names, appearance_index)
    initial_index = core.read(baseline / 'population-index.json')
    current_index = core.read(current_population / 'population-index.json')
    current_rows = {r['monster']: r for r in current_index['monsters']}
    receipt = {'schema': 'OTERYN_PRIMAL_DERIVED_REGISTRATIONS/v1',
        'baseline_index_sha256': hashlib.sha256((baseline / 'population-index.json').read_bytes()).hexdigest(),
        'current_index_sha256': hashlib.sha256((current_population / 'population-index.json').read_bytes()).hexdigest(),
        'actors': [], 'index_monsters': [], 'source_checks': [helper_proof, boss_proof],
        'runtime_qualified': False, 'ordinary_creature_substitution': False}
    for registered_name in names:
        parent_name = registered_name.removesuffix(' (Primal)')
        source_file = registered.get(parent_name)
        if not source_file:
            raise ValueError('no exact helper registration ' + registered_name)
        source_bytes, source_proof = proof(canary, source_file)
        parent_slug = cb.slug(parent_name)
        slug = cb.slug(registered_name)
        if slug in current_rows:
            raise ValueError('derived actor already prepared ' + slug)
        parent_bundle = current_population / 'bundles' / parent_slug
        if core.population.admission.bundle_digest(parent_bundle) != current_rows[parent_slug]['sha256']:
            raise ValueError('parent prepared bundle changed')
        relative = source_file.removeprefix(cb.MONSTER_DIR + '/').removesuffix('.lua')
        converter.pending_definitions = set()
        original_slug, monster, deps, catalog, manifest, _ = converter.convert(relative)
        if original_slug != parent_slug:
            raise ValueError('source registration mismatch')
        raw_name, raw, _ = cb.load_monster(canary / source_file, [])
        if raw_name != parent_name:
            raise ValueError('source registered name mismatch')
        helper_name = raw.get('name') or raw['description'].replace('an ', '').replace('a ', '').title()
        if helper_name + ' (Primal)' != registered_name:
            raise ValueError('helper registered identity differs from pool literal')
        # The converter has no wiki, official or secondary overrides. Derived
        # actors inherit donor numbers, never the prepared parent's estimates.
        mitigation = raw.get('defenses', {}).get('mitigation')
        if mitigation is None:
            monster['creature']['stats'].pop('mitigation_percent', None)
        elif monster['creature']['stats'].get('mitigation_percent') != cb.ratio(mitigation):
            raise ValueError('donor mitigation inheritance mismatch')
        hp = int(raw['maxHealth'] * 0.7)
        monster['creature']['stats'].update(max_health=hp, initial_health=hp, experience=0)
        monster['creature']['display_name'] = 'Primal Pack Beast'
        monster['creature']['inspection']['description'] = 'a primal pack beast'
        monster['creature']['name_forms'] = {'article': 'a'}
        monster['creature'].pop('bestiary', None)
        monster['creature'].pop('corpse_item', None)
        if 'loot' in monster:
            monster['loot']['entries'] = []
        vals = {'monster.json': monster, 'dependencies.json': deps, 'catalog.json': catalog,
                'manifest.json': manifest}
        mapping = {}
        for family in ('creature', 'behavior', 'presentation', 'loot', 'ability', 'effect', 'formula'):
            prefix = f'canary:{family}/{parent_slug}'
            replacement = f'canary:{family}/{slug}'
            # Only owned identities change. Shared spell/item/dependency keys keep
            # their exact identity; source paths, source names and texts stay intact.
            def keys(node):
                if isinstance(node, dict):
                    for key, value in node.items():
                        if key == 'key' and isinstance(value, str) and (value == prefix or value.startswith(prefix + '/')):
                            mapping[value] = replacement + value[len(prefix):]
                        else:
                            keys(value)
                elif isinstance(node, list):
                    for value in node:
                        keys(value)
            keys(vals)
        vals = replace_keys(vals, mapping)
        man = vals['manifest.json']
        for entry in man['entries']:
            if entry['source_field'] == 'RegisterPrimalPackBeast(monster)':
                entry['resolution'] = 'Separate helper-derived registration materialized as ' + registered_name + '; parent type remains unchanged; no native Lua callback is imported.'
            pointer = entry.get('destination', '')
            try:
                vm.resolve_pointer({'monster': vals['monster.json'], 'dependencies': vals['dependencies.json']}, pointer)
            except (KeyError, IndexError, ValueError, TypeError):
                if entry['status'] in ('mapped', 'resolved_native_behavior'):
                    entry.update(status='approved_omission', resolution='Derived helper removes the parent field: ' + entry['resolution'])
                    entry.pop('destination', None)
        for line, field, pointer in [(5, 'experience', '/monster/creature/stats/experience'),
            (6, 'loot', '/monster/loot'), (7, 'name', '/monster/creature/display_name'),
            (8, 'description', '/monster/creature/inspection/description'),
            (9, 'maxHealth', '/monster/creature/stats/max_health'), (10, 'health', '/monster/creature/stats/initial_health')]:
            man['entries'].append({'source_index': 0, 'source_file': HELPER, 'source_line': line,
                'source_field': 'RegisterPrimalPackBeast.' + field, 'kind': 'field', 'status': 'mapped',
                'destination': pointer, 'resolution': 'Exact pinned helper override for registered type ' + registered_name})
        for family, key in converter.pending_definitions:
            dependency = cb.ref(family, key)
            local_items = {i['identity']['key'] for i in vals['dependencies.json']['items']}
            if dependency not in vals['catalog.json']['definitions'] and key not in local_items:
                vals['catalog.json']['definitions'].append(dependency)
        errors = vm.validate(*(vals[f] for f in core.FILES))
        if errors:
            raise ValueError((slug, errors))
        for filename, value in vals.items():
            core.write(variants / 'bundles' / slug / filename, value)
        flags = ['SOURCE_DERIVED_PRIMAL_VARIANT', 'SOURCE_DONOR_STATS_NOT_GLOBAL_VERIFIED',
                 'GAMEPLAY_UNVERIFIED', 'ENCOUNTER_PLACEMENT_PENDING']
        if mitigation is None:
            flags.append('MITIGATION_UNKNOWN')
        row = {'monster': slug, 'file': source_file,
            'sha256': core.population.admission.bundle_digest(variants / 'bundles' / slug),
            'completion_flags': flags}
        row['binding'] = {'source_key': 'oteryn:source.canary', 'source_revision': core.PIN,
            'identity_namespace': 'canary/monster-file',
            'external_id': relative + '.lua#registered-type=' + registered_name}
        receipt['index_monsters'].append(row)
        receipt['actors'].append({'monster': slug, 'registered_name': registered_name, 'base_registered_name': parent_name,
            'base_identity': 'canary:creature/' + parent_slug, 'identity': 'canary:creature/' + slug,
            'binding': row['binding'], 'source_locator_kind': 'registered-type-in-file',
            'physical_source_file': source_file,
            'parent_prepared_bundle_sha256': current_rows[parent_slug]['sha256'],
            'parent_donor_source': source_proof, 'helper_source': helper_proof,
            'bundle_digest': row['sha256'], 'donor_mitigation': mitigation,
            'max_health': hp, 'global_parity': False})
        receipt['source_checks'].append(source_proof)
    # Source identity tuples must distinguish each derived registration from
    # every ordinary actor binding, including the shared parent physical file.
    existing_bindings = core.read(current_population / 'creature-admission-stage.json')['source_identity_bindings']
    fields = ('source_key', 'source_revision', 'identity_namespace', 'external_id')
    existing = {tuple(b[k] for k in fields) for b in existing_bindings}
    new_bindings = [tuple(r['binding'][k] for k in fields) for r in receipt['index_monsters']]
    if len(set(new_bindings)) != 11 or existing.intersection(new_bindings):
        raise ValueError('source identity binding collision')
    receipt['source_binding_closure'] = {'existing_bindings': len(existing), 'new_bindings': len(new_bindings),
        'collisions': 0, 'namespace': 'canary/monster-file', 'external_id_kind': 'explicit registered-type file fragment'}
    core.write(variants / 'completion.json', receipt)
    shutil.copytree(previous, output)
    m = core.read(output / 'completion.json')
    m['derived_source_reference_bindings'] = receipt['actors']
    m['new_variant_packet_sha256'] = hashlib.sha256((variants / 'completion.json').read_bytes()).hexdigest()
    scene = output / 'encounters' / 'the_primal_menace'
    manifest = core.read(scene / 'manifest.json')
    helper_encounter_proof = {k: v for k, v in helper_proof.items() if k != 'sha256'}
    if helper_encounter_proof not in manifest['sources']:
        manifest['sources'].append(helper_encounter_proof)
    core.write(scene / 'manifest.json', manifest)
    for scene_row in m['encounters']:
        if scene_row['encounter'] == 'the_primal_menace':
            scene_row['source_bound_allowlist'] = manifest['sources']
    if helper_proof not in m['source_checks']:
        m['source_checks'].append(helper_proof)
    core.write(output / 'completion.json', m)
    # Closed graph uses real definitions, not catalog declarations. New variant
    # references resolve only against the actual generated registered actor docs.
    defined = {core.read(current_population / 'bundles' / r['monster'] / 'monster.json')['creature']['identity']['key']
               for r in current_index['monsters']}
    defined.update(core.read(variants / 'bundles' / r['monster'] / 'monster.json')['creature']['identity']['key']
                   for r in receipt['index_monsters'])
    unresolved = []
    for p in list((output / 'encounters').glob('*/encounter.json')) + list((variants / 'bundles').glob('*/monster.json')) + list((variants / 'bundles').glob('*/dependencies.json')):
        q = core.read(p)
        for reference in ve.refs(q):
            if reference['family'] == 'Creature' and reference['key'] not in defined:
                unresolved.append({'encounter': p.parent.name, 'reference': reference})
    if unresolved:
        raise ValueError(unresolved)
    core.write(output / 'primal-reference-closure.json', {'definitions': len(defined), 'new_variants': len(names),
        'ordinary_creature_substitution': False, 'unresolved_creature_references': unresolved,
        'helper_source': helper_proof, 'source_pool': names})
    return receipt


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    for name in ('repo', 'baseline', 'current_population', 'previous', 'canary', 'variants', 'output'):
        parser.add_argument('--' + name.replace('_', '-'), dest=name, type=Path, required=True)
    result = prepare(**vars(parser.parse_args()))
    print(json.dumps({'new_primal_variants': len(result['actors']),
                      'missing_donor_mitigation': sum(a['donor_mitigation'] is None for a in result['actors'])}))
