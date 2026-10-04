#!/usr/bin/env python3
"""Prepare source-bound audio field patches; never rewrite baseline bundles.

Sound cue tokens retain donor numeric identities, not admitted audio assets.
Owner-accepted deterministic selection replaces donor registration-time melee RNG.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/monster-authoring'))
import canary_batch

PINS = {'opentibiabr/canary': ('canary', '47dfd51f45280a59a1d3e50ba7edd573d7234446'),
        'zimbadev/crystalserver': ('crystal', '00ce02a57ca5a12e48f32a3476e37471167e4c3f')}
STANDARD = frozenset(('melee', 'combat', 'drunk', 'speed', 'outfit', 'strength',
                      'firefield', 'energyfield', 'earthfield', 'poisonfield', 'condition',
                      'invisible', 'effect'))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_pinned_file(root, pin, relative):
    path = root / relative
    data = path.read_bytes()
    blob = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
    actual = subprocess.check_output(['git', 'rev-parse', pin + ':' + relative], cwd=root, text=True).strip()
    if actual != blob:
        raise ValueError('Pinned source bytes drifted: ' + relative)
    return path


def routing_correction(source, audio, loader_proof):
    if 'impact_cue' not in audio or source.get('explicit_impact'):
        return None
    return {'qualification': 'SOURCE_NON_GLOBAL_AUDIO_ROUTING_CORRECTION',
            'donor_call': 'spell:castSound(sounds.impact)',
            'native_binding': 'impact_cue',
            'loader_file': 'data/scripts/lib/register_monster_type.lua',
            'loader_sha256': loader_proof['loader_sha256'],
            'lines': loader_proof['impact_routing_lines']}


def deterministic_selection(identity, pin, candidates):
    if not candidates or len(candidates) != len(set(candidates)):
        raise ValueError('Sound choices must be nonempty and unique')
    digest = hashlib.sha256((identity + '\0' + pin).encode()).hexdigest()
    position = int(digest, 16) % len(candidates)
    return candidates[position], {'policy': 'SHA256_ACTOR_IDENTITY_NUL_SOURCE_PIN_MOD_ORDERED_CHOICES/v1',
                                 'sha256': digest, 'candidates': list(candidates), 'index': position,
                                 'donor_choice_time': 'monster_type_registration',
                                 'qualification': 'OWNER_ACCEPTED_NON_GLOBAL_SOURCE_SOUND_SELECTION'}


def sound_ids(text):
    body = re.search(r'enum\s+(?:class\s+)?SoundEffect_t[^\{]*\{([^}]+)', text)
    if not body:
        raise ValueError('Cannot identify donor SoundEffect_t enum')
    return {'SOUND_EFFECT_TYPE_' + name: int(value) for name, value in
            re.findall(r'\b([A-Z][A-Z_0-9]*)\s*=\s*(\d+)', body[1])}


class SoundDefaults:
    """Execute only the pinned donor's pure lookup functions in an isolated Lua VM."""
    def __init__(self, loader_path, enum_path):
        from lupa.luajit21 import LuaRuntime
        self.ids = sound_ids(enum_path.read_text())
        source = loader_path.read_text()
        start = source.index('local function loadcastSound(')
        end = source.index('\nfunction readSpell(', start)
        thresholds = '\n'.join(re.findall(r'^local (?:smallAreaRadius|superDrunkDuration)\s*=\s*\d+', source, re.M))
        self.lua = LuaRuntime(register_eval=False, register_builtins=False)
        self.lua.execute("os=nil;io=nil;package=nil;require=nil;dofile=nil;loadfile=nil;debug=nil;python=nil;"
                         "setmetatable(_G,{__index=function(_,k)return '@'..k end});"
                         "math.random=function(lo,hi)return math.min(__audio_choice,hi)end")
        self.derive = self.lua.execute(thresholds + '\n' + source[start:end] +
            '\nreturn function(row,distance) local mt={targetDistance=function() return distance end};'
            'return loadSpellSoundType(row,mt) end')
        self.proof = {'loader_sha256': sha(loader_path), 'enum_sha256': sha(enum_path),
                      'loader_line': source[:start].count('\n') + 1,
                      'default_function_line': source[:source.index('local function loadSpellSoundType(')].count('\n') + 1,
                      'impact_routing_lines': [i + 1 for i, line in enumerate(source.splitlines())
                                               if 'spell:castSound(sounds.impact)' in line]}

    def choices(self, row, distance):
        results = {'cast_cue': [], 'impact_cue': []}
        for draw in (1, 2, 3):
            self.lua.globals()['__audio_choice'] = draw
            value = self.derive(self.lua.table_from(row), distance)
            for field, output, explicit in (('cast_cue', 'cast', 'soundCast'), ('impact_cue', 'impact', 'impactCast')):
                token = str(row.get(explicit, value[output])).lstrip('@')
                if token not in self.ids:
                    raise ValueError('Unknown donor sound enum: ' + token)
                number = self.ids[token]
                if number and number not in results[field]:
                    results[field].append(number)
        return results


def choose_donor(entry, manifest, donors):
    candidates = []
    for row in manifest['entries']:
        path = row.get('source_file', '')
        n = row.get('source_index', 0)
        if '/monster/' not in path or not path.endswith('.lua') or type(n) is not int:
            continue
        if not 0 <= n < len(manifest['sources']):
            raise ValueError('Invalid source_index')
        repository = manifest['sources'][n].get('repository')
        if repository in PINS and (donors / PINS[repository][0] / path).is_file():
            candidates.append((n, path, repository))
    if not candidates:
        raise ValueError('No pinned monster source: ' + entry['monster'])
    n, path, repository = sorted(set(candidates))[0]
    root = donors / PINS[repository][0]
    return repository, root / path, path


def source_rows(value):
    return value if isinstance(value, list) else value.get('_list', []) if isinstance(value, dict) else []


def static_sound_rows(text, section):
    """Read flat source sound inputs without assigning unresolved quest intervals."""
    start = re.search(r'monster\.' + section + r'\s*=\s*\{', text)
    if not start:
        return []
    end = re.search(r'\nmonster\.\w+\s*=', text[start.end():])
    body = text[start.end():start.end() + end.start()] if end else text[start.end():]
    rows = []
    for flat in re.findall(r'\{([^{}]*)\}', body):
        name = re.search(r'\bname\s*=\s*["\']([^"\']+)["\']', flat)
        if not name:
            continue
        row = {'name': name[1]}
        for key in ('type', 'shootEffect', 'radius', 'length', 'spread', 'duration', 'soundCast', 'impactCast'):
            match = re.search(r'\b' + key + r'\s*=\s*(-?\d+|[A-Z_][A-Z_0-9]*)', flat)
            if match:
                row[key] = int(match[1]) if re.fullmatch(r'-?\d+', match[1]) else '@' + match[1]
        rows.append(row)
    return rows

def spell_registry(donors):
    registry = {}
    for repository, (folder, pin) in PINS.items():
        root = donors / folder
        paths = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', pin], cwd=root, text=True).splitlines()
        for relative in paths:
            if not relative.startswith(('data/scripts/spells/', 'data-otservbr-global/scripts/spells/', 'data-global/scripts/spells/')) or not relative.endswith('.lua'):
                continue
            path = root / relative
            if not path.is_file():
                continue  # A sparse reference cache never proves this script absent upstream.
            text = path.read_text()
            for variable, name in re.findall(r'(\w+):name\(["\']([^"\']+)["\']\)', text):
                if not re.search(r'\b' + re.escape(variable) + r'\s*=\s*(?:Spell|RuneSpell)\(', text):
                    continue
                key = 'canary:ability/spell/' + canary_batch.slug(name)
                registry.setdefault((repository, key), []).append((variable, relative))
    return registry


def registered_spell_audio(ability, repository, donors, enums, registry, dependencies):
    """Resolve exact registered spell name in the actual donor, never a filename alias."""
    key = ability['identity']['key']
    base = re.sub(r'/variant-\d+$', '', key)
    if base != key:
        parents = [p for p in dependencies['abilities'] if p['identity']['key'] == base
                   and any(r['key'] == key for r in p.get('variants', []))]
        if len(parents) != 1:
            return None
    options = registry.get((repository, base), [])
    if len(options) != 1:
        return None
    variable, relative = options[0]
    root = donors / PINS[repository][0]
    path = root / relative
    data = path.read_bytes()
    blob = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
    if subprocess.check_output(['git', 'rev-parse', PINS[repository][1] + ':' + relative], cwd=root, text=True).strip() != blob:
        raise ValueError('Registered spell source bytes drifted')
    text = data.decode()
    values = {}
    audio_calls = re.findall(r'\b' + re.escape(variable) + r':(?:castSound|impactSound)\(([^)]*)\)', text)
    for field, method in (('cast_cue', 'castSound'), ('impact_cue', 'impactSound')):
        symbols = re.findall(r'\b' + re.escape(variable) + ':' + method + r'\((SOUND_EFFECT_TYPE_[A-Z_0-9]+)\)', text)
        if len(set(symbols)) == 1 and symbols[0] in enums[repository].ids:
            number = enums[repository].ids[symbols[0]]
            if number:
                values[field] = f'{PINS[repository][0]}.sound:effect/{number}'
    proof = {'repository': repository, 'pin': PINS[repository][1], 'file': relative,
             'sha256': sha(path), 'registered_name_binding': ability['identity']['key'],
             'lines': [n + 1 for n, line in enumerate(text.splitlines())
                       if ':castSound(' in line or ':impactSound(' in line]}
    if values:
        if base != key:
            proof['status'] = 'VARIANT_SOUND_INHERITS_EXPLICIT_PARENT_CONFIGURATION'
            return {}, proof
        return values, proof
    # An empty source setter list is evidence of omission, not proof of Global silence.
    if not audio_calls and not re.search(r'(?:send\w*SoundEffect|COMBAT_PARAM_[A-Z_]*SOUND)', text):
        proof['status'] = 'REGISTERED_SOURCE_NO_AUDIO_CONFIGURATION'
        return {}, proof
    return None


def build(baseline, donors, output):
    baseline, donors, output = baseline.resolve(), donors.resolve(), output.resolve()
    if output == baseline or baseline in output.parents or output == ROOT or ROOT in output.parents:
        raise ValueError('Output must be outside repository and baseline')
    index_path = baseline / 'population-index.json'
    index = json.loads(index_path.read_text())
    enums = {}
    for repository, (folder, pin) in PINS.items():
        root = donors / folder
        if subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip() != pin:
            raise ValueError('Donor pin drift: ' + repository)
        loader = verify_pinned_file(root, pin, 'data/scripts/lib/register_monster_type.lua')
        header = verify_pinned_file(root, pin, 'src/creatures/creatures_definitions.hpp')
        enums[repository] = SoundDefaults(loader, header)
    patches, pending, selections, flags, receipts = [], [], [], {}, []
    cache = {}
    registry = spell_registry(donors)
    counts = Counter()
    for entry in index['monsters']:
        name = entry['monster']
        directory = baseline / 'bundles' / name
        monster = json.loads((directory / 'monster.json').read_text())
        dependencies = json.loads((directory / 'dependencies.json').read_text())
        manifest = json.loads((directory / 'manifest.json').read_text())
        repository, path, relative = choose_donor(entry, manifest, donors)
        root = donors / PINS[repository][0]
        data = path.read_bytes()
        blob = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
        if subprocess.check_output(['git', 'rev-parse', PINS[repository][1] + ':' + relative], cwd=root, text=True).strip() != blob:
            raise ValueError('Donor source bytes drifted: ' + relative)
        if path not in cache:
            errors = []
            _, value, _ = canary_batch.load_monster(path, errors)
            cache[path] = value, errors
        value, errors = cache[path]
        sound_rows = {section: source_rows(value.get(section)) for section in ('attacks', 'defenses')}
        if errors:
            for section in sound_rows:
                if not sound_rows[section]:
                    sound_rows[section] = static_sound_rows(data.decode(), section)
        proof = {'repository': repository, 'pin': PINS[repository][1], 'file': relative,
                 'sha256': hashlib.sha256(data).hexdigest(), 'lines': [], 'sound_rule': enums[repository].proof}
        actor_flags = {'SOURCE_AUDIO_CUE_CLIENT_EXECUTION_UNVERIFIED', 'DONOR_PERIODIC_AUDIO_NOT_CONFIGURED',
                       'DONOR_DEATH_AUDIO_DEFAULT_SILENCE', 'AUDIO_RUNTIME_UNVERIFIED'}
        if value.get('sounds'):
            raise ValueError('Unexpected explicit sounds table requires dedicated conversion')
        audio_by_key = {}
        choice_by_key = {}
        for section, label in (('attacks', 'attack'), ('defenses', 'defense')):
            for n, row in enumerate(sound_rows[section]):
                source_field = f'{section}[{n + 1}]'
                if row.get('name') not in STANDARD:
                    continue  # Registered instant spells do not use CombatSpell's default transfer.
                keys = [f"canary:ability/{name}/{label}-{n + 1}"]
                for mapped in manifest['entries']:
                    destination = mapped.get('destination', '')
                    if mapped.get('source_field') != source_field or not destination.startswith('/monster/behavior/' + section + '/'):
                        continue
                    try:
                        native_entry = monster['behavior'][section][int(destination.split('/')[4])]
                        keys.append(native_entry['ability']['key'])
                    except (IndexError, KeyError, ValueError):
                        pass
                choices = enums[repository].choices(row, value.get('flags', {}).get('targetDistance', 1))
                audio, policies = {}, {}
                for field, options in choices.items():
                    if not options:
                        continue  # Proven source SILENCE is not a fabricated sound cue.
                    chosen = options[0]
                    if len(options) > 1:
                        chosen, policy = deterministic_selection(monster['creature']['identity']['key'], PINS[repository][1], options)
                        policies[field] = policy
                    audio[field] = f'{PINS[repository][0]}.sound:effect/{chosen}'
                for key in set(keys):
                    audio_by_key[key] = (audio, source_field, 'impactCast' in row)
                    if policies:
                        choice_by_key[key] = policies
        changed = False
        declared_cues = {}
        sound_events = set()
        for n, ability in enumerate(dependencies['abilities']):
            key = ability['identity']['key']
            derived = audio_by_key.get(key)
            source = dict(proof)
            if derived:
                audio, field, explicit_impact = derived
                source['explicit_impact'] = explicit_impact
                correction = routing_correction(source, audio, enums[repository].proof)
                if correction:
                    source['routing_correction'] = correction
                    actor_flags.add('SOURCE_NON_GLOBAL_AUDIO_ROUTING_CORRECTION')
                    counts['default_impact_routing_corrections'] += 1
                source['source_field'] = field
                source['lines'] = sorted({e['source_line'] for e in manifest['entries']
                                         if e.get('source_field') == field and e.get('source_file') == relative})
            else:
                registered = registered_spell_audio(ability, repository, donors, enums, registry, dependencies)
                if not registered:
                    pending.append({'monster': name, 'ability': key,
                                    'reason': 'REGISTERED_SPELL_OR_DERIVED_CORE_AUDIO_REQUIRES_EXPLICIT_SOURCE_PROOF', 'architecture_issue': 162})
                    continue
                audio, source = registered
            if not audio:
                receipts.append({'monster': name, 'ability': key, 'source': source,
                                 'status': 'DONOR_SOUND_SILENCE_OR_NOT_CONFIGURED', 'global_audio_absence_asserted': False})
                counts[source.get('status', 'DEFAULT_SOURCE_SILENCE')] += 1
                continue
            old = ability.get('audio', {})
            if any(field in old and old[field] != cue for field, cue in audio.items()):
                raise ValueError('Refusing to overwrite existing sound cue: ' + key)
            new = dict(old, **audio)
            if new == old:
                continue
            patches.append({'monster': name, 'file': 'dependencies.json', 'pointer': f'/abilities/{n}/audio',
                            'expected_present': 'audio' in ability, 'expected_value': ability.get('audio'),
                            'value': new, 'source': source, 'reason': 'SOURCE_BOUND_DONOR_SOUND_IDS_NOT_ADMITTED_AUDIO_ASSETS'})
            changed = True
            for sound_field, cue in audio.items():
                declared_cues.setdefault(cue, source)
                sound_events.add((sound_field.replace('_cue', ''), cue))
            if key in choice_by_key:
                selections.append({'monster': name, 'ability': key, 'selections': choice_by_key[key], 'source': source})
                actor_flags.add('OWNER_ACCEPTED_NON_GLOBAL_SOURCE_SOUND_SELECTION')
        if declared_cues:
            old_events = monster['presentation']['audio']['event_bindings']
            existing_events = {(b['event'], b['cue_id']) for b in old_events}
            event_additions = [{'event': event, 'cue_id': cue, 'asset_binding': cue}
                               for event, cue in sorted(sound_events - existing_events)]
            if event_additions:
                patches.append({'monster': name, 'file': 'monster.json',
                                'pointer': '/presentation/audio/event_bindings', 'expected_present': True,
                                'expected_value': old_events, 'value': old_events + event_additions,
                                'source': proof, 'cue_sources': declared_cues,
                                'reason': 'SOURCE_AUDIO_EVENT_BINDINGS_ASSET_UNADMITTED_RUNTIME_UNVERIFIED'})
                counts['presentation_event_patches'] += 1
                counts['presentation_event_bindings'] += len(event_additions)
            catalog = json.loads((directory / 'catalog.json').read_text())
            old_assets = catalog.get('assets', [])
            additions = sorted(set(declared_cues) - set(old_assets))
            if additions:
                patches.append({'monster': name, 'file': 'catalog.json', 'pointer': '/assets',
                                'expected_present': 'assets' in catalog, 'expected_value': catalog.get('assets'),
                                'value': old_assets + additions, 'source': proof,
                                'reason': 'SOURCE_AUDIO_CUE_DECLARED_ASSET_UNADMITTED',
                                'cue_sources': {cue: declared_cues[cue] for cue in additions}})
                counts['catalog_declaration_patches'] += 1
                counts['source_cue_declarations'] += len(additions)
            actor_flags.add('SOURCE_AUDIO_CUE_DECLARED_ASSET_UNADMITTED')
        if errors:
            actor_flags.add('PARTIAL_DONOR_DECLARATION_AUDIO_SOURCE_RETAINED')
        if manifest['sources'][0].get('kind') == 'mediawiki':
            actor_flags.add('WIKI_AUTHORED_TEMPLATE_AUDIO_SOURCE_NON_GLOBAL')
        if changed:
            counts['actors_audio_filled'] += 1
        if any(p['monster'] == name for p in pending):
            actor_flags.add('REGISTERED_OR_DERIVED_ABILITY_AUDIO_SOURCE_PENDING')
        flags[name] = sorted(actor_flags)
        receipts.append({'monster': name, 'source': proof, 'donor_periodic_audio': 'NOT_CONFIGURED',
                         'donor_death_audio': 'DEFAULT_SILENCE', 'global_audio_absence_asserted': False})
    counts.update(actors=len(index['monsters']), ability_audio_patches=sum(p['file'] == 'dependencies.json' for p in patches),
                  total_field_patches=len(patches),
                  type_registration_selections=len(selections), pending_abilities=len(pending))
    packet = {'schema': 'OTERYN_MONSTER_FIELD_PATCH/v1', 'lane': 'audio', 'baseline_index_sha256': sha(index_path),
              'patches': patches, 'actor_flags': flags, 'counts': dict(counts),
              'sound_selection_policy': 'OWNER_ACCEPTED_NON_GLOBAL_SOURCE_SOUND_SELECTION',
              'type_registration_selections': selections, 'remaining': pending, 'source_absence_receipts': receipts,
              'runtime_qualified': False, 'audio_assets_admitted': False}
    output.mkdir(parents=True, exist_ok=True)
    (output / 'field-patches.json').write_text(json.dumps(packet, sort_keys=True, indent=2) + '\n')
    print(json.dumps({'output': str(output), 'counts': dict(counts)}))
    return packet


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--donors', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    build(args.baseline, args.donors, args.output)


if __name__ == '__main__':
    main()
