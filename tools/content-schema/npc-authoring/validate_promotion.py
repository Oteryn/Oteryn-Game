"""Validate an OTERYN_NPC_PROMOTION_CANDIDATES/v1 report (promotion_candidates.py output) against the
schema doc (OTERYN_NPC_AUTHORING_SCHEMA_V1.md §3 D4-D6, §8).

Semantic rules:
- schema == 'OTERYN_NPC_PROMOTION_CANDIDATES/v1', evidence == 'OTS_HYPOTHESIS_ONLY',
  snapshot_sha256 is 64 hex chars;
- each candidate identity is family NPC, key `oteryn:npc.<slug>` (D4) and revision 'definition-r1';
  the key suffix equals the slug of `name` (same slug() as promotion_candidates.py);
- candidate keys are unique and the candidates list is sorted by key;
- travel_service is null, or identity family Service, key `oteryn:service.travel.<same slug>`,
  revision 'definition-r1', with >=1 route; every route has an in-range destination
  ({x,y,z}, x/y 0..65535, z 0..15), an integer price >= 0, and a destination_keyword unique within
  the candidate;
- every candidate has >=1 placement, with an in-range position, a compass direction
  (NORTH/EAST/SOUTH/WEST) and spawn_interval_s in 1..86400;
- provenance has 1 or 2 of canary/crystal, each with a key namespaced to that source
  (`<source>:npc/...`) and a 64-hex sha256; a single-source candidate must carry a non-null `wiki`
  confirmation (D6: single-source NPCs need wiki confirmation);
- arbitration rows all have rule 'WIKI_ARBITER', `chosen` in canary/crystal, and `chosen` is one of
  the candidate's provenance sources;
- left_out rows have reason in GATED_ROUTE / ROUTE_CONFLICT_WIKI_UNDECIDED / ROUTE_UNCONFIRMED, and
  no left_out travel keyword also appears among the candidate's promoted routes;
- no text anywhere (D5): no 'text', 'description', 'voices' or 'dialogue' key at any depth of a
  candidate;
- totals.candidates == len(candidates) and totals.with_travel == the number of candidates with a
  non-null travel_service;
- held rows carry a non-empty reason and name.

Usage: python validate_promotion.py <report.json>
"""
import argparse
import json
import re
import sys
from pathlib import Path

from promotion_candidates import slug

SCHEMA = 'OTERYN_NPC_PROMOTION_CANDIDATES/v1'
EVIDENCE = 'OTS_HYPOTHESIS_ONLY'
KEY_RE = re.compile(r'^oteryn:npc\.[a-z0-9]+(_[a-z0-9]+)*$')
SHA256_RE = re.compile(r'^[0-9a-f]{64}$')
DIRECTIONS = {'NORTH', 'EAST', 'SOUTH', 'WEST'}
LEFT_OUT_REASONS = {'GATED_ROUTE', 'ROUTE_CONFLICT_WIKI_UNDECIDED', 'ROUTE_UNCONFIRMED'}
TEXT_KEYS = {'text', 'description', 'voices', 'dialogue'}
PROVENANCE_PREFIX = {'canary': 'canary:npc/', 'crystal': 'crystal:npc/'}


def _is_int(value):
    return isinstance(value, int) and not isinstance(value, bool)


def find_text_keys(value, path=''):
    found = []
    if isinstance(value, dict):
        for k, v in value.items():
            here = f'{path}.{k}' if path else k
            if k in TEXT_KEYS:
                found.append(here)
            found += find_text_keys(v, here)
    elif isinstance(value, list):
        for i, v in enumerate(value):
            found += find_text_keys(v, f'{path}[{i}]')
    return found


def in_range_point(point):
    if not isinstance(point, dict) or set(point) != {'x', 'y', 'z'}:
        return False
    x, y, z = point['x'], point['y'], point['z']
    return (_is_int(x) and _is_int(y) and _is_int(z)
            and 0 <= x <= 65535 and 0 <= y <= 65535 and 0 <= z <= 15)


def identity_errors(identity, label, family, key_re, expected_key):
    errs = []
    if not isinstance(identity, dict):
        return [f'{label}: identity missing or malformed']
    if identity.get('family') != family:
        errs.append(f"{label}: identity.family {identity.get('family')!r} != {family!r}")
    key = identity.get('key')
    if not isinstance(key, str) or not key_re.match(key):
        errs.append(f'{label}: identity.key {key!r} does not match the {family} key pattern')
    if expected_key is not None and key != expected_key:
        errs.append(f'{label}: identity.key {key!r} != expected {expected_key!r} (slug of name)')
    if identity.get('revision') != 'definition-r1':
        errs.append(f"{label}: identity.revision {identity.get('revision')!r} != 'definition-r1'")
    return errs


def _travel_key_re(name_slug):
    return re.compile(r'^oteryn:service\.travel\.' + re.escape(name_slug) + r'$')


def candidate_errors(candidate, index):
    errs = []
    label = f'candidates[{index}]'
    name = candidate.get('name')
    name_slug = slug(name) if isinstance(name, str) else None
    expected_key = f'oteryn:npc.{name_slug}' if name_slug is not None else None
    errs += identity_errors(candidate.get('identity'), label, 'NPC', KEY_RE, expected_key)

    placements = candidate.get('placements') or []
    if not placements:
        errs.append(f'{label}: no placements')
    for i, placement in enumerate(placements):
        plabel = f'{label}.placements[{i}]'
        if not in_range_point(placement.get('position')):
            errs.append(f"{plabel}: position {placement.get('position')!r} out of range or malformed")
        if placement.get('direction') not in DIRECTIONS:
            errs.append(f"{plabel}: direction {placement.get('direction')!r} not in {sorted(DIRECTIONS)}")
        interval = placement.get('spawn_interval_s')
        if not _is_int(interval) or not (1 <= interval <= 86400):
            errs.append(f'{plabel}: spawn_interval_s {interval!r} not an int in 1..86400')

    provenance = candidate.get('provenance') or {}
    if not isinstance(provenance, dict) or not (1 <= len(provenance) <= 2) \
            or not set(provenance).issubset(PROVENANCE_PREFIX):
        errs.append(f'{label}: provenance must have 1 or 2 of canary/crystal, got {sorted(provenance)}')
    for source, entry in provenance.items():
        plabel = f'{label}.provenance.{source}'
        entry = entry or {}
        prefix = PROVENANCE_PREFIX.get(source)
        key = entry.get('key')
        if prefix and (not isinstance(key, str) or not key.startswith(prefix)):
            errs.append(f'{plabel}: key {key!r} does not start with {prefix!r}')
        sha = entry.get('sha256')
        if not isinstance(sha, str) or not SHA256_RE.match(sha):
            errs.append(f'{plabel}: sha256 {sha!r} is not 64 hex chars')
    if len(provenance) == 1 and candidate.get('wiki') is None:
        errs.append(f'{label}: single-source candidate has no wiki confirmation (D6)')

    route_keywords = set()
    travel = candidate.get('travel_service')
    if travel is not None:
        tlabel = f'{label}.travel_service'
        travel_expected_key = f'oteryn:service.travel.{name_slug}' if name_slug is not None else None
        travel_key_re = _travel_key_re(name_slug) if name_slug is not None else re.compile(r'^oteryn:service\.travel\.')
        errs += identity_errors(travel.get('identity'), tlabel, 'Service', travel_key_re, travel_expected_key)
        routes = travel.get('routes') or []
        if not routes:
            errs.append(f'{tlabel}: no routes')
        for j, route in enumerate(routes):
            rlabel = f'{tlabel}.routes[{j}]'
            if not in_range_point(route.get('destination')):
                errs.append(f"{rlabel}: destination {route.get('destination')!r} out of range or malformed")
            price = route.get('price')
            if not _is_int(price) or price < 0:
                errs.append(f'{rlabel}: price {price!r} is not a non-negative integer')
            keyword = route.get('destination_keyword')
            if keyword in route_keywords:
                errs.append(f'{rlabel}: duplicate destination_keyword {keyword!r}')
            route_keywords.add(keyword)

    for i, row in enumerate(candidate.get('arbitration') or []):
        alabel = f'{label}.arbitration[{i}]'
        if row.get('rule') != 'WIKI_ARBITER':
            errs.append(f"{alabel}: rule {row.get('rule')!r} != 'WIKI_ARBITER'")
        chosen = row.get('chosen')
        if chosen not in ('canary', 'crystal'):
            errs.append(f'{alabel}: chosen {chosen!r} not in canary/crystal')
        elif chosen not in provenance:
            errs.append(f'{alabel}: chosen {chosen!r} is not one of this candidate\'s provenance sources')

    left_out_keywords = set()
    for i, row in enumerate(candidate.get('left_out') or []):
        llabel = f'{label}.left_out[{i}]'
        if row.get('reason') not in LEFT_OUT_REASONS:
            errs.append(f"{llabel}: reason {row.get('reason')!r} not in {sorted(LEFT_OUT_REASONS)}")
        fact = row.get('fact') or ''
        if fact.startswith('travel.'):
            left_out_keywords.add(fact[len('travel.'):])
    overlap = left_out_keywords & route_keywords
    if overlap:
        errs.append(f'{label}: left_out travel keyword(s) also promoted as route(s): {sorted(overlap)}')

    text_hits = find_text_keys(candidate)
    if text_hits:
        errs.append(f'{label}: forbidden text-bearing key(s) present: {text_hits}')

    return errs


def held_errors(held, index):
    label = f'held[{index}]'
    errs = []
    if not isinstance(held.get('reason'), str) or not held['reason']:
        errs.append(f'{label}: missing or empty reason')
    if not isinstance(held.get('name'), str) or not held['name']:
        errs.append(f'{label}: missing or empty name')
    return errs


def errors(report):
    errs = []
    if report.get('schema') != SCHEMA:
        errs.append(f"schema {report.get('schema')!r} != {SCHEMA!r}")
    if report.get('evidence') != EVIDENCE:
        errs.append(f"evidence {report.get('evidence')!r} != {EVIDENCE!r}")
    snapshot_sha = report.get('snapshot_sha256')
    if not isinstance(snapshot_sha, str) or not SHA256_RE.match(snapshot_sha):
        errs.append(f'snapshot_sha256 {snapshot_sha!r} is not 64 hex chars')

    candidates = report.get('candidates') or []
    keys = []
    for i, candidate in enumerate(candidates):
        errs += candidate_errors(candidate, i)
        keys.append(candidate.get('identity', {}).get('key'))
    if len(keys) != len(set(keys)):
        dupes = sorted({k for k in keys if keys.count(k) > 1})
        errs.append(f'duplicate candidate key(s): {dupes}')
    sort_key = [k if isinstance(k, str) else '' for k in keys]
    if sort_key != sorted(sort_key):
        errs.append('candidates are not sorted by identity.key')

    totals = report.get('totals') or {}
    if totals.get('candidates') != len(candidates):
        errs.append(f"totals.candidates {totals.get('candidates')!r} != {len(candidates)}")
    with_travel = sum(1 for c in candidates if c.get('travel_service'))
    if totals.get('with_travel') != with_travel:
        errs.append(f"totals.with_travel {totals.get('with_travel')!r} != {with_travel}")

    for i, held in enumerate(report.get('held') or []):
        errs += held_errors(held, i)

    return errs


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('report')
    args = parser.parse_args()
    report = json.loads(Path(args.report).read_text(encoding='utf-8'))
    all_errors = errors(report)

    candidates = report.get('candidates') or []
    total = len(candidates)
    by_candidate, other = {}, []
    for error in all_errors:
        match = re.match(r'candidates\[(\d+)\]', error)
        if match:
            by_candidate.setdefault(int(match.group(1)), []).append(error)
        else:
            other.append(error)

    for index in sorted(by_candidate):
        candidate = candidates[index]
        print(f"candidates[{index}] ({candidate.get('name')!r}, key={candidate.get('identity', {}).get('key')!r}):")
        for error in by_candidate[index][:10]:
            print(f'  {error[:300]}')
    if other:
        print('report:')
        for error in other[:20]:
            print(f'  {error[:300]}')

    valid = total - len(by_candidate)
    print(f'{valid}/{total} candidates valid')
    return 1 if all_errors or not total else 0


if __name__ == '__main__':
    sys.exit(main())
