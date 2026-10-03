"""Merge bounded source-backed loot and spell supplements into a new candidate.

Source inputs stay immutable. Numeric uncertainty and gameplay limitations are
retained in the output receipt; schema admission does not assert live parity.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

import complete_creature_dependencies as population


def read(path):
    return json.loads(path.read_text())


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def merge(baseline, loot, spells, encounters, output):
    if output.exists() and any(output.iterdir()):
        raise ValueError('output must be empty')
    protected = [baseline.resolve(), loot.resolve(), spells.resolve(), encounters.resolve(), population.ROOT]
    if any(output.resolve() == p or p in output.resolve().parents for p in protected):
        raise ValueError('output would modify a source input')
    index = read(baseline / 'population-index.json')
    rows = {r['monster']: r for r in index['monsters']}
    spell_report = read(spells / 'completion.json')
    loot_report = read(loot / 'loot-applied-receipt.json')
    if loot_report['index_input_sha256'] != sha(baseline / 'population-index.json'):
        raise ValueError('loot packet belongs to a different baseline')
    loot_report['actors'] = [
        {**r, 'original_bundle_digest': rows[r['monster']]['sha256'],
         'bundle_digest': r['bundle_sha256']} for r in loot_report['changed']]
    sources = {}
    flags = {}
    corrections = output / 'combined-bundles'
    for report, component in ((loot_report, loot), (spell_report, spells)):
        for actor in report['actors']:
            name = actor['monster']
            if name not in rows:
                raise ValueError('supplement adds an unexpected identity')
            original = baseline / 'bundles' / name
            if population.admission.bundle_digest(original) != actor['original_bundle_digest']:
                raise ValueError('supplement belongs to a different baseline')
            source = component / ('patched-bundles' if component == loot else 'bundles') / name
            if population.admission.bundle_digest(source) != actor['bundle_digest']:
                raise ValueError('supplement digest mismatch')
            if name in sources:
                # Each worker owns separate fields; combine only that exact split.
                documents = [read(sources[name] / f) for f in population.FILES]
                additions = [read(source / f) for f in population.FILES]
                if additions[0].get('loot') != read(original / 'monster.json').get('loot'):
                    raise ValueError('spell supplement changed loot')
                additions[0]['loot'] = documents[0]['loot']
                additions[0]['creature']['loot'] = documents[0]['creature']['loot']
                for definition in documents[2]['definitions']:
                    if definition not in additions[2]['definitions']:
                        additions[2]['definitions'].append(definition)
                def loot_entry(entry):
                    return (str(entry.get('destination', '')).startswith('/monster/loot/')
                            or str(entry.get('source_field', '')).startswith(('loot[', 'Loot2.')))
                additions[3]['entries'] = [e for e in additions[3]['entries'] if not loot_entry(e)]
                for entry in documents[3]['entries']:
                    if loot_entry(entry):
                        entry = copy.deepcopy(entry)
                        src = documents[3]['sources'][entry['source_index']]
                        if src not in additions[3]['sources']:
                            additions[3]['sources'].append(src)
                        entry['source_index'] = additions[3]['sources'].index(src)
                        additions[3]['entries'].append(entry)
                target = corrections / name
                for filename, value in zip(population.FILES, additions):
                    write(target / filename, value)
                source = target
            sources[name] = source
            flags.setdefault(name, set()).update(actor['completion_flags'])
    output.mkdir(parents=True, exist_ok=True)
    for row in spell_report.get('additional_index_rows', []):
        name = row['monster']
        if name in rows or name in sources:
            raise ValueError('additional source actor overwrites an existing identity')
        source = spells / 'bundles' / name
        if population.admission.bundle_digest(source) != row['sha256']:
            raise ValueError('additional source actor digest mismatch')
        rows[name] = row
        sources[name] = source
        flags[name] = set(row.get('completion_flags', []))
    supplements = [(p, {**rows[name], 'sha256': population.admission.bundle_digest(p),
                        'completion_flags': sorted(set(rows[name].get('completion_flags', [])) | flags[name])})
                   for name, p in sorted(sources.items())]
    merged = population.merge_population(index, baseline / 'bundles', output, supplements)
    write(output / 'population-index.json', merged)
    encounter_output = output / 'encounters'
    shutil.copytree(encounters, encounter_output)
    for entry in spell_report['encounters']:
        name = entry['encounter']
        for filename in ('encounter.json', 'manifest.json', 'catalog.json'):
            shutil.copyfile(spells / 'encounters' / name / filename, encounter_output / name / filename)
    quality = {'schema': 'OTERYN_MONSTER_SOURCE_GAP_COMPLETION/v1',
               'baseline_index_sha256': sha(baseline / 'population-index.json'),
               'prepared_count': len(merged['monsters']), 'changed_monsters': sorted(sources),
               'loot_completion': loot_report, 'spell_completion': spell_report,
               'baseline_quality': read(baseline / 'completion-quality.json'),
               'live_gameplay_verified': False, 'runtime_activated': False}
    write(output / 'completion-quality.json', quality)
    return quality


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('baseline', 'loot', 'spells', 'encounters', 'item-map', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    result = merge(args.baseline, args.loot, args.spells, args.encounters, args.output)
    subprocess.run([sys.executable, str(Path(population.admission.__file__)),
                    '--bundles', str(args.output / 'bundles'), '--index', str(args.output / 'population-index.json'),
                    '--encounters', str(args.output / 'encounters'), '--item-map', str(args.item_map),
                    '--out', str(args.output / 'creature-admission-stage.json')], check=True)
    stage = read(args.output / 'creature-admission-stage.json')
    result['native_counts'] = stage['counts']
    result['held'] = stage['deferred']
    write(args.output / 'completion-quality.json', result)
    print(json.dumps({'prepared': result['prepared_count'], 'changed': len(result['changed_monsters']),
                      'native_counts': stage['counts'], 'held': stage['deferred']}))


if __name__ == '__main__':
    main()
