"""Refresh strict inventory pins for the exact, source-qualified Cake successor."""
import argparse
from collections import Counter
import json
from pathlib import Path
import re

import cake_native_successor as cake
import raid_catalogue
from world_project_v2_to_tree import CREATURE_FAMILIES, git_blob_sha

ROOT = Path(__file__).resolve().parents[2]


def validate_inventory(root, rows, native, schedules):
    """Retain main's exact cohort while qualifying the branch's one Thaian."""
    from questpc7_thaian_authoring import PACKET
    packet, _ = cake.read_canonical(root / PACKET)
    if packet['source']['page_id'] != 48123 or packet['source']['revision'] != 445240:
        raise ValueError('Thaian source authoring revision')
    profiles = {row['target']['family']: row for row in packet['profiles']}
    records = {row['identity']['family']: row for row in packet['records']}
    key = 'oteryn:creature.thaian'
    canonical = [row for row in rows if row['definition']['identity']['key'] == key]
    active = [row for row in native if row['profile']['target']['key'] == key]
    if (len(canonical) != 1 or len(active) != 1
            or canonical[0]['definition'] != records['Creature']
            or canonical[0]['authoring'] != profiles['Creature']['data']
            or active[0]['profile'] != profiles['Creature']
            or active[0]['behavior'] != profiles['Behavior']):
        raise ValueError('Thaian source authoring closure')
    behavior = active[0]['behavior']['data']['profile']
    attacks = behavior.get('attacks', [])
    if len(attacks) != 1 or behavior.get('defenses', []):
        raise ValueError('Thaian exact melee schedule')
    ability = [row for row in active[0]['attack_definitions']['profiles']
               if row['target'] == attacks[0]['ability']]
    if len(ability) != 1 or ability[0]['data']['profile']['details']['kind'] != 'Melee':
        raise ValueError('Thaian exact melee ability')
    # Thaian extends the canonical Cake cohort; independently qualified
    # native-only Fugue and Elemental bodies retain their exact schedule proofs.
    from engenc2_fugue_profile import inventory_extension
    extra_native, extra_schedules = inventory_extension(root, native)
    from engenc_elemental_profiles import inventory_extension as elemental_inventory
    elemental_native, elemental_schedules = elemental_inventory(native)
    extra_native += elemental_native
    extra_schedules += elemental_schedules
    if (len(rows) - 1 != 1863 + 1 or len(native) != len(rows) + extra_native
            or schedules - 1 != 7201 + 4 + extra_schedules):
        raise ValueError('Non-seasonal canonical inventory changed; review its source first')
    return 1


def generated(root):
    read = lambda path: json.loads((root / path).read_bytes())
    index = read('content/creatures/definitions/index.json')
    rows = [row for path in index['shards'] for row in read(path)['records']]
    packet, _ = cake.read_canonical(root / cake.PACKET)
    declarations = cake.read_project_document(root, 'definitions/declarations.json')
    reference = cake.read_project_document(root, 'definitions/reference.json')
    # Crystal data-global/monster/quests/primal_ordeal_quest/the_primal_menace.lua:79,118:
    # replay main's hash-checked tree-first empty-loot admission before exact family equality.
    from primal_reference_overlay import apply_reference
    apply_reference(reference['records'], root)
    admitted = cake.qualify(rows, declarations, reference, packet,
        read('imports/crystalserver/bindings/items.json'))
    native = read('content/creatures/definitions/spell-native-profiles.json')['records']
    native = [row for row in native if row['profile']['target']['key'].startswith('oteryn:creature.')]
    selected, = [row for row in native if row['profile']['target']['key'] == cake.CAKE[1]]
    if selected['profile']['data'] != admitted['authoring']:
        raise ValueError('Cake native profile differs from its qualified source')
    behavior = selected['behavior']['data']['profile']
    # Crystal201189f cake_golem.lua:76-95: three attack slots, one defence slot.
    if (len(behavior['attacks']), len(behavior['defenses'])) != (3, 1):
        raise ValueError('Cake source schedule inventory')
    schedules = sum(len(row['behavior']['data']['profile'].get(key, []))
                    for row in native for key in ('attacks', 'defenses'))
    thaian_melee = validate_inventory(root, rows, native, schedules)
    from engenc_elemental_profiles import inventory_extension as elemental_inventory
    elemental_native, _ = elemental_inventory(native)
    result = {}

    def replace(path, pattern, replacement):
        text = result.get(path, (root / path).read_text())
        text, count = re.subn(pattern, replacement, text)
        if count != 1:
            raise ValueError('Missing or ambiguous inventory seam: ' + path)
        result[path] = text

    replace('crates/gameplay/monster-ai/src/ai_think/profile_schedule/tests.rs',
            r'assert_eq!\(creatures.len\(\), [0-9_]+, "complete canonical Creature corpus"\);',
            f'assert_eq!(creatures.len(), {len(rows)}, "complete canonical Creature corpus");')
    replace('crates/gameplay/monster-ai/src/ai_think/native_profile_tests.rs',
            r'\([0-9_]+, [0-9_]+, 15, 8\)', f'({len(native)}, {schedules}, 15, 8)')
    # The two source-qualified authored Elementals each reuse one CreateItem
    # schedule. Keep the source-census assertion exact for those occurrences.
    replace('crates/gameplay/content/src/content/native_gameplay.rs',
            r'assert_eq!\(\(create, remove\), \([0-9_]+, [0-9_]+\)\);',
            f'assert_eq!((create, remove), ({62 + elemental_native}, 20));')
    # Crystal cake_golem.lua:77-81 adds one melee and two native spell slots.
    # Keep the pre-existing unsupported set at116: no new source gap is admitted.
    path = 'crates/gameplay/encounters/src/bosses/native_combat/monster_combat_lane.rs'
    replace(path, r'\("melee:Ok\(\(\)\)", [0-9_]+\)',
            f'("melee:Ok(())", {1689 + 1 + thaian_melee})')
    replace(path, r'\("spell:Ok\(\(\)\)", [0-9_]+\)',
            f'("spell:Ok(())", {4198 + len(behavior["attacks"]) - 1})')
    if '("spell:Err(UnsupportedShape)", 116)' not in result[path]:
        raise ValueError('Existing native source gap pin changed')

    # Crystal cake_golem.lua:11-19: [2,3,5],30 => easy / very rare.
    histogram = Counter()
    for row in rows:
        bestiary = row['authoring']['profile'].get('bestiary')
        if bestiary:
            histogram[bestiary['difficulty'] + (' very rare' if bestiary['occurrence'] == 'very_rare' else '')] += 1
    path = 'crates/gameplay/progression/src/bestiary_charm_crystal_parity_tests.rs'
    text = (root / path).read_text()
    start = text.index('    let expected: BTreeMap<&str, usize> = [')
    end = text.index('    ]', start)
    block = text[start:end]
    if set(re.findall(r'\("([^"]+)", [0-9_]+\)', block)) != set(histogram):
        raise ValueError('Bestiary difficulty categories changed')
    block = re.sub(r'\("([^"]+)", [0-9_]+\)',
                   lambda m: f'("{m[1]}", {histogram[m[1]]})', block)
    result[path] = text[:start] + block + text[end:]
    replace(path, r'assert_eq!\(rows.len\(\), [0-9_]+\);',
            f'assert_eq!(rows.len(), {sum(histogram.values())});')

    outputs = raid_catalogue.outputs(root, register=False)
    if outputs[raid_catalogue.CATALOGUE] != (root / raid_catalogue.CATALOGUE).read_bytes():
        raise ValueError('Regenerate the source-qualified raid catalogue first')
    if outputs[raid_catalogue.INDEX] != (root / raid_catalogue.INDEX).read_bytes():
        raise ValueError('Regenerate the source-qualified raid index first')
    raids = json.loads(outputs[raid_catalogue.CATALOGUE])['records']
    active = sum(row['active'] for row in raids)
    if len(raids) != 239 + 1 or active != 136 + 1:
        raise ValueError('Non-seasonal raid inventory changed; review its source first')
    path = 'crates/gameplay/encounter-rules/src/raid/catalogue.rs'
    replace(path, r'assert_eq!\(catalogue.records\(\).len\(\), [0-9_]+\);',
            f'assert_eq!(catalogue.records().len(), {len(raids)});')
    replace(path, r'assert_eq!\(catalogue.active\(\).count\(\), [0-9_]+\);',
            f'assert_eq!(catalogue.active().count(), {active});')
    # A merge can retain the fresh shards but take the other side's old central
    # summaries. Verify exact canonical membership before republishing summaries.
    manifest, lock = read('content/manifest.json'), read('content/content.lock.json')
    managed = {row['path'] for row in manifest['managed_files']}
    for family, (node, stem) in CREATURE_FAMILIES.items():
        registration = manifest['families'][family]
        family_index = read(registration['index'])
        actual = [row['definition']['identity'] for shard in family_index['shards']
                  for row in read(shard)['records']]
        expected = [row['identity'] for row in reference['records']
                    if row['identity']['family'] == family]
        identities = lambda values: {cake.identity(value) for value in values}
        if (len(actual) != family_index['record_count']
                or len(identities(actual)) != len(actual)
                or identities(actual) != identities(expected)):
            raise ValueError('Canonical family summary mismatch: ' + family)
        shard_name = re.compile(re.escape(node + stem) + r'-[0-9]{5}-[0-9]{5}\.json$')
        if any(not shard_name.fullmatch(path) for path in family_index['shards']):
            raise ValueError('Canonical family shard path mismatch: ' + family)
        managed = {path for path in managed if not shard_name.fullmatch(path)}
        managed.update(family_index['shards'])
        registration['records'] = len(actual)
        lock['family_counts'][family] = len(actual)
    manifest['families']['Encounter.Raid']['records'] = len(raids)
    manifest['managed_files'] = [{'path': path} for path in sorted(managed)]
    lock['family_counts']['Encounter.Raid'] = len(raids)
    bindings = sum(len(row['source_bindings']) for row in rows)
    if bindings != index['attached_source_bindings']:
        raise ValueError('Canonical Creature binding summary mismatch')
    lock['source_binding_counts']['Creature'] = bindings
    lock['authoring_profile_counts'] = dict(sorted(Counter(
        row['target']['family'] for row in declarations['authoring_profiles']).items()))
    for name, locator in {'reference': 'definitions/reference.json',
            'declarations': 'definitions/declarations.json', 'editor': 'editor/author.json',
            'sources': 'provenance/sources.json', 'imports': 'provenance/imports.json'}.items():
        lock['legacy_blobs'][name] = git_blob_sha(root / 'content/world' / locator)
    for path, value in [('content/manifest.json', manifest), ('content/content.lock.json', lock)]:
        result[path] = json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + '\n'
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for path, text in generated(ROOT).items():
        if args.check:
            if (ROOT / path).read_text() != text:
                raise SystemExit('Stale seasonal inventory: ' + path)
        elif (ROOT / path).read_text() != text:
            (ROOT / path).write_text(text)
    print('PASS source-qualified seasonal Creature, schedule, Bestiary and raid pins')
