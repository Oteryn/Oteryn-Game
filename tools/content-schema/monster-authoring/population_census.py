"""Import readiness of every Canary monster file with the current converter and validator.

Evidence tooling only. Converts each file under MONSTER_DIR in memory, applies the reference-date
wiki values of WIKI (owner decision D15), validates the bundle structure and counts the manifest rows
that still block admission. The report lists the fully resolved monsters and, per blocker kind, how
many monsters it affects. With --bundles the four bundle files of every fully resolved monster are
written under that directory (outside the repository) and INDEX records one SHA-256 per bundle, so a
regenerated population can be checked against the committed index without committing ~40 MB.

Usage: python population_census.py --canary <Canary checkout> [--out FILE] [--bundles DIR]
"""
import argparse
import hashlib
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

import canary_batch as cb
import validate_monster as vm

ROOT = Path(__file__).resolve().parent
WIKI = ROOT / 'samples' / 'wiki-population-2026-07-28.json'
INDEX = ROOT / 'samples' / 'population-bundles-canary-47dfd51f.json'
BUNDLE_FILES = ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json')
OPEN = ('unsupported_source_field', 'unresolved_semantics', 'unresolved_dependency', 'partial_text')


def blocker(entry):
    field = re.sub(r'\[\d+\]', '[]', re.sub(r'=.*', '', entry['source_field']))
    pattern = re.search(r'D18 pattern `([a-z_]+)`', entry.get('resolution', ''))
    return f'{entry["status"]} {field}' + (f' -> {pattern.group(1)}' if pattern else '')


def dump(value):
    return json.dumps(value, ensure_ascii=False, indent=2) + '\n'


def bundle_digest(texts):
    """SHA-256 over the four bundle files in BUNDLE_FILES order, each prefixed by its name and byte length."""
    digest = hashlib.sha256()
    for name, text in zip(BUNDLE_FILES, texts):
        data = text.encode('utf-8')
        digest.update(f'{name}\0{len(data)}\0'.encode('ascii') + data)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--out', type=Path, default=ROOT / 'samples' / 'population-canary-47dfd51f.json')
    parser.add_argument('--bundles', type=Path, help='write the fully resolved bundles here and refresh INDEX')
    args = parser.parse_args()
    objects = cb.load_appearance_objects(args.canary / 'data/items/appearances.dat')
    items = cb.load_items_xml(args.canary / 'data/items/items.xml')
    names, index = cb.name_index(objects, items)
    converter = cb.Converter(args.canary, objects, items, names, index)
    wiki_text = WIKI.read_text(encoding='utf-8')
    converter.wiki = {m['monster']: m for m in json.loads(wiki_text)['monsters']}

    files = sorted((args.canary / cb.MONSTER_DIR).rglob('*.lua'))
    outcome = Counter()
    resolved, not_converted, invalid = [], [], []
    blockers, examples = Counter(), defaultdict(list)
    adopted, digests = Counter(), {}
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
        wiki_sources = {i for i, s in enumerate(manifest['sources']) if s.get('kind') == 'mediawiki'}
        wiki_rows = [e for e in manifest['entries'] if e['source_index'] in wiki_sources and e['status'] == 'mapped']
        for entry in wiki_rows:
            adopted[entry['source_field'].split('.')[0] if entry['source_field'].startswith('Loot2') else entry['source_field']] += 1
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
            outcome['wiki_adopted_monsters'] += bool(wiki_rows)
            texts = [dump(v) for v in (monster, deps, catalog, manifest)]
            digests[slug] = {'file': relative, 'sha256': bundle_digest(texts)}
            if args.bundles:
                target = args.bundles / slug
                target.mkdir(parents=True, exist_ok=True)
                for name, text in zip(BUNDLE_FILES, texts):
                    (target / name).write_text(text, encoding='utf-8', newline='\n')

    report = {'source': {'repository': cb.REPOSITORY, 'revision': cb.REVISION, 'monster_dir': cb.MONSTER_DIR},
              'wiki_reference': {'file': str(WIKI.relative_to(ROOT)), 'sha256': hashlib.sha256(wiki_text.encode('utf-8')).hexdigest()},
              'scope': 'In-memory conversion of every monster file with the D15 wiki values applied; structure validation '
                       'plus open manifest rows. Not runtime qualification.',
              'monster_files': len(files), 'outcome': dict(sorted(outcome.items())),
              'wiki_adopted_rows': dict(sorted(adopted.items(), key=lambda kv: (-kv[1], kv[0]))),
              'blockers_by_monster_count': [{'blocker': k, 'monsters': n, 'examples': examples[k]}
                                            for k, n in sorted(blockers.items(), key=lambda kv: (-kv[1], kv[0]))],
              'not_converted': not_converted, 'structure_invalid': invalid, 'fully_resolved': sorted(resolved)}
    args.out.write_text(dump(report), encoding='utf-8', newline='\n')
    head = {'source': report['source'], 'wiki_reference': report['wiki_reference'], 'generator': 'population_census.py',
            'digest': bundle_digest.__doc__, 'bundle_files': list(BUNDLE_FILES), 'bundles': len(digests), 'monsters': []}
    lines = ',\n'.join('    ' + json.dumps({'monster': s, **d}, ensure_ascii=False, separators=(',', ':')) for s, d in sorted(digests.items()))
    INDEX.write_text(dump(head)[:-len('\n  "monsters": []\n}\n')] + '\n  "monsters": [\n' + lines + '\n  ]\n}\n',
                     encoding='utf-8', newline='\n')
    print(json.dumps({'monster_files': len(files), **report['outcome']}))


if __name__ == '__main__':
    main()
