"""Fill absent base mitigation from six pinned, identity-matched Crystal field facts.

This is a D47 OTS hypothesis; it never proves Global or whole combat-profile parity.
"""
import hashlib
import json
import re
import subprocess
from decimal import Decimal, InvalidOperation
from fractions import Fraction
from pathlib import Path

CANARY = ('opentibiabr/canary', '47dfd51f45280a59a1d3e50ba7edd573d7234446')
CRYSTAL = ('zimbadev/crystalserver', '00ce02a57ca5a12e48f32a3476e37471167e4c3f')
QUALIFICATION = 'OTS_HYPOTHESIS_NEEDS_GLOBAL_VERIFICATION'
SAMPLE = Path(__file__).parent / 'samples/secondary-mitigation-crystal-00ce02a5.json'
NUMBER = r'(?:\d+(?:\.\d*)?|\.\d+)'


def fingerprint(name, raw):
    return {'registration_name': name, 'display_name': raw.get('name', name),
            'variant': raw.get('variant'), 'outfit': raw.get('outfit'),
            **{key: raw.get(key) for key in ('health', 'maxHealth', 'experience', 'race', 'raceId', 'corpse')}}


def checked_source(root, source, expected):
    path = Path(source['path'])
    if path.is_absolute() or '..' in path.parts or path.suffix != '.lua':
        raise ValueError('unsafe source path')
    if (source['repository'], source['revision']) != expected:
        raise ValueError('source epoch mismatch')
    content = (root / path).read_bytes()
    pinned = subprocess.run(['git', '-C', str(root), 'show', expected[1] + ':' + str(path)],
                            check=True, capture_output=True).stdout
    blob = hashlib.sha1(b'blob ' + str(len(content)).encode() + b'\0' + content).hexdigest()
    if content != pinned or hashlib.sha256(content).hexdigest() != source['sha256'] or blob != source['git_blob']:
        raise ValueError('source bytes mismatch')
    return content.decode('utf-8')


def literal_witness(text, name):
    """Recognize only the reviewed straight-line field assignment, never execute Lua."""
    # Replace comments/strings by whitespace without losing physical line numbers.
    tokens = re.compile(r'--\[(=*)\[.*?\]\1\]|--[^\n]*|\[(=*)\[.*?\]\2\]|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'', re.S)
    code = tokens.sub(lambda match: re.sub(r'[^\n]', ' ', match[0]), text)
    creates = list(re.finditer(r'local\s+mType\s*=\s*Game\.createMonsterType\(\s*"([^"\\]*)"\s*\)', text))
    locals_ = list(re.finditer(r'\blocal\s+monster\s*=\s*\{\s*\}', code))
    registers = list(re.finditer(r'\bmType\s*:\s*register\s*\(\s*monster\s*\)', code))
    defenses = list(re.finditer(r'\bmonster\s*\.\s*defenses\s*=\s*\{', code))
    if len(creates) != 1 or creates[0][1] != name or len(locals_) != 1 or len(registers) != 1 or len(defenses) != 1:
        raise ValueError('source registration/table is not unique')
    start = defenses[0].start();body = defenses[0].end()
    if not creates[0].start() < locals_[0].start() < start < registers[0].start():
        raise ValueError('field order mismatch')
    if re.search(r'\b(?:if|for|while|repeat|function|do|end)\b', code[:body]):
        raise ValueError('field initialization is conditional')
    if len(re.findall(r'\bmitigation\b', code)) != 1 or len(re.findall(r'\bmonster\s*\.\s*defenses\b', code)) != 1:
        raise ValueError('mitigation/defenses is reassigned or aliased')
    depth = 1;end = body
    while end < len(code) and depth:
        depth += (code[end] == '{') - (code[end] == '}');end += 1
    match = re.search(r'\bmitigation\s*=\s*(' + NUMBER + r')\s*[,}]', code[body:end])
    if not match or depth or end > registers[0].start():
        raise ValueError('mitigation is not a plain numeric literal')
    before = code[body:body + match.start()]
    if before.count('{') != before.count('}'):
        raise ValueError('mitigation is not a defenses field')
    value = Decimal(match[1])
    if not value.is_finite() or not 0 <= value <= 100:
        raise ValueError('mitigation outside percent range')
    line = code[:body + match.start()].count('\n') + 1
    if re.search(r'todo|fixme|guess|uncertain|unverified', text.splitlines()[line - 1], re.I):
        raise ValueError('literal source flags uncertainty')
    return {'literal': match[1], 'line': line, 'table_line': code[:locals_[0].start()].count('\n') + 1,
            'registration_line': code[:registers[0].start()].count('\n') + 1}


def adopt_secondary_mitigation(relative, registration_name, raw_monster, primary_path,
                               creature, rows, sources, crystal_root=None, primary_source=None):
    stats = creature.get('stats', {})
    if not isinstance(stats, dict) or 'stats' not in creature or 'mitigation_percent' in stats or 'mitigation' in raw_monster.get('defenses', {}) or not crystal_root:
        return False
    if not primary_source or (primary_source.get('repository'), primary_source.get('revision')) != CANARY:
        return False
    try:
        packet = json.loads(SAMPLE.read_text())
        if packet['qualification'] != QUALIFICATION:
            return False
        selected = [row for row in packet['candidates'] if row['relative'] == relative]
        if len(selected) != 1:
            return False
        row = selected[0];primary = row['primary_source'];secondary = row['secondary_source']
        if row['qualification'] != QUALIFICATION or type(row['unique_same_registration_secondary_count']) is not int or row['unique_same_registration_secondary_count'] != 1:
            return False
        identities = [fingerprint(registration_name, raw_monster), primary['fingerprint'], secondary['fingerprint']]
        if len({json.dumps(value, sort_keys=True) for value in identities}) != 1:
            return False
        path = Path(primary_path).resolve();suffix = Path(primary['path'])
        root = path.parents[len(suffix.parts) - 1]
        if root / suffix != path:
            return False
        checked_source(root, primary, CANARY)
        text = checked_source(Path(crystal_root), secondary, CRYSTAL)
        # Recheck source identity, rather than trusting the packet's identity claim.
        # Import only at call time: canary_batch itself imports this field adapter.
        import canary_batch
        primary_errors = [];secondary_errors = []
        actual_name, actual_primary, _ = canary_batch.load_monster(path, primary_errors)
        secondary_name, actual_secondary, _ = canary_batch.load_monster(Path(crystal_root) / secondary['path'], secondary_errors)
        if 'mitigation' in actual_primary.get('defenses', {}):
            return False
        actual_identities = [fingerprint(actual_name, actual_primary), fingerprint(secondary_name, actual_secondary)]
        if any(json.dumps(value, sort_keys=True) != json.dumps(identities[0], sort_keys=True) for value in actual_identities):
            return False
        if primary_errors != primary['bounded_eval_errors'] or secondary_errors != secondary['bounded_eval_errors']:
            return False
        witness = literal_witness(text, registration_name)
        numeric = secondary['mitigation']
        if actual_secondary.get('defenses', {}).get('mitigation') != numeric:
            return False
        if type(numeric) not in (int, float) or not Decimal(str(numeric)).is_finite():
            return False
        if witness != row['literal_witness'] or Decimal(witness['literal']) != Decimal(str(numeric)):
            return False
        ratio = Fraction(Decimal(witness['literal']))
        source = {'repository': CRYSTAL[0], 'revision': CRYSTAL[1]}
        index = sources.index(source) if source in sources else len(sources)
        resolution = (f'{QUALIFICATION}: exact Crystal base-percent literal {witness["literal"]}; '
            f'pin {CRYSTAL[1]}, SHA256 {secondary["sha256"]}, blob {secondary["git_blob"]}. '
            'Identity fingerprint only; full combat balance differs: ' + json.dumps(row['balance_differences'], sort_keys=True) + '. '
            'Only defenses.mitigation is adopted under owner fill authorization/D47; no effective getter or x1.5 scaling. '
            'Global value UNKNOWN. Partial field evidence only, not complete script/registration/runtime parity; source evaluation errors: '
            + json.dumps(secondary['bounded_eval_errors']) + '.')
        if source not in sources:
            sources.append(source)
        stats['mitigation_percent'] = {'numerator': ratio.numerator, 'denominator': ratio.denominator}
        rows.append({'source_index': index, 'source_file': secondary['path'], 'source_line': witness['line'],
                     'source_field': 'defenses.mitigation', 'kind': 'field', 'status': 'mapped',
                     'destination': '/monster/creature/stats/mitigation_percent', 'resolution': resolution})
        return True
    except (OSError, KeyError, TypeError, ValueError, IndexError, InvalidOperation, subprocess.CalledProcessError):
        return False
