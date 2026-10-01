"""Retain exact declared SOURCE readers; no initial value, bounds or native policy."""
from collections import Counter
import copy
import hashlib
from pathlib import Path
import re

import lua_writers
import scoped_storage_aliases

READ = re.compile(r'\b([A-Za-z_]\w*):getStorageValue\s*\(\s*(Storage(?:\.[A-Za-z_]\w*)+|[0-9]+)\s*\)')


def storage_readers(repos, sources, key_for, requested, declarations=None):
    """Evidence is from executable named getters, never numeric-ID coincidence."""
    readers = {}
    for server, repo in sorted(repos.items()):
        pack = sources[server]['datapack']
        for path in sorted((Path(repo) / pack / 'scripts/quests').rglob('*.lua')):
            raw = path.read_bytes(); text = raw.decode('utf-8', errors='strict')
            # Proven integer RHS table fields cannot mutate/escape the Storage root.
            # The shared guard otherwise mistakes their following field for an LHS.
            known = {d['expression'] for k, d in (declarations or {}).items()
                     if k.startswith(server + ':') and d.get('state') != 'CONFLICT'
                     and type(d.get('storage_id')) is int}
            guarded = lua_writers.mask_code(text)
            rhs = re.compile(r'(?<![.\w=])([A-Za-z_]\w*\s*=\s*)(Storage(?:\.[A-Za-z_]\w*)+)(?=\s*[,}])')
            guarded = rhs.sub(lambda m: m[1] + ('0' if m[2] in known else m[2]), guarded)
            pristine = lua_writers.builtin_binding_is_pristine(guarded.splitlines(), 'Storage')
            expanded = scoped_storage_aliases.expand_scoped_storage_aliases(text)
            aliases = lua_writers.storage_aliases(expanded.split('\n'))
            expanded = '\n'.join(lua_writers.expand_aliases(line, aliases) for line in expanded.split('\n'))
            code = lua_writers.mask_code(expanded)
            for match in READ.finditer(code):
                expression = match[2]
                if expression.isdigit():
                    if len(expression) > 10 or not 0 < int(expression) <= 2147483647:
                        continue
                    target = int(expression)
                elif pristine:
                    target = expression
                else:
                    continue
                key = server + ':quest-progress/' + key_for(target)
                if key not in requested:
                    continue
                line = code.count('\n', 0, match.start()) + 1
                proof = {'source': server, 'repository': sources[server]['repository'],
                    'revision': sources[server]['revision'], 'path': str(path.relative_to(repo)),
                    'line': line, 'blob_sha256': hashlib.sha256(raw).hexdigest(),
                    'line_sha256': hashlib.sha256(raw.splitlines()[line - 1]).hexdigest(),
                    'expression': expression, 'receiver_expression': match[1]}
                readers.setdefault(key, []).append(proof)
    return readers


def retain_reader_tracks(progress, readers, declarations, writer_index, key_for, transitions_for, transition_of):
    """Writer evidence is retained whole; ambiguous symbolic paths fail closed."""
    existing = {p['key']: p for p in progress}
    writers = {}
    for found in writer_index.values():
        for server, path in found['paths']:
            writers.setdefault(path, []).append(found)
    added, checks = [], []
    for key, proofs in sorted(readers.items()):
        path = key.split(':quest-progress/', 1)[1]
        if key in existing:
            continue
        if re.fullmatch(r'storage/[1-9][0-9]*', path) and all(p['expression'] == path.split('/')[1] for p in proofs):
            record = {'key': key, 'missions': [], 'start_of': [], 'read_by_gates': [],
                'auxiliary_of': [], 'owner_basis': 'UNKNOWN',
                'source_checks': {'owner': 'UNKNOWN: caller-only numeric SOURCE identity; owning quest, initial value, bounds, numeric writer inventory and native Character policy are not inferred',
                    'storage_declaration': 'UNKNOWN: literal getter has no proven named Storage declaration; never joined by numeric coincidence'},
                'writes': {'canary': 0, 'crystalserver': 0}, 'transitions': []}
            added.append(record); existing[key] = record
            checks.append({'key': key, 'reason': 'SOURCE literal caller retained; declaration, ownership and complete numeric writer inventory remain UNKNOWN', 'readers': proofs})
            continue
        declaration = declarations.get(key)
        if not declaration or declaration.get('state') == 'CONFLICT':
            checks.append({'key': key, 'reason': 'UNKNOWN: no unambiguous exact Storage declaration', 'readers': proofs})
            continue
        if any(p['expression'] != declaration['expression'] for p in proofs):
            checks.append({'key': key, 'reason': 'UNKNOWN: reader/declaration symbolic expression differs', 'readers': proofs})
            continue
        originals = [p for p in existing.values() if not p.get('alias_of')
                     and p['key'].split(':quest-progress/', 1)[1] == path]
        if originals:
            if len(originals) != 1 or declarations.get(originals[0]['key'], {}).get('expression') != declaration['expression']:
                checks.append({'key': key, 'reason': 'CONFLICT: no unique declared exact-symbolic alias target', 'readers': proofs})
                continue
            original = originals[0]
            owners = set(original.get('auxiliary_of', []) + original.get('start_of', []))
            owners.update(m.split('#', 1)[0] for m in original.get('missions', []))
            record = {'key': key, 'missions': [], 'start_of': [], 'read_by_gates': [],
                'alias_of': original['key'], 'auxiliary_of': sorted(owners),
                'owner_basis': 'exact source-path alias', 'source_storage': declaration,
                'note': 'SOURCE transcription alias for exact declared symbolic path; no numeric-ID or native Character equivalence is inferred',
                'writes': copy.deepcopy(original['writes']), 'transitions': copy.deepcopy(original['transitions'])}
            if not owners:
                record['source_checks'] = {'owner': original.get('source_checks', {}).get('owner',
                    'UNKNOWN: exact SOURCE alias target has no independently established quest owner')}
            added.append(record); existing[key] = record
            checks.append({'key': key, 'reason': 'SOURCE exact-symbolic alias retained; no native identity inferred', 'readers': proofs})
            continue
        matches = {id(found): found for found in writers.get(path, [])}
        if len(matches) > 1:
            checks.append({'key': key, 'reason': 'CONFLICT: multiple writer identities for exact reader path', 'readers': proofs})
            continue
        found = next(iter(matches.values()), {'count': Counter(), 'transitions': {}, 'paths': set()})
        occurrences = [o for t in found['transitions'].values() for o in t['source_occurrences']]
        if any(key_for(o['target']) != path or o['target'] != declaration['expression'] for o in occurrences):
            checks.append({'key': key, 'reason': 'CONFLICT: normalized writer path combines distinct symbolic expressions', 'readers': proofs})
            continue
        represented = Counter(o['source'] for o in occurrences)
        if any(represented[s] != n for s, n in found['count'].items()):
            raise ValueError('Reader track writer evidence does not cover observed writes: ' + key)
        record = {'key': key, 'missions': [], 'start_of': [], 'read_by_gates': [],
            'auxiliary_of': [], 'owner_basis': 'UNKNOWN', 'source_storage': declaration,
            'source_checks': {'owner': 'UNKNOWN: exact source reader/declaration retained; owning quest, initial value, bounds and native Character policy are not inferred'},
            'writes': {s: represented[s] for s in ('canary', 'crystalserver')},
            'transitions': [transition_of(t) for t in transitions_for(found)]}
        added.append(record); existing[key] = record
        checks.append({'key': key, 'reason': 'SOURCE reader/declaration retained; ownership remains UNKNOWN', 'readers': proofs})
    return added, checks
