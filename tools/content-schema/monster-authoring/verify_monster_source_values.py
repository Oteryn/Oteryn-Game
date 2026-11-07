"""Independent pinned Lua-table coverage; UNKNOWN is never a parity/readiness proof."""
import argparse
import collections
import hashlib
import json
import re
import subprocess
from pathlib import Path
from fractions import Fraction

PINS = {'canary': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
        'crystal': '00ce02a57ca5a12e48f32a3476e37471167e4c3f'}
# Independent identity/accepted simple normalizations. A mapped parent never proves
# a transformation; complex spell/script paths remain UNKNOWN until separately checked.
DIRECT = {'description': 'creature/inspection/description', 'experience': 'creature/stats/experience',
          'health': 'creature/stats/initial_health', 'maxHealth': 'creature/stats/max_health',
          'speed': 'creature/stats/speed', 'manaCost': 'creature/summoning/mana_cost', 'defenses.armor': 'creature/stats/armor',
          'defenses.defense': 'creature/stats/defense', 'Bestiary.CharmsPoints': 'creature/bestiary/charm_points',
          'Bestiary.Stars': 'creature/bestiary/stars', 'Bestiary.Locations': 'creature/bestiary/locations',
          'flags.attackable': 'creature/flags/attackable', 'flags.illusionable': 'creature/flags/illusionable',
          'flags.healthHidden': 'creature/flags/health_hidden', 'flags.summonable': 'creature/summoning/summonable',
          'flags.convinceable': 'creature/summoning/convinceable', 'flags.familiar': 'creature/summoning/is_familiar',
          'flags.hostile': 'behavior/targeting/hostile', 'flags.canTarget': 'behavior/targeting/can_target',
          'flags.targetDistance': 'behavior/targeting/target_distance_tiles',
          'flags.runHealth': 'behavior/targeting/flee_health', 'flags.staticAttackChance': 'behavior/targeting/static_attack_chance_percent',
          'flags.pushable': 'behavior/movement/pushable', 'flags.canPushItems': 'behavior/movement/push_items',
          'flags.canPushCreatures': 'behavior/movement/push_creatures', 'flags.canWalk': 'behavior/movement/can_walk',
          'flags.canWalkOnEnergy': 'behavior/movement/field_permissions/energy',
          'flags.canWalkOnFire': 'behavior/movement/field_permissions/fire', 'flags.canWalkOnPoison': 'behavior/movement/field_permissions/poison',
          'changeTarget.interval': 'behavior/targeting/change_target/interval_ms',
          'changeTarget.chance': 'behavior/targeting/change_target/chance_percent',
          'voices.interval': 'behavior/voices/interval_ms', 'voices.chance': 'behavior/voices/chance_percent',
          'summon.maxSummons': 'behavior/summons/max_summons',
          'light.level': 'presentation/light/level',
          'flags.isPreyable': 'creature/system_eligibility/prey',
          'flags.isPreyExclusive': 'creature/system_eligibility/exclusive_prey',
          'flags.isForgeCreature': 'creature/system_eligibility/forge',
          'flags.rewardBoss': 'creature/system_eligibility/reward_boss',
          'flags.isBlockable': 'creature/spawn_eligibility/blocked_by_nearby_players',
          'flags.critChance': 'creature/stats/critical_chance_percent',
          'strategiesTarget.nearest': 'behavior/targeting/strategy_weights/nearest',
          'strategiesTarget.health': 'behavior/targeting/strategy_weights/health',
          'strategiesTarget.damage': 'behavior/targeting/strategy_weights/damage',
          'strategiesTarget.random': 'behavior/targeting/strategy_weights/random',
          'Bestiary.class': 'creature/bestiary/class',
          'Bestiary.FirstUnlock': 'creature/bestiary/kill_thresholds/0',
          'Bestiary.SecondUnlock': 'creature/bestiary/kill_thresholds/1',
          'Bestiary.toKill': 'creature/bestiary/kill_thresholds/2'}
# Real donor taxonomy labels, excluding sentinel NONE/FIRST/LAST values.
TAXONOMY_LABELS = frozenset('AMPHIBIC AQUATIC BIRD CONSTRUCT DEMON DRAGON ELEMENTAL EXTRA_DIMENSIONAL FEY GIANT HUMAN HUMANOID LYCANTHROPE MAGICAL MAMMAL PLANT REPTILE SLIME UNDEAD VERMIN'.split())
NORMALIZED = {'defenses.mitigation': 'creature/stats/mitigation_percent',
              'Bestiary.Occurrence': 'creature/bestiary/occurrence',
              'Bestiary.race': 'creature/bestiary/taxonomy',
              'light.color': 'presentation/light/color_binding'}


def normalized_expected(path, value, raw):
    if path == 'defenses.mitigation' and isinstance(value, (int, float)) and not isinstance(value, bool):
        number = Fraction(str(value))
        return {'numerator': number.numerator, 'denominator': number.denominator}, 'D2_EXACT_DECIMAL_RATIO'
    if path == 'Bestiary.Occurrence' and type(value) is int and 0 <= value <= 3:
        return ('common', 'uncommon', 'rare', 'very_rare')[value], 'REGISTERED_BESTIARY_OCCURRENCE_ENUM'
    if path == 'Bestiary.race' and isinstance(value, str):
        label = value.removeprefix('@BESTY_RACE_')
        if value.startswith('@BESTY_RACE_') and label in TAXONOMY_LABELS:
            return label.lower(), 'REGISTERED_BESTIARY_TAXONOMY_LABEL'
    if path == 'light.color' and type(value) is int and 0 <= value <= 255:
        if type(raw.get('light.level')) is int and raw['light.level'] > 0:
            return 'canary.appearance:light-color/' + str(value), 'SOURCE_LIGHT_COLOR_BINDING_ENCODING'
    return None, None


def sha(data):
    return hashlib.sha256(data).hexdigest()


def pointer(value, path):
    if not path.startswith('/'):
        raise ValueError('absolute JSON pointer required')
    for token in path[1:].split('/'):
        token = token.replace('~1', '/').replace('~0', '~')
        value = value[int(token)] if isinstance(value, list) else value[token]
    return value


def leaves(value, path=''):
    if not hasattr(value, 'items'):
        yield path, value if isinstance(value, (str, int, float, bool, type(None))) else {'lua_type': type(value).__name__, 'semantics': 'UNKNOWN'}
        return
    entries = sorted(value.items(), key=lambda pair: str(pair[0]))
    if not entries:
        yield path, {}
    for key, child in entries:
        child_path = path + (f'[{key}]' if isinstance(key, int) else ('.' if path else '') + key)
        yield from leaves(child, child_path)


def evaluate(text):
    # Separate evaluator: no converter parser, transforms, defaults or Lua API recorder.
    from lupa.luajit21 import LuaRuntime
    lua = LuaRuntime(unpack_returned_tuples=True, register_eval=False, register_builtins=False)
    captured = lua.execute('''
      jit.off(); local rows = {}; local instructions = 0
      debug.sethook(function() instructions=instructions+10000; if instructions>1000000 then error("instruction bound") end end,"",10000)
      io=nil; os=nil; package=nil; require=nil; dofile=nil; loadfile=nil; python=nil
      setmetatable(_G,{__index=function(_,key) return "@"..key end})
      local function snapshot(value,seen)
        if type(value)~="table" then return value end
        seen=seen or {}; if seen[value] then error("cyclic registered table") end
        seen[value]=true; local copy={}
        for key,child in pairs(value) do copy[key]=snapshot(child,seen) end
        seen[value]=nil; return copy
      end
      Game={createMonsterType=function(name)
        return {register=function(_,mask) rows[#rows+1]={name=name,mask=snapshot(mask)} end}
      end}; return rows
    ''')
    error = None
    try:
        lua.execute(text)
    except Exception as exc:
        error = str(exc).splitlines()[0][:240]
    rows = [{'name': row['name'], 'leaves': dict(leaves(row['mask']))} for _, row in captured.items()]
    return rows, error


def source_lines(text, path):
    # Lexical locator, not invented exact AST positions. All candidate lines are retained.
    root = path.split('.')[0].split('[')[0]
    assignment = re.search(r'\b\w+\.' + re.escape(root) + r'\s*=', text)
    if not assignment:
        return []
    end = re.search(r'\n\s*\w+\.\w+\s*=', text[assignment.end():])
    stop = assignment.end() + end.start() if end else len(text)
    key = path.rsplit('.', 1)[-1].split('[')[0]
    matches = list(re.finditer(r'\b' + re.escape(key) + r'\s*=', text[assignment.start():stop]))
    return sorted({text[:assignment.start()+m.start()].count('\n')+1 for m in matches}) or [text[:assignment.start()].count('\n')+1]


def covers(parent, child):
    return child == parent or child.startswith(parent + '.') or child.startswith(parent + '[')


def approved_no_effect(path, value, monster):
    """Bounded source dispositions, independent of converter explanations."""
    creature, behavior = monster.get('creature', {}), monster.get('behavior', {})
    if path == 'corpse' and type(value) is int and value == 0:
        return 'corpse_item' not in creature, 'CREATURE_GET_CORPSE_ZERO_RETURNS_NO_ITEM'
    if value == {} and isinstance(value, dict):
        if path == 'loot':
            return 'loot' not in creature and 'loot' not in monster, 'REGISTERED_EMPTY_LOOT_ADDS_NO_ENTRIES'
        if path == 'attacks':
            return behavior.get('attacks') == [], 'REGISTERED_EMPTY_ATTACKS_ADDS_NO_ABILITIES'
        if path == 'voices':
            return 'voices' not in behavior, 'REGISTERED_EMPTY_VOICES_ADDS_NO_UTTERANCES'
        if path == 'events':
            return behavior.get('event_bindings') == [], 'REGISTERED_EMPTY_EVENTS_ADDS_NO_HANDLER_NAMES'
        if path == 'summon':
            return 'summons' not in behavior, 'REGISTERED_EMPTY_SUMMON_ADDS_NO_ENTRIES'
        if path == 'summons':
            return True, 'REGISTRAR_READS_SUMMON_NOT_OBSOLETE_SUMMONS'
    if path == 'maxSummons' and type(value) is int and value == 0:
        return True, 'REGISTRAR_READS_SUMMON_MAXSUMMONS_NOT_TOP_LEVEL'
    return None, None


def check_values(raw, monster, manifest, text, source_file, source_index):
    entries = manifest.get('entries', [])
    source_entries = [e for e in entries if e.get('source_index') == source_index and e.get('source_file') == source_file]
    document = {'monster': monster}
    rows, errors = [], []
    for path, value in raw.items():
        claimed = [e for e in source_entries if covers(e.get('source_field', ''), path)]
        if re.fullmatch(r'events\[\d+\]', path) and isinstance(value, str):
            claimed += [e for e in source_entries if e.get('source_field') == 'events=' + value]
        target = DIRECT.get(path)
        expected, normalization = value, None
        if path in NORMALIZED:
            expected, normalization = normalized_expected(path, value, raw)
            if normalization:
                target = NORMALIZED[path]
        if path == 'name':
            target = 'creature/display_name'
        status, actual = 'UNKNOWN_NORMALIZATION', None
        if path in ('raceId', 'bosstiary.bossRaceId'):
            status = 'SOURCE_METADATA_ONLY'
        elif not claimed:
            status = 'UNACCOUNTED_SOURCE_FIELD'
        elif any(e.get('status') == 'approved_omission' for e in claimed):
            status = 'UNKNOWN_DOCUMENTED_SOURCE_OMISSION'
        omissions = [e for e in claimed if e.get('source_field') == path and e.get('status') == 'approved_omission' and not e.get('destination')]
        if omissions:
            no_effect, disposition = approved_no_effect(path, value, monster)
            if no_effect is not None:
                normalization = disposition
                if no_effect:
                    status = 'VERIFIED_APPROVED_NO_EFFECT'
                elif path == 'loot' and any(e.get('source_index') != source_index and e.get('status') == 'mapped' and
                     e.get('destination', '').startswith('/monster/loot/') for e in entries):
                    status = 'UNKNOWN_DOCUMENTED_SOURCE_OVERRIDE'
                else:
                    status = 'SOURCE_NO_EFFECT_MISMATCH'
                    errors.append(path + ': ' + status)
        if path == 'light.color' and raw.get('light.level') == 0:
            status = 'NOT_APPLICABLE_IN_SOURCE'; target = None
        if path == 'manaCost' and raw.get('flags.summonable') is False and raw.get('flags.convinceable') is False:
            status = 'NOT_APPLICABLE_IN_SOURCE'; target = None
        if target:
            try:
                actual = pointer(document, '/monster/' + target)
                equal = type(expected) is type(actual) and expected == actual
                if normalization == 'D2_EXACT_DECIMAL_RATIO':
                    equal = equal and isinstance(actual, dict) and all(type(actual.get(k)) is int for k in ('numerator', 'denominator'))
                if isinstance(expected, (int, float)) and not isinstance(expected, bool):
                    equal = not isinstance(actual, bool) and expected == actual
                status = ('VERIFIED_ACCEPTED_NORMALIZATION' if normalization else 'VERIFIED_VALUE_PRESERVED') if equal else 'SOURCE_VALUE_MISMATCH'
            except (KeyError, IndexError, TypeError, ValueError):
                status = 'SOURCE_VALUE_DROPPED'
            # Overrides must bind the exact destination and an explicit other source.
            overrides = [e for e in entries if e.get('destination') == '/monster/' + target and
                         e.get('source_index') != source_index and e.get('status') == 'mapped']
            if status == 'SOURCE_VALUE_MISMATCH':
                overrides += [e for e in entries if e.get('source_index') != source_index and e.get('status') == 'mapped' and
                              ('/monster/' + target).startswith(e.get('destination', '') + '/')]
                omissions = [e for e in claimed if e.get('source_field') == path and e.get('status') == 'approved_omission']
                if overrides or omissions:
                    status = 'UNKNOWN_DOCUMENTED_SOURCE_OVERRIDE'
            if status == 'SOURCE_VALUE_DROPPED' and path == 'manaCost':
                summoning = monster.get('creature', {}).get('summoning', {})
                if summoning.get('summonable') is False and summoning.get('convinceable') is False:
                    if any(e.get('source_field') == path and e.get('status') == 'approved_omission' for e in claimed):
                        status = 'UNKNOWN_DOCUMENTED_SOURCE_OVERRIDE'
                    elif 'flags.summonable' not in raw and 'flags.convinceable' not in raw:
                        status = 'UNKNOWN_SOURCE_DEFAULT_NORMALIZATION'
            if status == 'SOURCE_VALUE_DROPPED' and path in ('voices.chance', 'voices.interval'):
                if not any(k.startswith('voices[') and k.endswith('.text') and v for k,v in raw.items()):
                    status = 'NOT_APPLICABLE_IN_SOURCE'
            if status == 'SOURCE_VALUE_DROPPED' and path == 'Bestiary.Locations' and value == '':
                status = 'SOURCE_EMPTY_TEXT'
            if path == 'flags.runHealth' and status == 'SOURCE_VALUE_MISMATCH' and isinstance(value, int):
                if isinstance(raw.get('maxHealth'), int) and actual == min(value, raw['maxHealth']):
                    status = 'UNKNOWN_DOCUMENTED_THRESHOLD_NORMALIZATION'
            if status in ('SOURCE_VALUE_MISMATCH', 'SOURCE_VALUE_DROPPED'):
                errors.append(path + ': ' + status)
        derived = None
        if path == 'Bestiary.Stars' and type(value) is int and 0 <= value <= 5:
            destination = '/monster/creature/bestiary/difficulty'
            label = ('harmless', 'trivial', 'easy', 'medium', 'hard', 'challenging')[value]
            try:
                difficulty = pointer(document, destination)
                derived_status = 'VERIFIED_ACCEPTED_NORMALIZATION' if difficulty == label else 'SOURCE_DERIVED_VALUE_MISMATCH'
            except (KeyError, IndexError, TypeError, ValueError):
                difficulty = None; derived_status = 'SOURCE_DERIVED_VALUE_DROPPED'
            if derived_status == 'SOURCE_DERIVED_VALUE_MISMATCH' and any(e.get('status') == 'mapped' and
                e.get('source_index') != source_index and e.get('destination') == destination for e in entries):
                derived_status = 'UNKNOWN_DOCUMENTED_DERIVATION_OVERRIDE'
            if derived_status.startswith('SOURCE_DERIVED_VALUE_'):
                errors.append(path + ': ' + derived_status)
            derived = {'destination': destination, 'expected_value': label, 'destination_value': difficulty,
                       'status': derived_status, 'normalization_rule': 'ACCEPTED_BESTIARY_STARS_DIFFICULTY_ENUM'}
        rows.append({'source_field': path, 'source_lines': source_lines(text, path), 'source_value': value,
                     'status': status, 'destination': '/monster/' + target if target else None,
                     'destination_value': actual, 'normalization_rule': normalization,
                     'expected_value': expected if normalization else None, 'derived_value_proof': derived, 'manifest_claims': [e.get('destination') for e in claimed]})
    flags = {k: v for k, v in raw.items() if k.startswith('flags.')}
    mana_state = 'NOT_APPLICABLE_IN_SOURCE' if flags.get('flags.summonable') is False and flags.get('flags.convinceable') is False else 'SOURCE_UNSPECIFIED'
    for path in ('defenses.mitigation', 'Bestiary', 'manaCost'):
        if not any(covers(path, present) for present in raw):
            rows.append({'source_field': path, 'source_lines': [], 'status': mana_state if path == 'manaCost' else 'SOURCE_UNSPECIFIED'})
    return rows, errors


def check_manifest(monster, deps, manifest):
    errors = []
    for entry in manifest.get('entries', []):
        if entry.get('status') not in ('mapped', 'resolved_native_behavior'):
            continue
        try:
            if pointer({'monster': monster, 'dependencies': deps}, entry['destination']) is None:
                raise ValueError('null')
        except (KeyError, IndexError, TypeError, ValueError):
            errors.append('missing manifest destination: ' + str(entry.get('destination')))
    return errors


def registered_calls(text):
    # Every lexical colon call gets an explicit UNKNOWN; comments/strings are masked.
    masked = re.sub(r'--\[\[.*?\]\]|--[^\n]*|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'',
                    lambda m: ''.join('\n' if ch == '\n' else ' ' for ch in m.group()), text, flags=re.S)
    types = set(re.findall(r'\b(\w+)\s*=\s*Game\.createMonsterType\s*\(', masked))
    rows = [{'receiver': m.group(1), 'method': m.group(2), 'line': masked[:m.start()].count('\n')+1,
             'status': 'REGISTRATION_CAPTURED' if m.group(1) in types and m.group(2) == 'register' else 'UNKNOWN_CALL_SEMANTICS'}
            for m in re.finditer(r'\b(\w+)\s*:\s*(\w+)\s*\(', masked)]
    for m in re.finditer(r'(?<![:.\w])((?:\w+\.)*\w+)\s*\(', masked):
        if m.group(1) not in ('if', 'while', 'for', 'function'):
            rows.append({'receiver': None, 'method': m.group(1), 'line': masked[:m.start()].count('\n')+1,
                         'status': 'UNKNOWN_LEXICAL_GLOBAL_OR_DOT_CALL'})
    return rows


def audit(index, bundles, sources):
    report = {'schema': 'OTERYN_INDEPENDENT_MONSTER_SOURCE_COVERAGE/v1', 'sources': {}, 'monsters': [],
              'scope': 'Registered Lua table leaves and lexical calls; independent direct values and bounded accepted ratio, Bestiary enum and active-light encoding checks. No arbitrary Lua, registrar branch, override correctness, asset existence or runtime parity proof.',
              'passed': False, 'runtime_qualified': False}
    for label, root in sources.items():
        revision = subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip()
        if revision != PINS[label]:
            raise ValueError(label + ': unpinned checkout')
        subprocess.run(['git', '-C', str(root), 'diff', '--quiet', 'HEAD', '--'], check=True)
        report['sources'][label] = {'revision': revision, 'worktree_clean': True}
    for item in index['monsters']:
        name = item['monster']; directory = bundles / name
        monster = json.loads((directory / 'monster.json').read_text())
        deps = json.loads((directory / 'dependencies.json').read_text())
        manifest = json.loads((directory / 'manifest.json').read_text())
        result = {'monster': name, 'errors': check_manifest(monster, deps, manifest), 'source_rows': [], 'calls': []}
        digest = hashlib.sha256()
        for file in ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json'):
            data = (directory / file).read_bytes()
            digest.update(f'{file}\0{len(data)}\0'.encode('ascii') + data)
        result['bundle_sha256'] = digest.hexdigest()
        if result['bundle_sha256'] != item['sha256']:
            result['errors'].append('bundle digest does not match index')
        source_index = next((i for i, s in enumerate(manifest['sources']) if s.get('repository') in ('opentibiabr/canary', 'zimbadev/crystalserver')), None)
        if source_index is None or item.get('binding', {}).get('identity_namespace', '').startswith('mediawiki'):
            result['status'] = 'UNKNOWN_NON_LUA_SOURCE'
        else:
            source = manifest['sources'][source_index]
            label = 'canary' if source['repository'] == 'opentibiabr/canary' else 'crystal'
            if source['revision'] != PINS[label]:
                raise ValueError(name + ': manifest revision mismatch')
            file = item.get('binding', {}).get('external_id') or 'data-otservbr-global/monster/' + item['file'] + '.lua'
            root = sources[label].resolve(); path = (root / file).resolve()
            if not path.is_relative_to(root):
                raise ValueError('source path escapes checkout')
            data = path.read_bytes(); text = data.decode('utf-8')
            result.update({'source_file': file, 'source_revision': source['revision'], 'source_sha256': sha(data)})
            tables, error = evaluate(text)
            result['evaluation_error'] = error
            result['calls'] = registered_calls(text)
            if len(tables) != 1:
                result['errors'].append('expected one registered table; observed ' + str(len(tables)))
            else:
                result['registration_name'] = tables[0]['name']
                result['source_rows'], failures = check_values(tables[0]['leaves'], monster, manifest, text, file, source_index)
                result['errors'].extend(failures)
            result['status'] = 'INCONSISTENT' if result['errors'] else 'UNKNOWN_LUA_CONTINUATION' if error else 'UNKNOWN_FULL_SEMANTIC_COVERAGE'
        report['monsters'].append(result)
    used = {(m.get('source_revision'), m.get('source_file')) for m in report['monsters']}
    report['unprepared_source_files'] = []
    for label, root in sources.items():
        subtree = 'data-otservbr-global/monster' if label == 'canary' else 'data-global/monster'
        for path in sorted((root / subtree).rglob('*.lua')):
            file = str(path.relative_to(root))
            if (PINS[label], file) in used:
                continue
            data = path.read_bytes(); text = data.decode('utf-8'); tables, error = evaluate(text)
            report['unprepared_source_files'].append({'source_file': file, 'source_revision': PINS[label],
                'source_sha256': sha(data), 'evaluation_error': error, 'calls': registered_calls(text),
                'registrations': [{'name': table['name'], 'source_rows': [
                    {'source_field': key, 'source_value': value, 'source_lines': source_lines(text, key),
                     'status': 'UNKNOWN_NO_PREPARED_BUNDLE'} for key,value in table['leaves'].items()]} for table in tables],
                'status': 'UNKNOWN_NO_PREPARED_BUNDLE'})
    report['status_counts'] = dict(collections.Counter(r['status'] for r in report['monsters']))
    report['leaf_status_counts'] = dict(collections.Counter(r['status'] for m in report['monsters'] for r in m['source_rows']))
    report['derived_value_status_counts'] = dict(collections.Counter(r['derived_value_proof']['status'] for m in report['monsters']
        for r in m['source_rows'] if r.get('derived_value_proof')))
    report['consistency_passed'] = not any(m['errors'] for m in report['monsters'])
    report['coverage_status'] = 'UNKNOWN'
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ('index', 'bundles', 'canary', 'crystal', 'out'):
        parser.add_argument('--' + arg, required=True, type=Path)
    parser.add_argument('--require-complete', action='store_true', help='fail on UNKNOWN coverage, not only inconsistencies')
    args = parser.parse_args()
    result = audit(json.loads(args.index.read_text()), args.bundles, {'canary': args.canary, 'crystal': args.crystal})
    result['input_index_sha256'] = sha(args.index.read_bytes())
    result['checker_sha256'] = sha(Path(__file__).read_bytes())
    contract = Path(__file__).resolve().parents[3] / 'docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md'
    result['normalization_contract'] = {'path': str(contract.relative_to(contract.parents[2])),
                                        'sha256': sha(contract.read_bytes()), 'runtime_qualified': False}
    args.out.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({k: v for k, v in result.items() if k not in ('monsters', 'sources', 'unprepared_source_files')}))
    raise SystemExit(not result['consistency_passed'] or args.require_complete)


if __name__ == '__main__':
    main()
