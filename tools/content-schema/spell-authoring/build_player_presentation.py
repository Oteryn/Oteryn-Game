"""Pinned presentation supplement; source calls are evidence, never automatic emission.

The existing authoring schema consumes cast/impact cues and Ability visual fields.
Conditional/direct/delayed Lua call sites stay reference-only until a runtime owner
binds their exact trigger. No sound waveform or proprietary asset is redistributed.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent
PINS = {'canary': '99902524e052f37574194466c2949c576e4ab269',
        'crystal': 'ff7ede593c69d4c658b382c97443e8155926924a'}
HEADERS = ['src/utils/utils_definitions.hpp', 'src/creatures/creatures_definitions.hpp',
           'src/lua/functions/core/game/lua_enums.cpp', 'src/creatures/combat/spells.hpp',
           'src/creatures/combat/spells.cpp', 'src/lua/functions/creatures/combat/spell_functions.cpp',
           'src/lua/functions/lua_functions_loader.hpp']
TOKEN = re.compile(r'\b(?:CONST_ME_[A-Z0-9_]+|CONST_ANI_[A-Z0-9_]+|SOUND_EFFECT_TYPE_[A-Z0-9_]+)\b')


def pinned(root, pin, path):
    committed = subprocess.check_output(['git', '-C', str(root), 'show', pin + ':' + path])
    local = root / path
    if local.exists() and local.read_bytes() != committed:
        raise ValueError('modified pinned source: ' + path)
    return committed


def aliases(value):
    result = set()
    if isinstance(value, dict):
        for child in value.values(): result.update(aliases(child))
    elif isinstance(value, list):
        for child in value: result.update(aliases(child))
    elif isinstance(value, str) and value.startswith(('canary.sound:', 'canary.appearance:')):
        result.add(value)
    return result


def constants(headers):
    values = {}
    for enum in ('MagicEffectClasses', 'ShootType_t', 'SoundEffect_t'):
        match = re.search(r'\benum\s+' + enum + r'\s*:[^{]+\{(.*?)\};', headers, re.S)
        if match is None:
            raise ValueError('missing cue enum: ' + enum)
        body = re.sub(r'//[^\n]*|/\*.*?\*/', '', match.group(1), flags=re.S)
        next_value = 0
        for entry in body.split(','):
            entry = entry.strip()
            if not entry: continue
            parsed = re.fullmatch(r'([A-Z][A-Z_0-9]*)(?:\s*=\s*(0x[0-9a-fA-F]+|[0-9]+|[A-Z][A-Z_0-9]*))?', entry)
            if parsed is None:
                raise ValueError('unresolved cue enum expression: ' + entry)
            name, explicit = parsed.groups()
            number = (values[explicit] if explicit in values else int(explicit, 0)) if explicit is not None else next_value
            values[name] = number
            next_value = number + 1
    registered = set(re.findall(r'registerEnumNamespace\(L,\s*soundNamespace,\s*SoundEffect_t::([A-Z0-9_]+)\s*\)', headers))
    return values, registered


def cue(token, values, registered):
    if token.startswith('SOUND_EFFECT_TYPE_'):
        member = token.removeprefix('SOUND_EFFECT_TYPE_')
        if member not in registered or member not in values:
            return {'source_constant': token, 'status': 'unregistered_lua_constant'}
        kind, alias, key = 'sound', 'canary.sound:' + member.lower(), member
    else:
        prefix = 'CONST_ME_' if token.startswith('CONST_ME_') else 'CONST_ANI_'
        kind = 'effect' if prefix == 'CONST_ME_' else 'projectile'
        alias = 'canary.appearance:' + ('effect/' if kind == 'effect' else 'missile/') + token.removeprefix(prefix).lower()
        key = token
        if key not in values or token == 'CONST_ANI_WEAPONTYPE':
            return {'source_constant': token, 'status': 'dynamic_or_unresolved_source_constant'}
    return {'source_constant': token, 'kind': kind, 'source_id': values[key],
            'source_alias': alias, 'status': ('source_noop' if values[key] == 0 else 'source_qualified')}


def build(sources, catalog_path, census_path):
    catalog_raw, census_raw = catalog_path.read_bytes(), census_path.read_bytes()
    catalog, census = json.loads(catalog_raw), json.loads(census_raw)
    if len(catalog['bundles']) != 246:
        raise ValueError('expected complete 246-definition catalog')
    indices, tables, proofs = {}, {}, []
    for source, pin in PINS.items():
        root = sources / source
        head = subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip()
        if head != pin: raise ValueError('unexpected source HEAD')
        headers = []
        for path in HEADERS:
            raw = pinned(root, pin, path)
            proofs.append({'source': source, 'revision': pin, 'path': path,
                           'sha256': hashlib.sha256(raw).hexdigest()})
            headers.append(raw.decode())
        tables[source] = constants('\n'.join(headers))
        indices[source] = {(r['spell_type'], r['name'].lower()): r for r in census[source]}
    rows, counts = [], Counter()
    for bundle in catalog['bundles']:
        spell = bundle['bundle']['spell']
        mapped = aliases({'spell': spell, 'dependencies': bundle['dependencies']})
        sources_rows = []
        for source, pin in PINS.items():
            record = indices[source].get((spell['carrier'], spell['name'].lower()))
            if record is None: continue
            raw = pinned(sources / source, pin, record['file'])
            text, calls, found = raw.decode(), [], {}
            for line_number, line in enumerate(text.splitlines(), 1):
                # Comments are deliberately excluded. Expressions/triggers are retained,
                # never executed or interpreted as an unconditional presentation event.
                code = line.split('--', 1)[0].strip()
                for token in TOKEN.findall(code):
                    item = cue(token, *tables[source])
                    item['present_in_authoring_catalog'] = item.get('source_alias') in mapped
                    found[token] = item
                if any(k in code for k in ('sendMagicEffect(', 'sendDistanceEffect(',
                       'sendSingleSoundEffect(', 'sendDoubleSoundEffect(', 'addEvent(')):
                    calls.append({'line': line_number, 'expression': code,
                                  'status': 'reference_only_trigger_not_bound',
                                  'constants': TOKEN.findall(code)})
            sources_rows.append({'source': source, 'revision': pin, 'path': record['file'],
                                'sha256': hashlib.sha256(raw).hexdigest(),
                                'cues': list(found.values()), 'call_sites': calls})
        primary = next((x for x in sources_rows if x['source'] == 'canary'), sources_rows[0])
        missing = [x for x in primary['cues'] if x['status'] == 'source_qualified'
                   and not x['present_in_authoring_catalog']]
        counts['definitions'] += 1
        counts['definitions_with_primary_source_cue_omissions'] += bool(missing)
        counts['primary_source_cue_omissions'] += len(missing)
        counts['definitions_with_reference_only_direct_or_delayed_calls'] += bool(primary['call_sites'])
        counts['definitions_without_explicit_primary_sound_constant'] += not any(
            x.get('kind') == 'sound' for x in primary['cues'])
        rows.append({'identity': spell['identity'], 'name': spell['name'], 'carrier': spell['carrier'],
                     'authoring_aliases': sorted(mapped), 'preferred_reference_source': primary['source'],
                     'sources': sources_rows,
                     'client_verification': 'not_verified_no_full_effect_or_audio_playback_test'})
    return {'schema': 'OTERYN_PLAYER_PRESENTATION_SOURCE_SUPPLEMENT/v1',
            'authority': 'reference_only_source_hypothesis_not_runtime_activation',
            'catalog_sha256': hashlib.sha256(catalog_raw).hexdigest(),
            'census_sha256': hashlib.sha256(census_raw).hexdigest(),
            'engine_proofs': proofs, 'summary': dict(counts), 'spells': rows,
            'limits': ['Absent explicit sound constants do not imply silence: Spell defaults cast to SPELL_OR_RUNE.',
                       'Dynamic helpers, branch selection and callback timing require runtime-owned binding.',
                       'Source numeric IDs are reference enum values, not new native IDs or verified client assets.',
                       'Preferred reference source is Canary when present, not proof of selected runtime execution. Crystal aliases retain Crystal provenance; numeric equivalence alone is not canonical binding.']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sources', type=Path, required=True)
    parser.add_argument('--catalog', type=Path, required=True)
    parser.add_argument('--census', type=Path, default=ROOT / 'samples/spell-census-canary-99902524-crystal-ff7ede5.json')
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    document = build(args.sources, args.catalog, args.census)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(document, sort_keys=True, indent=2) + '\n')
    print(json.dumps(document['summary'], sort_keys=True))

if __name__ == '__main__': main()
