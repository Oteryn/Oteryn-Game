"""Read-only comparison of captured GuildStats spell bounds with authoring ASTs.

GuildStats publishes min/max spell ranges, not averages. Its 'Best hit' and
'Best PvP hit' labels belong to melee/distance autoattacks. The captured magic
calculator uses a legacy linear level contribution and JavaScript Math.round;
agreement here is an independent observation, never acceptance of Game truth.
No downloaded JavaScript is executed, and this module makes no network requests.
"""
import argparse
import hashlib
from html.parser import HTMLParser
import json
import math
from pathlib import Path
import re

from validate_spell import evaluate

SOURCE_URL = 'https://guildstats.eu/character-hits'
PROBES = [(1, 0), (200, 13), (200, 50), (1000, 50), (1101, 50), (200, 100)]
_NUMBER = r'-?\d+(?:\.\d+)?'
_STRING = r"'((?:\\.|[^'\\])*)'"
_ROWS = re.compile(
    r'\{\s*name:\s*' + _STRING + r',\s*words:\s*' + _STRING
    + r',\s*min:\s*(.*?),\s*max:\s*(.*?)\s*\}')
_BOUND = re.compile(r'r\(\s*(' + _NUMBER + r')\s*,\s*(' + _NUMBER + r')\s*\)')


def _compact(source):
    return re.sub(r'\s+', '', source)


def _string(value):
    if re.search(r"\\[^'\\]", value):
        raise ValueError('unsupported JavaScript string escape')
    return value.replace("\\'", "'").replace('\\\\', '\\')


def _bound(source):
    if source.strip() == '0':
        return {'constant': 0}
    match = _BOUND.fullmatch(source.strip())
    if not match:
        return None
    return {'magic_coefficient': float(match[1]), 'offset': float(match[2])}


def _variables(expression):
    variables = {expression['var']} if 'var' in expression else set()
    for argument in expression.get('args', []):
        variables.update(_variables(argument))
    return variables


class _ScriptBodies(HTMLParser):
    """Collect raw inline script bodies with the HTML tokenizer, not a tag regex."""

    def __init__(self):
        super().__init__(convert_charrefs=False)
        self.bodies, self._current = [], None

    def handle_starttag(self, tag, attrs):
        if tag == 'script':
            self._current = []

    def handle_endtag(self, tag):
        if tag == 'script' and self._current is not None:
            self.bodies.append(''.join(self._current))
            self._current = None

    def handle_data(self, data):
        if self._current is not None:
            self._current.append(data)


def _script_bodies(text):
    parser = _ScriptBodies()
    parser.feed(text)
    parser.close()
    return parser.bodies


def extract_snapshot(html, expected_sha256=None):
    """Extract only the recognized calculator grammar; fail on helper changes.

    The caller retains the HTML capture. Hashes qualify this exact snapshot,
    rather than claiming that a mutable website has a known game version.
    """
    if isinstance(html, str):
        html = html.encode('utf-8')
    digest = hashlib.sha256(html).hexdigest()
    if expected_sha256 is not None and digest != expected_sha256:
        raise ValueError('GuildStats HTML capture hash mismatch')
    text = html.decode('utf-8')
    scripts = _script_bodies(text)
    candidates = [script for script in scripts if 'function hitsCalc()' in script]
    if len(candidates) != 1:
        raise ValueError('expected one inline hitsCalc calculator')
    script = candidates[0]
    compact = _compact(script)
    required = [
        'constr=(a,b)=>Math.round((lvl*0.2)+(ml*a)+b);',
        'constmlvlLegsVal=this.mlvlLegs?1:0;',
        'constmlvlPotionVal=this.mlvlPotion?3:0;',
        'constmlvlCakeVal=this.mlvlCake?5:0;',
        'constmlvlSpellVal=this.mlvlSpell?1:0;',
        'constml=mlvlSpellVal+mlvlLegsVal+this.mlvlHelm+this.magic+'
        'this.mlvlShield+mlvlPotionVal+mlvlCakeVal+this.mlvlArm;',
    ]
    if any(fragment not in compact for fragment in required):
        raise ValueError('unsupported GuildStats level, rounding, or effective ML helper')
    if "s.min + ' - ' + s.max + ' hp'" not in text:
        raise ValueError('spell bounds presentation could not be confirmed')
    rows, unsupported = [], []
    for group, block in re.findall(r'this\.spells\.(\w+)\s*=\s*\[(.*?)\];', script, re.S):
        matches = list(_ROWS.finditer(block))
        if len(matches) != block.count('{'):
            raise ValueError('unrecognized GuildStats spell row structure')
        for match in matches:
            name, words = _string(match[1]), _string(match[2])
            lower, upper = _bound(match[3]), _bound(match[4])
            if lower is None or upper is None:
                unsupported.append({'name': name, 'words': words,
                                    'reason': 'non-magic formula; no spell-bound comparison'})
                continue
            rows.append({'name': name, 'words': words, 'group': group,
                         'minimum': lower, 'maximum': upper})
    if not rows or len({row['words'] for row in rows}) != len(rows):
        raise ValueError('missing or ambiguous GuildStats spell rows')
    return {'source_url': SOURCE_URL, 'html_sha256': digest,
            'calculator_sha256': hashlib.sha256(script.encode('utf-8')).hexdigest(),
            'source_game_version': None,
            'semantics': {'measurement': 'spell minimum and maximum',
                          'level_term': 'level * 0.2', 'rounding': 'JavaScript Math.round',
                          'pvp': 'spell PvP bounds are not published'},
            'spells': rows, 'unsupported': unsupported}


def effective_magic_level(base, *, spellbook=0, helmet=0, armor=0,
                          legs=False, potion=False, cake=False, spell=False):
    """Apply exactly the captured magic inputs, once, before either bound."""
    numeric = (base, spellbook, helmet, armor)
    if any(isinstance(value, bool) or not isinstance(value, (int, float))
           or not math.isfinite(value) or value < 0 for value in numeric):
        raise ValueError('magic level and equipment bonuses must be finite and nonnegative')
    if any(type(value) is not bool for value in (legs, potion, cake, spell)):
        raise ValueError('magic extra flags must be booleans')
    # Preserve the website's authored addition order, including float inputs.
    return (int(spell) + int(legs) + helmet + base + spellbook
            + 3 * int(potion) + 5 * int(cake) + armor)


def source_bounds(row, level, magic_level):
    """GuildStats raw ranges, with no world normalization or invented PvP rule."""
    if (isinstance(level, bool) or not isinstance(level, (int, float))
            or not math.isfinite(level) or level < 1):
        raise ValueError('level must be finite and at least one')
    magic_level = effective_magic_level(magic_level)
    result = []
    for field in ('minimum', 'maximum'):
        bound = row[field]
        if 'constant' in bound:
            result.append(bound['constant'])
        else:
            value = (level * 0.2) + (magic_level * bound['magic_coefficient']) + bound['offset']
            # All recognized source terms are nonnegative. Python round would
            # tie to even, unlike Math.round, so it is deliberately not used.
            integer = math.floor(value)
            result.append(integer + (value - integer >= 0.5))
    return result


def compare_formula(row, formula, base_power, probes=PROBES):
    """Compare identical level/effective-ML inputs to one base component only."""
    if formula.get('kind') != 'player_expression' or formula.get('inputs') != 'level_magic':
        raise ValueError('comparison requires a level_magic player expression')
    variables = _variables(formula['minimum']) | _variables(formula['maximum'])
    if variables - {'level', 'magic_level', 'base_power'}:
        raise ValueError('formula needs additional inputs absent from this calculator')
    if 'base_power' in variables and base_power is None:
        raise ValueError('formula needs a declared base_power')
    comparisons = []
    for level, magic_level in probes:
        env = {'level': level, 'magic_level': magic_level}
        if base_power is not None:
            env['base_power'] = base_power
        ours = [math.trunc(evaluate(formula[field], env))
                for field in ('minimum', 'maximum')]
        source = source_bounds(row, level, magic_level)
        comparisons.append({'level': level, 'effective_magic_level': magic_level,
                            'guildstats': source, 'authoring': ours,
                            'agrees': source == ours})
    return comparisons


def compare_bundles(snapshot, directory, probes=PROBES):
    """Match incantations, skip conjures and ambiguous components explicitly."""
    by_words = {row['words']: row for row in snapshot['spells']}
    results, skipped = [], []
    matched = set()
    for spell_path in sorted(Path(directory).glob('*/spell.json')):
        spell = json.loads(spell_path.read_text())['spell']
        row = by_words.get(spell.get('words'))
        if row is None:
            continue
        dependency_path = spell_path.with_name('dependencies.json')
        dependencies = json.loads(dependency_path.read_text())
        formulas = dependencies.get('formulas', [])
        magic = [f for f in formulas if f.get('kind') == 'player_expression'
                 and f.get('inputs') == 'level_magic']
        if not magic:
            skipped.append({'bundle': str(spell_path.parent), 'reason': 'no magic base formula'})
            continue
        if len(magic) != 1:
            skipped.append({'bundle': str(spell_path.parent),
                            'reason': 'multiple formula components; no implicit central/flank choice'})
            continue
        matched.add(row['words'])
        try:
            observations = compare_formula(row, magic[0], spell.get('base_power'), probes)
        except ValueError as error:
            skipped.append({'bundle': str(spell_path.parent), 'reason': str(error)})
            matched.discard(row['words'])
            continue
        results.append({'spell': spell['name'], 'guildstats_spell': row['name'],
                        'formula_identity': magic[0]['identity'],
                        'spell_sha256': hashlib.sha256(spell_path.read_bytes()).hexdigest(),
                        'dependencies_sha256': hashlib.sha256(dependency_path.read_bytes()).hexdigest(),
                        'probes': observations})
    return {'source': {key: value for key, value in snapshot.items() if key != 'spells'},
            'results': results, 'skipped_bundles': skipped,
            'unmatched_source_spells': [r['name'] for r in snapshot['spells'] if r['words'] not in matched],
            'total_comparisons': sum(len(r['probes']) for r in results),
            'differences': sum(not p['agrees'] for r in results for p in r['probes']),
            'limitation': 'Raw calculator ranges only; legacy linear level term and rounded '
                          'coefficients may differ from qualified S5/world formulas. No spell PvP, '
                          'RNG mean, skill spell, modifier, or full-cast qualification.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('html', type=Path, help='saved complete public character-hits HTML')
    parser.add_argument('--expected-sha256', help='require this exact HTML capture')
    parser.add_argument('--bundles', type=Path, help='directory of authored spell bundle folders')
    parser.add_argument('--out', type=Path, help='write the JSON report here')
    args = parser.parse_args()
    snapshot = extract_snapshot(args.html.read_bytes(), args.expected_sha256)
    report = compare_bundles(snapshot, args.bundles) if args.bundles else snapshot
    output = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
    if args.out:
        args.out.write_text(output)
    else:
        print(output, end='')


if __name__ == '__main__':
    main()
