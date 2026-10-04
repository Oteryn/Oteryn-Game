"""Structural and semantic validation of the Spell authoring schema candidate v1, never runtime activation."""
import argparse
import json
import math
import re
from decimal import Decimal
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

ROOT = Path(__file__).resolve().parent
# S9: the declared closed catalogue of cooldown groups and the roles each may take.
COOLDOWN_GROUPS = {g['key']: tuple(g['roles']) for g in
                   json.loads((ROOT / 'cooldown-groups.json').read_text(encoding='utf-8'))['groups']}
MONSTER = ROOT.parent / 'monster-authoring'
SCHEMA_FILES = {
    'spell.schema.json': ROOT / 'spell.schema.json',
    'spell-dependencies.schema.json': ROOT / 'spell-dependencies.schema.json',
    'monster.schema.json': MONSTER / 'monster.schema.json',
    'monster-import-readiness.schema.json': MONSTER / 'monster-import-readiness.schema.json',
}
SCHEMAS = {name: json.loads(path.read_text(encoding='utf-8')) for name, path in SCHEMA_FILES.items()}
REGISTRY = Registry().with_resources((s['$id'], Resource.from_contents(s)) for s in SCHEMAS.values())
CATALOG_FAMILIES = {'Item', 'Interaction', 'Creature', 'Ability', 'Effect', 'Formula', 'Spell'}
INPUT_SETS = {
    'level_magic': {'level', 'magic_level', 'base_power', 'shielding_skill'},
    'skill': {'level', 'attack_skill', 'attack_value', 'attack_factor', 'base_power', 'shielding_skill',
              'shield_defense'},
}
# Evaluation grid for the semantic range check: every combination must give 0 <= minimum <= maximum.
GRID = {
    'level': (1, 8, 20, 50, 100, 300, 500, 1000, 1500, 2500),
    'magic_level': (0, 1, 10, 30, 60, 100, 130),
    'attack_skill': (10, 30, 60, 100, 130),
    'attack_value': (7, 20, 50, 100),
    'attack_factor': (Decimal('1'), Decimal('1.2')),
    'shielding_skill': (10, 60, 110),
    'shield_defense': (1, 20, 40),
}
MAX_EXPRESSION_NODES = 256
MAX_EXPRESSION_DEPTH = 32


def read(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))


def pointer(path):
    return '/' + '/'.join(str(k).replace('~', '~0').replace('/', '~1') for k in path)


def walk(value, path=()):
    yield path, value
    if isinstance(value, dict):
        for k, v in value.items():
            yield from walk(v, path + (k,))
    elif isinstance(value, list):
        for i, v in enumerate(value):
            yield from walk(v, path + (i,))


def structural(name, data):
    validator = Draft202012Validator(SCHEMAS[name], registry=REGISTRY)
    return [f'{name}{pointer(e.absolute_path)}: {e.message}'
            for e in sorted(validator.iter_errors(data), key=lambda e: str(list(e.absolute_path)))]


def ident(family, value):
    return family, value['key'], value['revision']


def level_base_damage_healing(level):
    """S5: TibiaWiki `Formulae` damage/healing level curve (identical to Crystal calculateBaseDamageHealing)."""
    step = math.floor((math.sqrt(2 * level + 2025) + 5) / 10)
    return math.floor((level + 1000) / step) + 50 * step - 450


def evaluate(expr, env):
    if 'const' in expr:
        return float(expr['const'])
    if 'var' in expr:
        return float(env[expr['var']])
    if 'fn' in expr:
        return float(level_base_damage_healing(evaluate(expr['args'][0], env)))
    args = [evaluate(a, env) for a in expr['args']]
    op = expr['op']
    if op == 'add':
        return args[0] + args[1]
    if op == 'sub':
        return args[0] - args[1]
    if op == 'mul':
        return args[0] * args[1]
    if op == 'div':
        if args[1] == 0:
            raise ZeroDivisionError('division by zero')
        return args[0] / args[1]
    if op == 'neg':
        return -args[0]
    if op == 'floor':
        return float(math.floor(args[0]))
    if op == 'ceil':
        return float(math.ceil(args[0]))
    if op == 'sqrt':
        if args[0] < 0:
            raise ValueError('square root of a negative value')
        return math.sqrt(args[0])
    if op == 'abs':
        return abs(args[0])
    if op == 'min':
        return min(args)
    if op == 'max':
        return max(args)
    raise ValueError('unknown operation ' + op)


def expression_shape(expr, depth=1):
    """(nodes, max depth, variables) of an expression tree."""
    nodes, deepest, names = 1, depth, set()
    if 'var' in expr:
        names.add(expr['var'])
    for arg in expr.get('args', []):
        n, d, v = expression_shape(arg, depth + 1)
        nodes += n
        deepest = max(deepest, d)
        names |= v
    return nodes, deepest, names


def grid(inputs, used, base_power):
    names = sorted((used - {'base_power'}) | ({'level'} if inputs == 'skill' else set()))
    envs = [{}]
    for name in names:
        envs = [{**env, name: value} for env in envs for value in GRID[name]]
    for env in envs:
        if base_power is not None:
            env['base_power'] = base_power
        yield env


def check_formula(formula, base_power, label, errors, needs_shield=False):
    used = set()
    for bound in ('minimum', 'maximum'):
        nodes, depth, names = expression_shape(formula[bound])
        used |= names
        if nodes > MAX_EXPRESSION_NODES or depth > MAX_EXPRESSION_DEPTH:
            errors.append(f'{label}/{bound}: expression exceeds {MAX_EXPRESSION_NODES} nodes or depth {MAX_EXPRESSION_DEPTH}')
            return
    extra = used - INPUT_SETS[formula['inputs']]
    if extra:
        errors.append(f'{label}: inputs {formula["inputs"]} do not provide ' + ', '.join(sorted(extra)))
        return
    if 'base_power' in used and base_power is None:
        errors.append(f'{label}: uses base_power but the casting Spell has no base_power')
        return
    if 'shield_defense' in used and not needs_shield:
        errors.append(f'{label}: uses shield_defense but the casting Spell does not need a shield (S27 D.4)')
        return
    for env in grid(formula['inputs'], used, base_power):
        try:
            low = math.trunc(evaluate(formula['minimum'], env))
            high = math.trunc(evaluate(formula['maximum'], env))
        except (ZeroDivisionError, ValueError, OverflowError) as exc:
            errors.append(f'{label}: {exc} at {env}')
            return
        if low < 0 or high < 0:
            errors.append(f'{label}: negative magnitude at {env} (a Formula stores magnitudes; the Effect gives the sign)')
            return
        if low > high:
            errors.append(f'{label}: minimum {low} exceeds maximum {high} at {env}')
            return


def check_matrix(rows, label, errors, hit_centre=False):
    if len({len(row) for row in rows}) != 1:
        errors.append(label + ': rows must have equal length')
    if sum(row.count('c') + row.count('C') for row in rows) != 1:
        errors.append(label + ': exactly one centre cell required')
    if hit_centre and any('c' in row for row in rows):
        errors.append(label + ': party_buff centre must be hit (C, not c)')


def validate(bundle, deps, catalog=None, manifest=None):
    errors = structural('spell.schema.json', bundle) + structural('spell-dependencies.schema.json', deps)
    if manifest is not None:
        errors += structural('monster-import-readiness.schema.json', manifest)
    if errors:
        return errors
    spell = bundle['spell']
    records = {ident('Spell', spell['identity']): spell}
    for section, family in (('abilities', 'Ability'), ('effects', 'Effect'), ('formulas', 'Formula')):
        for value in deps[section]:
            key = ident(family, value['identity'])
            if key in records:
                errors.append('duplicate local definition ' + repr(key))
            records[key] = value
    for reference in (catalog or {}).get('definitions', []):
        if not isinstance(reference, dict) or set(reference) != {'family', 'key', 'revision'}:
            errors.append('catalog: malformed definition reference')
            continue
        if reference['family'] not in CATALOG_FAMILIES:
            errors.append('catalog: unknown family ' + str(reference['family']))
            continue
        key = ident(reference['family'], reference)
        if key in records:
            errors.append('duplicate catalog/local definition ' + repr(key))
        records[key] = None
    for label, data in (('spell', bundle), ('dependencies', deps)):
        for path, value in walk(data):
            if isinstance(value, dict) and set(value) == {'family', 'key', 'revision'}:
                if ident(value['family'], value) not in records:
                    errors.append(label + pointer(path) + ': unresolved exact definition ' + repr(ident(value['family'], value)))
    words = spell.get('words')
    if words is not None and words != re.sub(r'\s+', ' ', words.strip().lower()):
        errors.append('spell/words: must be lowercase with single spaces and no outer whitespace')
    if spell['requirements'].get('learning_required') is True:
        errors.append('spell/requirements/learning_required: since patch 15.22 no spell is taught (S16)')
    vocations = spell['requirements']['vocations']
    if vocations != sorted(vocations):
        errors.append('spell/requirements/vocations: must be sorted')
    groups = [g['group'] for g in spell['groups']]
    if len(set(groups)) != len(groups):
        errors.append('spell/groups: group keys must differ')
    for index, group in enumerate(groups):
        role = 'primary' if index == 0 else 'secondary'
        if role not in COOLDOWN_GROUPS.get(group, ()):
            errors.append(f'spell/groups/{index}/group: {group!r} is not a declared {role} cooldown group (S9, '
                          'cooldown-groups.json)')
    execution = spell['execution']
    if 'conjure' in execution and spell['carrier'] != 'instant':
        errors.append('spell/execution/conjure: only an instant spell conjures')
    if 'rune' in spell and 'ability' not in execution and 'native_behavior' not in execution:
        errors.append('spell/rune: a rune spell executes an Ability or a native behaviour')
    abilities = {ident('Ability', a['identity']): a for a in deps['abilities']}
    effects = {ident('Effect', e['identity']): e for e in deps['effects']}
    formulas = {ident('Formula', f['identity']): f for f in deps['formulas']}
    reached = set()
    for effect in deps['effects']:
        if 'duration_range_ms' in effect:
            duration = effect['duration_range_ms']
            if duration['maximum'] < duration['minimum']:
                errors.append('create_item duration range is reversed')
            if any(value % 1000 for value in duration.values()):
                errors.append('create_item integer-second duration requires whole seconds')
    for ability in deps['abilities']:
        label = 'ability ' + ability['identity']['key']
        for reference in ability.get('variants', []):
            variant = abilities.get(ident('Ability', reference))
            if variant is None:
                errors.append(label + '/variants: variant needs a local Ability payload')
            elif 'variants' in variant:
                errors.append(label + '/variants: a variant cannot have variants')
        if 'windup' in ability and (not ability['needs_target'] or
                                   any(k in ability for k in ('area', 'variants', 'chain', 'encounter'))):
            errors.append(label + '/windup: only on a single-target Ability without area, variants, chain or encounter')
        for orientation, rows in ability.get('area', {}).get('matrix', {}).items():
            check_matrix(rows, label + '/area/matrix/' + orientation, errors)
    native = execution.get('native_behavior', {})
    if native.get('key') == 'party_buff':
        parameters = native['parameters']
        check_matrix(parameters['area'], 'spell/execution/native_behavior/parameters/area', errors, hit_centre=True)
        mana = parameters['mana']
        if mana['mode'] == 'scaled':
            percent = mana['falloff'] * 100
            if not math.isfinite(percent) or abs(percent - round(percent)) > 1e-9:
                errors.append('party_buff mana falloff must be a whole percent in (0, 1]')
        effect = effects.get(ident('Effect', parameters['effect']))
        if effect is None:
            errors.append('party_buff effect needs a local Effect payload')
        elif 'formula' in effect:
            reached.add(ident('Formula', effect['formula']))
    if 'ability' in execution:
        ability = abilities.get(ident('Ability', execution['ability']))
        if ability is None:
            errors.append('spell/execution/ability: include the local Ability payload')
        else:
            if ability['kind'] != 'spell':
                errors.append('spell/execution/ability: a player Spell executes an Ability of kind spell')
            for variant in [abilities.get(ident('Ability', r)) for r in ability.get('variants', [])] + [ability]:
                for ref in (variant or {}).get('effects', []):
                    effect = effects.get(ident('Effect', ref))
                    if effect and 'formula' in effect:
                        reached.add(ident('Formula', effect['formula']))
    for key, formula in formulas.items():
        if formula['kind'] != 'player_expression':
            if key in reached and formula['kind'] == 'caster_magnitude':
                errors.append('formula ' + key[1] + ': caster_magnitude needs a monster schedule; a player cast has none')
            continue
        if key not in reached:
            errors.append('formula ' + key[1] + ': player_expression is not reached from this Spell')
            continue
        check_formula(formula, spell.get('base_power'), 'formula ' + key[1], errors,
                      spell.get('needs_shield', False))
    if manifest is not None:
        for entry in manifest['entries']:
            if entry['source_index'] >= len(manifest['sources']):
                errors.append('manifest: invalid source index')
            status = entry['status']
            if status in ('unsupported_source_field', 'unresolved_semantics', 'unresolved_dependency', 'partial_text'):
                errors.append(f"manifest: {status} {entry['source_file']}:{entry['source_line']} {entry['source_field']}")
            if entry['kind'] == 'script' and status not in ('resolved_native_behavior', 'approved_omission'):
                errors.append('manifest: custom script requires explicit native behavior resolution or approved omission')
            if status in ('mapped', 'resolved_native_behavior'):
                try:
                    resolve_pointer({'spell': bundle, 'dependencies': deps}, entry['destination'])
                except (KeyError, IndexError, ValueError, TypeError):
                    errors.append('manifest: missing destination ' + entry['destination'])
    return errors


def resolve_pointer(document, p):
    if not isinstance(p, str) or not p.startswith('/'):
        raise ValueError('expected absolute JSON pointer')
    value = document
    for part in p[1:].split('/'):
        if re.search(r'~(?:[^01]|$)', part):
            raise ValueError('invalid JSON pointer escape')
        key = part.replace('~1', '/').replace('~0', '~')
        if isinstance(value, list):
            if re.fullmatch(r'0|[1-9][0-9]*', key) is None:
                raise ValueError('invalid JSON pointer array index')
            value = value[int(key)]
        elif isinstance(value, dict):
            value = value[key]
        else:
            raise ValueError('JSON pointer cannot traverse a scalar')
    return value


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('spell')
    parser.add_argument('dependencies')
    parser.add_argument('--catalog')
    parser.add_argument('--manifest')
    args = parser.parse_args()
    issues = validate(read(args.spell), read(args.dependencies), read(args.catalog) if args.catalog else None,
                      read(args.manifest) if args.manifest else None)
    print(json.dumps({'valid': not issues, 'scope': 'local authoring structure and declared dependency closure; '
                      'no runtime qualification', 'import_manifest_checked': args.manifest is not None,
                      'source_coverage_proven': False, 'runtime_qualified': False, 'errors': issues},
                     ensure_ascii=False, indent=2))
    raise SystemExit(bool(issues))
