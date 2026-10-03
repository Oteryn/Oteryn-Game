"""Compose guarded field patches into a new authoring population, never a rollout.

All expectations are checked against immutable baseline bytes before composition.
Provenance retains superseded source mappings; fresh sources must match a captured
revision. Unchanged files are hardlinked and are never subsequently overwritten.
"""
import argparse
import copy
import gzip
import hashlib
import json
import os
from pathlib import Path
from urllib.parse import unquote, urlsplit

import complete_creature_dependencies as population
import classify_monster_population as classification

FILES = population.FILES


def read(path):
    path = Path(path)
    if path.suffix == '.gz':
        with gzip.open(path, 'rt') as stream:
            return json.load(stream)
    return json.loads(path.read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False)


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        raise ValueError('refusing to overwrite existing output: ' + str(path))
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def safe_relative(value):
    path = Path(value)
    if path.is_absolute() or not path.parts or any(p in ('.', '..') for p in path.parts):
        raise ValueError('unsafe relative path: ' + value)
    return path


def tokens(pointer):
    if not pointer.startswith('/'):
        raise ValueError('invalid JSON pointer')
    return [x.replace('~1', '/').replace('~0', '~') for x in pointer.split('/')[1:]]


def lookup(document, pointer):
    value = document
    try:
        for token in tokens(pointer):
            if isinstance(value, list):
                if token == '-':
                    return False, None
                value = value[int(token)]
            else:
                value = value[token]
        return True, value
    except (KeyError, IndexError):
        return False, None


def identity(value):
    if isinstance(value, dict):
        key = value.get('identity', value)
        if 'key' in key and 'revision' in key:
            return key.get('family'), key['key'], key['revision']
    return None


def assign(document, pointer, value):
    parts = tokens(pointer)
    current = document
    for i, part in enumerate(parts[:-1]):
        if isinstance(current, list):
            current = current[int(part)]
        else:
            if part not in current:
                if parts[i + 1] == '-':
                    current[part] = []
                else:
                    current[part] = {}
            current = current[part]
    last = parts[-1]
    if isinstance(current, list):
        if last == '-':
            match = identity(value)
            for position, old in enumerate(current):
                if old == value:
                    return '/' + '/'.join(pointer.split('/')[1:-1] + [str(position)])
                if match is not None and identity(old) == match:
                    raise ValueError('conflicting appended definition: ' + pointer)
            current.append(copy.deepcopy(value))
            return pointer[:-1] + str(len(current) - 1)
        current[int(last)] = copy.deepcopy(value)
    else:
        current[last] = copy.deepcopy(value)
    return pointer


def source_corpus(paths):
    records = {}
    for path in paths:
        for page in read(path)['pages']:
            key = (page['revision_id'], page['content_sha256'])
            existing = records.get(key)
            if existing and existing['page_id'] != page['page_id']:
                raise ValueError('ambiguous captured wiki revision')
            records[key] = page
    return records


def manifest_source(source, corpus, packet_sha):
    if source.get('kind') == 'oteryn_balance_estimate':
        keys = ('kind', 'ledger_sha256', 'qualification', 'evidence_classification', 'global_parity', 'owner_acceptance')
        return {key: source[key] for key in keys}
    if source.get('kind') == 'owner_accepted_non_global_proxy':
        return {'kind': 'oteryn_balance_estimate', 'ledger_sha256': packet_sha,
                'qualification': 'OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE',
                'evidence_classification': 'DERIVED', 'global_parity': False,
                'owner_acceptance': 'Owner authorized maximal practical field completion with flagged non-Global source proxies.'}
    if 'repository' in source:
        revision = source.get('revision', source.get('pin'))
        if not revision or len(revision) != 40:
            raise ValueError('Git source lacks pinned revision')
        return {'repository': source['repository'], 'revision': revision}
    if 'revision_id' in source:
        page = corpus.get((source['revision_id'], source['content_sha256']))
        if page is None:
            raise ValueError('wiki source lacks actual captured page proof')
        host = urlsplit(source['url']).netloc
        if host != urlsplit(page['url']).netloc:
            raise ValueError('wiki source origin differs from captured page')
        title = unquote(urlsplit(source['url']).path.split('/wiki/', 1)[-1]).replace('_', ' ')
        if title.casefold() != page['page_title'].casefold():
            raise ValueError('wiki source title differs from captured page')
        return {'kind': 'mediawiki', 'api': 'https://' + host + '/api.php',
                'title': page['page_title'], 'page_id': page['page_id'],
                'revision_id': page['revision_id'], 'content_sha256': page['content_sha256']}
    raise ValueError('unsupported patch provenance')


def verified_source_details(source, resolved, corpus, packet_name):
    details = copy.deepcopy(source)
    if resolved.get('kind') == 'mediawiki':
        details['source_file'] = resolved['title']
        if not isinstance(details.get('source_line'), int):
            page = corpus[(resolved['revision_id'], resolved['content_sha256'])]
            fields = details.get('source_field', '').split(',')
            candidates = [page.get('field_lines', {}).get(field) for field in fields]
            candidates = [line for line in candidates if isinstance(line, int)]
            if not candidates:
                raise ValueError('wiki mapped field lacks verified source line: ' + details.get('source_field', ''))
            details['source_line'] = min(candidates)
    elif resolved.get('kind') == 'oteryn_balance_estimate' and source.get('kind') == 'owner_accepted_non_global_proxy':
        details['source_file'] = packet_name
        details['source_line'] = 1
    return details


def extend_manifest(manifest, patch, source, destination, lane):
    prefix = {'monster.json': '/monster', 'dependencies.json': '/dependencies',
              'catalog.json': '/catalog'}[patch['file']]
    target = prefix + destination
    # Appends do not replace an existing ancestor array mapping.
    if not patch['pointer'].endswith('/-'):
        for entry in manifest['entries']:
            old = entry.get('destination', '')
            if entry['status'] in ('mapped', 'resolved_native_behavior') and (old == target or old.startswith(target + '/')):
                previous = entry.get('resolution', '')
                entry['status'] = 'metadata_only'
                entry['resolution'] = 'Superseded by guarded field-fill ' + lane + ' at ' + target + '. Original destination retained as history. Previous: ' + previous
    if source not in manifest['sources']:
        manifest['sources'].append(source)
    original = patch['source']
    lines = original.get('lines', original.get('source_lines', []))
    source_line = original.get('source_line') or (lines[0] if lines else 1)
    file = original.get('source_file', original.get('file', original.get('path', source.get('title', 'owner-accepted-field-fill-ledger'))))
    manifest['entries'].append({'source_index': manifest['sources'].index(source),
        'source_file': file, 'source_line': source_line,
        'source_field': original.get('source_field', patch['pointer']),
        'kind': 'dependency' if patch['file'] == 'catalog.json' else 'field',
        'status': 'metadata_only' if patch['file'] == 'catalog.json' else 'mapped', 'destination': target,
        'resolution': 'Guarded field-fill ' + lane + ': ' + patch['reason']})


def link_file(source, target):
    if source.is_symlink():
        raise ValueError('symlink in input: ' + str(source))
    target.parent.mkdir(parents=True, exist_ok=True)
    os.link(source, target)


def compose(baseline, packets, output, wiki_paths=(), validate=True):
    baseline, output = Path(baseline).resolve(), Path(output).resolve()
    packets = [Path(p).resolve() for p in packets]
    if output.exists():
        raise ValueError('output must be new')
    for source in [baseline, population.ROOT, *[p.parent for p in packets]]:
        if output == source or source in output.parents or output in source.parents:
            raise ValueError('output overlaps input')
    index_path = baseline / 'population-index.json'
    baseline_sha = sha(index_path)
    index = read(index_path)
    rows = {r['monster']: copy.deepcopy(r) for r in index['monsters']}
    if len(rows) != len(index['monsters']):
        raise ValueError('duplicate baseline monsters')
    corpus = source_corpus(wiki_paths)
    original, documents, modified, flags, resolved = {}, {}, set(), {}, {}
    grouped, writes, receipts, encounters, restored = {}, {}, [], {}, {}
    for packet_path in packets:
        packet_bytes = packet_path.read_bytes()
        packet = json.loads(packet_bytes)
        if packet['baseline_index_sha256'] != baseline_sha:
            raise ValueError('packet belongs to another baseline')
        lane = packet['lane']
        receipt = {'lane': lane, 'packet_sha256': hashlib.sha256(packet_bytes).hexdigest(), 'packet': str(packet_path), 'patch_count': len(packet['patches'])}
        receipts.append(receipt)
        for name, additions in packet.get('actor_flags', {}).items():
            if name not in rows:
                raise ValueError('flag actor outside population')
            flags.setdefault(name, set()).update(additions)
        if lane == 'soul-core':
            for pending in packet.get('unresolved', []):
                name = pending['monster']
                if name not in rows:
                    raise ValueError('pending soul-core actor outside population')
                flags.setdefault(name, set()).add('SOUL_CORE_ITEM_BINDING_PENDING')
        for name, removals in packet.get('resolved_actor_flags', {}).items():
            inherited = set(rows.get(name, {}).get('completion_flags', []))
            if not set(removals) <= inherited:
                raise ValueError('resolved flag not inherited: ' + name)
            resolved.setdefault(name, set()).update(removals)
        for patch in packet['patches']:
            name, file, pointer = patch['monster'], patch['file'], patch['pointer']
            if name not in rows or safe_relative(name).name != name or file not in FILES[:-1]:
                raise ValueError('invalid patch owner/file')
            if name not in original:
                folder = baseline / 'bundles' / name
                if population.admission.bundle_digest(folder) != rows[name]['sha256']:
                    raise ValueError('baseline bundle SHA differs: ' + name)
                original[name] = {f: read(folder / f) for f in FILES}
                documents[name] = copy.deepcopy(original[name])
                for old_source in original[name]['manifest.json']['sources']:
                    if old_source.get('kind') == 'mediawiki':
                        key = (old_source['revision_id'], old_source['content_sha256'])
                        corpus.setdefault(key, {**old_source, 'page_title': old_source['title'],
                            'url': old_source['api'].split('/api.php')[0] + '/wiki/' + old_source['title'].replace(' ', '_')})
            present, value = lookup(original[name][file], pointer)
            if present != patch['expected_present'] or (present and value != patch['expected_value']):
                raise ValueError('baseline field expectation differs: ' + name + ':' + pointer)
            key = (name, file)
            for old_pointer, old_value in writes.setdefault(key, []):
                if pointer.endswith('/-') and old_pointer == pointer:
                    continue
                if pointer == old_pointer:
                    if patch['value'] != old_value:
                        raise ValueError('conflicting field writers: ' + name + ':' + pointer)
                elif pointer.startswith(old_pointer + '/') or old_pointer.startswith(pointer + '/'):
                    raise ValueError('overlapping field writers: ' + name + ':' + pointer)
            writes[key].append((pointer, patch['value']))
            patch = copy.deepcopy(patch)
            source = manifest_source(patch['source'], corpus, receipt['packet_sha256'])
            patch['source'] = verified_source_details(patch['source'], source, corpus, packet_path.name)
            if source.get('kind') == 'oteryn_balance_estimate' and patch['source'].get('kind') == 'oteryn_balance_estimate':
                ledger = packet_path.parent / safe_relative(patch['source']['source_file'])
                if not ledger.is_file() or sha(ledger) != source['ledger_sha256']:
                    raise ValueError('balance ledger does not match exact pinned source')
            supports = []
            if patch['source'].get('kind') == 'owner_accepted_non_global_proxy':
                wiki_support = {**patch['source'], 'kind': 'mediawiki'}
                wiki_resolved = manifest_source(wiki_support, corpus, receipt['packet_sha256'])
                wiki_support = verified_source_details(wiki_support, wiki_resolved, corpus, packet_path.name)
                supports.append((wiki_support, wiki_resolved))
            for support in patch.get('supporting_sources', []):
                supports.append((support, manifest_source(support, corpus, receipt['packet_sha256'])))
            for field in ('item_presence_source', 'item_identity_source'):
                if field in patch['source']:
                    support = patch['source'][field]
                    supports.append((support, manifest_source(support, corpus, receipt['packet_sha256'])))
            grouped.setdefault(name, []).append((patch, source, lane, supports))
        for row in packet.get('restored_source_rows', []):
            restored.setdefault(row['monster'], []).append(copy.deepcopy(row))
        for row in packet.get('encounters', []):
            relative = safe_relative(row['relative_dir'])
            if len(relative.parts) != 2 or relative.parts[0] != 'encounters' or relative.name != row['slug']:
                raise ValueError('invalid Encounter path')
            source = packet_path.parent / relative
            if row['slug'] in encounters or (baseline / relative).exists():
                raise ValueError('duplicate/new Encounter collides with baseline')
            for filename in ('encounter.json', 'manifest.json', 'catalog.json'):
                if not (source / filename).is_file() or (source / filename).is_symlink():
                    raise ValueError('incomplete Encounter packet')
            if row.get('expected_sha256') is not None:
                raise ValueError('Encounter replacement is not supported by this new-only packet')
            encounters[row['slug']] = source
    for name, operations in grouped.items():
        doc = documents[name]
        for patch, source, lane, supports in operations:
            destination = assign(doc[patch['file']], patch['pointer'], patch['value'])
            extend_manifest(doc['manifest.json'], patch, source, destination, lane)
            for support, supporting_source in supports:
                manifest = doc['manifest.json']
                if supporting_source not in manifest['sources']:
                    manifest['sources'].append(supporting_source)
                line = support.get('source_line', 1)
                if not isinstance(line, int):
                    # Raw source lines remain in the immutable full packet receipt.
                    line = 1
                manifest['entries'].append({'source_index': manifest['sources'].index(supporting_source),
                    'source_file': support.get('source_file', supporting_source.get('title', 'supporting-source')),
                    'source_line': line, 'source_field': support.get('source_field', 'supporting rule'),
                    'kind': 'field', 'status': 'metadata_only',
                    'resolution': 'Supporting source for guarded ' + lane + ' patch at ' + destination + '; primary mapping separately qualified.'})
        for restored_row in restored.get(name, []):
            for entry in doc['manifest.json']['entries']:
                if entry['source_field'] == restored_row['source_field'] and entry['status'] == 'approved_omission':
                    entry['resolution'] = 'Superseded in part by typed partial core ' + str(restored_row.get('encounter', restored_row.get('direct_typed_core'))) + '; remaining limitation: ' + restored_row['limitation'] + '; original omission history: ' + entry.get('resolution', '')
        if validate:
            errors = population.validator.validate(*[doc[f] for f in FILES])
            if errors:
                raise ValueError(name + ': invalid composed bundle: ' + str(errors))
        modified.add(name)
    # Removed qualifications must have actual replacement proof in this packet.
    for name, removals in resolved.items():
        if 'OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE' in removals and not any(p['pointer'] == '/creature/stats/mitigation_percent' and s.get('kind') == 'mediawiki' for p, s, _, _ in grouped.get(name, [])):
            raise ValueError('estimate flag cannot be removed without exact replacement mitigation proof')
    if sha(index_path) != baseline_sha or any(sha(r['packet']) != r['packet_sha256'] for r in receipts):
        raise ValueError('input changed during composition')
    output.mkdir(parents=True)
    for name, row in sorted(rows.items()):
        for filename in FILES:
            target = output / 'bundles' / name / filename
            if name in modified and documents[name][filename] != original[name][filename]:
                write(target, documents[name][filename])
            else:
                link_file(baseline / 'bundles' / name / filename, target)
        row['sha256'] = population.admission.bundle_digest(output / 'bundles' / name)
        row['completion_flags'] = sorted((set(row.get('completion_flags', [])) - resolved.get(name, set())) | flags.get(name, set()))
    for path in sorted((baseline / 'encounters').rglob('*')):
        if path.is_file():
            link_file(path, output / path.relative_to(baseline))
    for name, source in encounters.items():
        for path in sorted(source.rglob('*')):
            if path.is_file():
                link_file(path, output / 'encounters' / name / path.relative_to(source))
    index['monsters'] = [rows[name] for name in sorted(rows)]
    index['completion_generation'] = {'baseline_index_sha256': baseline_sha, 'field_patch_receipts': receipts}
    write(output / 'population-index.json', index)
    quality = {'schema': 'OTERYN_MONSTER_FIELD_FILL_COMPLETION/v1', 'baseline_quality': read(baseline / 'completion-quality.json'),
        'baseline_index_sha256': baseline_sha, 'packet_receipts': receipts, 'prepared_count': len(rows),
        'changed_bundle_count': len(modified), 'new_encounters': sorted(encounters),
        'actor_flags': {n: rows[n]['completion_flags'] for n in sorted(rows)},
        'resolved_actor_flags': {n: sorted(v) for n, v in resolved.items()}, 'restored_source_rows': restored,
        'production_activated': False, 'live_gameplay_verified': False, 'runtime_stage_required': True}
    write(output / 'completion-quality.json', quality)
    annotation_path = baseline / 'classification-source-evidence.json'
    if annotation_path.exists():
        annotation = read(annotation_path)
        annotation['population_index_sha256'] = sha(output / 'population-index.json')
        def refresh_evidence(value):
            if isinstance(value, dict):
                if value.get('kind') in ('bundle_field', 'bundle_field_absent', 'bundle_summon_reference'):
                    path = safe_relative(value['source'])
                    present, observed = lookup(read(output / path), value['pointer'])
                    if present and ('value' in value or value['kind'] == 'bundle_field_absent'):
                        if value.get('value') != observed:
                            value['qualification'] = 'Prepared field refreshed by guarded field-fill; original annotation retained in baseline. ' + value.get('qualification', '')
                        value['value'] = observed
                        if value['kind'] == 'bundle_field_absent':
                            value['kind'] = 'bundle_field'
                for child in value.values():
                    refresh_evidence(child)
            elif isinstance(value, list):
                for child in value:
                    refresh_evidence(child)
        refresh_evidence(annotation['annotations'])
        for name in encounters:
            encounter = read(output / 'encounters' / name / 'encounter.json')
            for position, participant in enumerate(encounter['participants']):
                for creature in participant['creatures']:
                    owner = next((n for n, r in rows.items() if read(output / 'bundles' / n / 'monster.json')['creature']['identity'] == {k: creature[k] for k in ('key', 'revision')}), None)
                    if owner is None:
                        raise ValueError('new Encounter participant is outside population')
                    annotation['annotations'][owner]['encounter_memberships'].append({'encounter': encounter['identity'], 'role': participant['role'],
                        'evidence': [{'kind': 'encounter_participant', 'source': 'encounters/' + name + '/encounter.json', 'pointer': '/participants/' + str(position),
                            'qualification': 'Typed partial source core; placement and live dispatch unverified.'}]})
        write(output / 'classification-source-evidence.json', annotation)
        result = classification.build(output / 'population-index.json', output / 'bundles', output / 'classification-source-evidence.json')
        write(output / 'monster-classification.json', result)
        write(output / 'field-fill-classification-flags.json', {'schema': 'OTERYN_CLASSIFICATION_QUALIFICATION_SIDECAR/v1',
            'population_index_sha256': sha(output / 'population-index.json'),
            'classification_sha256': sha(output / 'monster-classification.json'), 'actor_flags': quality['actor_flags']})
    receipt = {k: quality[k] for k in ('prepared_count', 'changed_bundle_count', 'new_encounters', 'baseline_index_sha256')}
    receipt.update(population_index_sha256=sha(output / 'population-index.json'),
        files_sha256={str(p.relative_to(output)): sha(p) for p in sorted(output.rglob('*.json'))},
        production_activated=False, live_gameplay_verified=False)
    write(output / 'field-fill-receipt.json', receipt)
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', required=True, type=Path)
    parser.add_argument('--packet', required=True, action='append', type=Path)
    parser.add_argument('--wiki-pages', action='append', default=[], type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    result = compose(args.baseline, args.packet, args.output, args.wiki_pages)
    print(json.dumps({k: result[k] for k in ('prepared_count', 'changed_bundle_count', 'new_encounters', 'population_index_sha256')}))


if __name__ == '__main__':
    main()
