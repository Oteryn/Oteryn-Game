"""Check the creature facts shown by a monster infobox without inventing absent values.

Usage: python verify_visible_fields.py --repo ROOT [--stage FILE] [--bundles DIR] [--out REPORT]
Bundles are the existing population_census.py output. This check compares authoring
facts to native admitted profiles; it does not establish runtime execution or Global
Tibia parity. Absent optional source data and inapplicable mana costs are distinct.
"""
import argparse
import json
from collections import Counter
from fractions import Fraction
from math import gcd
from pathlib import Path


DIFFICULTIES = ('harmless', 'trivial', 'easy', 'medium', 'hard', 'challenging')
OCCURRENCES = ('common', 'uncommon', 'rare', 'very_rare')
FIELDS = {
    'max_health': ('stats', 'max_health'),
    'experience': ('stats', 'experience'),
    'armor': ('stats', 'armor'),
    'speed': ('stats', 'speed'),
    'mitigation_percent': ('stats', 'mitigation_percent'),
    'summonable': ('summoning', 'summonable'),
    'convinceable': ('summoning', 'convinceable'),
    'mana_cost': ('summoning', 'mana_cost'),
    'charm_points': ('bestiary', 'charm_points'),
    'difficulty': ('bestiary', 'difficulty'),
    'occurrence': ('bestiary', 'occurrence'),
    'stars': ('bestiary', 'stars'),
}


def native_creature(record):
    profile = record['authoring']['profile']
    details = profile.get('details', {})
    result = {
        'identity': record['definition']['identity'],
        'display_name': details.get('display_name'),
        'stats': {target: profile[source] for source, target in
                  (('health', 'max_health'), ('experience', 'experience'),
                   ('armor', 'armor'), ('speed', 'speed'),
                   ('mitigation', 'mitigation_percent')) if source in profile},
        'summoning': details.get('summoning', {}),
    }
    if 'bestiary' in profile or 'bestiary' in details:
        result['bestiary'] = {**profile.get('bestiary', {}), **details.get('bestiary', {})}
    return result


def value_at(creature, path):
    parent = creature.get(path[0], {})
    return parent.get(path[1]) if isinstance(parent, dict) else None


def valid(field, value):
    if field in ('summonable', 'convinceable'):
        return isinstance(value, bool)
    if field == 'difficulty':
        return value in DIFFICULTIES
    if field == 'occurrence':
        return value in OCCURRENCES
    if field == 'mitigation_percent':
        if not isinstance(value, dict) or set(value) != {'numerator', 'denominator'}:
            return False
        n, d = value['numerator'], value['denominator']
        return (type(n) is int and type(d) is int and d > 0 and 0 <= n <= 100 * d
                and gcd(n, d) == 1)
    minimum, maximum = (1, None) if field == 'max_health' else (0, None)
    if field == 'stars':
        maximum = 5
    if field == 'charm_points':
        maximum = 65535
    return type(value) is int and value >= minimum and (maximum is None or value <= maximum)


def equal(field, left, right):
    if field == 'mitigation_percent' and valid(field, left) and valid(field, right):
        return Fraction(**left) == Fraction(**right)
    return type(left) is type(right) and left == right


def inspect_creature(creature, source=None):
    """A supplied source is the converted bundle, not a claim of wiki verification."""
    result = {'identity': creature['identity']['key'], 'name': creature.get('display_name'),
              'fields': {}, 'problems': []}
    for field, path in FIELDS.items():
        value = value_at(creature, path)
        expected = value_at(source, path) if source is not None else None
        entry = {'status': 'PRESENT', 'value': value}
        flags = creature.get('summoning', {})
        impossible = flags.get('summonable') is False and flags.get('convinceable') is False
        if field == 'mana_cost' and impossible:
            entry = {'status': 'NOT_APPLICABLE', 'reason': 'Summon and convince are explicitly disabled.'}
            if value is not None:
                entry = {'status': 'INVALID', 'reason': 'Mana cost is forbidden when both actions are disabled.',
                         'value': value}
        elif value is None:
            if expected is not None:
                entry = {'status': 'DATA_OMISSION', 'expected': expected}
            elif field == 'mana_cost' and (flags.get('summonable') is True or flags.get('convinceable') is True):
                entry = {'status': 'DATA_OMISSION', 'reason': 'Enabled summon or convince requires a mana cost.'}
            elif path[0] == 'bestiary' and source is not None and 'bestiary' not in source:
                entry = {'status': 'NOT_APPLICABLE_IN_SOURCE',
                         'reason': 'The converted source declares no Bestiary profile; Global eligibility is unverified.'}
            elif field == 'mitigation_percent' and source is not None:
                entry = {'status': 'SOURCE_UNSPECIFIED',
                         'reason': 'The converted source provides no mitigation value; omission is not zero.'}
            else:
                entry = {'status': 'UNKNOWN', 'reason': 'No value or explicit source applicability evidence.'}
        elif not valid(field, value):
            entry['status'] = 'INVALID'
        elif source is not None and not equal(field, value, expected):
            entry.update(status='SOURCE_MISMATCH', expected=expected)
        result['fields'][field] = entry
        if entry['status'] in ('DATA_OMISSION', 'INVALID', 'SOURCE_MISMATCH'):
            result['problems'].append(field)
    return result


def load_bundles(directory):
    result = {}
    for path in sorted(directory.glob('*/monster.json')):
        creature = json.loads(path.read_text(encoding='utf-8'))['creature']
        if path.parent.name in result:
            raise ValueError(f'duplicate bundle slug: {path.parent.name}')
        result[path.parent.name] = creature
    if not result:
        raise ValueError(f'no monster bundles under {directory}')
    return result


def profile_records(repo, stage=None):
    if stage is not None:
        data = json.loads(stage.read_text(encoding='utf-8'))
        for profile in data['authoring_profiles']:
            if profile['target']['family'] == 'Creature':
                yield {'definition': {'identity': profile['target']}, 'authoring': profile['data']}, str(stage)
    else:
        for path in sorted((repo / 'content/creatures/definitions').glob('creatures-*.json')):
            for record in json.loads(path.read_text(encoding='utf-8'))['records']:
                yield record, str(path.relative_to(repo))


def schema_fields(path):
    definitions = json.loads(path.read_text(encoding='utf-8'))['$defs']
    return {field: {'path': f'/$defs/{group}/properties/{name}',
                    'status': 'PRESENT' if name in definitions.get(group, {}).get('properties', {}) else 'SCHEMA_OMISSION'}
            for field, (group, name) in FIELDS.items()}


def inventory(repo, bundles=None, stage=None):
    records = []
    seen = set()
    matched_bundles = set()
    for record, profile_file in profile_records(repo, stage):
        creature = native_creature(record)
        key = creature['identity']['key']
        if key in seen:
            raise ValueError(f'duplicate native creature: {key}')
        seen.add(key)
        # The admission mapper uses creature.<bundle slug>, including underscores.
        slug = key.removeprefix('oteryn:creature.')
        source = bundles.get(slug) if bundles is not None else None
        if source is not None:
            matched_bundles.add(slug)
        row = inspect_creature(creature, source)
        row['profile_file'] = profile_file
        row['bundle_available'] = source is not None
        records.append(row)
    if not records:
        raise ValueError(f'no admitted creature profiles under {repo}')
    counts = {field: dict(Counter(row['fields'][field]['status'] for row in records)) for field in FIELDS}
    unadmitted = []
    if bundles is not None:
        for slug in sorted(set(bundles) - matched_bundles):
            row = inspect_creature(bundles[slug], bundles[slug])
            row['bundle_slug'] = slug
            unadmitted.append(row)
    return {
        'scope': 'Infobox facts, structural completeness and authoring-to-native preservation; no runtime qualification.',
        'source_semantics': 'NOT_APPLICABLE_IN_SOURCE does not establish Global Tibia eligibility. '
                            'SOURCE_UNSPECIFIED and UNKNOWN never mean zero. Speed is the accepted engine value.',
        'admitted_count': len(records), 'matched_bundle_count': len(matched_bundles),
        'unadmitted_bundle_count': len(unadmitted), 'field_counts': counts,
        'problem_count': sum(len(row['problems']) for row in records),
        'monsters': records, 'unadmitted_bundles': unadmitted,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('--repo', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--bundles', type=Path)
    parser.add_argument('--stage', type=Path, help='check Creature authoring profiles in an admission-stage candidate')
    parser.add_argument('--out', type=Path)
    args = parser.parse_args()
    result = inventory(args.repo, load_bundles(args.bundles) if args.bundles else None, args.stage)
    result['schema_fields'] = schema_fields(args.repo / 'tools/content-schema/monster-authoring/monster.schema.json')
    result['problem_count'] += sum(row['status'] != 'PRESENT' for row in result['schema_fields'].values())
    if args.out:
        args.out.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({key: value for key, value in result.items()
                      if key not in ('monsters', 'unadmitted_bundles')}, ensure_ascii=False, indent=2))
    return 1 if result['problem_count'] else 0


if __name__ == '__main__':
    raise SystemExit(main())
