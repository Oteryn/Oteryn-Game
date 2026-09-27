"""Import readiness of every Canary monster file with the current converter and validator.

Evidence tooling only. Converts each file under MONSTER_DIR in memory (nothing is written except the
report), validates the bundle structure and counts the manifest rows that still block admission.
The report lists the fully resolved monsters and, per blocker kind, how many monsters it affects.

Usage: python population_census.py --canary <Canary checkout> [--out FILE]
"""
import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

import canary_batch as cb
import validate_monster as vm

ROOT = Path(__file__).resolve().parent
OPEN = ('unsupported_source_field', 'unresolved_semantics', 'unresolved_dependency', 'partial_text')


def blocker(entry):
    field = re.sub(r'\[\d+\]', '[]', re.sub(r'=.*', '', entry['source_field']))
    return f'{entry["status"]} {field}'


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--out', type=Path, default=ROOT / 'samples' / 'population-canary-47dfd51f.json')
    args = parser.parse_args()
    objects = cb.load_appearance_objects(args.canary / 'data/items/appearances.dat')
    items = cb.load_items_xml(args.canary / 'data/items/items.xml')
    names, index = cb.name_index(objects, items)
    converter = cb.Converter(args.canary, objects, items, names, index)

    files = sorted((args.canary / cb.MONSTER_DIR).rglob('*.lua'))
    outcome = Counter()
    resolved, not_converted, invalid = [], [], []
    blockers, examples = Counter(), defaultdict(list)
    for path in files:
        relative = str(path.relative_to(args.canary / cb.MONSTER_DIR))[:-4]
        converter.pending_definitions = set()
        try:
            slug, monster, deps, catalog, manifest, _ = converter.convert(relative)
        except Exception as exc:  # files that need quest configuration or are not monster types
            outcome['not_converted'] += 1
            not_converted.append({'file': relative, 'error': f'{type(exc).__name__}: {str(exc).splitlines()[0][:120]}'})
            continue
        local = {i['identity']['key'] for i in deps['items']}
        for family, key in sorted(converter.pending_definitions):
            if cb.ref(family, key) not in catalog['definitions'] and key != monster['creature']['identity']['key'] and key not in local:
                catalog['definitions'].append(cb.ref(family, key))
        errors = vm.validate(monster, deps, catalog, None)
        if errors:
            outcome['structure_invalid'] += 1
            invalid.append({'file': relative, 'errors': errors[:3]})
            continue
        open_rows = {blocker(e) for e in manifest['entries'] if e['status'] in OPEN}
        if open_rows:
            outcome['blocked'] += 1
            for kind in open_rows:
                blockers[kind] += 1
                if len(examples[kind]) < 5:
                    examples[kind].append(slug)
        else:
            outcome['fully_resolved'] += 1
            resolved.append(slug)

    report = {'source': {'repository': cb.REPOSITORY, 'revision': cb.REVISION, 'monster_dir': cb.MONSTER_DIR},
              'scope': 'In-memory conversion of every monster file; structure validation plus open manifest rows. '
                       'Not runtime qualification and no wiki comparison.',
              'monster_files': len(files), 'outcome': dict(sorted(outcome.items())),
              'blockers_by_monster_count': [{'blocker': k, 'monsters': n, 'examples': examples[k]}
                                            for k, n in sorted(blockers.items(), key=lambda kv: (-kv[1], kv[0]))],
              'not_converted': not_converted, 'structure_invalid': invalid, 'fully_resolved': sorted(resolved)}
    args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps({'monster_files': len(files), **report['outcome']}))


if __name__ == '__main__':
    main()
