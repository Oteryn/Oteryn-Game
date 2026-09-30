"""Convert the Canary and Crystal Server player spells into Spell authoring bundles (plan phase P2).

Evidence tooling only: every bundle is a candidate built from OtsHypothesisOnly sources and wiki
observations, never Game truth. Keys use the provisional `candidate:` namespace (no native key minting).

Field rules (docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md section 5):
- S3/S11/S13/S14: a value TibiaWiki BR or Fandom states decides; on a BR/Fandom conflict the official change in
  official-changes.json decides, then a vote: each wiki, tibiopedia.pl and the Canary 15.30 branch (S14) back one
  value, the most votes win and a tie goes to the 15.30 branch; otherwise the wiki page with the newer revision.
- S16: learning_required is false (patch 15.22 unlocks spells at their level); a Wheel of Destiny spell carries
  wheel_unlock (S6), stated by the wiki or else by the Canary 15.30 needLearn.
- S4: a value only one of Canary and Crystal has is taken from it. S21: a value they disagree on (with the engine
  default for an absent call) and that no wiki or tibia.com states follows the Canary 15.30 branch.
- S5: player damage/heal formulas are the source expression trees; `level / 5` and Canary's
  calculateFlatDamageHealing become the world curve `level_base_damage_healing`. When the sources'
  executions differ only in the formula, the formula that consumes the wiki base power wins.
- S1/S2: execution is an Ability (monster Ability/Effect definitions, converted with the monster
  converter's `combat_ability`), a conjure, or an unresolved native behaviour for custom scripts.

Usage:
    python convert_spells.py --canary <canary@99902524> --crystal <crystalserver@ff7ede5> \
        [--out DIR] [--only NAME ...] [--readiness samples/spell-readiness.json]
"""
import argparse
import copy
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
MONSTER = ROOT.parent / 'monster-authoring'
sys.path.insert(0, str(MONSTER))
sys.path.insert(0, str(ROOT))

import canary_batch  # noqa: E402  monster converter: combat_ability, engine parameter binding
import spell_scripts  # noqa: E402
import wiki_spells as ws  # noqa: E402
from spell_census import negate  # noqa: E402
from validate_spell import COOLDOWN_GROUPS, evaluate, validate  # noqa: E402

SAMPLES = ROOT / 'samples'
CENSUS = SAMPLES / 'spell-census-canary-99902524-crystal-ff7ede5.json'
FANDOM_FACTS = SAMPLES / 'wiki-spell-facts-fandom-2026-09-27.json'
BR_FACTS = SAMPLES / 'wiki-spell-facts-br-2026-09-27.json'
TIBIOPEDIA_FACTS = SAMPLES / 'tibiopedia-spell-facts-2026-09-28.json'
TIBIACOM_LIST = SAMPLES / 'tibiacom-spell-list-2026-09-28.json'
# S15: tibia.com list names that differ from the source spell names (source name -> tibia.com name).
TIBIACOM_NAMES = {'invisibility': 'invisible', 'paralyze rune': 'paralyse rune', 'monk familiar': 'summon monk familiar'}
OFFICIAL = ROOT / 'official-changes.json'
# S23: accepted chain parameters of player spells (OTERYN_SPELL_CHAIN_BEHAVIOUR_CANDIDATE_V1.md).
CHAINS = json.loads((ROOT / 'chain-behaviours.json').read_text(encoding='utf-8'))['spells']
# S27 C.3: accepted party_buff parameters of the party spells (OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md).
PARTY = json.loads((ROOT / 'party-behaviours.json').read_text(encoding='utf-8'))
# S27 B.5/C.5/D.3: onCastSpell guards now expressed by spell fields (guard-behaviours.json).
GUARDS = json.loads((ROOT / 'guard-behaviours.json').read_text(encoding='utf-8'))['spells']
CHAIN_FIELDS = ('max_targets', 'range_tiles', 'backtracking', 'shape', 'initial_range_tiles', 'damage_step_percent')
CANARY_DECIDES = 'S21: the Canary 15.30 branch decides a Canary/Crystal conflict no wiki or tibia.com states'
REVISION = 'spell-p2-r14'  # r2: S13; r3: S14 (Canary 15.30 branch source and tie vote); r4: S18 presentation; r5: S15 list; r6: wiki spellid; r7: S19 library text; r8: S20 cast options, S21 Canary precedence, S22 Wheel level; r9: S23 chains; r10: S24 removed spells, rune groups from the wiki runegroup; r11: S25 unstated secondary groups, Dawnport conjure spells; r12: S26 Harmony role; r13: S27 party_buff; r14: S27 accepted guards, Train Party (D213)
SOURCES = {'canary': {'repository': 'opentibiabr/canary', 'branch': 'dudantas/fix-tibia-15-30-regressions',
                      'revision': '99902524e052f37574194466c2949c576e4ab269', 'tag': 'canary-99902524'},  # S14
           'crystal': {'repository': 'zimbadev/crystalserver', 'revision': 'ff7ede593c69d4c658b382c97443e8155926924a',
                       'tag': 'crystal-ff7ede5'}}
VOCATIONS = {'druid': 'elder_druid', 'sorcerer': 'master_sorcerer', 'knight': 'elite_knight', 'paladin': 'royal_paladin',
             'monk': 'exalted_monk'}
SOURCE_VOCATION = {'druid': 'druid', 'elder druid': 'elder_druid', 'sorcerer': 'sorcerer', 'master sorcerer': 'master_sorcerer',
                   'knight': 'knight', 'elite knight': 'elite_knight', 'paladin': 'paladin', 'royal paladin': 'royal_paladin',
                   'monk': 'monk', 'exalted monk': 'exalted_monk'}
LEVEL_UNLOCK_NOTE = ('since patch 15.22 (27 January 2026) spells unlock automatically and free at their level; trainers '
                     'no longer teach spells (https://tibiopedia.pl/updates/15.22.c93366).')
# Registrar calls whose Canary 15.30 value votes in a BR/Fandom conflict (S14a); same units as the wiki crosswalk.
VOTE_METHODS = {'level', 'mana', 'soul', 'cooldown', 'groupCooldown', 'basePower', 'magicLevel', 'range'}
# Canary/Crystal engine defaults for an absent registrar call (src/creatures/combat/spells.hpp, actions.hpp).
DEFAULTS = {'isAggressive': True, 'isSelfTarget': False, 'needTarget': False, 'needDirection': False,
            'needCasterTargetOrDirection': False, 'blockWalls': True, 'allowOnSelf': True, 'checkFloor': True,
            'setPzLocked': False, 'needWeapon': False, 'needLearn': False, 'isPremium': False, 'soul': 0,
            'cooldown': 1000, 'allowFarUse': False, 'hasParams': False, 'hasPlayerNameParam': False, 'range': None,
            'manaPercent': 0, 'charges': None, 'magicLevel': 0}
FORMULA_VARS = {'level': 'level', 'magic_level': 'magic_level', 'base_power': 'base_power', 'attack_skill': 'attack_skill',
                'attack_value': 'attack_value', 'attack_factor': 'attack_factor', 'skill:SKILL_SHIELD': 'shielding_skill'}
FORMULA_OPS = {'add', 'sub', 'mul', 'div', 'neg', 'floor', 'ceil', 'sqrt', 'abs', 'min', 'max'}
SIGN_PROBE = {'level': 100, 'magic_level': 50, 'base_power': 100, 'attack_skill': 80, 'attack_value': 50,
              'attack_factor': 1, 'shielding_skill': 80}


class Unresolved(Exception):
    pass


def slug(name):
    return re.sub(r'[^a-z0-9]+', '_', str(name).lower()).strip('_')


def ref(family, key):
    return {'family': family, 'key': key, 'revision': REVISION}


def ident(key):
    return {'key': key, 'revision': REVISION}


def git_blob(data):
    return hashlib.sha1(b'blob %d\0' % len(data) + data).hexdigest()


# ------------------------------------------------------------------------------------------------
# Wiki resolution (S3, S11, S13)
# ------------------------------------------------------------------------------------------------

class Wikis:
    def __init__(self, fandom, br, official, tibiopedia=None, tibiacom=None):
        # tibiopedia.pl only breaks a BR/Fandom tie (S13); it never states a value the wikis do not.
        # tibia.com decides every field it states (S15), ahead of the wikis.
        self.docs = {'fandom': fandom, 'br': br, **({'tibiopedia': tibiopedia} if tibiopedia else {}),
                     **({'tibiacom': tibiacom} if tibiacom else {})}
        self.spells, self.spell_names, self.runes = {}, {}, {}
        for wiki, doc in self.docs.items():
            by_words, by_name, runes = {}, {}, {}
            for page in doc['pages']:
                if page.get('template') == 'Infobox Spell':
                    words = ws.words_key(page['fields'].get('words', ''))
                    if words:
                        by_words.setdefault(words, []).append(page)
                    by_name[ws.plain(page['fields'].get('name', page['title'])).lower()] = page
                elif page.get('template') == 'Infobox Object':
                    key = ws.rune_key(page['fields'])
                    if key not in ('', None):
                        runes.setdefault(key, page)
                        name = ws.plain(page['fields'].get('name', page['title'])).lower()
                        runes.setdefault(name, page)
            self.spells[wiki], self.spell_names[wiki], self.runes[wiki] = by_words, by_name, runes
        self.official = {(c['spell'], c['field']): c for c in official['changes']}

    def spell_page(self, wiki, record, words=None):
        if wiki == 'tibiacom':  # S15: joined by name; its words may correct the sources
            if record['spell_type'] != 'instant':  # a "Rune" row describes the conjuring spell, not rune use
                return None
            name = str(record['name']).lower()
            return self.spell_names[wiki].get(TIBIACOM_NAMES.get(name, name))
        reg = record['registrar']
        words = ws.words_key(words if words else reg.get('words', ''))
        pages = self.spells[wiki].get(words, [])
        if len(pages) != 1:
            pages = [p for key, group in self.spells[wiki].items() if key.startswith(words + ' ') and words
                     for p in group if reg.get('hasParams') or reg.get('hasPlayerNameParam')]
        if len(pages) == 1:
            return pages[0]
        return self.spell_names[wiki].get(str(record['name']).lower())

    def rune_page(self, wiki, record):
        if wiki == 'tibiacom':  # the list view describes spells, not rune use
            return None
        reg = record['registrar']
        return self.runes[wiki].get(reg.get('runeId')) or self.runes[wiki].get(str(record['name']).lower())

    def resolve(self, pages, field, spell_name, branch_vote=None):
        """(value, wiki provenance, note) for one wiki field; value None when no wiki states it.

        branch_vote is the Canary 15.30 branch value of this field in wiki units (S14a), or None."""
        official_page = pages.get('tibiacom')
        if official_page is not None and official_page['fields'].get(field) not in (None, ''):
            value = ws.crosswalk_value(field, official_page['fields'][field])
            if value is not None:
                return value, [('tibiacom', official_page)], 'S15: the official tibia.com spell library states this value.'
        values = {}
        tie_breaker = pages.get('tibiopedia')
        for wiki, page in pages.items():
            if wiki in ('tibiopedia', 'tibiacom'):
                continue
            if page is not None and page['fields'].get(field) not in (None, ''):
                value = ws.crosswalk_value(field, page['fields'][field])
                if value is not None:
                    values[wiki] = (value, page)
        if not values:
            return None, [], None
        distinct = {json.dumps(v[0], sort_keys=True) for v in values.values()}
        if len(distinct) == 1:
            return next(iter(values.values()))[0], [(w, p) for w, (_, p) in values.items()], None
        official = self.official.get((str(spell_name).lower(), field))
        if official:
            value = ws.crosswalk_value(field, official['value'])
            note = (f'S11: BR {values["br"][0]!r} and Fandom {values["fandom"][0]!r} disagree; the official change of '
                    f'{official["date"]} decides ({official["fact"]}; {official["source"]}).')
            chosen = [(w, p) for w, (v, p) in values.items() if v == value]
            return value, chosen or list((w, p) for w, (_, p) in values.items()), note
        third = tie_breaker['fields'].get(field) if tie_breaker is not None else None
        third = ws.crosswalk_value(field, third) if third not in (None, '') else None
        # S13 + S14a: each wiki votes for its value, tibiopedia.pl and the Canary 15.30 branch vote for the wiki value
        # they equal; the most votes win, and a tie goes to the side of the 15.30 branch.
        votes = {w: 1 + (third is not None and v == third) + (branch_vote is not None and v == branch_vote)
                 for w, (v, _) in values.items()}
        best = max(votes.values())
        leaders = [w for w, n in votes.items() if n == best]
        if len(leaders) > 1 and branch_vote is not None:
            leaders = [w for w in leaders if values[w][0] == branch_vote] or leaders
        if len(leaders) == 1 and best > 1:
            winner = leaders[0]
            other = 'fandom' if winner == 'br' else 'br'
            backers = ([f'tibiopedia.pl ({tie_breaker["url"]})'] if third == values[winner][0] else []) + \
                      (['the Canary 15.30 branch'] if branch_vote == values[winner][0] else [])
            rule = 'S13' if backers and backers[0].startswith('tibiopedia') and len(backers) == 1 else 'S13/S14a'
            note = (f'{rule}: BR {values["br"][0]!r} and Fandom {values["fandom"][0]!r} disagree and no official change is '
                    f'recorded; {" and ".join(backers)} side with {winner} over {other} '
                    f'(votes {votes[winner]}:{votes[other]}).')
            return values[winner][0], [(winner, values[winner][1])], note
        newer = max(values, key=lambda w: values[w][1].get('timestamp', ''))
        other = 'fandom' if newer == 'br' else 'br'
        note = (f'S11/S13/S14a: BR {values["br"][0]!r} and Fandom {values["fandom"][0]!r} disagree, no official change is '
                f'recorded and neither tibiopedia.pl nor the Canary 15.30 branch sides with either; the newer revision ({newer}, {values[newer][1].get("timestamp")}) decides over {other} '
                f'({values[other][1].get("timestamp")}).')
        return values[newer][0], [(newer, values[newer][1])], note


# ------------------------------------------------------------------------------------------------
# Formulas (S5)
# ------------------------------------------------------------------------------------------------

def is_level_div_5(expr):
    if expr.get('op') == 'div' and expr['args'] == [{'var': 'level'}, {'const': '5'}]:
        return True
    return expr.get('op') == 'mul' and sorted(json.dumps(a, sort_keys=True) for a in expr['args']) == sorted(
        [json.dumps({'var': 'level'}), json.dumps({'const': '0.2'})])


def convert_expr(expr, notes):
    if 'const' in expr:
        return {'const': expr['const']}
    if 'var' in expr:
        if expr['var'] not in FORMULA_VARS:
            raise Unresolved(f'formula input {expr["var"]} has no authoring variable')
        return {'var': FORMULA_VARS[expr['var']]}
    if 'fn' in expr:
        if expr['fn'] == 'flat_damage_healing':
            notes.add('S5: Canary Player::calculateFlatDamageHealing (defective above level 1100) is replaced by the '
                      'world curve level_base_damage_healing.')
        elif expr['fn'] != 'base_damage_healing':
            raise Unresolved(f'formula function {expr["fn"]}')
        return {'fn': 'level_base_damage_healing', 'args': [convert_expr(a, notes) for a in expr['args']]}
    if is_level_div_5(expr):
        notes.add('S5: the pre-2022 level contribution level / 5 is replaced by the world curve '
                  'level_base_damage_healing (official scaling since 13.05.12657).')
        return {'fn': 'level_base_damage_healing', 'args': [{'var': 'level'}]}
    if expr.get('op') not in FORMULA_OPS:
        raise Unresolved(f'formula operation {expr.get("op")}')
    return {'op': expr['op'], 'args': [convert_expr(a, notes) for a in expr['args']]}


def player_formula(callback, base_power, notes):
    formula = callback.get('formula') or {}
    if formula.get('status') != 'resolved':
        raise Unresolved('player formula: ' + formula.get('error', 'not resolved'))
    minimum, maximum = convert_expr(formula['minimum'], notes), convert_expr(formula['maximum'], notes)
    env = dict(SIGN_PROBE, base_power=base_power or SIGN_PROBE['base_power'])
    low, high = evaluate(minimum, env), evaluate(maximum, env)
    if low <= 0 and high <= 0:
        minimum, maximum = negate(minimum), negate(maximum)
        low, high = -low, -high
    elif low < 0 or high < 0:
        raise Unresolved('player formula bounds have mixed signs')
    if low > high:
        minimum, maximum = maximum, minimum
    inputs = 'skill' if callback['kind'] == 'CALLBACK_PARAM_SKILLVALUE' else 'level_magic'
    return {'kind': 'player_expression', 'inputs': inputs, 'minimum': minimum, 'maximum': maximum}


def uses_base_power(formula):
    return 'base_power' in json.dumps(formula)


# ------------------------------------------------------------------------------------------------
# Execution (S1/S2) through the monster converter
# ------------------------------------------------------------------------------------------------

SOUND_PREFIX = 'SOUND_EFFECT_TYPE_'
RUNE_ITEM_IDS = set()  # S18: item ids some rune spell uses; filled from the census in main()
LUA_ENUMS = 'src/lua/functions/core/game/lua_enums.cpp'


def source_text(root, relative):
    """A source file from the checkout, or from its HEAD commit when a sparse checkout leaves it out."""
    path = Path(root) / relative
    if path.exists():
        return path.read_text(encoding='utf-8', errors='replace')
    return subprocess.run(['git', '-C', str(root), 'show', f'HEAD:{relative}'], capture_output=True, text=True,
                          check=True).stdout


class Execution:
    def __init__(self, source, root):
        self.source, self.root = source, root
        self.converter = canary_batch.Converter(root, {}, {}, {}, {})
        self.converter.spell_scripts = spell_scripts.SpellScripts(
            root, player_chains=True, accepted_guards={n: {g['canary_body']} for n, g in GUARDS.items()})
        self.converter.pending_definitions = set()
        self.tag = SOURCES[source]['tag']
        self.canonical = self.converter  # S18: replaced by the Canary 15.30 tables once both sources exist
        # Lua names SOUND_EFFECT_TYPE_<member> exist only for the SoundEffect_t members lua_enums.cpp registers.
        self.sounds = set(re.findall(r'SoundEffect_t::([A-Z0-9_]+)', source_text(root, LUA_ENUMS)))

    def sound(self, value):
        """S18: the cue key of a castSound/impactSound constant, or None when Lua reads it as nil (silence)."""
        if not isinstance(value, str) or not value.startswith(SOUND_PREFIX):
            return None
        member = value[len(SOUND_PREFIX):]
        return f'canary.sound:{member.lower()}' if member in self.sounds else None

    def canonical_visuals(self, text):
        """S18: one appearance key per client effect/missile id, named by the Canary 15.30 enums, whichever source
        converted the spell (the two engines name 47 of the same effect ids differently)."""
        own, canon = self.converter, self.canonical

        def rename(match):
            kind, name = match.group(1), match.group(2)
            prefix, table, names = (('CONST_ME_', own.magic_effects, canon.magic_effect_names) if kind == 'effect'
                                    else ('CONST_ANI_', own.missiles, canon.missile_names))
            number = int(name[3:]) if name.startswith('id-') else table[prefix + name.upper()]
            canonical = names.get(number)
            return (f'"canary.appearance:{kind}/' + (canonical[len(prefix):].lower() if canonical else f'id-{number}')
                    + '"')
        return re.sub(r'"canary\.appearance:(effect|missile)/([a-z0-9_-]+)"', rename, text)

    def ability(self, record, base_power, notes):
        """(ability key, dependencies, created item ids) for a plain/random combat spell of this source."""
        try:
            info = self.converter.spell_scripts.evaluate(str(record['name']).lower())
        except Exception as exc:  # e.g. a chain-value callback that reads the player caster
            raise Unresolved(f'{self.source}: the script cannot be evaluated without a player caster '
                             f'({str(exc).splitlines()[0][:120]})')
        if info is None or 'error' in info or info.get('tier') in ('P4', 'NOOP'):
            raise Unresolved(f'{self.source}: the registered script does not evaluate to plain combats '
                             f'({(info or {}).get("error") or (info or {}).get("tier")})')
        reg = record['registrar']
        geometry = {'needs_target': bool(reg.get('needTarget') or reg.get('needCasterTargetOrDirection')),
                    'needs_direction': bool(reg.get('needDirection'))}
        range_tiles = int(reg.get('range') or 0) if (reg.get('range') or 0) > 0 else 0
        deps = {'abilities': [], 'effects': [], 'formulas': []}
        key = f'candidate:ability/spell/{"rune/" if record["spell_type"] == "rune" else ""}{slug(record["name"])}'
        self.converter.pending_definitions = set()
        order = list(dict.fromkeys(info['variants']))
        keys = [key] if len(order) == 1 else [f'{key}/variant-{n}' for n in range(1, len(order) + 1)]
        for ability_key, combat_index in zip(keys, order):
            local = []
            combat = info['combats'][combat_index]
            guard = GUARDS.get(str(record['name']).lower(), {})
            dropped = guard.get('drop_params', []) if guard.get('spell_type') == record['spell_type'] else []
            if dropped:
                combat = {**combat, 'params': {k: v for k, v in combat['params'].items() if k not in dropped},
                          'param_calls': [c for c in combat['param_calls'] if c[0] not in dropped]}
                local.append('S27: ' + ', '.join(dropped) + ' is not authored (guard-behaviours.json).')
            chain = None
            if 'CALLBACK_PARAM_CHAINVALUE' in combat['callbacks']:
                chain = CHAINS.get(str(record['name']).lower())
                if chain is None:
                    raise Unresolved(f'{self.source}: a chain spell without accepted chain parameters (S23)')
                combat = {**combat, 'chain': [chain['max_targets'], chain['range_tiles'], chain['backtracking']]}
            try:
                self.converter.combat_ability(ability_key, combat, geometry, range_tiles, deps, lambda a: a, local)
            except canary_batch.SpellUnresolved as exc:
                raise Unresolved(f'{self.source}: {exc}')
            if chain is not None:
                ability = next(a for a in deps['abilities'] if a['identity']['key'] == ability_key)
                ability['chain'].update({k: chain[k] for k in CHAIN_FIELDS[3:] if k in chain})
                local.append('S23: chain parameters from chain-behaviours.json (' + chain['sources'] + ')')
            notes.update(n.strip() for n in local if n.strip() and 'monster caster' not in n and 'player formula' not in n)
            census_combat = record['combats'][combat_index]
            callbacks = [c for c in census_combat.get('callbacks', []) if 'formula' in c]
            for effect in list(deps['effects']):
                if not effect['identity']['key'].startswith(ability_key + '/'):
                    continue
                if effect.get('formula', {}).get('key') == canary_batch.CASTER_MAGNITUDE:
                    if not callbacks and not census_combat.get('set_formula'):
                        # Combat::getCombatDamage leaves the value 0 without a formula: the combat deals no direct
                        # damage; its conditions or created field items do.
                        notes.add('A combat type without a player formula deals no direct damage (combat.cpp '
                                  'Combat::getCombatDamage keeps 0); its conditions or created items apply.')
                        for field in ('operation', 'damage_type', 'formula', 'mitigated_by', 'affects'):
                            effect.pop(field, None)
                        if effect.get('presentation'):
                            effect['operation'] = 'presentation_only'
                        else:
                            deps['effects'].remove(effect)
                            for ability in deps['abilities']:
                                ability['effects'] = [e for e in ability.get('effects', [])
                                                      if e['key'] != effect['identity']['key']]
                        continue
                    if len(callbacks) != 1:
                        raise Unresolved(f'{self.source}: damage/heal combat without exactly one player formula callback')
                    formula_key = (f'candidate:formula/spell/{"rune/" if record["spell_type"] == "rune" else ""}'
                                   f'{slug(record["name"])}/combat-{combat_index + 1}')
                    body = player_formula(callbacks[0], base_power, notes)
                    if not any(f['identity']['key'] == formula_key for f in deps['formulas']):
                        deps['formulas'].append({'identity': ident(formula_key), **body})
                    effect['formula'] = ref('Formula', formula_key)
        if len(keys) > 1:
            deps['abilities'].append({'identity': ident(key), 'kind': 'spell', 'range_tiles': range_tiles, **geometry,
                                      'variants': [ref('Ability', k) for k in keys]})
        deps['formulas'] = [f for f in deps['formulas'] if f['identity']['key'] != canary_batch.CASTER_MAGNITUDE]
        text = self.canonical_visuals(json.dumps(deps).replace(canary_batch.REV, REVISION))
        items = sorted(int(k.rsplit('/', 1)[1]) for f, k in self.converter.pending_definitions if f == 'Item')
        return key, json.loads(text.replace('canary:item/', 'candidate:item/')), items


def execution_signature(deps):
    """Execution payload without formula bodies (for the S4 comparison of the two sources)."""
    stripped = copy.deepcopy(deps)
    stripped['formulas'] = [{'identity': f['identity']} for f in stripped['formulas']]
    text = json.dumps(stripped, sort_keys=True)
    return re.sub(r'"(canary|crystal)\.appearance:', '"appearance:', text)


# ------------------------------------------------------------------------------------------------
# One spell
# ------------------------------------------------------------------------------------------------

class Bundle:
    def __init__(self, name, records, wikis, executions, sources_text):
        self.name, self.records, self.wikis, self.executions, self.text = name, records, wikis, executions, sources_text
        self.rows, self.sources, self.source_index, self.notes = [], [], {}, set()
        self.catalog = set()

    # --- provenance -----------------------------------------------------------------------------
    def git_source(self, source):
        if source not in self.source_index:
            self.source_index[source] = len(self.sources)
            self.sources.append({'repository': SOURCES[source]['repository'], 'revision': SOURCES[source]['revision']})
        return self.source_index[source]

    def wiki_source(self, wiki, page):
        if wiki == 'tibiacom':
            key = (wiki, page['title'])
            if key not in self.source_index:
                self.source_index[key] = len(self.sources)
                self.sources.append({'kind': 'official_capture', 'url': page['url'], 'title': page['title'],
                                     'captured': self.wikis.docs[wiki]['target_cut'],
                                     'content_sha256': page['content_sha256']})
            return self.source_index[key]
        key = (wiki, page['page_id'])
        if key not in self.source_index:
            self.source_index[key] = len(self.sources)
            self.sources.append({'kind': 'mediawiki', 'api': self.wikis.docs[wiki]['api'], 'title': page['title'],
                                 'page_id': page['page_id'], 'revision_id': page['revision_id'],
                                 'content_sha256': page['content_sha256']})
        return self.source_index[key]

    def line(self, source, method):
        record = self.records[source]
        text = self.text[(source, record['file'])]
        for number, content in enumerate(text.splitlines(), 1):
            if re.search(r':' + re.escape(method) + r'\(', content):
                return number
        return 1

    def row(self, status, field, destination=None, resolution=None, source=None, wiki=None, method=None, kind='field'):
        if wiki:
            index, file, line = self.wiki_source(*wiki), wiki[1]['title'], 1
        else:
            index = self.git_source(source)
            file, line = self.records[source]['file'], self.line(source, method) if method else 1
        entry = {'source_index': index, 'source_file': file, 'source_line': line, 'source_field': field, 'kind': kind,
                 'status': status}
        if destination is not None:
            entry['destination'] = destination
        if resolution is not None:
            entry['resolution'] = resolution
        self.rows.append(entry)

    # --- field resolution -----------------------------------------------------------------------
    def source_value(self, method, transform=lambda v: v):
        """{source: effective value} with engine defaults for an absent call."""
        out = {}
        for source, record in self.records.items():
            value = record['registrar'].get(method, DEFAULTS.get(method))
            out[source] = transform(value) if value is not None else None
        return out

    def wheel_unlock(self, base, pages):
        """S16: since patch 15.22 every spell unlocks at its level; a Wheel of Destiny spell is gated (S6)."""
        for source, record in self.records.items():
            if record['registrar'].get('needLearn') is not None:
                self.row('approved_omission', 'needLearn', resolution=f'S16: {LEVEL_UNLOCK_NOTE} learning_required '
                         'is false; the source needLearn only marks Wheel spells (wheel_unlock).', source=source,
                         method='needLearn')
        value, provenance, note = self.wikis.resolve(pages, 'wheelspell', self.name)
        destination = base + '/requirements/wheel_unlock'
        official_level = self.official_level(pages)
        if value == 'yes' and official_level is not None and [w for w, _ in provenance] == ['fandom']:
            # S15: tibia.com lists a revelation spell without a level; one it gives a level unlocks at that level.
            self.row('approved_omission', 'wheelspell', resolution=f'S15: tibia.com states level {official_level}, so '
                     'the spell unlocks at its level; the Fandom wheelspell marking (BR states none) is superseded.',
                     wiki=provenance[0])
            return None
        if value is not None and (note is not None or value not in ('yes', 'no')):
            # Fail closed: an unrecognised value or a BR/Fandom conflict never makes a Wheel spell castable.
            self.row('unresolved_semantics', 'wheelspell', resolution=f'S6/S16: the wiki Wheel marking {value!r} is '
                     + ('contested (' + note + ')' if note else 'not a recognised yes/no') + '; the spell stays gated.',
                     wiki=provenance[0])
            return True
        if value is not None:
            for wiki, page in provenance:
                self.row('mapped', 'wheelspell', destination, note or 'S6/S16: the wiki marks a Wheel of Destiny '
                         'revelation spell (Fandom wheelspell, BR wheelSpellType Revelação) or not.', wiki=(wiki, page))
            return value == 'yes'  # a stated "no" (a conviction perk or a plain spell) is kept explicitly
        branch = self.records.get('canary', {}).get('registrar', {}).get('needLearn')
        if branch:
            self.row('mapped', 'needLearn', destination, 'S6/S16: no wiki states it; the Canary 15.30 branch, the only '
                     'source that implements the 15.22 unlock, keeps needLearn only for Wheel spells.', source='canary',
                     method='needLearn')
            return True
        return None

    @staticmethod
    def official_level(pages):
        """The level the tibia.com list states for the spell, or None ("-" or no row)."""
        page = pages.get('tibiacom')
        value = page['fields'].get('levelrequired') if page is not None else None
        return ws.crosswalk_value('levelrequired', value) if value not in (None, '') else None

    def sound_cues(self, base):
        """S18: cast and impact sound cues. No wiki states them; a constant Lua reads as nil is silence."""
        cues = {}
        for method, field in (('castSound', 'cast_cue'), ('impactSound', 'impact_cue')):
            keys = {}
            for source, record in self.records.items():
                value = record['registrar'].get(method)
                if value is None:
                    continue
                key = self.executions[source].sound(value)
                if key is None:
                    self.row('approved_omission', method, resolution=f'S18: {value} is not a registered SoundEffect_t '
                             'constant, so Lua reads nil and the engine plays no sound.', source=source, method=method)
                else:
                    keys[source] = key
            if not keys:
                continue
            chosen = keys.get('canary', next(iter(keys.values())))
            for source, key in keys.items():
                if key == chosen:
                    self.row('mapped', method, f'{base}/presentation/{field}', 'S18: sound cue named by its '
                             'SoundEffect_t member.', source=source, method=method)
                else:
                    self.row('approved_omission', method, resolution=f'S18/S14: {key} differs; the Canary 15.30 '
                             f'branch value {chosen} is kept.', source=source, method=method)
            cues[field] = chosen
        return cues

    def branch_vote(self, method):
        """The Canary (15.30 branch, S14) registrar value of a numeric field, in wiki units, as a tie vote."""
        record = self.records.get('canary')
        if record is None or method not in VOTE_METHODS:
            return None
        value = record['registrar'].get(method, DEFAULTS.get(method))
        if isinstance(value, list):
            value = value[0] if value else None
        return value if isinstance(value, int) and not isinstance(value, bool) else None

    def field(self, destination, wiki_field, method, pages, transform=lambda v: v, wiki_transform=lambda v: v,
              required=True):
        """Resolve one Spell field under S3/S4/S11/S13; returns the value (None when nothing states it)."""
        value, provenance, note = (None, [], None)
        if wiki_field:
            value, provenance, note = self.wikis.resolve(pages, wiki_field, self.name, self.branch_vote(method))
        sources = self.source_value(method, transform) if method else {}
        if value is not None:
            value = wiki_transform(value)
            for wiki, page in provenance:
                self.row('mapped', wiki_field, destination, note or 'S3: the wiki states this value.', wiki=(wiki, page))
            for source, source_value in sources.items():
                if source_value is not None and source_value != value:
                    rule = 'S15: superseded by the official tibia.com' if provenance[0][0] == 'tibiacom' else \
                        'S3: superseded by the wiki'
                    self.row('approved_omission', method, resolution=f'{rule} value {value!r} '
                             f'(source {source_value!r}).', source=source, method=method)
            return value
        present = {s: v for s, v in sources.items() if v is not None}
        if not present:
            if required:
                self.row('unresolved_semantics', method or wiki_field, resolution='no source or wiki states this value.',
                         source=next(iter(self.records)))
            return None
        distinct = {json.dumps(v, sort_keys=True) for v in present.values()}
        if len(distinct) > 1:
            if 'canary' not in present:
                self.row('unresolved_semantics', method, resolution='S4 conflict: ' + ', '.join(
                    f'{s} {v!r}' for s, v in present.items()) + '; no wiki states this value.',
                    source=next(iter(present)), method=method)
                return None
            value = present['canary']
            self.row('mapped', method, destination, CANARY_DECIDES + ' (' + ', '.join(
                f'{s} {v!r}' for s, v in present.items()) + ').', source='canary', method=method)
            for source in present:
                if source != 'canary':
                    self.row('approved_omission', method, resolution=f'{CANARY_DECIDES}: {value!r} supersedes '
                             f'{present[source]!r}.', source=source, method=method)
            return value
        value = next(iter(present.values()))
        for source in present:
            self.row('mapped', method, destination, 'S4: source value' + (' (both sources agree).' if len(present) > 1 else '.'),
                     source=source, method=method)
        return value

    # --- assembly -------------------------------------------------------------------------------
    def convert(self):
        primary = self.records.get('crystal') or self.records['canary']
        carrier = primary['spell_type']
        # S15: the official words (joined by name) also select the wiki pages, since the sources can swap words.
        official = self.wikis.spell_page('tibiacom', primary) if 'tibiacom' in self.wikis.docs else None
        official_words = official['fields'].get('words') if official is not None else None
        pages = {w: (self.wikis.rune_page(w, primary) if carrier == 'rune' else
                     self.wikis.spell_page(w, primary, official_words)) for w in self.wikis.docs}
        spell_pages = pages
        if carrier == 'rune':
            spell_pages = {w: self.wikis.spell_page(w, primary, official_words) for w in self.wikis.docs}
        # A rune and its conjuring spell share a name ("sudden death rune"), so the carrier is part of the key.
        key = f'candidate:spell/{"rune/" if carrier == "rune" else ""}{slug(self.name)}'
        spell = {'identity': ident(key), 'name': primary['name'], 'carrier': carrier}
        base = '/spell/spell'
        removed = self.wikis.official.get((self.name, 'removed'))
        if removed and removed['value'] in ('yes', carrier):
            # S24: an official announcement dated before the target date removed the spell from the game.
            self.row('unresolved_semantics', 'spell', resolution=(f'S24: removed from the game by the official change of '
                     f'{removed["date"]} ({removed["fact"]}; {removed["source"]}).' if removed['value'] == 'yes' else
                     f'{removed["fact"]} ({removed["source"]})'), source=next(iter(self.records)),
                     kind='script')
        if carrier == 'instant':
            normal = lambda v: re.sub(r'\s+', ' ', v.strip().lower())  # noqa: E731
            words = self.field(base + '/words', 'words' if pages.get('tibiacom') else None, 'words', pages,
                               transform=normal, wiki_transform=normal)
            if words is not None:
                spell['words'] = words
        spell_id = self.field(base + '/reference_spell_id', 'spellid', 'id', pages, required=False)
        if isinstance(spell_id, int) and spell_id > 0:
            spell['reference_spell_id'] = spell_id
        library_pages = spell_pages
        if carrier == 'rune':  # S19: rune use shares the library entry of its conjuring spell (the rune page's words)
            rune_words = {w: p['fields'].get('words') for w, p in pages.items() if p is not None}
            library_pages = {w: self.wikis.spell_page(w, primary, rune_words.get(w)) if rune_words.get(w) else None
                             for w in self.wikis.docs}
        library_text = self.field(base + '/library_text', 'librarytext', None, library_pages, required=False)
        if library_text:
            spell['library_text'] = library_text
        requirements = {}
        vocations = self.vocations(pages, carrier)
        if vocations:
            requirements['vocations'] = vocations
        wheel = self.wheel_unlock(base, pages)
        if wheel and self.official_level(pages) is None:
            # S22: the client and tibia.com list a Wheel revelation spell without a level; the Wheel gates it (S6/S16).
            requirements['level'] = 0
            self.row('mapped', 'level', base + '/requirements/level', 'S22: a Wheel of Destiny revelation spell has '
                     'level 0, as the client spell list shows and tibia.com states no level; the Wheel unlock gates it.',
                     source=next(iter(self.records)), method='level')
        else:
            level = self.field(base + '/requirements/level', 'levelrequired', 'level', pages)
            requirements['level'] = level if level is not None else 0
        premium = self.field(base + '/requirements/premium', 'premium', 'isPremium', pages,
                             wiki_transform=lambda v: v == 'yes')
        requirements['premium'] = bool(premium)
        requirements['learning_required'] = False
        if wheel is not None:
            requirements['wheel_unlock'] = wheel
        spell['requirements'] = requirements
        costs = {}
        mana = self.field(base + '/costs/mana', 'mana', 'mana', pages if carrier == 'instant' else {}, required=False)
        if isinstance(mana, int):
            costs['mana'] = mana
        elif mana == 'varies' and 'base_mana' in PARTY['spells'].get(self.name, {}):
            self.row('mapped', 'mana', base + '/costs/mana', 'S27 C.3: the wiki mana varies; the party_buff parameters '
                     'define it and costs.mana is 0.', source=next(iter(self.records)), method='mana')
            costs['mana'] = 0
        elif mana == 'varies':
            self.row('unresolved_semantics', 'mana', resolution='the wiki mana varies (party spells); a native '
                     'behaviour must define it.', source=next(iter(self.records)), method='mana')
            costs['mana'] = 0
        percent = self.source_value('manaPercent')
        if any(percent.values()) and 'mana' not in costs:
            costs['mana_percent'] = max(v for v in percent.values() if v)
        if carrier == 'rune' and 'mana' not in costs and 'mana_percent' not in costs:
            costs['mana'] = 0
        if 'mana' not in costs and 'mana_percent' not in costs and mana is None:
            costs['mana'] = 0
        soul = self.field(base + '/costs/soul', 'soul' if carrier == 'instant' else None, 'soul', pages)
        costs['soul'] = soul or 0
        spell['costs'] = costs
        cooldown = self.field(base + '/cooldown_ms', 'cooldown', 'cooldown', spell_pages)
        spell['cooldown_ms'] = cooldown or 1000
        spell['groups'] = self.groups(spell_pages, carrier)
        # S26: only Canary 15.30 states the monk Harmony role (S21); Crystal has no such call.
        role = self.field(base + '/harmony_role', None, 'monkSpellType', pages, required=False,
                          transform=lambda v: {'MonkSpell_Builder': 'builder', 'MonkSpell_Spender': 'spender'}.get(str(v).lstrip('@')))
        if role:
            spell['harmony_role'] = role
        spell['targeting'] = self.targeting(spell_pages)
        spell['pz_locks_caster'] = bool(self.field(base + '/pz_locks_caster', None, 'setPzLocked', pages))
        spell['needs_weapon'] = bool(self.field(base + '/needs_weapon', None, 'needWeapon', pages))
        presentation = self.sound_cues(base)
        if presentation:
            spell['presentation'] = presentation
        base_power = self.field(base + '/base_power', 'basepower', 'basePower', pages, required=False)
        if base_power:
            spell['base_power'] = base_power
        if carrier == 'rune':
            spell['rune'] = self.rune(pages)
        deps = {'abilities': [], 'effects': [], 'formulas': []}
        spell['execution'] = self.execution(deps, base_power, spell_pages)
        bundle = {'spell': spell}
        catalog = {'definitions': [ref('Item', f'candidate:item/{i}') for i in sorted(self.catalog)]}
        for note in sorted(self.notes):
            self.row('metadata_only', 'note', resolution=note, source=next(iter(self.records)))
        manifest = {'sources': self.sources, 'entries': self.rows}
        return bundle, deps, catalog, manifest

    def vocations(self, pages, carrier):
        wiki_field = 'voc' if carrier == 'instant' else 'vocrequired'
        wiki_value, provenance, note = self.wikis.resolve(pages, wiki_field, self.name)
        source_bases = {}
        for source, record in self.records.items():
            names = [str(v).split(';')[0].strip().lower() for v in record['registrar'].get('vocation', [])]
            keys = {SOURCE_VOCATION[n] for n in names if n in SOURCE_VOCATION}
            source_bases[source] = sorted({k for k in keys if k in VOCATIONS} |
                                          {b for b, p in VOCATIONS.items() if p in keys})
        if wiki_value:
            for wiki, page in provenance:
                self.row('mapped', wiki_field, '/spell/spell/requirements/vocations',
                         (note or 'S3: the wiki states the base vocations') + '; S8 adds each promoted vocation.',
                         wiki=(wiki, page))
            bases = wiki_value
        else:
            present = {s: v for s, v in source_bases.items() if v}
            if not present:
                if carrier == 'rune':
                    self.row('mapped', 'vocation', '/spell/spell/requirements/vocations', 'No vocation restriction is '
                             'registered, so every vocation may use the rune (engine default).',
                             source=next(iter(self.records)))
                    bases = sorted(VOCATIONS)
                else:
                    self.row('unresolved_semantics', 'vocation', resolution='no vocation is stated.',
                             source=next(iter(self.records)))
                    return None
            else:
                if len({json.dumps(v) for v in present.values()}) > 1:
                    if 'canary' not in present:
                        self.row('unresolved_semantics', 'vocation', resolution='S4 conflict: ' + ', '.join(
                            f'{s} {v}' for s, v in present.items()) + '; no wiki states the vocations.',
                            source=next(iter(present)), method='vocation')
                        return None
                    for source in present:
                        if source != 'canary':
                            self.row('approved_omission', 'vocation', resolution=f'{CANARY_DECIDES}: '
                                     f'{present["canary"]} supersedes {present[source]}.', source=source, method='vocation')
                    present = {'canary': present['canary']}
                bases = next(iter(present.values()))
                for source in present:
                    self.row('mapped', 'vocation', '/spell/spell/requirements/vocations', 'S4: source vocations.',
                             source=source, method='vocation')
        return sorted({b for b in bases} | {VOCATIONS[b] for b in bases})

    def groups(self, pages, carrier='instant'):
        groups = []
        # A rune's group is the wiki runegroup; subclass is the group of the spell that conjures the rune.
        primary = self.field('/spell/spell/groups/0/group', 'runegroup' if carrier == 'rune' else 'subclass', 'group', pages,
                             transform=lambda v: (v[0] if isinstance(v, list) else v).lower())
        if isinstance(primary, str) and 'primary' not in COOLDOWN_GROUPS.get(primary, ()):
            sources = {s: (r['registrar'].get('group') if not isinstance(r['registrar'].get('group'), list)
                           else r['registrar']['group'][0]) for s, r in self.records.items()}
            fallback = {str(v).lower() for v in sources.values() if v}
            self.row('metadata_only', 'subclass', resolution=f'The wiki category {primary!r} is not a cooldown group; '
                     f'the source group {sorted(fallback)} is used.', source=next(iter(self.records)))
            primary = sorted(fallback)[0] if len(fallback) == 1 else None
        cooldown = self.field('/spell/spell/groups/0/cooldown_ms', 'cooldowngroup', 'groupCooldown', pages,
                              transform=lambda v: v[0] if isinstance(v, list) else v)
        if primary:
            groups.append({'group': primary, 'cooldown_ms': cooldown or 1000})
        secondary = self.field('/spell/spell/groups/1/group', 'secondarygroup', 'group', pages,
                               transform=lambda v: v[1].lower() if isinstance(v, list) and len(v) > 1 else None,
                               wiki_transform=lambda v: re.sub(r'[^a-z]', '', str(v).lower()), required=False)
        if secondary:
            cd2 = self.wikis.resolve(pages, 'cooldowngroup2', self.name)[0]
            if cd2 is None:
                values = {json.dumps(r['registrar']['groupCooldown'][1]) for r in self.records.values()
                          if isinstance(r['registrar'].get('groupCooldown'), list) and len(r['registrar']['groupCooldown']) > 1}
                cd2 = json.loads(next(iter(values))) if len(values) == 1 else None
            if cd2:
                groups.append({'group': secondary, 'cooldown_ms': cd2})
            else:
                # S25: a secondary group no source gives a cooldown is dropped; the official library lists one group.
                for entry in self.rows:
                    if entry.get('destination', '').startswith('/spell/spell/groups/1'):
                        entry['status'] = 'approved_omission'
                        entry['resolution'] = (f'S25: the secondary group {secondary} has no stated cooldown and the '
                                               'official library lists only the primary group; it is not authored.')
                        del entry['destination']
        return groups

    def targeting(self, pages):
        t, base = {}, '/spell/spell/targeting/'
        for name, method in (('aggressive', 'isAggressive'), ('self_target', 'isSelfTarget'), ('needs_target', 'needTarget'),
                             ('needs_direction', 'needDirection'), ('target_or_direction', 'needCasterTargetOrDirection'),
                             ('block_walls', 'blockWalls'), ('allow_on_self', 'allowOnSelf'), ('check_floor', 'checkFloor')):
            value = self.field(base + name, None, method, pages, transform=bool)
            t[name] = bool(value) if value is not None else DEFAULTS[method]
        spell_range = self.field(base + 'range_tiles', 'spellrange', 'range', pages, required=False)
        if isinstance(spell_range, int) and spell_range >= 0:
            t['range_tiles'] = spell_range
        name_param = any(r['registrar'].get('hasPlayerNameParam') for r in self.records.values())
        text_param = any(r['registrar'].get('hasParams') for r in self.records.values())
        t['parameter'] = 'player_name' if name_param else 'text' if text_param else 'none'
        # S20: the cast options of patch 15.25. Aim at Target is stated by BR only; the positional cast by Canary 15.30.
        aim = self.field(base + 'aim_at_target', 'aimattarget', None, pages, wiki_transform=lambda v: v == 'yes',
                         required=False)
        if aim and t['needs_direction']:
            t['aim_at_target'] = True
        elif aim:
            self.row('approved_omission', 'aimattarget', resolution='S20: the wiki marks Aim at Target on a spell '
                     'without a cast direction; not applied.', source=next(iter(self.records)))
        if self.field(base + 'cast_at_position', None, 'optionalTarget', pages, transform=bool, required=False):
            t['cast_at_position'] = True
        guard = self.guard().get('targeting')
        if guard:
            t.update(guard)
            self.row('mapped', 'onCastSpell', base + 'allowed_targets', 'S27: the onCastSpell target guard as '
                     + json.dumps(guard, sort_keys=True) + ' (guard-behaviours.json).', source=next(iter(self.records)))
        return t

    def rune(self, pages):
        base = '/spell/spell/rune/'
        item = self.field(base + 'item', None, 'runeId', pages)
        if item:
            self.catalog.add(int(item))
        charges = self.field(base + 'charges', None, 'charges', pages)
        magic_level = self.field(base + 'magic_level', 'mlrequired', 'magicLevel', pages)
        far = self.field(base + 'allow_far_use', None, 'allowFarUse', pages)
        blocking = self.source_value('isBlocking', lambda v: v if isinstance(v, list) else [v])
        flags = {json.dumps([bool(x) for x in (v or [False, False])] + [False] * (2 - len(v or []))) for v in blocking.values()}
        solid, creature = (json.loads(next(iter(flags)))[:2] if len(flags) == 1 else (False, False))
        if len(flags) > 1:
            if 'canary' in blocking:
                canary = [bool(x) for x in (blocking['canary'] or [False, False])] + [False, False]
                solid, creature = canary[:2]
                self.row('approved_omission', 'isBlocking', resolution=f'{CANARY_DECIDES} on rune:isBlocking.',
                         source=next(s for s in self.records if s != 'canary'), method='isBlocking')
            else:
                self.row('unresolved_semantics', 'isBlocking', resolution='S4 conflict on rune:isBlocking.',
                         source=next(iter(self.records)), method='isBlocking')
        return {'item': ref('Item', f'candidate:item/{int(item or 0)}'), 'charges': int(charges or 1),
                'magic_level': int(magic_level or 0), 'allow_far_use': bool(far), 'blocking': {'solid': solid, 'creature': creature}}

    def execution(self, deps, base_power, pages):
        tiers = {s: r['cast']['tier'] for s, r in self.records.items()}
        if all(t == 'conjure' for t in tiers.values()):
            conjures = {s: r['cast'].get('conjure') or {} for s, r in self.records.items()}
            distinct = {json.dumps({k: v for k, v in c.items() if k != 'count'}, sort_keys=True) for c in conjures.values()}
            if len(distinct) > 1 and 'canary' in conjures:
                self.row('approved_omission', 'conjureItem', resolution=f'{CANARY_DECIDES} on the conjured items: '
                         + json.dumps(conjures), source=next(s for s in conjures if s != 'canary'), kind='script')
                conjures = {'canary': conjures['canary']}
            elif len(distinct) > 1:
                self.row('unresolved_semantics', 'conjureItem', resolution='S4 conflict on the conjured items: ' +
                         json.dumps(conjures), source=next(iter(self.records)), kind='script')
            conjure = next(iter(conjures.values()))
            count = self.wikis.resolve(pages, 'amount', self.name)[0]
            source_count = conjure.get('count')
            if count is None:
                count = source_count
            elif source_count not in (None, count):
                self.row('approved_omission', 'conjureItem', resolution=f'S3: the wiki amount {count} supersedes '
                         f'{source_count}.', source=next(iter(self.records)), kind='script')
            result = conjure.get('result_item_id')
            if not isinstance(result, int) or not isinstance(count, int) or count < 1:
                self.row('unresolved_semantics', 'conjureItem', resolution='conjure arguments are not literal item ids.',
                         source=next(iter(self.records)), kind='script')
                return {'conjure': {'result': ref('Item', 'candidate:item/0'), 'count': 1}}
            self.catalog.add(result)
            body = {'result': ref('Item', f'candidate:item/{result}'), 'count': count}
            # S18: Player:conjureItem shows magic_red for a rune, else its optional effect argument (nil: none).
            source = next(iter(self.records))
            effect = 'CONST_ME_MAGIC_RED' if result in RUNE_ITEM_IDS else conjure.get('effect')
            if isinstance(effect, str) and effect.startswith('CONST_ME_'):
                body['effect_asset_binding'] = json.loads(self.executions[source].canonical_visuals(json.dumps(
                    'canary.appearance:effect/' + effect[len('CONST_ME_'):].lower())))
            reagent = conjure.get('reagent_item_id')
            if isinstance(reagent, int) and reagent > 0:
                self.catalog.add(reagent)
                body['reagent'] = ref('Item', f'candidate:item/{reagent}')
            for source in self.records:
                self.row('resolved_native_behavior', 'onCastSpell', '/spell/spell/execution/conjure',
                         'Player:conjureItem(reagent, result, count) is the S2 conjure execution.', source=source,
                         kind='script')
            return {'conjure': body}
        if self.name in PARTY['spells']:
            return self.party_buff(PARTY['spells'][self.name], deps)
        plain = {s for s, t in tiers.items() if t in ('plain_combat', 'random_combat') or self.guard()}
        if not plain:
            patterns = sorted({p for r in self.records.values() for p in r['cast'].get('patterns', [])})
            self.row('unresolved_semantics', 'onCastSpell', resolution='custom script; needs a native behaviour (S7): '
                     + ', '.join(patterns), source=next(iter(self.records)), kind='script')
            return {'native_behavior': {'key': 'unresolved', 'parameters': {'patterns': patterns}}}
        converted, failures = {}, {}
        for source in sorted(plain):
            notes = set()
            try:
                converted[source] = (*self.executions[source].ability(self.records[source], base_power, notes), notes)
            except Unresolved as exc:
                failures[source] = str(exc)
        for source, reason in failures.items():
            self.row('unresolved_semantics' if not converted else 'approved_omission', 'onCastSpell',
                     resolution=reason + ('' if not converted else ' (the other source converts).'), source=source,
                     kind='script')
        for source in set(tiers) - plain:
            self.row('approved_omission', 'onCastSpell', resolution=f'{source} runs custom logic here ({tiers[source]}); '
                     'the plain combat of the other source is used (S4).', source=source, kind='script')
        if not converted:
            return {'native_behavior': {'key': 'unresolved', 'parameters': {}}}
        chosen = next(iter(converted))
        if len(converted) == 2:
            chosen = 'crystal'
            signatures = {s: execution_signature(c[1]) for s, c in converted.items()}
            if signatures['canary'] != signatures['crystal']:
                self.row('approved_omission', 'onCastSpell', resolution=f'{CANARY_DECIDES}: the Canary and Crystal '
                         'combats differ (area, effects, conditions or parameters) and no wiki states the execution; '
                         'the Canary combat is used.', source='crystal', kind='script')
                chosen = 'canary'
            formulas = {s: [f for f in c[1]['formulas'] if f['kind'] == 'player_expression'] for s, c in converted.items()}
            if chosen == 'canary':
                pass
            elif json.dumps(formulas['canary'], sort_keys=True) == json.dumps(formulas['crystal'], sort_keys=True):
                chosen = 'crystal'
            else:
                with_power = [s for s in ('crystal', 'canary') if formulas[s] and all(uses_base_power(f) for f in formulas[s])]
                chosen = with_power[0] if with_power and base_power else 'crystal'
                other = 'canary' if chosen == 'crystal' else 'crystal'
                self.row('approved_omission', 'onGetFormulaValues', resolution=f'S5: the {chosen} formula '
                         + ('consumes the wiki base power' if with_power and base_power else 'is used')
                         + f'; the {other} formula is superseded.', source=other, method='setCallback')
        key, dep, items, notes = converted[chosen]
        guard = self.guard()
        if guard:
            ability = next(a for a in dep['abilities'] if a['identity']['key'] == key)
            for extra in guard.get('extra_effects', []):
                effect_key = f'{key}/effect-{extra["suffix"]}'
                dep['effects'].append({'identity': ident(effect_key),
                                       **{k: v for k, v in extra.items() if k != 'suffix'}})
                ability['effects'].append(ref('Effect', effect_key))
            self.row('resolved_native_behavior', 'onCastSpell', '/spell/spell/execution/ability', 'S27: the '
                     'onCastSpell guard is expressed by spell fields and effects from guard-behaviours.json ('
                     + guard['sources'] + ')', source=chosen, kind='script')
        self.notes.update(notes)
        deps.update(dep)
        self.catalog.update(items)
        self.row('resolved_native_behavior', 'onCastSpell', '/spell/spell/execution/ability',
                 f'Plain combat converted with the monster combat_ability rules ({chosen}).', source=chosen,
                 method='setCallback' if deps['formulas'] else None, kind='script')
        return {'ability': ref('Ability', key)}

    def guard(self):
        """S27: the accepted onCastSpell guard of this spell and carrier (guard-behaviours.json), or {}."""
        guard = GUARDS.get(self.name, {})
        return guard if guard.get('spell_type') == next(iter(self.records.values()))['spell_type'] else {}

    def party_buff(self, party, deps):
        """S27 C.3: the party_buff execution from party-behaviours.json; the scripts are custom in both sources."""
        if 'blocked' in party:
            self.row('unresolved_semantics', 'onCastSpell', resolution=party['blocked'], source=next(iter(self.records)),
                     kind='script')
            return {'native_behavior': {'key': 'unresolved', 'parameters': {'patterns': ['party']}}}
        condition = party['condition']
        key = f'candidate:spell/{slug(self.name)}/effect-{condition["type"]}'
        deps['effects'].append({'identity': ident(key), 'operation': 'condition', 'duration_ms': 120000,
                                'condition': {**condition, 'lifetime': 'fixed_duration', 'buff_spell': True}})
        for source in self.records:
            self.row('resolved_native_behavior', 'onCastSpell', '/spell/spell/execution/native_behavior',
                     'S27 C.3: party_buff from party-behaviours.json (' + party['sources'] + ')', source=source,
                     kind='script')
        return {'native_behavior': {'key': 'party_buff', 'parameters': {
            'area': PARTY['area'], 'same_floor': True, 'min_affected': 2, 'requires_party': True,
            'mana': {'mode': 'scaled', 'base': party['base_mana'], 'falloff': 0.9, 'rounding': 'up'},
            'effect': ref('Effect', key)}}}


# ------------------------------------------------------------------------------------------------

def run(args):
    census = json.loads(CENSUS.read_text(encoding='utf-8'))
    wikis = Wikis(json.loads(FANDOM_FACTS.read_text(encoding='utf-8')), json.loads(BR_FACTS.read_text(encoding='utf-8')),
                  json.loads(OFFICIAL.read_text(encoding='utf-8')),
                  json.loads(TIBIOPEDIA_FACTS.read_text(encoding='utf-8')),
                  json.loads(TIBIACOM_LIST.read_text(encoding='utf-8')))
    roots = {'canary': args.canary, 'crystal': args.crystal}
    executions = {s: Execution(s, r) for s, r in roots.items()}
    RUNE_ITEM_IDS.update(int(r['registrar']['runeId']) for s in ('canary', 'crystal') for r in census[s]
                         if r['spell_type'] == 'rune' and isinstance(r['registrar'].get('runeId'), int))
    for execution in executions.values():
        execution.canonical = executions['canary'].converter
    groups = {}
    for source in ('canary', 'crystal'):
        for record in census[source]:
            if str(record['registrar'].get('words', '')).startswith('#'):
                continue
            groups.setdefault((record['spell_type'], str(record['name']).lower()), {})[source] = record
    texts = {}
    for (spell_type, name), records in groups.items():
        for source, record in records.items():
            texts[(source, record['file'])] = (roots[source] / record['file']).read_text(encoding='utf-8', errors='replace')
    only = {n.lower() for n in args.only or []}
    results = []
    for (spell_type, name), records in sorted(groups.items()):
        if only and name not in only:
            continue
        bundle = Bundle(name, records, wikis, executions, texts)
        try:
            spell, deps, catalog, manifest = bundle.convert()
        except Exception as exc:  # a tool defect must not hide behind a readiness status
            raise RuntimeError(f'{spell_type} {name}: {exc}') from exc
        errors = validate(spell, deps, catalog, manifest)
        blockers = [f"{e['source_field']}: {e.get('resolution', e['status'])}" for e in manifest['entries']
                    if e['status'] in ('unsupported_source_field', 'unresolved_semantics', 'unresolved_dependency', 'partial_text')]
        invalid = [e for e in errors if not e.startswith('manifest: unresolved') and not e.startswith('manifest: custom script')]
        # A blocked bundle may carry placeholders for the unresolved values, so its schema errors follow the blockers.
        status = 'blocked' if blockers else 'invalid' if invalid else 'ready'
        files = {'spell.json': spell, 'dependencies.json': deps, 'catalog.json': catalog, 'manifest.json': manifest}
        data = b''.join(json.dumps(files[f], ensure_ascii=False, sort_keys=True).encode() for f in sorted(files))
        results.append({'spell_type': spell_type, 'name': name, 'sources': sorted(records), 'status': status,
                        'blockers': blockers, 'errors': invalid,
                        'bundle_sha256': hashlib.sha256(data).hexdigest()})
        if args.out:
            target = args.out / f'{spell_type}-{slug(name)}'
            target.mkdir(parents=True, exist_ok=True)
            for filename, value in files.items():
                (target / filename).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8',
                                               newline='\n')
    return results


def summarize(results):
    from collections import Counter
    status = Counter(r['status'] for r in results)
    reasons = Counter()
    for r in results:
        for b in r['blockers']:
            text = re.sub(r'\(.*', '', b)
            text = re.sub(r'custom script; needs a native behaviour \(S7\): .*', 'custom script (S7)', text)
            text = re.sub(r'(S4 conflict)[: ].*', r'\1', text)
            reasons[text[:90].strip()] += 1
    return {'spells': len(results), 'status': dict(sorted(status.items())),
            'top_blockers': dict(reasons.most_common(25))}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--out', type=Path, help='write every bundle under this directory')
    parser.add_argument('--only', nargs='*', help='spell names (lower case) to convert')
    parser.add_argument('--readiness', type=Path, help='write the readiness census here')
    args = parser.parse_args(argv)
    results = run(args)
    summary = summarize(results)
    if args.readiness:
        document = {'schema': 'OTERYN_SPELL_READINESS/v1', 'revision': REVISION,
                    'sources': {s: {k: v for k, v in c.items() if k != 'tag'} for s, c in SOURCES.items()},
                    'wiki_facts': [FANDOM_FACTS.name, BR_FACTS.name, TIBIOPEDIA_FACTS.name, TIBIACOM_LIST.name], 'official_changes': OFFICIAL.name,
                    'summary': summary, 'spells': results}
        ws.write_lines(args.readiness, document, None)
    print(json.dumps(summary, indent=1, ensure_ascii=False))
    return 0


if __name__ == '__main__':
    sys.exit(main())
