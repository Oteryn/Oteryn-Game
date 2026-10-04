"""Produce source-bound field patches; never mutate population or invent Global parity."""
import argparse
import copy
import csv
import gzip
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

BASELINE = '43cf343cba51a843ce6116260bfdedf9f8765fb79d030f9ef4b7ec61906538b1'
# Qualified source variants: a shared ordinary Wiki page does not override prepared donor phase values.
SHARED_PAGE_VARIANTS = {'cosmic_energy_prism_' + v + '_invu' for v in 'abcd'} | {
    'energy_cannon_right', 'giant_spider_wyda', 'orc_sambackpack', 'the_sandking_fake'} | {
    'spyrat_' + v for v in ['east', 'north', 'south', 'west']}


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def get(document, pointer):
    node = document
    try:
        for part in pointer.strip('/').split('/'):
            part = part.replace('~1', '/').replace('~0', '~')
            if part == '-':
                return False, None
            node = node[int(part)] if isinstance(node, list) else node[part]
        return True, node
    except (KeyError, IndexError, ValueError, TypeError):
        return False, None


def patch(monster, file, document, pointer, value, source, reason):
    present, previous = get(document, pointer)
    return dict(monster=monster, file=file, pointer=pointer,
                expected_present=present, expected_value=previous, value=value,
                source=source, reason=reason)


def ref(family, key, revision):
    return dict(family=family, key=key, revision=revision)


def prepare(population, audit, output):
    population, audit, output = map(Path, (population, audit, output))
    if sha(population / 'population-index.json') != BASELINE:
        raise ValueError('Unexpected population-index SHA; regenerate audit before applying')
    index = read(population / 'population-index.json')
    actors = {r['monster']: r for r in index['monsters']}
    patches, flags, exclusions = [], {}, []
    pages = {p['url']: p for p in json.load(gzip.open(audit.parent / 'wiki/pages.json.gz'))['pages']}
    rows = csv.DictReader(gzip.open(audit / 'fresh-wiki-behavior-comparison.csv.gz', 'rt'))
    for row in rows:
        if row['comparison'] != 'FRESH_WIKI_BOOLEAN_MISMATCH':
            continue
        key = row['monster']
        accepted = actors[key].get('completion_flags', [])
        if key in SHARED_PAGE_VARIANTS or 'SOURCE_DERIVED_PRIMAL_VARIANT' in accepted or 'SOURCE_TEMPLATE_GLOBAL_UNVERIFIED' in accepted:
            exclusions.append(dict(monster=key, pointer=row['native_path'], reason='SHARED_WIKI_TITLE_FIELD_UNVERIFIED: preserve prepared donor variant field; base-page Boolean is not direct variant evidence'))
            continue
        value = {'sim': True, 'não': False, 'nao': False, 'yes': True, 'no': False}.get(row['wiki_value'].strip().lower())
        if value is None:
            raise ValueError('Mismatch row without a definite Boolean')
        document = read(population / 'bundles' / key / 'monster.json')
        _, previous = get(document, row['native_path'])
        if str(previous) != row['native_value']:
            raise ValueError('Audit cell drift: ' + key)
        source = dict(kind='mediawiki', url=row['page_url'], revision_id=int(row['revision_id']),
                      content_sha256=row['content_sha256'], source_field=row['wiki_field'], source_file=pages[row['page_url']]['page_title'], source_line=pages[row['page_url']]['field_lines'][row['wiki_field']],
                      method=row['read_method'], observed_value=row['wiki_value'])
        patches.append(patch(key, 'monster.json', document, row['native_path'], value, source,
                             'Owner requests current Wiki data: replace older source value with definite fresh WikiBR Boolean.'))
        flags.setdefault(key, []).append('OWNER_ACCEPTED_CURRENT_WIKI_BEHAVIOR_CORRECTION')
    # The Wiki gives only 0-??? earth-wave damage. The owner authorizes an explicitly non-Global balance proxy.
    key = 'dark_merudri'
    bundle = population / 'bundles' / key
    monster, dependencies, catalog = (read(bundle / n) for n in ['monster.json', 'dependencies.json', 'catalog.json'])
    source_page = None
    with gzip.open(audit / 'fresh-wiki-ability-comparison.csv.gz', 'rt') as handle:
        for row in csv.DictReader(handle):
            if row['monster'] == key and row['wiki_field'] == 'hab_earth':
                source_page = row
                break
    if source_page is None or '???' not in source_page['wiki_text']:
        raise ValueError('Missing explicit Wiki unknown earth-wave observation')
    source = dict(kind='owner_accepted_non_global_proxy', global_parity=False,
                  url=source_page['page_url'], revision_id=int(source_page['revision_id']),
                  content_sha256=source_page['content_sha256'], source_field='hab_earth', source_file=pages[source_page['page_url']]['page_title'], source_line=pages[source_page['page_url']]['field_lines']['hab_earth'],
                  observed_value=source_page['wiki_text'], method=source_page['read_method'],
                  template_file='dark_merudri/dependencies.json', template_sha256=sha(bundle / 'dependencies.json'),
                  template_formula='canary:formula/dark_merudri/attack-2-1',
                  qualification='OWNER_ACCEPTED_NON_GLOBAL_BALANCE',
                  estimated_fields=['damage_range', 'rear_wave_shape', 'range_tiles', 'schedule'])
    donor = next(f for f in dependencies['formulas'] if f['identity']['key'] == source['template_formula'])
    maximum = donor['magnitude']['maximum']
    revision = 'oteryn-wiki-field-fill-20261002'
    identities = {f: 'canary:' + f.lower() + '/dark_merudri/field-fill-earth-wave' for f in ['Ability', 'Effect', 'Formula']}
    ability = dict(identity=dict(key=identities['Ability'], revision=revision), kind='spell',
                   range_tiles=5, needs_target=True, needs_direction=True,
                   area=dict(matrix=dict(north=['...c...', '...x...', '..xxx..', '.xxxxx.', 'xxxxxxx', 'xxxxxxx'])), effects=[ref('Effect', identities['Effect'], revision)])
    effect = dict(identity=dict(key=identities['Effect'], revision=revision), operation='damage',
                  damage_type='earth', formula=ref('Formula', identities['Formula'], revision))
    formula = dict(identity=dict(key=identities['Formula'], revision=revision), kind='range',
                   magnitude=dict(minimum=donor['magnitude']['minimum'], maximum=maximum))
    schedule = dict(ability=ref('Ability', identities['Ability'], revision), interval_ms=2000, chance_percent=20)
    reason = 'Wiki confirms an earth wave with unknown maximum. Oteryn proxy uses own existing energy-burst damage range; rear-wave size/range/schedule are accepted balance assumptions, not Global facts.'
    for field, value in [('abilities', ability), ('effects', effect), ('formulas', formula)]:
        patches.append(patch(key, 'dependencies.json', dependencies, '/' + field + '/-', value, source, reason))
    patches.append(patch(key, 'monster.json', monster, '/behavior/attacks/-', schedule, source, reason))
    flags[key] = sorted(set(flags.get(key, []) + ['SOURCE_NON_GLOBAL_PROXY', 'OWNER_ACCEPTED_NON_GLOBAL_BALANCE', 'WIKI_EARTH_WAVE_DAMAGE_ESTIMATED', 'OWNER_ACCEPTED_NON_GLOBAL_MECHANIC_PROXY', 'GAMEPLAY_UNVERIFIED']))
    packet = dict(schema='OTERYN_MONSTER_FIELDPATCH/v1', lane='mechanics', baseline_index_sha256=BASELINE,
                  patches=patches, actor_flags={k: sorted(set(v)) for k, v in flags.items()},
                  qualification='SOURCE_BACKED_FIELDS_WITH_EXPLICIT_OWNER_ACCEPTED_NON_GLOBAL_PROXY',
                  excluded_cells=exclusions,
                  counts=dict(boolean_cells=sum(p['source']['kind'] == 'mediawiki' for p in patches),
                              proxy_abilities=4, wiki_unknown_damage_proxy_abilities=1, patches=len(patches), affected_actors=len(flags)),
                  audit_summary_sha256=sha(audit / 'summary.json'))
    review = output.parent / 'shared-variant-boolean-review.json'
    if review.exists():packet['shared_variant_review_sha256'] = sha(review)
    output.mkdir(parents=True, exist_ok=True)
    additional_cores(population, audit, output, packet)
    (output / 'fieldpatch.json').write_text(json.dumps(packet, ensure_ascii=False, indent=2) + '\n')
    return packet


def additional_cores(population, audit, output, packet):
    """Bounded source-backed Encounter cores; all observer bindings require placement."""
    repo = Path(__file__).resolve().parents[2]
    sys.path[:0] = [str(repo / 'tools/content-schema/monster-authoring'), str(repo / 'tools/content-schema/encounter-authoring')]
    import canary_batch as cb
    import validate_encounter as ve
    from complete_remaining_monster_mechanics import rule
    donor = Path('/workspace/monster-reference-sources/canary')
    pin, revision = '47dfd51f45280a59a1d3e50ba7edd573d7234446', 'canary-47dfd51f'
    patches, restored, scenes = packet['patches'], [], []
    def creature(name):
        return ref('Creature', 'canary:creature/' + name, revision)
    def source(path):
        data = subprocess.check_output(['git', '-C', str(donor), 'show', pin + ':' + path])
        return dict(kind='git', repository='opentibiabr/canary', revision=pin, path=path,
                    sha256=hashlib.sha256(data).hexdigest(), source_lines=list(range(1, len(data.splitlines()) + 1)))
    def scene(name, sourcepath, fields, others=(), limitation='', register=True, stage_suffix=''):
        slug = 'field_fill_' + name
        eref = ref('Encounter', 'canary:encounter/' + slug, revision)
        e = dict(identity=dict(key=eref['key'], revision=revision), display_name='Source core: ' + name,
                 scope='instance_per_party', participants=[dict(role=n, creatures=[creature(n)]) for n in [name, *others]],
                 anchors=[], phases=[], state=dict(counters=[], flags=[], timers=[]), rules=[], outcomes=[])
        src = source(sourcepath)
        catalog = dict(definitions=[creature(n) for n in [name, *others]])
        manifest = dict(encounter=eref['key'], classification='SOURCE_BACKED_OTERYN_SIMPLIFICATION',
                        covers={f.replace('events=', ''): [creature(name)['key']] for f in fields}, sources=[src],
                        entries=[dict(source_index=0, source_lines=src['source_lines'], status='mapped',
                                      destination='/encounter/rules', resolution=limitation)], outcome_evidence=[])
        doc = read(population / 'bundles' / name / 'catalog.json')
        patches.append(patch(name, 'catalog.json', doc, '/definitions/-', eref, src, limitation))
        packet['actor_flags'].setdefault(name, []).extend(['SOURCE_TYPED_CORE_RESTORED', 'SOURCE_BACKED_OTERYN_SIMPLIFICATION', 'SOURCE_BEHAVIOR_PARTIAL', 'ENCOUNTER_PLACEMENT_PENDING', 'GAMEPLAY_UNVERIFIED'])
        restored.extend(dict(monster=name, source_field=f, encounter=eref['key'], limitation=limitation, source=src) for f in fields)
        scenes.append((slug, e, catalog, manifest))
        return e, catalog, src
    def spell(name, field, filename, limitation, others=()):
        manifest = read(population / 'bundles' / name / 'manifest.json')
        row = next(x for x in manifest['entries'] if x['source_field'] == field)
        raw = cb.load_monster(donor / row['source_file'])[1]
        section, number = field.split('['); schedule = raw[section]
        schedule = schedule if isinstance(schedule, list) else schedule['_list']
        schedule = schedule[int(number[:-1]) - 1]
        e, cat, src = scene(name, 'data-otservbr-global/scripts/spells/monster/' + filename, [field], others=others, limitation=limitation)
        src['monster_source'] = source(row['source_file'])
        aref = ref('Ability', 'canary:ability/' + name + '/field-fill-' + cb.slug(schedule['name']), revision)
        deps, mon = (read(population / 'bundles' / name / f) for f in ['dependencies.json', 'monster.json'])
        ability = dict(identity=dict(key=aref['key'], revision=revision), kind='spell', range_tiles=schedule.get('range', 0),
                       needs_target=bool(schedule.get('target', False)), needs_direction=False,
                       encounter=ref('Encounter', e['identity']['key'], revision))
        ability['encounter']['key'] = e['identity']['key']
        patches.append(patch(name, 'dependencies.json', deps, '/abilities/-', ability, src, limitation))
        patches.append(patch(name, 'monster.json', mon, '/behavior/' + section + '/-',
                             dict(ability=aref, interval_ms=schedule['interval'], chance_percent=schedule.get('chance', 100)), src, limitation))
        cat['definitions'].append(aref)
        return e, dict(kind='ability_cast', role=name, ability=aref)
    for name, field, filename, threshold, amount, delay, cooldown in [
            ('lisa', 'defenses[2]', 'lisa_heal.lua', 7, (18000, 23000), 6000, 6000),
            ('tyrn', 'defenses[3]', 'tyrn_heal.lua', 20, (5000, 7500), 0, 900000)]:
        e, trigger = spell(name, field, filename, 'HP gate, source amount/delay/cooldown retained; local flag replaces regeneration subid88888; fractional regeneration tick omitted.')
        e['state']['flags'].append(dict(name='healing_locked', initial=False))
        e['state']['timers'].append(dict(name='healing_reset', duration_ms=cooldown, repeat=False))
        heal = dict(kind='heal', subject=dict(role=name), amount=dict(min=amount[0], max=amount[1]))
        start = [dict(kind='flag', flag='healing_locked', value=True), dict(kind='timer', timer='healing_reset', operation='start')]
        finish = [dict(kind='flag', flag='healing_locked', value=False)]
        (finish if delay else start).append(heal)
        e['rules'].append(rule('begin_heal', trigger, start,
                               [dict(kind='health_percent', role=name, op='<', value=threshold), dict(kind='flag', flag='healing_locked', value=False)]))
        e['rules'].append(rule('finish_heal', dict(kind='timer_elapsed', timer='healing_reset', each=name), finish))
    quest = 'data-otservbr-global/scripts/quests/'
    for name, event, percent, types in [('soul_cage', 'SoulCageHealthChange', 10, []), ('goshnar_s_malice', "Goshnar's-Malice", 100, ['physical', 'death'])]:
        e, _, _ = scene(name, quest + 'soul_war/soul_war_mechanics.lua', ['events=' + event], limitation='Source player-triggered positive-damage reflection percent/types retained; native reflect primitive semantics need live qualification.')
        e['rules'].append(rule('reflect_player_damage', dict(kind='damage_taken', role=name, source='player'), [dict(kind='reflect_damage', role=name, percent=percent, damage_types=types)]))
    e, _, _ = scene('aspect_of_power', quest + 'soul_war/soul_war_mechanics.lua', ['events=SoulWarAspectOfPowerDeath'], limitation='Five-second replacement at death position retained; boss-position selection, random outfit1303–1307 and Megalomania counter omitted.')
    e['rules'].append(rule('aspect_respawns', dict(kind='creature_died', role='aspect_of_power'), [dict(kind='spawn', creature=creature('aspect_of_power'), count=1, at='death_position', owner='none', health='full')], delay=5000))
    e, _, _ = scene('destabilized_ferumbras', quest + 'ferumbras_ascension/creaturescripts_ferumbras_mortal_shell_death.lua', ['events=FerumbrasMortalShell'], ['ferumbras_mortal_shell'], 'Death spawns registered Mortal Shell; current death-position proxy replaces configured boss tile. Killer voice/reward shared state omitted.')
    e['rules'].append(rule('mortal_shell_appears', dict(kind='creature_died', role='destabilized_ferumbras'), [dict(kind='spawn', creature=creature('ferumbras_mortal_shell'), count=1, at='death_position', owner='none', health='full')]))
    e, trigger = spell('cerebellum', 'attacks[2]', 'heal_brain_head.lua', '300–500 allied Brain Head heal at pinned tile retained; source top-creature-only selection is approximated by exact participant role on tile.', ['brain_head'])
    e['anchors'].append(dict(key='brain_head_tile', kind='point', description='Pinned Brain Head tile.', location=dict(x=31954, y=32325, floor=10)))
    e['rules'].append(rule('heal_brain_head', trigger, [dict(kind='heal', subject=dict(role='brain_head'), amount=dict(min=300, max=500))], [dict(kind='in_anchor', subject=dict(role='brain_head'), anchor='brain_head_tile')]))
    e, _, _ = scene('bone_capsule', quest + 'ferumbras_ascension/creaturescripts_bone_capsule.lua', ['events=BoneCapsule'], ['ragiaz'], 'Fixed capsule respawn and25–35k Ragiaz heal retained; teleport to fixed capsule spawn tile proxies dynamic corpse tile. Top-creature selection approximated by registered Ragiaz role at source tile.')
    for anchor, x, y in [('capsule_tile', 33485, 32333), ('ragiaz_tile', 33487, 32333)]:
        e['anchors'].append(dict(key=anchor, kind='point', description='Pinned source point.', location=dict(x=x, y=y, floor=14)))
    died = dict(kind='creature_died', role='bone_capsule')
    e['rules'].append(rule('capsule_respawns', died, [dict(kind='spawn', creature=creature('bone_capsule'), count=1, at=dict(anchor='capsule_tile'), owner='none', health='full')]))
    packet['actor_flags']['bone_capsule'].extend(['SOURCE_NON_GLOBAL_PROXY', 'OWNER_ACCEPTED_NON_GLOBAL_MECHANIC_PROXY'])
    e['rules'].append(rule('ragiaz_recovers', died, [dict(kind='teleport', who=dict(role='ragiaz'), to='capsule_tile'), dict(kind='heal', subject=dict(role='ragiaz'), amount=dict(min=25000, max=35000))], [dict(kind='in_anchor', subject=dict(role='ragiaz'), anchor='ragiaz_tile')]))
    _, e, _, manifest = next(p for p in scenes if p[0] == 'field_fill_soul_cage')
    e['anchors'].append(dict(key='soul_cage_tile', kind='point', description='SoulCagePosition from pinned quest library.', location=dict(x=33709, y=31596, floor=14)))
    e['anchors'].append(dict(key='soul_cage_presence', kind='area', description='Source fixed tile; instance-local presence proxy.', location=dict(boxes=[dict(x=[33709,33709], y=[31596,31596], floor=14)])))
    e['rules'].append(rule('soul_cage_respawns', dict(kind='creature_died', role='soul_cage'), [dict(kind='spawn', creature=creature('soul_cage'), count=1, at=dict(anchor='soul_cage_tile'), owner='none', health='full')], [dict(kind='has_master', role='soul_cage', value=False), dict(kind='creature_present', role='soul_cage', anchor='soul_cage_presence', present=False)], delay=23000))
    src = source('data-otservbr-global/lib/quests/soul_war.lua'); manifest['sources'].append(src)
    manifest['covers']['SoulCageDeath'] = [creature('soul_cage')['key']]
    manifest['entries'].append(dict(source_index=1, source_lines=src['source_lines'], status='mapped', destination='/encounter/rules', resolution='23second fixed-tile respawn retained; source global existing-cage test uses local role-presence proxy; 40second removal/boss-buff chain omitted.'))
    restored.append(dict(monster='soul_cage', source_field='events=SoulCageDeath', encounter=e['identity']['key'], source=src, limitation=manifest['entries'][-1]['resolution']))
    def direct(name, field, sourcepath, effect, formula, area=None, others=(), limitation='', register=True, stage_suffix=''):
        bundle = population / 'bundles' / name
        mon, deps, cat, old = (read(bundle / f) for f in ['monster.json', 'dependencies.json', 'catalog.json', 'manifest.json'])
        row = next(x for x in old['entries'] if x['source_field'] == field)
        section, number = field.split('['); raw = cb.load_monster(donor / row['source_file'])[1][section]
        raw = raw if isinstance(raw, list) else raw['_list']; schedule = raw[int(number[:-1]) - 1]
        suffix = 'field-fill-' + field.replace('[', '-').replace(']', '') + stage_suffix
        refs = {f: ref(f, 'canary:' + f.lower() + '/' + name + '/' + suffix, revision) for f in ['Ability', 'Effect', 'Formula']}
        ability = dict(identity=dict(key=refs['Ability']['key'], revision=revision), kind='spell', range_tiles=schedule.get('range', 0), needs_target=bool(schedule.get('target', False)), needs_direction=False, effects=[refs['Effect']])
        if area:ability['area'] = area
        effect['identity'] = dict(key=refs['Effect']['key'], revision=revision)
        formula['identity'] = dict(key=refs['Formula']['key'], revision=revision)
        if effect['operation'] == 'condition':effect['condition']['speed_formula'] = refs['Formula']
        else:effect['formula'] = refs['Formula']
        src = source(sourcepath); src.update(monster_source=source(row['source_file']), source_field=field)
        if name in {'minotaur_cult_prophet', 'angry_sugar_fairy'}:src.update(global_parity=False, qualification='OWNER_ACCEPTED_NON_GLOBAL_MECHANIC_PROXY')
        for collection, value in [('abilities', ability), ('effects', effect), ('formulas', formula)]:patches.append(patch(name, 'dependencies.json', deps, '/' + collection + '/-', value, src, limitation))
        if register:patches.append(patch(name, 'monster.json', mon, '/behavior/' + section + '/-', dict(ability=refs['Ability'], interval_ms=schedule['interval'], chance_percent=schedule.get('chance', 100)), src, limitation))
        for other in others:
            if creature(other) not in cat['definitions']:patches.append(patch(name, 'catalog.json', cat, '/definitions/-', creature(other), src, limitation))
        packet['actor_flags'].setdefault(name, []).extend(['SOURCE_TYPED_CORE_RESTORED', 'SOURCE_BEHAVIOR_PARTIAL', 'SOURCE_BACKED_OTERYN_SIMPLIFICATION', 'GAMEPLAY_UNVERIFIED'])
        restored.append(dict(monster=name, source_field=field, source=src, limitation=limitation, direct_typed_core=True))
        return refs['Ability']
    direct('minotaur_cult_prophet', 'defenses[1]', 'data-otservbr-global/scripts/spells/monster/minotaur_cult_prophet_mass_healing.lua',
           dict(operation='heal', damage_type='healing', affects=dict(kind='named_creatures', creatures=[creature(n) for n in ['minotaur_cult_prophet', 'minotaur_cult_follower', 'minotaur_cult_zealot']], top_creature_only=True, excludes_caster_name=False, includes_caster=True)),
           dict(kind='range', magnitude=dict(minimum=200, maximum=350)), dict(matrix=dict(north=['..xxx..', '.xxxxx.', 'xxxxxxx', 'xxxCxxx', 'xxxxxxx', '.xxxxx.', '..xxx..'])),
           ['minotaur_cult_follower', 'minotaur_cult_zealot'], 'Source named allies/caster, Circle3x3 matrix and200–350 magnitude preserved. Oteryn rolls per cast instead of donor fixed random server-start roll: accepted non-Global proxy.')
    packet['actor_flags']['minotaur_cult_prophet'].extend(['SOURCE_NON_GLOBAL_PROXY', 'OWNER_ACCEPTED_NON_GLOBAL_MECHANIC_PROXY'])
    direct('spider_queen', 'attacks[1]', 'data-otservbr-global/scripts/spells/monster/spider_queen_wrap.lua',
           dict(operation='condition', duration_ms=30000, condition=dict(type='paralyze', lifetime='fixed_duration')),
           dict(kind='speed_modifier', speed=dict(minimum_multiplier=dict(numerator=0, denominator=1), minimum_offset=40, maximum_multiplier=dict(numerator=0, denominator=1), maximum_offset=40)),
           limitation='30second paralyze retained. Negative source ConditionSpeed factors clamp speed40 per existing converter/engine rule. Player-only gate, outfit422,4500ms nest teleport and mission storage remain omitted.')
    for field, amount in [('attacks[2]', (100, 230)), ('attacks[3]', (130, 280))]:
        path = next(x['source_file'] for x in read(population / 'bundles/angry_sugar_fairy/manifest.json')['entries'] if x['source_field'] == field)
        direct('angry_sugar_fairy', field, path, dict(operation='damage', damage_type='untyped'), dict(kind='range', magnitude=dict(minimum=amount[0], maximum=amount[1])), dict(radius_tiles=3) if field == 'attacks[3]' else None,
               limitation='Source omitted/undefined element preserved as explicit Oteryn untyped-damage proxy; source amount/range/schedule retained. No claim that an element was verified from Wiki.')
        packet['actor_flags']['angry_sugar_fairy'].extend(['SOURCE_NON_GLOBAL_PROXY', 'OWNER_ACCEPTED_NON_GLOBAL_MECHANIC_PROXY'])
    path = 'data-otservbr-global/scripts/spells/monster/foam_splash.lua'
    text = subprocess.check_output(['git', '-C', str(donor), 'show', pin + ':' + path], text=True)
    areas = re.findall(r'createCombatArea\(\{(.*?)\}\)\)', text, re.S)
    if len(areas) != 3:raise ValueError('Unexpected source FoamSplash area blocks')
    e, trigger = spell('foam_stalker', 'attacks[2]', 'foam_splash.lua', 'Source three area matrices,1/2/3second delays and100–300ice magnitude retained. Native current-caster-position casts replace original stored var/position; identity/position retention remains flagged.')
    cat = scenes[-1][2]
    for i, block in enumerate(areas, 1):
        matrix = [[int(n) for n in re.findall(r'\d+', row)] for row in re.findall(r'\{([^{}]+)\}', block)]
        ability = direct('foam_stalker', 'attacks[2]', path, dict(operation='damage', damage_type='ice'), dict(kind='range', magnitude=dict(minimum=100, maximum=300)),
                         dict(matrix=dict(north=cb.Converter.area_rows(matrix))), limitation='Exact source area phase and donor magnitude; caster-position identity proxy qualified separately.', register=False, stage_suffix='-stage-' + str(i))
        cat['definitions'].append(ability)
        e['rules'].append(rule('foam_phase_' + str(i), trigger, [dict(kind='cast', ability=ability, at='subject_position')], delay=i * 1000))
    for slug, e, catalog, manifest in scenes:
        for rule_row in e['rules']:
            for condition in rule_row['conditions']:
                if condition['kind'] != 'in_anchor':continue
                anchor = next(a for a in e['anchors'] if a['key'] == condition['anchor'])
                if anchor['kind'] == 'point':
                    p = anchor['location']; key = anchor['key'] + '_presence'
                    if not any(a['key'] == key for a in e['anchors']):
                        e['anchors'].append(dict(key=key, kind='area', description='Exact source tile predicate.', location=dict(boxes=[dict(x=[p['x'],p['x']], y=[p['y'],p['y']], floor=p['floor'])])))
                    condition['anchor'] = key
        errors = ve.validate(e, catalog, manifest)
        if errors:
            raise ValueError(slug + ': ' + str(errors))
        directory = output / 'encounters' / slug
        directory.mkdir(parents=True, exist_ok=True)
        for filename, obj in [('encounter.json', e), ('catalog.json', catalog), ('manifest.json', manifest)]:
            (directory / filename).write_text(json.dumps(obj, ensure_ascii=False, indent=2) + '\n')
    packet['encounters'] = [dict(slug=slug, relative_dir='encounters/' + slug, expected_sha256=None) for slug, *_ in scenes]
    unique = {}
    for row in restored:unique.setdefault((row['monster'], row['source_field']), row)
    packet['restored_source_rows'] = list(unique.values())
    packet['actor_flags'] = {k: sorted(set(v)) for k, v in packet['actor_flags'].items()}
    remaining_packet(population, audit, output, packet)
    packet['counts'].update(patches=len(patches), affected_actors=len(packet['actor_flags']), source_core_rows=len(packet['restored_source_rows']), encounters=len(scenes))

def remaining_packet(population, audit, output, packet):
    done = {(r['monster'], r['source_field']) for r in packet['restored_source_rows']}
    known = [r for r in csv.DictReader(gzip.open(audit / 'approved-omissions.csv.gz', 'rt'))
             if r['classification'] == 'FLAGGED_PARTIAL_CORE_RETAINED_SOURCE_DEBT' and r['source_field'] != 'corpse']
    old = read(audit / 'source-residual-mechanics.json')
    groups = { (r['monster'], r['source_field']): (group, value['reason']) for group, value in old['root_cause_groups'].items() for r in value['rows'] }
    for r in old['remaining']:
        group, reason = groups[(r['monster'], r['source_field'])]
        known.append(dict(monster=r['monster'], source_field=r['source_field'], resolution=r['source_gap'], owner_group=group, concrete_gap=reason))
    pending = []
    for row in known:
        if (row['monster'], row['source_field']) in done:continue
        m = read(population / 'bundles' / row['monster'] / 'manifest.json')
        source_row = next(e for e in m['entries'] if e['source_field'] == row['source_field'])
        text = row.get('resolution', ''); lower = text.lower()
        group = row.get('owner_group') or ('QUEST_INTERACTION_STORAGE_OR_CORPSE' if any(s in lower for s in ['quest', 'corpse', 'storage', 'targuna', 'portal', 'vortex']) else 'TARGET_SELECTION_OR_FIGHT_OWNER_STATE')
        if 'skill reducer' in lower:group = 'PER_TARGET_VOCATION_DISPATCH'
        if 'healing 2' in lower:group = 'GROUND_ITEM_PREDICATE'
        if 'doctor marrow' in lower:group = 'TARGET_POSITION_RETENTION_DISTANCE_FALLOFF_AND_REPEATED_PARALYZE'
        if 'omrafir beam' in lower:group = 'CASTER_FACING_DELAYED_AREA_AND_REGENERATION_SUBID'
        name = row['monster']
        if not row.get('owner_group') and (name.startswith('goshnar') or name in {'soul_sphere','symbol_of_hatred','necromantic_focus'}):group = 'SOUL_WAR_ZONE_TORMENT_AND_MUTABLE_BOSS_STATE'
        if name == 'grand_master_oberon':group = 'OBERON_DIALOGUE_CHALLENGE_COOLDOWN_AND_IMMUNITY'
        if name in {'lord_azaram','azaram_s_soul','earl_osam','magical_sphere'}:group = 'GRAVE_DANGER_SHARED_PHASE_COUNTERS_AND_LINKED_ACTORS'
        if name == 'ghulosh':group = 'LINKED_BOSS_STAGES_AND_SHARED_FIGHT_OWNER'
        if row['source_field'].startswith('events=Targuna'):group = 'EVENT_REGISTRATION_AND_QUEST_PROGRESS_SOURCE_VERIFICATION'
        operations = []
        if row['source_field'].startswith('mType.'):
            raw = subprocess.check_output(['git','-C','/workspace/monster-reference-sources/canary','show','47dfd51f45280a59a1d3e50ba7edd573d7234446:' + source_row['source_file']], text=True)
            operations = sorted(set(re.findall(r'(getStorageValue|setStorageValue|createMonster|teleportTo|setOutfit|getSpectators|setTarget|applyZoneEffect|increaseTorment|setReward|remove|setHealth|addHealth)\s*\(', raw)))
        pending.append(dict(monster=row['monster'], source_field=row['source_field'], source_row=source_row,
                            owner_group=group, concrete_gap=row.get('concrete_gap', text), source_file_operations_context=operations, architecture_issue=162,
                            proposed_resolution='Bind original callback to a source-specific encounter owner/roles and explicit predicates; retain practical actor core and flags until its exact contract is accepted.',
                            current_source_core_restored=False))
    document = dict(schema='OTERYN_MONSTER_OWNER_RESIDUAL_PACKET/v1', baseline_index_sha256=BASELINE,
                    known_source_rows_before=80, restored_source_rows=len(done), unconverted_core_rows=pending,
                    separate_familiar_challenge_rows=2, exact_parity_debt_retained_for_restored_rows=True,
                    missing_dark_merudri_global_parameters=['damage range', 'exact rear-wave shape', 'range and schedule'])
    (output / 'decisions-162.json').write_text(json.dumps(document, ensure_ascii=False, indent=2) + '\n')
    packet['remaining_owner_packet'] = dict(file='decisions-162.json', sha256=sha(output / 'decisions-162.json'), unconverted_core_rows=len(pending))

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--population', type=Path, required=True)
    parser.add_argument('--audit', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(prepare(args.population, args.audit, args.output)['counts']))
