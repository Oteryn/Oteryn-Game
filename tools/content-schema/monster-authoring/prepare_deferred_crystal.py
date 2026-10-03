"""Prepare the 22 deferred Crystal candidates without changing population/admission.

The pinned OTS facts remain OTS_HYPOTHESIS_ONLY. Drafts use the normal schemas and
converter, but neither successful conversion nor validation authorizes admission.
The unpinned wiki preparation probe is reported separately and never adopted.
Usage: python prepare_deferred_crystal.py --canary CHECKOUT --crystal CHECKOUT --out EXTERNAL_DIR
"""
import argparse
import hashlib
import json
import re
from pathlib import Path, PurePosixPath

import canary_batch as cb
import crystal_batch
import official_library
import validate_monster as vm
import wiki_only_candidates

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
OPEN = {'unsupported_source_field', 'unresolved_semantics', 'unresolved_dependency', 'partial_text'}
BUNDLE_FILES = ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json')
FACT_FIELDS = ('experience', 'health', 'maxHealth', 'raceId', 'outfit', 'race', 'corpse', 'speed', 'manaCost',
               'Bestiary', 'Bosstiary', 'flags', 'changeTarget', 'strategiesTarget', 'light', 'loot', 'attacks',
               'defenses', 'elements', 'immunities', 'voices', 'summons', 'events')


def deferred_rows(sample):
    """Select only the classified nonordinary Crystal rows, never widen the default roster."""
    rows = [row for row in sample['monsters'] if row['crystal_other_dir'] and row.get('kind') != 'real monster']
    paths = []
    for row in rows:
        path = PurePosixPath(row['crystal_other_dir'])
        if path.is_absolute() or '..' in path.parts or path.suffix != '.lua' or '\\' in str(path):
            raise ValueError('unsafe candidate source path: ' + str(path))
        paths.append(str(path))
    if len(paths) != len(set(paths)):
        raise ValueError('duplicate deferred candidate source path')
    return rows


def outside_repository(out):
    out = out.resolve()
    if out == REPO or REPO in out.parents:
        raise ValueError('draft output must be outside the repository; admission is a separate operation')
    return out


def source_evidence(row, crystal):
    path = crystal / crystal_batch.MONSTER_ROOT / row['crystal_other_dir']
    content = path.read_bytes()
    lines = content.decode('utf-8').splitlines()
    evaluation_error = None
    try:
        name, raw, callbacks = cb.load_monster(path)
    except Exception as exc:
        # Quest configuration can prevent evaluation before register(). Keep only
        # literal scalar assignments; do not fabricate configuration or defaults.
        evaluation_error = type(exc).__name__ + ': ' + str(exc).splitlines()[0][:300]
        name, raw, callbacks = crystal_batch.created_name(path), {}, {}
        for line in lines:
            match = re.match(r'^\s*monster\.(\w+)\s*=\s*(.*?)\s*(?:--.*)?$', line)
            if match:
                try:
                    raw[match[1]] = json.loads(match[2])
                except (ValueError, TypeError):
                    pass
    if not name or name.lower() != row['name']:
        raise ValueError('candidate name does not match its registered source type')
    fields = {key: {'value': raw[key], 'source_line': next((i for i, line in enumerate(lines, 1)
              if re.match(r'^\s*monster\.' + re.escape(key) + r'\s*=', line)), None)}
              for key in FACT_FIELDS if key in raw}
    present = {match[1] for line in lines if (match := re.match(r'^\s*monster\.(\w+)\s*=', line))}
    return {'classification': 'OTS_HYPOTHESIS_ONLY', 'repository': crystal_batch.REPOSITORY,
            'revision': crystal_batch.REVISION, 'file': str(path.relative_to(crystal)),
            'blob_sha1': cb.blob_id(content), 'sha256': hashlib.sha256(content).hexdigest(),
            'registered_name': name, 'fields': fields, 'callbacks': callbacks,
            'evaluation_error': evaluation_error,
            'source_uncertainty_notes': [{'source_line': i, 'note': line.split('--', 1)[1].strip()}
                for i, line in enumerate(lines, 1) if '--' in line and
                re.search(r'confirm|todo|fixme|guess|verif|placeholder', line.split('--', 1)[1], re.I)],
            'source_absent_fields': [key for key in FACT_FIELDS if key not in present],
            'source_unevaluated_fields': [key for key in FACT_FIELDS if key in present and key not in fields]}


def preparation_issues(row, evidence):
    """Expose source/probe conflicts and unknowns without choosing either value."""
    fields = evidence['fields']
    issues = ['classification/encounter admission pending (§10.3)',
              'reference-date wiki evidence not collected for this deferred source']
    for probe, source in (('wiki_hp', 'health'), ('wiki_exp', 'experience')):
        value = row.get(probe)
        if value is None:
            issues.append(probe + ' unknown in the preparation probe')
        elif source in fields and str(fields[source]['value']) != value.replace(',', ''):
            issues.append(probe + ' conflicts with Crystal ' + source)
    flags = fields.get('flags', {}).get('value', {})
    for probe, flag in (('wiki_summon', 'summonable'), ('wiki_convince', 'convinceable')):
        value = row.get(probe)
        if value is None:
            issues.append(probe + ' unknown in the preparation probe')
        elif flag in flags and flags[flag] != (value not in ('--', '-', 'no')):
            issues.append(probe + ' conflicts with Crystal flags.' + flag)
    if evidence['callbacks']:
        issues.append('source callbacks require native behavior/encounter review')
    if evidence.get('evaluation_error'):
        issues.append('quest configuration prevents complete source evaluation; no defaults guessed')
    if evidence.get('source_uncertainty_notes'):
        issues.append('source explicitly marks uncertain facts; confirmation pending')
    return issues


def dump(value):
    return json.dumps(value, ensure_ascii=False, indent=2) + '\n'


def prepare(canary, crystal, out):
    out = outside_repository(out)
    sample = json.loads(wiki_only_candidates.SAMPLE.read_text(encoding='utf-8'))
    rows = deferred_rows(sample)
    paths = tuple(crystal_batch.MONSTER_ROOT + '/' + row['crystal_other_dir'] for row in rows)
    crystal_batch.require_pinned(crystal, 'CrystalServer', crystal_batch.REVISION, paths, extra=False)
    tracked = set(crystal_batch.git(crystal, 'ls-tree', '-r', '--name-only', crystal_batch.REVISION, '--', *paths).splitlines())
    if set(paths) - tracked:
        raise ValueError('deferred candidate source is not tracked at the pinned revision')
    conv = crystal_batch.converter(canary, crystal)
    # Only qualified existing cut/library records may be adopted, never wiki_* probe values.
    conv.wiki = {m['monster']: m for m in json.loads(crystal_batch.WIKI.read_text())['monsters']}
    conv.official = {m['monster']: m for file in (official_library.CRYSTAL_SAMPLE, official_library.CRYSTAL_EXTRA_SAMPLE)
                     for m in json.loads(file.read_text())['monsters']}
    out.mkdir(parents=True, exist_ok=True)
    report = {'classification': 'OTS_HYPOTHESIS_ONLY', 'admission_authorized': False,
              'scope': 'Deferred source preparation only; no population, binding or content changes.',
              'reference_cut': '2026-09-27', 'source_revision': crystal_batch.REVISION,
              'shared_sources': crystal_batch.shared_sources(crystal), 'monsters': []}
    for row in rows:
        evidence = source_evidence(row, crystal)
        entry = {'name': row['name'], 'kind': row['kind'], 'admission_authorized': False,
                 'source_evidence': evidence, 'wiki_preparation_probe': {key: value for key, value in row.items()
                    if key.startswith('wiki_')}, 'preparation_issues': preparation_issues(row, evidence)}
        entry['wiki_probe_classification'] = 'UNPINNED_PREPARATION_PROBE_NOT_ADOPTED'
        conv.pending_definitions = set()
        try:
            slug, monster, deps, catalog, manifest, _ = conv.convert(row['crystal_other_dir'][:-4])
            local = {item['identity']['key'] for item in deps['items']}
            pending = sorted(conv.pending_definitions)
            for family, key in pending:
                if cb.ref(family, key) not in catalog['definitions'] and key != monster['creature']['identity']['key'] and key not in local:
                    catalog['definitions'].append(cb.ref(family, key))
            entry['pending_definition_references'] = [cb.ref(family, key) for family, key in pending]
            entry['structure_errors'] = vm.validate(monster, deps, catalog, None)
            entry['readiness_errors'] = vm.validate(monster, deps, catalog, manifest)
            entry['open_manifest_rows'] = [item for item in manifest['entries'] if item['status'] in OPEN]
            entry['status'] = 'draft_schema_valid' if not entry['structure_errors'] else 'draft_structure_invalid'
            target = out / slug
            target.mkdir(exist_ok=True)
            for filename, value in zip(BUNDLE_FILES, (monster, deps, catalog, manifest)):
                (target / filename).write_text(dump(value), encoding='utf-8', newline='\n')
            entry['bundle_directory'] = slug
        except Exception as exc:
            entry['status'] = 'conversion_blocked'
            entry['conversion_error'] = type(exc).__name__ + ': ' + str(exc).splitlines()[0][:300]
        report['monsters'].append(entry)
    (out / 'deferred-crystal-preparation.json').write_text(dump(report), encoding='utf-8', newline='\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--crystal', required=True, type=Path)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    report = prepare(args.canary, args.crystal, args.out)
    counts = {status: sum(row['status'] == status for row in report['monsters']) for status in
              ('draft_schema_valid', 'draft_structure_invalid', 'conversion_blocked')}
    print(json.dumps({'candidates': len(report['monsters']), **counts, 'admission_authorized': False}))


if __name__ == '__main__':
    main()
