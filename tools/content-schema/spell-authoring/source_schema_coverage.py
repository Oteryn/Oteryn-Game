"""Offline source-first player registrar capture and honest schema/value gap audit.

No wiki imports, engine execution, registration re-evaluation or network access.
Explicit registrar calls are preserved independently for each immutable source.
Matching current authoring data does not establish execution/mechanics parity.
"""
import argparse
from collections import Counter
import hashlib
import gzip
import io
import importlib.util
import re
import json
from pathlib import Path
import subprocess

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
AUDIT = ROOT / 'docs/reference/spells/r22-audit/source-completeness'
SOURCES = ('canary', 'crystal', 'canary-main-current', 'crystal-summer-current')
CUES = HERE.parent / 'native-gameplay/spell-cues.json'
CUE_VALUES = {r['alias']: r['value'] for r in json.loads(CUES.read_text())['records'] if r['kind'] == 'sound'}
CLASSES = {'registered_player_spell_or_rune', 'disabled_example_or_test_fixture'}
# Paths are JSON pointers in the existing spell.schema.json authoring record.
DIRECT = {
    'name': '/name', 'words': '/words', 'id': '/reference_spell_id',
    'level': '/requirements/level', 'mana': '/costs/mana', 'soul': '/costs/soul',
    'isPremium': '/requirements/premium', 'needLearn': '/requirements/learning_required',
    'cooldown': '/cooldown_ms', 'basePower': '/base_power',
    'harmony': '/harmony_cost', 'stance': '/stance_slot',
    'isAggressive': '/targeting/aggressive', 'isSelfTarget': '/targeting/self_target',
    'needTarget': '/targeting/needs_target', 'needDirection': '/targeting/needs_direction',
    'needCasterTargetOrDirection': '/targeting/target_or_direction',
    'blockWalls': '/targeting/block_walls', 'isBlockingWalls': '/targeting/block_walls',
    'allowOnSelf': '/targeting/allow_on_self', 'range': '/targeting/range_tiles',
    'needPosition': '/targeting/cast_at_position', 'optionalTarget': '/targeting/cast_at_position',
    'setPzLocked': '/pz_locks_caster', 'needWeapon': '/needs_weapon',
    'charges': '/rune/charges', 'magicLevel': '/rune/magic_level',
    'allowFarUse': '/rune/allow_far_use',
}
TRANSFORMED = {
    'group': '/groups', 'groupCooldown': '/groups', 'vocation': '/requirements/vocations',
    'runeId': '/rune/item', 'isBlocking': '/rune/blocking',
    'hasParams': '/targeting/parameter', 'hasPlayerNameParam': '/targeting/parameter',
    'castSound': '/presentation/cast_cue', 'impactSound': '/presentation/impact_cue',
    'monkSpellType': '/harmony_role', 'element': '/elemental_cast_type',
}
# These need distinct semantics, not arbitrary fields in native parameters.
GAPS = {}  # New typed candidate header fields represent these registrar semantics.


def digest(data):
    return hashlib.sha256(data).hexdigest()


def pointer(value, path):
    for part in path.lstrip('/').split('/'):
        if not isinstance(value, dict) or part not in value:
            return False, None
        value = value[part]
    return True, value


def schema_pointer(schema, path):
    """Resolve local defs without pretending external definitions were checked."""
    node = schema['$defs']['spell']
    for part in path.lstrip('/').split('/'):
        while '$ref' in node and node['$ref'].startswith('#/$defs/'):
            node = schema['$defs'][node['$ref'].split('/')[-1]]
        node = node.get('properties', {}).get(part)
        if node is None:
            return False
    return True



VOCATIONS = {name: name.replace(' ', '_') for name in (
    'none', 'druid', 'elder druid', 'sorcerer', 'master sorcerer', 'knight', 'elite knight',
    'paladin', 'royal paladin', 'monk', 'exalted monk')}


def transformed(name, registrar, sounds):
    """Return full normalized expected value, with unmapped semantics explicit."""
    value = registrar[name]
    if name in ('group', 'groupCooldown'):
        groups = registrar.get('group')
        cooldowns = registrar.get('groupCooldown')
        if groups is None or cooldowns is None:
            raise ValueError('group pair incomplete; no engine defaults inferred')
        groups = groups if isinstance(groups, list) else [groups]
        cooldowns = cooldowns if isinstance(cooldowns, list) else [cooldowns]
        if len(groups) != len(cooldowns):
            raise ValueError('group arity mismatch; secondary cooldown omitted in source')
        if any(not re.fullmatch('[a-z][a-z0-9_]*', g) for g in groups):
            raise ValueError('unknown group syntax')
        return [{'group': g, 'cooldown_ms': cd} for g, cd in zip(groups, cooldowns)], {}
    if name == 'vocation':
        entries = []
        for v in value:
            parts = v.split(';')
            if len(parts) > 2 or parts[0] not in VOCATIONS:
                raise ValueError('unknown vocation or source argument syntax')
            entries.append({'vocation': VOCATIONS[parts[0]],
                            'show_in_description': len(parts) == 2 and parts[1] == 'true'})
        if len({v['vocation'] for v in entries}) != len(entries):
            raise ValueError('duplicate vocation overwrite requires ordered call proof')
        return sorted(v['vocation'] for v in entries), {'unrepresented_source_semantics': entries}
    if name == 'runeId':
        return {'family': 'Item', 'key': 'candidate:item/' + str(value)}, {'revision': 'owning item provider, not source numeric id'}
    if name == 'isBlocking':
        values = value if isinstance(value, list) else [value]
        return {'solid': values[0], 'creature': values[1] if len(values) == 2 else False}, {
            'default_rule': 'luaSpellBlocking reads omitted argument 3 through Lua::getBoolean -> false'}
    if name in ('hasParams', 'hasPlayerNameParam'):
        mode = ('player_name' if registrar.get('hasPlayerNameParam', False) else 'text') if registrar.get('hasParams', False) else 'none'
        return mode, {'default_rule': 'hasParam and hasPlayerNameParam default false; name lookup is inside hasParam branch'}
    if name == 'element':
        elements = {'COMBAT_' + k.upper() + 'DAMAGE': k for k in ('fire', 'earth', 'energy', 'ice', 'death')}
        if value not in elements:
            raise ValueError('unsupported source elemental cast type')
        return elements[value], {}
    if name == 'monkSpellType':
        roles = {'MonkSpell_Builder': 'builder', 'MonkSpell_Spender': 'spender'}
        if value not in roles:
            raise ValueError('unknown MonkSpell role')
        return roles[value], {}
    if name in ('castSound', 'impactSound'):
        symbol = value.removeprefix('SOUND_EFFECT_TYPE_')
        if not value.startswith('SOUND_EFFECT_TYPE_') or symbol not in sounds:
            raise ValueError('sound enum unavailable or not registered in owning source')
        alias = None if sounds[symbol] == 0 else 'canary.sound:' + symbol.lower()
        if alias is not None and CUE_VALUES.get(alias) != sounds[symbol]:
            raise ValueError('sound alias not bound to same numeric value in runtime registry')
        return alias, {
            'source_enum': symbol, 'source_enum_value': sounds[symbol],
            'default_rule': 'SILENCE maps to absence, not an audible alias'}
    raise ValueError('unknown semantic transform')



def source_sound_reference(value, sounds):
    """Keep an unresolved source symbol typed, never infer a native binding."""
    if not re.fullmatch(r'SOUND_EFFECT_TYPE_[A-Z_0-9]+', value):
        raise ValueError('invalid source sound constant syntax')
    symbol = value.removeprefix('SOUND_EFFECT_TYPE_')
    all_values = sounds.get('__cpp_values__', sounds)
    reference = {'constant': value}
    if symbol not in all_values:
        reference['binding_status'] = 'unbound'
    else:
        reference['source_numeric_value'] = all_values[symbol]
        reference['binding_status'] = 'knownbound' if symbol in sounds else 'not_lua_registered'
    return reference

def source_rules(repo, revision):
    """Bind normalizations to local immutable method implementations and enums."""
    files = ['src/lua/functions/creatures/combat/spell_functions.cpp',
             'src/creatures/combat/spells.cpp', 'src/creatures/combat/spells.hpp',
             'src/lua/functions/lua_functions_loader.hpp',
             'src/creatures/creatures_definitions.hpp',
             'src/lua/functions/core/game/lua_enums.cpp', 'data/XML/vocations.xml']
    blobs = {p: subprocess.check_output(['git', '-C', str(repo), 'show', revision + ':' + p]) for p in files}
    # Reuse the strict owning enum parser; Crystal's effect registration differs,
    # so only qualify the needed sound enum and its actual Lua registrations.
    spec = importlib.util.spec_from_file_location('spell_cue_enum_audit', HERE.parent / 'native-gameplay/build_spell_cues.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    values = module.enum_values(blobs[files[4]].decode(), 'SoundEffect_t')
    lua = module.without_comments(blobs[files[5]].decode())
    registered = set(re.findall(r'\bregisterEnumNamespace\(L,\s*soundNamespace,\s*SoundEffect_t::([A-Z_0-9]+)\s*\)', lua))
    if 'soundNamespace = "SOUND_EFFECT_TYPE_"' not in lua or not registered:
        raise ValueError('source sound namespace registration unavailable')
    methods, engine, header, loader = (blobs[p].decode() for p in files[:4])
    required = ((methods, 'primaryGroupCd[, secondaryGroupCd]'),
                (methods, 'spell->setBlockingCreature(Lua::getBoolean(L, 3))'),
                (methods, 'spell->addVocMap(vocationId, false)'),
                (engine, '} else if (hasParam) {'), (engine, 'if (getHasPlayerNameParam())'),
                (header, 'bool hasParam = false'), (header, 'bool hasPlayerNameParam = false'),
                (loader, 'lua_toboolean(L, arg) != 0'))
    for text, rule in required:
        if rule not in text:
            raise ValueError('source transform rule not established: ' + rule)
    qualified = {k: v for k, v in values.items() if k in registered}
    qualified['__cpp_values__'] = values
    return qualified, [
        {'file': p, 'sha256': digest(b)} for p, b in blobs.items()]


def audit_rows(rows, catalog, spell_schema, sounds=None):
    """Keep source identities, never deduplicate by spell name or engine id."""
    seen = set()
    results = []
    validator = Draft202012Validator(json.loads((HERE / 'spell-source-registrar.schema.json').read_text()))
    for row in sorted(rows, key=lambda r: r['registration_key']):
        key = row['registration_key']
        if key in seen:
            raise ValueError('duplicate source registration identity: ' + key)
        seen.add(key)
        unknown = set(row['registrar']) - set(DIRECT) - set(TRANSFORMED) - set(GAPS)
        if unknown:
            raise ValueError('unmapped registrar fields: ' + ', '.join(sorted(unknown)))
        validator.validate(row['registrar'])
        logical = tuple(row['logical_key'])
        targets = catalog.get(logical, [])
        fields = []
        for name, value in sorted(row['registrar'].items()):
            path = DIRECT.get(name) or TRANSFORMED.get(name)
            item = {'source_field': name, 'source_value': value, 'authoring_path': path}
            if name in DIRECT:
                item['normalized_expected_value'] = value
            if name in GAPS:
                item.update(status='schema_semantic_gap', reason=GAPS[name])
            elif not schema_pointer(spell_schema, path):
                item.update(status='schema_path_missing')
            elif name in TRANSFORMED:
                try:
                    expected, details = transformed(name, row['registrar'], sounds or {})
                    item.update(normalized_expected_value=expected, transform_details=details)
                    comparisons = []
                    for target in targets:
                        exists, actual = pointer(target, path)
                        comparable = actual
                        if name == 'runeId' and isinstance(actual, dict):
                            comparable = {k: actual.get(k) for k in ('family', 'key')}
                        elif name == 'vocation' and isinstance(actual, list):
                            comparable = sorted(actual)
                        same = comparable == expected and type(comparable) is type(expected)
                        if name in ('castSound', 'impactSound') and expected is None:
                            same = not exists
                        comparisons.append({'identity': target['identity'], 'present': exists,
                                            'authoring_value': actual, 'equal': same})
                    item['authoring_comparisons'] = comparisons
                    item['status'] = ('source_transform_value_present' if any(c['equal'] for c in comparisons)
                                      else 'source_transform_variant_missing_or_changed' if targets
                                      else 'no_matching_authoring_variant')
                    if name == 'vocation':
                        unsupported = sorted(set(expected) - set(spell_schema['$defs']['vocation']['enum']))
                        item['unsupported_authoring_vocations'] = unsupported
                        item['normalized_vocation_display_flags'] = details['unrepresented_source_semantics']
                        item['vocation_display_flags_authoring_path'] = '/requirements/vocation_display_flags'
                        if unsupported:
                            item['status'] = 'schema_semantic_gap_vocation_keys'
                        else:
                            display_matches = [pointer(target, '/requirements/vocation_display_flags')[1] == details['unrepresented_source_semantics'] for target in targets]
                            if not any(display_matches):
                                item['status'] = 'source_vocation_display_flags_missing_in_selected_catalog'
                except ValueError as error:
                    if name in ('castSound', 'impactSound'):
                        reference = source_sound_reference(value, sounds or {})
                        reference_path = '/presentation/source_cast_sound' if name == 'castSound' else '/presentation/source_impact_sound'
                        item.update(status='typed_source_sound_reference_preserved_native_cue_unbound',
                                    authoring_path=reference_path, normalized_expected_value=reference,
                                    native_cue_binding_established=False, reason=str(error))
                    else:
                        item.update(status='typed_source_preserved_semantic_transform_unverified', reason=str(error))
            elif not targets:
                item.update(status='no_matching_authoring_variant')
            else:
                comparisons = []
                for target in targets:
                    exists, actual = pointer(target, path)
                    # Case-folded names are the logical key, not source byte equality.
                    same = actual == value and type(actual) is type(value)
                    if name == 'name' and isinstance(actual, str):
                        same = actual.casefold() == value.casefold()
                    comparisons.append({'identity': target['identity'], 'present': exists,
                                        'authoring_value': actual, 'equal': exists and same})
                item['authoring_comparisons'] = comparisons
                item['status'] = ('source_value_present' if any(c['equal'] for c in comparisons)
                                  else 'source_variant_value_missing_or_changed')
            fields.append(item)
        results.append({k: row[k] for k in ('registration_key', 'logical_key', 'file', 'git_blob',
                                           'sha256', 'source_classification', 'engine_enabled_path',
                                           'catalog_match', 'cast_tier')} | {
            'registrar': row['registrar'], 'registrar_schema': 'urn:oteryn:spell-source-registrar:1',
            'field_coverage': fields,
            'execution_data_coverage': 'not_established_by_registrar_capture'})
    return results


def verify_source_bytes(rows, repo, revision):
    """Verify recorded SHA256 and immutable Git blob against local exact tree."""
    tree = subprocess.check_output(['git', '-C', str(repo), 'ls-tree', '-r', '-z', revision])
    blobs = {}
    for line in tree.split(b'\0'):
        if line:
            metadata, path = line.split(b'\t')
            blobs[path.decode()] = metadata.decode().split()[2]
    files = {}
    for row in rows:
        if blobs.get(row['file']) != row['git_blob']:
            raise ValueError('source tree identity mismatch: ' + row['file'])
        identity = (row['git_blob'], row['sha256'])
        if row['file'] in files and files[row['file']] != identity:
            raise ValueError('conflicting source file digest: ' + row['file'])
        files[row['file']] = identity
    names = sorted(files)
    payload = subprocess.check_output(['git', '-C', str(repo), 'cat-file', '--batch'],
                                     input=('\n'.join(files[n][0] for n in names) + '\n').encode())
    offset = 0
    for name in names:
        end = payload.index(b'\n', offset)
        header = payload[offset:end].decode().split()
        size = int(header[2])
        data = payload[end + 1:end + 1 + size]
        offset = end + size + 2
        blob, sha = files[name]
        actual_blob = hashlib.sha1(b'blob ' + str(size).encode() + b'\0' + data).hexdigest()
        if header[0] != blob or actual_blob != blob or digest(data) != sha:
            raise ValueError('source content digest mismatch: ' + name)
    return len(files)


def build(source_root):
    summary = json.loads((AUDIT / 'source-completeness-summary.json').read_text())
    catalog_path = HERE / 'samples/executable-spell-catalog.json'
    catalog = {}
    for entry in json.loads(catalog_path.read_text())['bundles']:
        spell = entry['bundle']['spell']
        catalog.setdefault((spell['carrier'], spell['name'].casefold()), []).append(spell)
    schema = json.loads((HERE / 'spell.schema.json').read_text())
    sources = {}
    for source in SOURCES:
        path = AUDIT / (source + '-registration-inventory.json')
        rows = [r for r in json.loads(path.read_text()) if r['source_classification'] in CLASSES]
        revision = summary['sources'][source]['revision']
        file_count = verify_source_bytes(rows, source_root / source.split('-')[0], revision)
        sounds, rules = source_rules(source_root / source.split('-')[0], revision)
        result = audit_rows(rows, catalog, schema, sounds)
        statuses = Counter(f['status'] for r in result for f in r['field_coverage'])
        sources[source] = {'revision': revision, 'inventory_sha256': digest(path.read_bytes()),
                           'verified_source_files': file_count, 'registrations': len(result),
                           'transform_source_proofs': rules,
                           'unmatched_catalog_registrations': [
                               {'registration_key': r['registration_key'], 'logical_key': r['logical_key'],
                                'catalog_match': r['catalog_match'], 'cast_tier': r['cast_tier'],
                                'source_file': r['file'], 'source_sha256': r['sha256'],
                                'execution_scope': 'registrar captured; onCastSpell/custom dependencies not normalized by this tool'}
                               for r in result if tuple(r['logical_key']) not in catalog],
                           'field_status_counts': dict(sorted(statuses.items())), 'records': result}
    return {'schema': 'OTERYN_PLAYER_SOURCE_SCHEMA_COVERAGE/v1',
            'scope': 'All classified player and disabled fixture registrations in four local pinned trees; source-only typed registrar values',
            'external_sources_used': False, 'registrar_typed_capture_complete': True,
            'source_to_authoring_1_to_1_complete': False,
            'execution_1_to_1_complete': False,
            'limitations': ['Existing inventories record effective registrar values, not ordered calls or Lua callback graphs',
                            'Absent registrar calls stay absent; engine defaults not inferred',
                            'Transform gaps and source variant differences remain explicit per field',
                            'Catalog comparisons expose existing external overrides; they do not import them into source records'],
            'schema_sha256': digest((HERE / 'spell-source-registrar.schema.json').read_bytes()),
            'authoring_schema_sha256': digest((HERE / 'spell.schema.json').read_bytes()),
            'catalog_sha256': digest(catalog_path.read_bytes()),
            'runtime_sound_registry_sha256': digest(CUES.read_bytes()), 'sources': sources}


def export_source_data(report, output):
    """Write a standalone small JSONL/Gzip import, not a second coverage report."""
    schema_path = HERE / 'player-source-import.schema.json'
    registrar_path = HERE / 'spell-source-registrar.schema.json'
    schema = json.loads(schema_path.read_text())
    registrar_schema = json.loads(registrar_path.read_text())
    registry = Registry().with_resource(registrar_schema['$id'], Resource.from_contents(registrar_schema))
    validator = Draft202012Validator(schema, registry=registry)
    records, keys = [], set()
    for snapshot, source in sorted(report['sources'].items()):
        for row in source['records']:
            record = {'schema': 'OTERYN_PLAYER_SOURCE_REGISTRAR_IMPORT/v1',
                      'snapshot': snapshot, 'source_revision': source['revision'],
                      'registration_key': row['registration_key'], 'logical_key': row['logical_key'],
                      'source_file': row['file'], 'git_blob': row['git_blob'],
                      'source_sha256': row['sha256'], 'source_classification': row['source_classification'],
                      'engine_enabled_path': row['engine_enabled_path'], 'catalog_match': row['catalog_match'],
                      'cast_tier': row['cast_tier'], 'registrar': row['registrar'], 'execution_mapped': False}
            validator.validate(record)
            key = record['registration_key']
            prefix = snapshot + '/' + record['source_file'] + '#'
            if key in keys or not re.fullmatch(re.escape(prefix) + r'[1-9][0-9]*', key):
                raise ValueError('duplicate or mismatched import identity: ' + key)
            if record['logical_key'][1] != record['registrar']['name'].casefold():
                raise ValueError('import logical name mismatches registrar: ' + key)
            keys.add(key)
            records.append(record)
    records.sort(key=lambda r: r['registration_key'])
    data = b''.join((json.dumps(r, sort_keys=True, separators=(',', ':')) + '\n').encode() for r in records)
    buffer = io.BytesIO()
    with gzip.GzipFile(fileobj=buffer, mode='wb', filename='', mtime=0) as archive:
        archive.write(data)
    compressed = buffer.getvalue()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(compressed)
    return {'schema': 'OTERYN_PLAYER_SOURCE_REGISTRAR_IMPORT_PROOF/v1',
            'scope': 'Importable effective source registrar facts only; no wiki or executable callback normalization',
            'artifact': output.name, 'artifact_sha256': digest(compressed),
            'payload_sha256': digest(data), 'records': len(records), 'unique_registration_keys': len(keys),
            'all_records_schema_valid': True,
            'record_schema': 'urn:oteryn:player-source-registrar-import:1',
            'record_schema_sha256': digest(schema_path.read_bytes()),
            'registrar_schema_sha256': digest(registrar_path.read_bytes()),
            'coverage_report_sha256': digest((json.dumps(report, indent=2, sort_keys=True) + '\n').encode()),
            'source_revisions': {s: v['revision'] for s, v in sorted(report['sources'].items())},
            'verified_source_file_counts': {s: v['verified_source_files'] for s, v in sorted(report['sources'].items())},
            'counts_by_snapshot': dict(sorted(Counter(r['snapshot'] for r in records).items())),
            'counts_by_catalog_match': dict(sorted(Counter(r['catalog_match'] for r in records).items())),
            'external_sources_used': False, 'execution_complete': False,
            'source_to_authoring_1_to_1_complete': False}


def put_projection(spell, path, value):
    parts = path.lstrip('/').split('/')
    target = spell
    for part in parts[:-1]:
        target = target.setdefault(part, {})
    if parts[-1] in target and target[parts[-1]] != value:
        raise ValueError('conflicting source setters for authoring path ' + path)
    target[parts[-1]] = value


def project_source_row(row):
    spell, gaps = {}, []
    carrier = row['logical_key'][0]
    if carrier in ('instant', 'rune'):
        spell['carrier'] = carrier
    else:
        if carrier not in ('@spell_instant', '@spell_rune'):
            raise ValueError('unsupported source carrier symbol')
        spell['source_carrier_symbol'] = carrier
    for field in row['field_coverage']:
        name = field['source_field']
        path = field['authoring_path']
        if name == 'runeId':
            spell['reference_rune_item_id'] = row['registrar'][name]
            continue
        if path is None or 'normalized_expected_value' not in field:
            gaps.append({'source_field': name, 'reason': field.get('reason', 'typed source fact retained; authoring transform unresolved')})
            continue
        value = field['normalized_expected_value']
        if name in ('castSound', 'impactSound') and path in ('/presentation/cast_cue', '/presentation/impact_cue') and value is not None:
            if CUE_VALUES.get(value) != field.get('transform_details', {}).get('source_enum_value'):
                raise ValueError('source/native cue numeric mismatch before projection write')
        if name in ('castSound', 'impactSound') and value is None:
            # Explicit SILENCE remains in source facts; omission has no cue.
            continue
        put_projection(spell, path, value)
        if name == 'vocation':
            put_projection(spell, '/requirements/vocation_display_flags', field['normalized_vocation_display_flags'])
    return spell, sorted(gaps, key=lambda g: g['source_field'])


def export_source_projection(report, output):
    schema_path = HERE / 'player-source-projection.schema.json'
    resources = [json.loads(p.read_text()) for p in (
        schema_path, HERE / 'spell.schema.json', HERE.parent / 'monster-authoring/monster.schema.json')]
    registry = Registry().with_resources((s['$id'], Resource.from_contents(s)) for s in resources)
    validator = Draft202012Validator(resources[0], registry=registry)
    records, keys = [], set()
    for snapshot, source in sorted(report['sources'].items()):
        for row in source['records']:
            spell, gaps = project_source_row(row)
            record = {'schema': 'OTERYN_PLAYER_SOURCE_PROJECTION/v1',
                      'snapshot': snapshot, 'source_revision': source['revision'],
                      'registration_key': row['registration_key'], 'logical_key': row['logical_key'],
                      'source_file': row['file'], 'git_blob': row['git_blob'], 'source_sha256': row['sha256'],
                      'source_classification': row['source_classification'],
                      'engine_enabled_path': row['engine_enabled_path'], 'catalog_match': row['catalog_match'],
                      'cast_tier': row['cast_tier'], 'spell': spell, 'unmapped_fields': gaps, 'execution_mapped': False}
            validator.validate(record)
            if record['registration_key'] in keys:
                raise ValueError('duplicate source projection identity')
            keys.add(record['registration_key'])
            records.append(record)
    records.sort(key=lambda r: r['registration_key'])
    data = b''.join((json.dumps(r, sort_keys=True, separators=(',', ':')) + '\n').encode() for r in records)
    buffer = io.BytesIO()
    with gzip.GzipFile(fileobj=buffer, mode='wb', filename='', mtime=0) as archive:
        archive.write(data)
    compressed = buffer.getvalue()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(compressed)
    return {'schema': 'OTERYN_PLAYER_SOURCE_PROJECTION_PROOF/v1',
            'scope': 'Source-first partial candidate Spell authoring fields only; no defaults or executable callback inference',
            'artifact': output.name, 'artifact_sha256': digest(compressed), 'payload_sha256': digest(data),
            'records': len(records), 'unique_registration_keys': len(keys), 'all_records_schema_valid': True,
            'source_header_fields_represented': all(not r['unmapped_fields'] for r in records),
            'projection_schema_sha256': digest(schema_path.read_bytes()),
            'authoring_schema_sha256': digest((HERE / 'spell.schema.json').read_bytes()),
            'coverage_report_sha256': digest((json.dumps(report, indent=2, sort_keys=True) + '\n').encode()),
            'counts_by_snapshot': dict(sorted(Counter(r['snapshot'] for r in records).items())),
            'unmapped_field_counts': dict(sorted(Counter(g['source_field'] for r in records for g in r['unmapped_fields']).items())),
            'full_executable_spell': False, 'execution_complete': False, 'external_sources_used': False,
            'native_consumption_established': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, default=Path('/workspace/spell-sources'))
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--source-data-out', type=Path)
    parser.add_argument('--source-proof-out', type=Path)
    parser.add_argument('--projection-data-out', type=Path)
    parser.add_argument('--projection-proof-out', type=Path)
    args = parser.parse_args()
    if bool(args.source_data_out) != bool(args.source_proof_out):
        parser.error('--source-data-out and --source-proof-out are required together')
    if bool(args.projection_data_out) != bool(args.projection_proof_out):
        parser.error('--projection-data-out and --projection-proof-out are required together')
    result = build(args.source_root)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    if args.source_data_out:
        proof = export_source_data(result, args.source_data_out)
        args.source_proof_out.parent.mkdir(parents=True, exist_ok=True)
        args.source_proof_out.write_text(json.dumps(proof, indent=2, sort_keys=True) + '\n')
    if args.projection_data_out:
        proof = export_source_projection(result, args.projection_data_out)
        args.projection_proof_out.parent.mkdir(parents=True, exist_ok=True)
        args.projection_proof_out.write_text(json.dumps(proof, indent=2, sort_keys=True) + '\n')
    print(json.dumps({k: {'registrations': v['registrations'], 'field_status_counts': v['field_status_counts']}
                      for k, v in result['sources'].items()}, sort_keys=True))
