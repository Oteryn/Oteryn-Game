"""Prepare all audited missing donor cores, with explicit owner-authorized omissions.

No source, baseline, production world, or shared converter is modified. Registered
stats and loot stay source-backed; omitted callbacks remain a visible quality debt.
"""
import argparse
import copy
import csv
import hashlib
import json
from pathlib import Path
import re
import sys
from unittest.mock import patch

import complete_creature_dependencies as population
import canary_batch as cb
import crystal_batch as crystal
import official_library
import population_census
import spell_scripts
import spell_probes
import validate_encounter

OPEN = set(population_census.OPEN)
SKIP = {'Druid familiar', 'Sorcerer familiar'}
UNADMITTED_CORPSES = {'angry_sugar_fairy': 48340, 'rootthing_amber_shaper': 48402,
                     'rootthing_nutshell': 48397}
RECOVERY = {
    'goshnars_cruelty.lua': ('attacks[6]', 'cruelty transform elemental'),
    'goshnars_malice.lua': ('zone preparation', 'Zone.getByName'),
}


def read(path):
    return json.loads(Path(path).read_text())


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def checked_relative(row, directory):
    value = row['source_file']
    if value.startswith(directory + '/'):
        value = value[len(directory) + 1:]
    path = Path(value)
    if path.is_absolute() or '..' in path.parts or path.suffix or '\\' in value:
        raise ValueError('unsafe inventory path')
    return value


def recover_table(path, errors=None):
    """Recover declarative facts without supplying quest globals or running callbacks.

    Two pinned failures occur in world-dependent preparation, not in the stat
    declarations. The unknown-interval action is omitted; world zone queries are
    removed only from evaluation. The converter still cites original source bytes.
    """
    from lupa.luajit21 import LuaRuntime
    source = path.read_text()
    field, token = RECOVERY[path.name]
    lines = source.splitlines()
    if path.name == 'goshnars_cruelty.lua':
        matches = [i for i, line in enumerate(lines) if 'name = "' + token + '"' in line]
        if len(matches) != 1 or 'SoulWarQuest.goshnarsCrueltyWaveInterval' not in lines[matches[0]]:
            raise ValueError('unexpected source recovery shape')
        lines[matches[0]] = ''
    else:
        expected = ['local zone = Zone.getByName("boss.goshnar\'s-malice")',
                    'local zonePositions = zone:getPositions()']
        for line in expected:
            if lines.count(line) != 1:
                raise ValueError('unexpected zone preparation shape')
            lines[lines.index(line)] = ''
    lua = LuaRuntime(unpack_returned_tuples=True)
    registered, callbacks, _ = lua.execute(cb.LUA_PRELUDE)
    lua.execute('\n'.join(lines))
    if registered['monster'] is None:
        raise ValueError('recovered source did not register')
    return registered['name'], cb.lua_value(registered['monster']), dict(callbacks.items())


def omission(source, field, line, reason):
    return {'source_index': 0, 'source_file': source, 'source_line': line,
            'source_field': field, 'kind': 'script', 'status': 'approved_omission',
            'resolution': 'OWNER_AUTHORIZED_FLAGGED_CORE: ' + reason}


def qualify(manifest):
    debt = []
    for row in manifest['entries']:
        if row['status'] not in OPEN:
            continue
        if row['status'] != 'unresolved_semantics':
            raise ValueError('numeric/dependency uncertainty cannot be waived: ' + str(row))
        if row['source_field'] == 'top-level script before mType:register':
            raise ValueError('partial pre-register table cannot be admitted')
        debt.append(copy.deepcopy(row))
        row.pop('destination', None)
        row['status'] = 'approved_omission'
        row['resolution'] = 'OWNER_AUTHORIZED_FLAGGED_CORE: omitted source behavior; original debt retained in receipt. ' + row.get('resolution', '')
    return debt


def omit_unadmitted_corpse(monster, deps, catalog, manifest, slug):
    """Remove only an unallocated corpse and unused decay payloads, preserving proof."""
    expected = 'canary:item/' + str(UNADMITTED_CORPSES[slug])
    corpse = monster['creature'].get('corpse_item')
    if corpse is None or corpse['key'] != expected:
        raise ValueError('unadmitted corpse differs from audited source')
    entries = [r for r in manifest['entries'] if r.get('destination') == '/monster/creature/corpse_item']
    if len(entries) != 1:
        raise ValueError('corpse lacks exact source proof')
    original = {'source_row': copy.deepcopy(entries[0]), 'corpse_reference': corpse,
                'corpse_items': []}
    del monster['creature']['corpse_item']
    pending = {expected}
    by_key = {r['identity']['key']: r for r in deps['items']}
    while True:
        extra = {by_key[key]['temporal']['decay_target']['key'] for key in pending
                 if key in by_key and 'decay_target' in by_key[key].get('temporal', {})}
        if extra <= pending:
            break
        pending |= extra
    other = json.dumps([monster, {k: v for k, v in deps.items() if k != 'items'}])
    removed = {key for key in pending if key not in other}
    original['corpse_items'] = [copy.deepcopy(r) for r in deps['items'] if r['identity']['key'] in removed]
    deps['items'] = [r for r in deps['items'] if r['identity']['key'] not in removed]
    catalog['definitions'] = [r for r in catalog['definitions'] if r['key'] not in removed]
    entry = entries[0]
    entry.pop('destination')
    entry.update(status='approved_omission', resolution='OWNER_AUTHORIZED_FLAGGED_CORE: corpse Item is not allocated in native registry; corpse and unused decay chain omitted. Exact original reference/payload retained in receipt. No alias or placeholder allocated; loot tables unchanged.')
    return original


class CorrectedSpellScripts(spell_scripts.SpellScripts):
    def _evaluate(self, key):
        result = super()._evaluate(key)
        if key == 'targetfirering' and 'combats' in result:
            result['oteryn_source_correction'] = {'original_combat_calls': copy.deepcopy(result['combats']),
                'correction': 'COMBAT_PARAM_SHOOT_EFFECT -> COMBAT_PARAM_DISTANCEEFFECT; preserves explicit fire type instead of undefined enum overwrite',
                'global_confirmed': False}
            for combat in result['combats'].values():
                for call in combat['param_calls']:
                    if call[0] == 'COMBAT_PARAM_SHOOT_EFFECT':
                        call[0] = 'COMBAT_PARAM_DISTANCEEFFECT'
        return result


def restore_simple_cores(slug, conv, monster, deps, catalog, manifest, output):
    """Stateless multi-cast and self-removal cores use existing Encounter contracts."""
    names = {'energy_pulse': 'energy pulse explosion', 'rootthing_amber_shaper': 'rotthingshaper',
             'rootthing_nutshell': 'rotthingwave'}
    if slug not in names:
        return []
    name = names[slug]
    info = conv.spell_scripts.evaluate(name)
    probe = spell_probes.Probe(conv.canary, info['script'], conv.spell_scripts.areas, conv.spell_scripts.enums)
    source = conv.canary / info['script']
    key = 'canary:ability/spell/' + cb.slug(name)
    encounter_key = 'canary:encounter/source_core_' + slug
    assets = set(catalog['assets'])
    def asset(value):
        assets.add(value)
        return value
    combat_objects = list(probe.rec['combats'].values())
    actions = []
    for i, combat_object in enumerate(combat_objects):
        combat = conv.spell_scripts._combat(probe.lua, combat_object)
        formula = combat.pop('formula', None)
        core_key = key + '/component-' + str(i + 1)
        geometry = {'needs_target': False, 'needs_direction': slug == 'rootthing_nutshell'}
        conv.combat_ability(core_key, combat, geometry, 0, deps, asset, [])
        if formula:
            # Explicit Oteryn correction: intended magnitude columns, not Global parity.
            bounds = sorted([int(formula[2]), int(formula[3])])
        else:
            bounds = [5000, 6000]
        fixed_key = core_key + '/magnitude'
        deps['formulas'].append({'identity': cb.ident(fixed_key), 'kind': 'range',
            'magnitude': {'minimum': bounds[0], 'maximum': bounds[1]}})
        for effect in deps['effects']:
            if effect['identity']['key'].startswith(core_key + '/') and effect.get('formula', {}).get('key') == cb.CASTER_MAGNITUDE:
                effect['formula'] = cb.ref('Formula', fixed_key)
        actions.append({'kind': 'cast', 'ability': cb.ref('Ability', core_key), 'at': 'subject_position'})
    if slug == 'energy_pulse':
        actions.append({'kind': 'remove', 'triggering': True})
    deps['abilities'].append({'identity': cb.ident(key), 'kind': 'spell', 'range_tiles': 0,
        'needs_target': False, 'needs_direction': slug == 'rootthing_nutshell',
        'encounter': cb.ref('Encounter', encounter_key)})
    source_monster = conv.monster_root / conv.monster_dir / (conv.current_relative + '.lua')
    _, raw, _ = cb.load_monster(source_monster, [])
    spells = raw['attacks']
    index, spell = next((i, row) for i, row in enumerate(spells, 1) if row['name'].lower() == name)
    schedule = {'ability': cb.ref('Ability', key), 'interval_ms': spell['interval'],
                'chance_percent': spell['chance']}
    destination = '/monster/behavior/attacks/' + str(len(monster['behavior']['attacks']))
    monster['behavior']['attacks'].append(schedule)
    field = 'attacks[' + str(index) + ']'
    row = next(r for r in manifest['entries'] if r['source_field'] == field)
    original = copy.deepcopy(row)
    row.update(status='mapped', destination=destination, resolution='SOURCE_CORE_RESTORED: complete stateless Encounter cast components; original donor limitations and any explicit non-Global source correction retained in receipt.')
    catalog['definitions'].append(cb.ref('Encounter', encounter_key))
    catalog['assets'] = sorted(assets)
    encounter = {'identity': cb.ident(encounter_key), 'display_name': monster['creature']['display_name'] + ' source core',
        'scope': 'channel_shared', 'participants': [{'role': slug, 'creatures': [cb.ref('Creature', monster['creature']['identity']['key'])]}],
        'anchors': [], 'state': {'flags': [], 'counters': [], 'timers': []}, 'phases': [], 'outcomes': [],
        'rules': [{'key': 'cast_core', 'trigger': {'kind': 'ability_cast', 'role': slug, 'ability': cb.ref('Ability', key)},
                   'conditions': [], 'actions': actions}]}
    proof = {'kind': 'git', 'repository': cb.REPOSITORY, 'revision': cb.REVISION,
             'path': info['script'], 'blob_sha1': cb.blob_id(source.read_bytes())}
    enc_manifest = {'encounter': encounter_key, 'classification': 'SOURCE_BACKED_FLAGGED_PLAYABLE_APPROXIMATION',
        'covers': {'cast_core': [monster['creature']['identity']['key']]}, 'sources': [proof],
        'entries': [{'source_index': 0, 'source_lines': [1], 'destination': '/encounter/rules/0', 'status': 'mapped', 'resolution': 'Typed stateless core; numeric source interpretation correction is explicitly non-Global.'}]}
    enc_catalog = {'definitions': list(validate_encounter.refs(encounter)), 'assets': sorted(assets)}
    errors = validate_encounter.validate(encounter, enc_catalog, enc_manifest)
    if errors:
        raise ValueError(errors)
    directory = output / 'encounters' / ('source_core_' + slug)
    for filename, value in [('encounter.json', encounter), ('catalog.json', enc_catalog), ('manifest.json', enc_manifest)]:
        write(directory / filename, value)
    return [{'original_source_row': original, 'spell_source': proof, 'source_sha256': sha(source),
             'encounter': 'source_core_' + slug, 'non_global_source_correction': bool(slug != 'energy_pulse')}]


def converters(canary, crystal_root):
    crystal.require_revision(canary, crystal_root)
    crystal.require_pinned(canary, 'Canary', cb.REVISION,
        ('data/items/items.xml', 'data/items/appearances.dat', 'data/scripts/lib/register_monster_type.lua'), False)
    objects = cb.load_appearance_objects(canary / 'data/items/appearances.dat')
    items = cb.load_items_xml(canary / 'data/items/items.xml')
    names, index = cb.name_index(objects, items)
    primary = cb.Converter(canary, objects, items, names, index)
    primary.secondary_mitigation_root = crystal_root
    primary.spell_scripts = CorrectedSpellScripts(canary)
    for attr, file in [('wiki', population_census.WIKI), ('br', population_census.BR_FILL),
                       ('official', official_library.SAMPLE)]:
        setattr(primary, attr, {row['monster']: row for row in read(file)['monsters']})
    secondary = crystal.converter(canary, crystal_root)
    secondary.wiki = {row['monster']: row for row in read(crystal.WIKI)['monsters']}
    secondary.official = {row['monster']: row for file in (official_library.CRYSTAL_SAMPLE,
                         official_library.CRYSTAL_EXTRA_SAMPLE) for row in read(file)['monsters']}
    return {'canary': primary, 'crystal': secondary}


def prepare(inventory, canary, crystal_root, baseline, output):
    output = output.resolve()
    for protected in (population.ROOT, canary.resolve(), crystal_root.resolve(), baseline.resolve()):
        if output == protected or protected in output.parents:
            raise ValueError('output would modify protected inputs')
    if output.exists() and any(output.iterdir()):
        raise ValueError('output must be new/empty')
    rows = list(csv.DictReader(inventory.open()))
    known = {row['monster'] for row in read(baseline / 'population-index.json')['monsters']}
    convs = converters(canary, crystal_root)
    report = {'schema': 'OTERYN_REMAINING_SOURCE_CORES/v1', 'inventory_sha256': sha(inventory),
              'baseline_index_sha256': sha(baseline / 'population-index.json'),
              'runtime_activated': False, 'live_gameplay_verified': False,
              'classification': 'SOURCE_BACKED_FLAGGED_PLAYABLE_APPROXIMATION',
              'encounters': [], 'actors': [], 'index_monsters': [], 'blocked': [], 'delegated': []}
    original_loader = cb.load_monster
    for row in rows:
        if row['name'] in SKIP:
            report['delegated'].append(row)
            continue
        conv = convs[row['repository']]
        relative = checked_relative(row, conv.monster_dir)
        path = conv.monster_root / conv.monster_dir / (relative + '.lua')
        if sha(path) != row['source_sha256']:
            raise ValueError('inventory source changed: ' + row['name'])
        source = str(path.relative_to(conv.monster_root))
        conv.current_relative = relative
        special = []
        corrections = []
        def loader(p, errors=None):
            result = recover_table(p, errors) if p.name in RECOVERY else original_loader(p, errors)
            if p.name == 'grimeleech.lua':
                name, raw, callbacks = result
                raw = copy.deepcopy(raw)
                action = raw['attacks'][2]
                if action['type'] != '@COMBAT_LIFEDRAINDAMAGE':
                    raise ValueError('unexpected Grimeleech source typo')
                corrections.append({'original_action': copy.deepcopy(action),
                    'correction': 'undefined COMBAT_LIFEDRAINDAMAGE -> registered COMBAT_LIFEDRAIN',
                    'global_confirmed': False})
                action['type'] = '@COMBAT_LIFEDRAIN'
                result = name, raw, callbacks
            if p.name == 'lisa.lua':
                name, raw, callbacks = result
                raw = copy.deepcopy(raw)
                action = raw['defenses']['_list'].pop(0)
                if action['name'] != 'lisa summon':
                    raise ValueError('unexpected Lisa summon')
                result = name, raw, callbacks
            return result
        try:
            conv.pending_definitions = set()
            with patch.object(cb, 'load_monster', loader):
                slug, monster, deps, catalog, manifest, _ = conv.convert(relative)
            if slug in known:
                raise ValueError('missing source already exists')
            if path.name == 'lisa.lua':
                for entry in manifest['entries']:
                    if entry['source_field'].startswith('defenses['):
                        entry['source_field'] = re.sub(r'\[(\d+)\]', lambda m: '[' + str(int(m[1])+1) + ']', entry['source_field'], count=1)
                        entry['source_line'] = next(i for i, line in enumerate(path.read_text().splitlines(), 1) if 'name = "lisa heal"' in line)
                special.append(omission(source, 'defenses[1]', 109,
                    'Lisa summon script references unregistered Glooth Anemone2; no invented alias. Entire invalid summon omitted; heal source index preserved.'))
            if path.name in RECOVERY:
                field, token = RECOVERY[path.name]
                number = next(i for i, line in enumerate(path.read_text().splitlines(), 1) if token in line)
                special.append(omission(source, field, number,
                    'World-dependent preparation/action excluded from declarative evaluation; no quest defaults guessed. All later numeric stat, loot and combat tables recovered from original source.'))
            if slug in ('priestess_of_the_wild_sun', 'sister_hetai'):
                correction = copy.deepcopy(conv.spell_scripts.evaluate('targetfirering')['oteryn_source_correction'])
                correction['spell_source_sha256'] = sha(canary / conv.spell_scripts.evaluate('targetfirering')['script'])
                corrections.append(correction)
            if corrections:
                field = 'attacks[3]' if slug == 'grimeleech' else 'attacks[2]'
                for entry in manifest['entries']:
                    if entry['source_field'] == field:
                        entry['resolution'] += ' Explicit accepted Oteryn source typo correction, not Global confirmed; original declarations retained in completion receipt.'
            manifest['entries'].extend(special)
            restored = restore_simple_cores(slug, conv, monster, deps, catalog, manifest, output)
            local = {item['identity']['key'] for item in deps['items']}
            for family, key in sorted(conv.pending_definitions):
                reference = cb.ref(family, key)
                if reference not in catalog['definitions'] and key != monster['creature']['identity']['key'] and key not in local:
                    catalog['definitions'].append(reference)
            debts = qualify(manifest) + special
            corpse_debt = omit_unadmitted_corpse(monster, deps, catalog, manifest, slug) if slug in UNADMITTED_CORPSES else None
            errors = population.validator.validate(monster, deps, catalog, manifest)
            if errors:
                raise ValueError(str(errors))
            target = output / 'bundles' / slug
            for filename, document in zip(population.FILES, (monster, deps, catalog, manifest)):
                write(target / filename, document)
            flags = ['GAMEPLAY_UNVERIFIED', 'OTS_HYPOTHESIS_ONLY']
            if restored:
                flags += ['SOURCE_TYPED_CORE_RESTORED', 'ENCOUNTER_PLACEMENT_PENDING']
            if slug in ('grimeleech', 'priestess_of_the_wild_sun', 'sister_hetai', 'rootthing_amber_shaper', 'rootthing_nutshell'):
                flags.append('OWNER_ACCEPTED_NON_GLOBAL_SOURCE_CORRECTION')
            if corpse_debt:
                flags.append('UNADMITTED_CORPSE_ITEM_OMITTED')
            if debts:
                flags += ['SOURCE_BEHAVIOR_PARTIAL', 'SOURCE_MECHANICS_OMITTED']
            if 'mitigation_percent' not in monster['creature']['stats']:
                flags.append('MITIGATION_UNKNOWN')
            index = {'monster': slug, 'file': relative, 'sha256': population.admission.bundle_digest(target),
                     'completion_flags': sorted(flags)}
            if row['repository'] == 'crystal':
                index['file'] = source[:-4]
                index['binding'] = {'source_key': 'oteryn:source.crystalserver',
                    'source_revision': crystal.REVISION, 'identity_namespace': 'crystalserver/monster-file',
                    'external_id': source}
            report['index_monsters'].append(index)
            report['actors'].append({'monster': slug, 'source': row, 'omitted_behavior': debts,
                'source_corrections': corrections, 'restored_cores': restored, 'omitted_corpse': corpse_debt, 'stats': monster['creature']['stats'], 'attacks': len(monster['behavior']['attacks']),
                'defenses': len(monster['behavior']['defenses']),
                'loot_entries': len((monster.get('loot') or {}).get('entries', [])),
                'pending_definition_references': [cb.ref(*r) for r in sorted(conv.pending_definitions)]})
            report['encounters'].extend({'encounter': entry['encounter'],
                'classification': 'STATELESS_TYPED_SOURCE_CORE', 'live_gameplay_verified': False}
                for entry in restored)
            known.add(slug)
        except Exception as error:
            report['blocked'].append({'source': row, 'reason': str(error)[:1500]})
    report['counts'] = {'prepared': len(report['index_monsters']), 'blocked': len(report['blocked']),
                       'delegated': len(report['delegated']),
                       'omitted_behavior_rows': sum(len(r['omitted_behavior']) for r in report['actors']),
                       'restored_source_spell_rows': sum(len(r['restored_cores']) + len(r['source_corrections']) for r in report['actors'])}
    write(output / 'completion.json', report)
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('inventory', 'canary', 'crystal', 'baseline', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(prepare(args.inventory, args.canary, args.crystal, args.baseline, args.output)['counts']))
