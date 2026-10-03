"""Apply exact pinned requirement literals; retain interval and mission-level scope."""
import copy
import re
from collections import Counter


def level_bounds(raw):
    if raw.isdigit():
        return int(raw), None
    if raw == 'None':
        return 0, None  # explicit absent minimum; never used for blank/uncertain levels
    match = re.fullmatch(r'(\d+)\+', raw)
    if match:
        return int(match[1]), None
    match = re.fullmatch(r'(\d+)\s*-\s*(\d+)', raw)
    if match and int(match[1]) <= int(match[2]):
        return int(match[1]), int(match[2])
    raise ValueError('curated level must be an unqualified lower bound or interval')


def interpret_requirements(quests, facts, curations):
    """Reject stale scope, revision, raw value or an already contradictory minimum."""
    rows = copy.deepcopy(quests)
    by_key = {q['identity']['key']: q for q in rows}
    if len(by_key) != len(rows):
        raise ValueError('duplicate source quest identity')
    pages = facts['fresh_wiki']['pages']
    pins = facts['fresh_wiki']['revision_manifest']
    checks, seen = [], set()
    for entry in curations:
        key, witness = entry['quest'], entry['wiki']
        if key not in by_key or key in seen:
            raise ValueError('missing or duplicate requirement curation quest')
        seen.add(key)
        q = by_key[key]
        matches = [p for p in pages if all(p.get(k) == v for k, v in witness.items())]
        if len(matches) != 1 or not any(all(p.get(k) == witness[k] for k in
                                         ('provider', 'pageid', 'revid', 'content_sha256')) for p in pins):
            raise ValueError('requirement witness does not match accepted wiki snapshot')
        page = matches[0]
        if (witness['provider'] != 'tibia_fandom' or not page['available'] or page['redirect_target']
                or any(q.get('wiki', {}).get(k) != witness[k] for k in ('title', 'pageid', 'revid'))
                or page['source_fields'].get('level_note') != entry.get('scope_note')
                or page['source_fields'].get('level') != entry['level_raw']
                or q.get('requirements_from_wiki', {}).get('lvl') != entry['level_raw']):
            raise ValueError('requirement witness scope or raw level changed')
        if entry['level_raw'] == 'None' and entry.get('scope_note'):
            raise ValueError('explicit absent minimum cannot be mission-scoped')
        minimum, maximum = level_bounds(entry['level_raw'])
        current = q.get('requirements', {}).get('min_level')
        if current is not None and (type(current) is not int or current != minimum):
            raise ValueError('curated minimum contradicts existing value')
        q.setdefault('requirements', {})['min_level'] = minimum
        unparsed = q.setdefault('requirements_unparsed', {})
        if entry.get('scope_note'):
            unparsed['min_level'] = 'note'
        elif maximum is None:
            unparsed.pop('min_level', None)
        else:
            unparsed['min_level'] = 'range'
        checks.append({'quest': key, 'wiki': copy.deepcopy(witness), 'level_raw': entry['level_raw'],
                       'min_level': minimum, 'max_level': maximum, 'classification': 'DERIVED',
                       'coverage_gap': 'Source maximum level is preserved but not lowered into Native Quest requirements'
                       if maximum is not None else ('Source level scope note preserved but mission-level requirement is not lowered into Native Quest requirements'
                                                   if entry.get('scope_note') else None)})
        if entry.get('scope_note'):
            checks[-1].update(scope_note=entry['scope_note'], interpretation='scoped_level_requirement')
        if entry['level_raw'] == 'None':
            checks[-1].update(source_literal='None', interpretation='explicit_no_level_requirement')
    return rows, checks


def requirement_holds(checks, quest_keys):
    """A parsed minimum cannot hide the rest of a source interval from readiness."""
    holds, seen = Counter(), set()
    for check in checks:
        key = check['quest']
        if key not in quest_keys or key in seen:
            raise ValueError('stale or duplicate requirement interpretation')
        seen.add(key)
        minimum, maximum = level_bounds(check['level_raw'])
        note = check.get('scope_note')
        if note is not None and (not isinstance(note, str) or not note.strip()):
            raise ValueError('invalid source level scope note')
        if check.get('interpretation') == 'scoped_level_requirement' and not note:
            raise ValueError('scoped level requirement lost its note')
        if check['level_raw'] == 'None' and (note or (check.get('source_literal') != 'None'
                or check.get('interpretation') != 'explicit_no_level_requirement')):
            raise ValueError('explicit absent minimum lost its source qualification')
        if (type(check['min_level']) is not int or check['min_level'] != minimum
                or check['max_level'] != maximum or (maximum is not None and type(check['max_level']) is not int)
                or bool(check['coverage_gap']) != (maximum is not None or bool(note))):
            raise ValueError('source level interpretation lost its interval hold')
        if maximum is not None or note:
            holds[key] += 1
    return holds
