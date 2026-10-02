"""Prepare all D18 source patterns as review evidence, never executable admission.

Reads exact pinned Git objects and current population manifests. Historical grouping
supplies names only: none of its model-assisted parameter values are copied.
"""
import argparse
import hashlib
import json
import re
import subprocess
from collections import Counter, defaultdict
from pathlib import Path

import canary_batch as cb
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parent
GROUPING = ROOT / 'samples/p4-behaviour-patterns-canary-47dfd51f.json'
SCHEMA = ROOT / 'custom-pattern-preparation.schema.json'
REGISTERED = re.compile(r'registered (?:instant|rune) spell "([^"]+)" \(([^)]+)\)')
CONTRACTS = {
    'conditional_summon': 'D18 §8.6: summon_creature fixed/fill cap, ownership, caster-relative offset only.',
    'heal_allies_in_area': 'D18 §8.6: affects typed groups/names; fixed-position and load-random cases remain open.',
    'area_damage_named_target': 'D18 §8.6: affects named_creatures; extra targeting actions remain open.',
    'remove_magic_walls': 'D18/SW-2: remove_items and top_item_first_tile selection.',
    'path_trail_missile': 'D18 accepted exact path_trail template; not a real damage chain.',
    'plain_combat_unsupported_schema': 'D19: mitigated_by/remove_condition; undefined constants follow engine semantics.',
    'chain_bounce': 'D12/S23: typed chain filter/shape/count/range; source callback must fit the accepted filter.',
    'escalating_dot_curse': 'D21: geometric damage schedule; source computation must match exactly.',
    'fear': 'Accepted SW-1/Q3a: windup with caster target at completion.',
}
ENCOUNTER_PATTERNS = {'boss_form_swap', 'boss_escape_utility', 'map_or_quest_specific'}


def pinned(canary, path):
    content = subprocess.run(['git', '-C', str(canary), 'show', cb.REVISION + ':' + path],
                             check=True, capture_output=True).stdout
    return content, cb.blob_id(content)


def source_facts(text):
    """Literal lexemes are facts; symbolic/arithmetic expressions stay unevaluated."""
    facts = []
    for number, line in enumerate(text.splitlines(), 1):
        code = line.strip()
        match = re.match(r'(?:local\s+)?([\w.]+)\s*=\s*(.+)', code)
        if match and not code.startswith('--'):
            expression = match[2].split(' --', 1)[0].rstrip(',;')
            literal = bool(re.fullmatch(r'-?\d+(?:\.\d+)?|true|false|"(?:[^"\\]|\\.)*"', expression))
            facts.append({'source_line': number, 'kind': 'assignment', 'name': match[1],
                          'source_expression': expression, 'value_state': 'SOURCE_LITERAL' if literal else 'UNKNOWN_NOT_EVALUATED'})
        if re.search(r':(?:setParameter|setCallback|setArea|setFormula|addDamage|setChainValueCallback|id)\(', code):
            facts.append({'source_line': number, 'kind': 'parameter_call', 'source_expression': code,
                          'value_state': 'UNKNOWN_NOT_EVALUATED'})
        if re.search(r'\b(?:Condition|createCombatArea|Game\.createMonster|Game\.createItem|addEvent|math\.random)\(', code):
            facts.append({'source_line': number, 'kind': 'constructor_or_world_call', 'source_expression': code,
                          'value_state': 'UNKNOWN_NOT_EVALUATED'})
    return facts


def dispositions(directories):
    by_spell = defaultdict(list)
    hashes = {}
    for directory in directories:
        for path in sorted(directory.glob('*/manifest.json')):
            manifest = json.loads(path.read_text())
            if manifest['sources'][0].get('repository') != cb.REPOSITORY:
                continue
            if manifest['sources'][0].get('revision') != cb.REVISION:
                raise ValueError('Canary manifest revision does not match the pinned source: ' + str(path))
            hashes[str(path)] = hashlib.sha256(path.read_bytes()).hexdigest()
            for row in manifest['entries']:
                match = REGISTERED.search(row.get('resolution', ''))
                if match:
                    if row.get('source_index') != 0:
                        raise ValueError('registered spell row is not bound to the pinned Canary source: ' + str(path))
                    by_spell[match[1].lower()].append({'monster': path.parent.name, 'status': row['status'],
                        'source_field': row['source_field'], 'source_file': row['source_file'],
                        'source_line': row['source_line'], 'script': match[2], 'resolution': row['resolution']})
    return by_spell, hashes


def prepare(canary, directories):
    grouping = json.loads(GROUPING.read_text())
    if grouping['source']['revision'] != cb.REVISION or len(grouping['patterns']) != 19 or len(grouping['spells']) != 93:
        raise ValueError('D18 pinned grouping changed')
    refs, hashes = dispositions(directories)
    records = {r['spell'].lower(): r for r in grouping['spells']}
    for name, rows in refs.items():
        paths = {r['script'] for r in rows}
        if len(paths) != 1:
            raise ValueError('ambiguous script binding for ' + name)
        if name in records and paths != {records[name]['script']}:
            raise ValueError('historical/current script binding mismatch for ' + name)
        if name not in records and any(r['status'] in ('unresolved_semantics', 'unsupported_source_field') for r in rows):
            records[name] = {'spell': name, 'script': next(iter(paths)), 'pattern': 'UNCLASSIFIED_CURRENT_SOURCE'}
    scripts = []
    for name, row in sorted(records.items()):
        content, blob = pinned(canary, row['script'])
        text = content.decode('utf-8')
        if not re.search(r':name\(\s*["\']' + re.escape(row['spell']) + r'["\']\s*\)', text, re.I):
            raise ValueError('pinned registration mismatch: ' + name)
        current = refs.get(name, [])
        pattern = row['pattern']
        known = CONTRACTS.get(pattern)
        boundary = 'EXISTING_TYPED_CONTRACT_REQUIRES_EXACT_TRANSLATION' if known else 'PARAMETER_CONTRACT_OR_RUNTIME_OWNER_REQUIRED'
        if pattern in ENCOUNTER_PATTERNS:
            boundary = 'ENCOUNTER_OWNED_D9_D18; accepted fight-specific rules required'
        open_count = sum(r['status'] in ('unresolved_semantics', 'unsupported_source_field') for r in current)
        scripts.append({'spell': row['spell'], 'pattern': pattern,
                        'source': {'repository': cb.REPOSITORY, 'revision': cb.REVISION, 'file': row['script'],
                                   'blob_sha1': blob, 'sha256': hashlib.sha256(content).hexdigest()},
                        'source_facts': source_facts(text),
                        'lexical_api_methods': sorted(set(re.findall(r':([A-Za-z_]\w*)\(', text))),
                        'current_monster_dispositions': current,
                        'current_open_references': open_count,
                        'source_translation_status': 'HAS_OPEN_AUTHORING_REFERENCES' if open_count else
                            ('HAS_RESOLVED_AUTHORING_REFERENCES' if current else 'NO_CURRENT_REFERENCE_OBSERVED'),
                        'accepted_contract': known, 'remaining_boundary': boundary,
                        'parameter_values_from_model_assisted_grouping_adopted': False,
                        'admission_authorized': False, 'runtime_qualified': False})
    counts = Counter(row['pattern'] for row in scripts)
    patterns = [{'id': p['id'], 'source_spell_count': counts[p['id']], 'accepted_contract': CONTRACTS.get(p['id']),
                 'current_open_reference_count': sum(r['current_open_references'] for r in scripts if r['pattern'] == p['id']),
                 'admission_authorized': False} for p in grouping['patterns']]
    report = {'schema': 'OTERYN_CUSTOM_PATTERN_PREPARATION/v1', 'source_revision': cb.REVISION,
            'scope': 'Source-backed review evidence, not executable Ability/native-behavior data.',
            'grouping_sha256': hashlib.sha256(GROUPING.read_bytes()).hexdigest(),
            'counts': {'accepted_group_names': 19, 'historical_source_spells': 93, 'source_scripts_prepared': len(scripts),
                       'current_open_registered_spells': sum(bool(r['current_open_references']) for r in scripts)},
            'input_manifest_sha256': hashes, 'patterns': patterns, 'spells': scripts,
            'admission_authorized': False, 'runtime_qualified': False}
    schema = json.loads(SCHEMA.read_text())
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--manifests', type=Path, nargs='+', required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    out = args.out.resolve()
    if ROOT.parents[2] in (out, *out.parents):
        raise ValueError('output must be external; admission is not authorized')
    report = prepare(args.canary, args.manifests)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(report['counts']))


if __name__ == '__main__':
    main()
