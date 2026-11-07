"""Source-positive optional-field audit and bounded native typed cores; no runtime mutation."""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

BASELINE = '1b87ba6bee339bbd0e485cecaf7d8862607a8bf473f8f5f44221142b79afa1c5'
PIN = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
REV = 'canary-47dfd51f'


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def reference(family, slug):
    return dict(family=family, key='canary:' + family.lower() + '/' + slug, revision=REV)


def patch(monster, file, document, pointer, value, source, reason):
    parts = pointer.strip('/').split('/')
    previous = document
    present = True
    try:
        for part in parts:
            if part == '-':
                present, previous = False, None
                break
            previous = previous[int(part)] if isinstance(previous, list) else previous[part]
    except (KeyError, IndexError):
        present, previous = False, None
    return dict(monster=monster, file=file, pointer=pointer, expected_present=present,
                expected_value=previous, value=value, source=source, reason=reason)


def source_evidence(repository, path, start=None):
    data = subprocess.check_output(['git', '-C', str(repository), 'show', PIN + ':' + path])
    lines = list(range(start or 1, len(data.splitlines()) + 1))
    return dict(kind='git', repository='opentibiabr/canary', revision=PIN, path=path,
                source_file=path, source_line=start or 1, source_lines=lines,
                sha256=hashlib.sha256(data).hexdigest())


def source_positive_audit(population):
    """Check source dispositions against the concrete target, not against schema optionality."""
    counts, observations, unresolved = {}, [], []
    for bundle in sorted((population / 'bundles').iterdir()):
        monster, manifest = read(bundle / 'monster.json'), read(bundle / 'manifest.json')
        c = monster['creature']
        for key in ['damage_reflection', 'healing_from_damage', 'death_residue', 'bosstiary', 'flags', 'immunities']:
            label = key + ('_present' if key in c else '_absent')
            counts[label] = counts.get(label, 0) + 1
        for entry in manifest['entries']:
            field = entry['source_field']
            if not field.startswith(('reflects[', 'heals[', 'race', 'flags.', 'immunities', 'bosstiary')):
                continue
            state = 'SOURCE_DISPOSITION_' + entry['status'].upper()
            dest = entry.get('destination', '')
            if entry['status'] == 'mapped' and dest.startswith('/monster/'):
                value, exists = monster, True
                try:
                    for part in dest[len('/monster/'):].split('/'):
                        value = value[int(part)] if isinstance(value, list) else value[part]
                except (KeyError, IndexError):
                    exists = False
                state = 'SOURCE_POSITIVE_PRESENT' if exists else 'MAPPED_DESTINATION_MISSING_REQUIRES_REVIEW'
                if not exists:
                    unresolved.append(dict(monster=bundle.name, entry=entry))
            observations.append(dict(monster=bundle.name, source_field=field, source_file=entry.get('source_file'),
                                     source_line=entry.get('source_line'), state=state, destination=dest,
                                     resolution=entry.get('resolution')))
    return dict(schema='OTERYN_SOURCE_POSITIVE_BEHAVIOR_AUDIT/v1', baseline_index_sha256=BASELINE,
                counts=counts, observations=observations, mapped_destination_gaps=unresolved,
                applicability=['Empty reflection/healing collections are valid where no positive source table exists.',
                               '534 source race none dispositions must not gain a fabricated residue.',
                               'Boss classification does not imply Bosstiary registration eligibility.',
                               'Player-only scripted reflection already has an Encounter core; adding passive reflection would duplicate or widen it.',
                               'Creature.damage_conditions and Creature.flags.boss are not accepted schema fields.'])


def census_dispositions(population):
    import gzip
    path = Path('/workspace/monster-field-next-20261002/census/fill-candidates.json.gz')
    rows = json.loads(gzip.decompress(path.read_bytes()))
    out = []
    for row in rows:
        pointer = row['pointer']
        if pointer == '/creature/encyclopedia':
            continue
        if row['monster'] in {'a_greedy_eye', 'icicle'}:
            status, reason = 'TYPED_CORE_RESTORED_NON_GLOBAL_PROXY', 'Packet restores source area/schedule and exact named target; raw HP mutation/aggro parity retained as debt.'
        elif pointer in {'/creature/summoning/mana_cost', '/creature/resistances'}:
            status, reason = 'STATISTICS_LANE_REVIEW', 'Dedicated statistics producer owns guarded applicable value or source-variant/immunity qualification.'
        elif pointer == '/creature/corpse_item':
            status, reason = 'ITEM_ADMISSION_LANE_REVIEW', 'Dedicated reference producer owns exact Item identity; no invented native Item aliases.'
        elif pointer == '/formulas':
            status, reason = 'NO_NUMERIC_FORMULA_APPLICABLE', 'Appearance transformations and no-op skill reducers have no numeric damage/heal magnitude; existing typed effects are present.'
        elif pointer == '/behavior/voices':
            status, reason = 'SOURCE_EMPTY_TEXT_NOOP', 'The sole source utterance is empty text; accepted voice schema requires nonempty text. No audible phrase to import.'
        else:
            status, reason = 'ARCHITECT_DOMAIN_162', 'Source boolean rewardBoss/event names do not identify a typed reward Encounter or native event binding owner; retain explicit source data and domain proposal in 162.'
        proof = row.get('proof', {})
        entries = read(population/'bundles'/row['monster']/'manifest.json')['entries']
        selected = [e for e in entries if e['source_field'].startswith(row.get('source_field', '__none__'))]
        out.append(dict(monster=row['monster'], pointer=pointer, disposition=status, reason=reason, proof=proof,
                        existing_manifest_evidence=selected))
    counts = {}
    for row in out:
        counts[row['disposition']] = counts.get(row['disposition'], 0) + 1
    return dict(baseline_index_sha256=BASELINE, census_sha256=sha(path), counts=counts, rows=out)


def prepare(population, donor, output):
    population, donor, output = map(Path, (population, donor, output))
    if sha(population / 'population-index.json') != BASELINE:
        raise ValueError('Unexpected baseline index SHA')
    sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools/content-schema/monster-authoring'))
    import canary_batch as cb
    patches, flags, restored = [], {}, []
    cases = [
        ('a_greedy_eye', 'attacks[1]', 'data-otservbr-global/scripts/quests/soul_war/spell-eye_beam.lua',
         ['x', 'x', 'C'], 'poor_soul', 1000, True),
        ('icicle', 'defenses[1]', 'data-otservbr-global/scripts/spells/monster/icicle_heal.lua',
         ['..xxx..', '.xxxxx.', 'xxxxxxx', 'xxxCxxx', 'xxxxxxx', '.xxxxx.', '..xxx..'], 'dragon_egg', 100, False)]
    for name, field, script, matrix, target, amount, drowning in cases:
        bundle = population / 'bundles' / name
        monster, deps, catalog, manifest = (read(bundle / f) for f in ['monster.json', 'dependencies.json', 'catalog.json', 'manifest.json'])
        original = next(e for e in manifest['entries'] if e['source_field'] == field)
        pinned_monster = source_evidence(donor, original['source_file'], original.get('source_line'))
        if sha(donor/original['source_file']) != pinned_monster['sha256']:
            raise ValueError('Donor file differs from pinned source: ' + original['source_file'])
        raw = cb.load_monster(donor / original['source_file'])[1]
        section, number = field.split('[')
        rows = raw[section] if isinstance(raw[section], list) else raw[section]['_list']
        schedule = rows[int(number[:-1]) - 1]
        prefix = name + '/optional-field-core-' + field.replace('[', '-').replace(']', '')
        aref, fref, eref = (reference(f, prefix) for f in ['Ability', 'Formula', 'Effect'])
        affects = dict(kind='named_creatures', creatures=[reference('Creature', target)],
                       top_creature_only=drowning, excludes_caster_name=False, includes_caster=False)
        effects = [dict(identity=dict(key=eref['key'], revision=REV), operation='damage', damage_type='untyped',
                        formula=fref, affects=affects, presentation=dict(impact_asset_binding='canary.appearance:effect/' + ('smallclouds' if drowning else 'magic_blue')))]
        if drowning:
            normal = reference('Effect', prefix + '-drowning')
            effects.insert(0, dict(identity=dict(key=normal['key'], revision=REV), operation='damage',
                                   damage_type='drowning', formula=fref, presentation=dict(impact_asset_binding='canary.appearance:effect/smallclouds')))
        ability = dict(identity=dict(key=aref['key'], revision=REV), kind='spell', range_tiles=schedule.get('range', 0),
                       needs_target=bool(schedule.get('target', False)), needs_direction=drowning,
                       area=dict(matrix=dict(north=matrix)), effects=[reference('Effect', e['identity']['key'].split('canary:effect/')[1]) for e in effects])
        formula = dict(identity=dict(key=fref['key'], revision=REV), kind='range', magnitude=dict(minimum=amount, maximum=amount))
        src = source_evidence(donor, script)
        src.update(source_field=field, monster_source=pinned_monster,
                   original_omitted_row=original, global_parity=False, qualification='OWNER_ACCEPTED_NON_GLOBAL_HP_DELTA_PROXY')
        if not drowning:
            src['area_definition'] = source_evidence(donor, 'data/scripts/lib/register_spells.lua')
        reason = ('Source named-target health reduction restored as supported typed damage. This can differ from addHealth(-N) through immunity, '
                  'mitigation, combat admission and target selection; never claimed exact Global. ' +
                  ('Source ordinary drowning1000 beam retained separately; named PoorSoul1000 proxy keeps top-creature filter.' if drowning else
                   'Only registered DragonEgg is affected; source retargeting to egg remains omitted.'))
        for collection, values in [('abilities', [ability]), ('effects', effects), ('formulas', [formula])]:
            for value in values:
                patches.append(patch(name, 'dependencies.json', deps, '/' + collection + '/-', value, src, reason))
        patches.append(patch(name, 'monster.json', monster, '/behavior/' + section + '/-',
                             dict(ability=aref, interval_ms=schedule['interval'], chance_percent=schedule.get('chance', 100)), src, reason))
        asset = 'canary.appearance:effect/' + ('smallclouds' if drowning else 'magic_blue')
        if asset not in catalog['assets']:
            patches.append(patch(name, 'catalog.json', catalog, '/assets/-', asset, src, reason))
        cref = reference('Creature', target)
        if cref not in catalog['definitions']:
            patches.append(patch(name, 'catalog.json', catalog, '/definitions/-', cref, src, reason))
        flags[name] = ['SOURCE_TYPED_CORE_RESTORED', 'SOURCE_BEHAVIOR_PARTIAL', 'SOURCE_NON_GLOBAL_PROXY',
                       'OWNER_ACCEPTED_NON_GLOBAL_HP_DELTA_PROXY', 'GAMEPLAY_UNVERIFIED']
        restored.append(dict(monster=name, source_field=field, target=cref, source=src, remaining_exact_parity_debt=reason))
    output.mkdir(parents=True, exist_ok=True)
    audit = source_positive_audit(population)
    dispositions = census_dispositions(population)
    (output/'census-dispositions.json.gz').write_bytes(__import__('gzip').compress(json.dumps(dispositions, ensure_ascii=False).encode(), mtime=0))
    import gzip
    (output / 'source-positive-audit.json.gz').write_bytes(gzip.compress(json.dumps(audit, ensure_ascii=False).encode(), mtime=0))
    packet = dict(schema='OTERYN_MONSTER_FIELDPATCH/v1', lane='behavior', baseline_index_sha256=BASELINE,
                  patches=patches, actor_flags=flags, restored_source_rows=restored,
                  source_positive_audit_sha256=sha(output / 'source-positive-audit.json.gz'),
                  census_dispositions_sha256=sha(output/'census-dispositions.json.gz'),
                  counts=dict(patches=len(patches), affected_actors=len(flags), source_rows_restored=2,
                              source_positive_destination_gaps=len(audit['mapped_destination_gaps'])),
                  qualification='SOURCE_POSITIVE_TYPED_CORE_WITH_EXPLICIT_NON_GLOBAL_HP_DELTA_PROXY')
    (output / 'fieldpatch.json').write_text(json.dumps(packet, ensure_ascii=False, indent=2) + '\n')
    return packet


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ['population', 'donor', 'output']:
        parser.add_argument('--' + arg, required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(prepare(args.population, args.donor, args.output)['counts']))
