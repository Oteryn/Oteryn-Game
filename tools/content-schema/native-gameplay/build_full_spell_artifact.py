#!/usr/bin/env python3
"""Compose explicit local NativeGameplayInput file pins, without activating content.

The native compiler remains the authority for typed qualification. The producer
preserves source DefinitionRefs and exact provider UTF8 bytes; its separate proof
reports retained dependency references, including unresolved source operations.
Original Lua, DAT, XML, OTBM and client assets are never copied into the output.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
NATIVE = ROOT / 'tools/content-schema/native-gameplay'
SAMPLES = ROOT / 'tools/content-schema/spell-authoring/samples'
CANARY = '99902524e052f37574194466c2949c576e4ab269'
CRYSTAL = 'ff7ede593c69d4c658b382c97443e8155926924a'
FORMAL = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
VOCATIONS = ('druid', 'knight', 'monk', 'paladin', 'sorcerer')
LIMITS = {'catalog': 32 * 1024**2, 'source_selection': 256 * 1024,
          'creature_profiles': 8 * 1024**2, 'presentation_profiles': 8 * 1024**2,
          'item_profiles': 8 * 1024**2, 'spell_appearances': 8 * 1024**2,
          'build_training': 8 * 1024**2, 'familiar_config': 4096,
          'familiar_defenses': 32 * 1024, 'wheel_profile': 64 * 1024,
          'source_world': 8 * 1024**2}


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def encoded(document: dict) -> bytes:
    return (json.dumps(document, ensure_ascii=False, indent=2) + '\n').encode('utf-8')


def read(path: Path) -> tuple[bytes, dict]:
    raw = path.read_bytes()
    return raw, json.loads(raw.decode('utf-8'))


def identity(ref: dict) -> tuple[str, str, str]:
    return ref['family'], ref['key'], ref['revision']


def refs(value):
    if isinstance(value, dict):
        if set(value) == {'family', 'key', 'revision'}:
            yield identity(value)
        else:
            for child in value.values():
                yield from refs(child)
    elif isinstance(value, list):
        for child in value:
            yield from refs(child)


def reference(value: tuple) -> dict:
    return dict(zip(('family', 'key', 'revision'), value))


def merged_profiles(familiars: Path) -> tuple[dict, dict, dict]:
    proof_raw, proof = read(familiars / 'familiar-profile-source-proof.json')
    if proof['definition_revision'] != FORMAL or proof['current_revision'] != CANARY:
        raise ValueError('Familiar source revision mismatch')
    unchanged = proof['unchanged_full_definitions']
    expected_paths = {f'data-otservbr-global/monster/familiars/{v}_familiar.lua' for v in VOCATIONS}
    if {row['path'] for row in unchanged} != expected_paths:
        raise ValueError('Familiar definition source set mismatch')
    for row in unchanged:
        if (row['byte_identical'] is not True or row['definition_revision'] != FORMAL
                or row['current_revision'] != CANARY
                or row['definition_git_blob'] != row['current_git_blob']):
            raise ValueError('Familiar unchanged source proof mismatch')
    for name, sha in proof['outputs'].items():
        if digest((familiars / name).read_bytes()) != sha:
            raise ValueError(f'Familiar export pin mismatch: {name}')
    _, creatures = read(familiars / 'familiar-creature-profiles.json')
    _, presentations = read(familiars / 'familiar-presentation-profiles.json')
    expected = {('Creature', f'canary:creature/{v}_familiar', 'canary-47dfd51f') for v in VOCATIONS}
    if {identity(r['profile']['target']) for r in creatures['records']} != expected or len(creatures['records']) != 5:
        raise ValueError('Familiar exact DefinitionRefs mismatch')
    _, base_c = read(NATIVE / 'creature_profiles.json')
    _, base_p = read(NATIVE / 'presentation_profiles.json')
    rats = [r for r in base_c['records'] if identity(r['profile']['target']) == ('Creature', 'canary:creature/rat', 'canary-47dfd51f')]
    if len(rats) != 1:
        raise ValueError('Missing exact Rat source profile')
    rat = rats[0]
    rat_p = [r for r in base_p['records'] if r['target'] == rat['presentation']]
    if len(rat_p) != 1:
        raise ValueError('Missing exact Rat presentation')
    creatures = {'schema': creatures['schema'], 'records': [rat, *creatures['records']]}
    presentations = {'schema': presentations['schema'], 'records': [*rat_p, *presentations['records']]}
    c_ids = [identity(r['profile']['target']) for r in creatures['records']]
    p_ids = [identity(r['target']) for r in presentations['records']]
    if len(set(c_ids)) != 6 or len(set(p_ids)) != 6 or any(identity(r['presentation']) not in p_ids for r in creatures['records']):
        raise ValueError('Creature/presentation closure mismatch')
    return creatures, presentations, proof


def merge_skeleton(creatures, presentations, directory):
    _, proof = read(directory / 'skeleton-profile-source-proof.json')
    if proof['definition_revision'] != FORMAL or proof['current_revision'] != CANARY:
        raise ValueError('Skeleton source revision mismatch')
    rows = proof['unchanged_full_definitions']
    if len(rows) != 1 or rows[0]['path'] != 'data-otservbr-global/monster/undeads/skeleton.lua':
        raise ValueError('Skeleton exact source file mismatch')
    row = rows[0]
    if row['byte_identical'] is not True or row['definition_revision'] != FORMAL or row['current_revision'] != CANARY or row['definition_git_blob'] != row['current_git_blob']:
        raise ValueError('Skeleton full source definition changed')
    for name, sha in proof['outputs'].items():
        if digest((directory/name).read_bytes()) != sha:
            raise ValueError(f'Skeleton export pin mismatch: {name}')
    _, extra = read(directory/'skeleton-creature-profiles.json')
    _, cues = read(directory/'skeleton-presentation-profiles.json')
    if len(extra['records']) != 1 or identity(extra['records'][0]['profile']['target']) != ('Creature', 'canary:creature/skeleton', 'canary-47dfd51f'):
        raise ValueError('Skeleton exact formal DefinitionRef mismatch')
    creatures['records'].extend(extra['records']); presentations['records'].extend(cues['records'])
    if len({identity(row['profile']['target']) for row in creatures['records']}) != 7:
        raise ValueError('Skeleton creature closure duplicate')
    if any(identity(row['presentation']) not in {identity(cue['target']) for cue in presentations['records']} for row in creatures['records']):
        raise ValueError('Skeleton presentation closure missing')
    return proof


def merge_companion_population(creatures, presentations, directory):
    _, proof = read(directory/'companion-population-source-proof.json')
    if proof['source_revision'] != FORMAL or proof['current_revision'] != CANARY:
        raise ValueError('Companion population source pin differs')
    if proof['ordinary_group_can_summon_all'] or proof['ordinary_group_can_convince_all']:
        raise ValueError('Companion population ordinary group bypass is not qualified')
    for name, sha in proof['outputs'].items():
        if digest((directory/name).read_bytes()) != sha:
            raise ValueError('Companion population source export digest differs')
    for row in proof['unchanged_full_definitions']:
        if row['byte_identical'] is not True or row['definition_revision'] != FORMAL or row['current_revision'] != CANARY or row['definition_git_blob'] != row['current_git_blob']:
            raise ValueError('Companion population full Lua blob differs')
        if row['source_initial_health'] != row['source_maximum_health']:
            raise ValueError('Companion population source initial health differs from maximum')
    for row in proof.get('unchanged_foreign_definitions', []):
        if row['canonical_target'] != {'family':'Creature','key':'oteryn:creature.stag','revision':'definition-r1'} or row['current_revision'] != CRYSTAL or row['definition_revision'] != '00ce02a57ca5a12e48f32a3476e37471167e4c3f' or row['byte_identical'] is not True or row['definition_git_blob'] != row['current_git_blob'] or row['source_initial_health'] != row['source_maximum_health']:
            raise ValueError('Crystal Stag exact source identity/blob/initial health differs')
    _, extra = read(directory/'companion-population-creature-profiles.json')
    _, cues = read(directory/'companion-population-presentation-profiles.json')
    bindings = {identity(b['target']): b for b in proof['exact_source_bindings']}
    existing = {identity(row['profile']['target']) for row in creatures['records']}
    for row in extra['records']:
        ref = identity(row['profile']['target'])
        if ref in existing or ref not in bindings or bindings[ref]['disposition'] != 'EXACT':
            raise ValueError('Companion population exact source identity was duplicated/substituted')
        flags = row['profile']['data']['profile']['details']['summoning']
        if not (flags['summonable'] or flags['convinceable']) or 'mana_cost' not in flags:
            raise ValueError('Companion population source eligibility/cost is missing')
        existing.add(ref)
    creatures['records'].extend(extra['records']); presentations['records'].extend(cues['records'])
    if len(extra['records']) != proof['added_creature_count']:
        raise ValueError('Companion population source count differs')
    if any(identity(row['presentation']) not in {identity(cue['target']) for cue in presentations['records']} for row in creatures['records']):
        raise ValueError('Companion population exact presentation closure is missing')
    return proof


def qualify_missing_companion_audit(directory):
    """Verify complete reexports whose adopted policies exclude acquisition."""
    _, proof = read(directory/'missing-companion-source-proof.json')
    if proof['definition_revision'] != FORMAL or proof['current_revision'] != CANARY:
        raise ValueError('Missing companion audit source pin differs')
    index_raw, index = read(ROOT/'tools/content-schema/monster-authoring/samples/population-bundles-canary-47dfd51f.json')
    if digest(index_raw) != proof['formal_population_index_sha256']:
        raise ValueError('Missing companion formal census digest differs')
    for row in proof['reference_captures']:
        if digest((ROOT/row['path']).read_bytes()) != row['sha256']:
            raise ValueError('Missing companion adopted capture digest differs')
    for name, sha in proof['outputs'].items():
        if digest((directory/name).read_bytes()) != sha:
            raise ValueError('Missing companion source export digest differs')
    _, profiles = read(directory/'missing-companion-creature-profiles.json')
    if len(profiles['records']) != 6 or proof['normalized_eligible_count'] != 0:
        raise ValueError('Missing companion audit count/eligibility differs')
    by_identity = {identity(r['profile']['target']):r for r in profiles['records']}
    indexed = {r['file']:r['sha256'] for r in index['monsters']}
    for row in proof['unchanged_full_definitions']:
        if row['byte_identical'] is not True or row['definition_git_blob'] != row['current_git_blob']:
            raise ValueError('Missing companion full Lua blob differs')
        file = row['path'].removeprefix('data-otservbr-global/monster/').removesuffix('.lua')
        if indexed[file] != row['formal_bundle_sha256'] or not row['population_census_sha256_matched']:
            raise ValueError('Missing companion formal bundle differs from census')
        slug = row['canonical_target']['key'].rsplit('/',1)[-1]
        bundle_hash = hashlib.sha256()
        for name in ('monster.json','dependencies.json','catalog.json','manifest.json'):
            raw = (directory/'formal-bundles'/slug/name).read_bytes()
            bundle_hash.update(f'{name}\0{len(raw)}\0'.encode('ascii')+raw)
        if bundle_hash.hexdigest() != row['formal_bundle_sha256']:
            raise ValueError('Missing companion actual bundle bytes differ from census')
        flags = by_identity[identity(row['canonical_target'])]['profile']['data']['profile']['details']['summoning']
        if flags != row['normalized_summoning'] or flags['summonable'] or flags['convinceable']:
            raise ValueError('Missing companion normalized exclusion was substituted')
    return proof


def qualify_item_closure(catalog, provider):
    """Exact authored refs plus independent SourceIdentityBinding/definition match.

    Source namespace and actual production identities are independently qualified;
    a numeric key or a revised authored label alone grants no Item authority.
    """
    requested = {ref for ref in refs(catalog['bundles']) if ref[0] == 'Item'}
    if not requested:
        return {'item_ref_count': 0, 'exact_used_bindings': []}
    from build_spell_item_identities import BINDINGS, binding_index
    raw = BINDINGS.read_bytes(); bindings = binding_index(raw)
    packet_path = ROOT/'imports/canary/bindings/spell-items-candidate.json'
    packet_raw, packet = read(packet_path)
    if packet['qualification']['source_revision'] != CANARY or len(packet['bindings']) != 1:
        raise ValueError('Actual Item40450 complete source packet mismatch')
    indexed = {identity(row['authoring']['item']): row for row in provider['records']}
    if len(indexed) != len(provider['records']):
        raise ValueError('Duplicate authored Item provider identity')
    qualified = []
    for ref in sorted(requested):
        if ref not in indexed:
            raise ValueError(f'Missing exact authored Item provider reference: {ref}')
        row = indexed[ref]; external = ref[1].removeprefix('candidate:item/')
        if ref[2] != 'spell-p2-r21' or external == ref[1] or not external.isdigit():
            raise ValueError(f'Wrong Item importer reference: {ref}')
        expected = packet['bindings'][0] if external == '40450' else bindings.get(external)
        if not expected or row['production_binding'] != expected:
            raise ValueError(f'Actual SourceIdentityBinding mismatch: {ref}')
        if external == '40450' and row['production_binding_qualification'] != packet:
            raise ValueError('Item40450 full qualification packet was changed or truncated')
        target = expected['target']
        definition = {'family':target['family'], 'production_key':target['key'], 'revision_ref':target['revision']}
        if row['production_definition'] != definition:
            raise ValueError(f'Item production definition/binding mismatch: {ref}')
        qualified.append({'authoring': reference(ref), 'binding': expected, 'production_definition': definition})
    return {'item_ref_count': len(requested), 'binding_table_sha256':digest(raw),
            'complete_canary_candidate_packet_sha256':digest(packet_raw), 'exact_used_bindings':qualified}


def qualify_appearance_closure(creatures, presentations, provider):
    """Every illusionable exact Creature needs its source-qualified Outfit link."""
    current = {identity(row['profile']['target']): row for row in creatures['records']}
    cues = {identity(row['target']): row for row in presentations['records']}
    links = {}
    for row in provider['records']:
        if row['creature'] is None:
            continue
        ref = identity(row['creature'])
        if ref not in current or ref in links:
            raise ValueError('Appearance Creature link is inactive or duplicated')
        creature = current[ref]
        details = creature['profile']['data']['profile']['details']
        if not details['flags']['illusionable']:
            raise ValueError('Appearance authorizes a non-illusionable Creature')
        cue = cues[identity(creature['presentation'])]['data']['profile']
        if cue['asset_binding'] != f'canary.appearance:outfit/{row["look_type"]}':
            raise ValueError('Appearance source Outfit differs from exact Creature Presentation')
        links[ref] = row
    required = {ref for ref, row in current.items()
                if row['profile']['data']['profile']['details']['flags']['illusionable']}
    if required != set(links):
        raise ValueError(f'Missing exact illusionable Creature appearance: {sorted(required-set(links))}')
    return {'illusionable_creature_count': len(required), 'exact_source_links': [
        {'creature': reference(ref), 'look_type': links[ref]['look_type'],
         'source': links[ref]['source'], 'qualification_sha256': links[ref]['qualification_sha256']}
        for ref in sorted(required)]}


def dependency_report(creatures: dict, presentations: dict, familiars: Path, proof: dict, skeleton: Path | None = None, population: Path | None = None) -> dict:
    raw, dependency = read(familiars / 'familiar-dependency-profiles.json')
    declared = {identity(p['target']) for p in dependency['authoring_profiles']}
    records = {identity(r['identity']) for r in dependency['records']}
    if declared != records or len(records) != len(dependency['records']):
        raise ValueError('Familiar dependency declaration mismatch')
    extra_proofs = []
    extra_retained = set()
    record_only = []
    if population:
        extra_raw, extra = read(population/'companion-population-dependency-profiles.json')
        extra_records = {identity(r['identity']) for r in extra['records']}
        extra_profiles = {identity(p['target']) for p in extra['authoring_profiles']}
        if len(extra_records) != len(extra['records']) or not extra_profiles <= extra_records:
            raise ValueError('Companion population source-only declaration carrier differs')
        declared |= extra_records; records |= extra_records
        extra_retained |= set(refs(extra))
        record_only = extra['record_only_dependencies']
        extra_proofs.append({'path':'companion-population-dependency-profiles.json','sha256':digest(extra_raw),
                             'declarations':len(extra_records),'authoring_profiles':len(extra_profiles)})
    if skeleton:
        extra_raw, extra = read(skeleton/'skeleton-dependency-profiles.json')
        extra_declared = {identity(p['target']) for p in extra['authoring_profiles']}
        extra_records = {identity(r['identity']) for r in extra['records']}
        if extra_declared != extra_records or len(extra_records) != len(extra['records']):
            raise ValueError('Skeleton dependency declaration mismatch')
        declared |= extra_declared; records |= extra_records
        extra_retained |= set(refs(extra))
        extra_proofs.append({'path':'skeleton-dependency-profiles.json','sha256':digest(extra_raw),'declarations':len(extra_records)})
    declared |= {identity(r['profile']['target']) for r in creatures['records']}
    declared |= {identity(r['behavior']['target']) for r in creatures['records'] if r.get('behavior')}
    declared |= {identity(r['target']) for r in presentations['records']}
    retained = set(refs(creatures)) | set(refs(presentations)) | set(refs(dependency)) | extra_retained
    unresolved = [row for bundle in proof['bundles'] for row in bundle['unresolved_manifest_rows']]
    unresolved_ids = {identity(row['retained_schedule']['ability']) for row in unresolved}
    absent = retained - declared
    return {'schema': 'OTERYN_FULL_SPELL_SOURCE_CLOSURE_REPORT/v1',
            'dependency_profile_sha256': digest(raw),
            'declared_dependency_count': len(records), 'extra_dependency_sources':extra_proofs,
            'retained_reference_count': len(retained),
            'missing_exact_declarations': [reference(r) for r in sorted(absent)],
            'unresolved_source_operations': unresolved,
            'source_only_custom_ability_refs': [reference(r) for r in sorted(unresolved_ids)],
            'runtime_qualification': 'Native loader qualifies Creature/Behavior/Presentation and optional owner profiles; dependency carrier is a separate source sidecar and does not authorize monster AI.'}


def build(args) -> dict:
    output = args.out
    output.mkdir(parents=True, exist_ok=True)
    catalog_raw, catalog = read(args.catalog)
    selection_raw, selection = read(args.selection)
    census_count = len(catalog['bundles']) + len(catalog.get('removed', []))
    if census_count != args.expected_spells:
        raise ValueError(f'Expected {args.expected_spells} census identities, found {census_count}')
    if selection['catalog_sha256'] != digest(catalog_raw) or selection['revision'] != catalog['revision']:
        raise ValueError('Executable catalogue/source selection mismatch')
    for row in selection['selections']:
        for source in row['source_proofs']:
            expected = {'opentibiabr/canary': CANARY, 'zimbadev/crystalserver': CRYSTAL}.get(source['repository'])
            if expected and source['revision'] != expected:
                raise ValueError('Current source selection revision mismatch')
    if not re.fullmatch(r'[A-Za-z0-9:._/-]+', args.build_revision):
        raise ValueError('Explicit Character build content revision required')
    creatures, presentations, familiar_proof = merged_profiles(args.familiars)
    skeleton = getattr(args, 'skeleton', None)
    skeleton_proof = merge_skeleton(creatures, presentations, skeleton) if skeleton else None
    population = getattr(args, 'population', None)
    population_proof = merge_companion_population(creatures, presentations, population) if population else None
    closure = dependency_report(creatures, presentations, args.familiars, familiar_proof, skeleton, population)
    if population:
        _, dep = read(population/'companion-population-dependency-profiles.json')
        closure['source_only_record_dependencies_without_authoring_profiles'] = dep['record_only_dependencies']
        closure['unsupported_companion_source_profiles'] = population_proof['unsupported_source_profiles']
    missing_companions = getattr(args, 'missing_companions', None)
    missing_proof = qualify_missing_companion_audit(missing_companions) if missing_companions else None
    if missing_proof:
        closure['reconverted_source_only_ineligible_companions'] = missing_proof['bundles']
        for name in (*missing_proof['outputs'], 'missing-companion-source-proof.json'):
            (output/name).write_bytes((missing_companions/name).read_bytes())
    payloads = {'catalog': catalog_raw, 'source_selection': selection_raw,
                'creature_profiles': encoded(creatures), 'presentation_profiles': encoded(presentations)}
    providers = {'item_profiles': args.items, 'spell_appearances': args.appearances,
                 'build_training': args.training, 'familiar_config': args.familiar_config,
                 'familiar_defenses': args.familiar_defenses, 'wheel_profile': args.wheel,
                 'source_world': args.source_world}
    for key, path in providers.items():
        if path:
            raw, _ = read(path)
            payloads[key] = raw
    appearance_source = getattr(args, 'appearance_source', None)
    if appearance_source:
        import build_spell_appearances
        payloads['spell_appearances'] = encoded(build_spell_appearances.build(appearance_source, creatures,
            population_proof['appearance_source_paths'] if population_proof else None))
    closure['executable_appearance_closure'] = qualify_appearance_closure(
        creatures, presentations, json.loads(payloads['spell_appearances']))
    item_closure = qualify_item_closure(catalog, json.loads(payloads['item_profiles']))
    closure['executable_item_closure'] = item_closure
    if not json.loads(payloads['item_profiles'])['records']:
        raise ValueError('Full input requires actual nonempty Item policies')
    item_source_proof = None
    if getattr(args, 'item_proof', None):
        item_proof_raw, item_proof = read(args.item_proof)
        if item_proof['output_sha256'] != digest(payloads['item_profiles']):
            raise ValueError('Item provider/source proof hash mismatch')
        # Retain derivation hashes and qualification facts without raw XML excerpts.
        item_source_proof = {key: value for key, value in item_proof.items()
                             if key != 'source_rows'}
        item_source_proof['complete_source_proof_sha256'] = digest(item_proof_raw)
        (output / 'item-source-proof.json').write_bytes(encoded(item_source_proof))
    manifest = {'schema': 'OTERYN_NATIVE_GAMEPLAY_MANIFEST/v5',
                'native_map_profile': args.native_map_profile}
    for key, raw in payloads.items():
        if len(raw) > LIMITS[key]:
            raise ValueError(f'Native input exceeds bounded provider size: {key}')
        filename = key.replace('_', '-') + '.json'
        (output / filename).write_bytes(raw)
        manifest[key] = {'path': filename, 'sha256': digest(raw)}
    manifest['build_training']['content_revision'] = args.build_revision
    (output / 'manifest.json').write_bytes(encoded(manifest))
    for name in ('familiar-dependency-profiles.json', 'familiar-profile-source-proof.json'):
        (output / name).write_bytes((args.familiars / name).read_bytes())
    if skeleton:
        for name in ('skeleton-dependency-profiles.json', 'skeleton-profile-source-proof.json'):
            (output/name).write_bytes((skeleton/name).read_bytes())
    if population:
        for name in ('companion-population-dependency-profiles.json', 'companion-population-source-proof.json',
                     'companion-population-source-only-profiles.json'):
            (output/name).write_bytes((population/name).read_bytes())
    (output / 'source-closure-report.json').write_bytes(encoded(closure))
    evidence = {'schema': 'OTERYN_FULL_SPELL_NATIVE_INPUT_PROOF/v1',
                'classification': 'Explicit local candidate; OTS source hypotheses only',
                'current_sources': {'canary': CANARY, 'crystal': CRYSTAL},
                'familiar_definition_revision': FORMAL,
                'spell_identity_count': census_count,
                'executable_spell_count': len(catalog['bundles']),
                'excluded_spell_count': len(catalog.get('removed', [])), 'creature_count': len(creatures['records']),
                'skeleton_source_proof_sha256':digest(encoded(skeleton_proof)) if skeleton_proof else None,
                'missing_companion_source_proof_sha256':digest(encoded(missing_proof)) if missing_proof else None,
                'companion_population_source_proof_sha256':digest(encoded(population_proof)) if population_proof else None,
                'item_policy_count': len(json.loads(payloads['item_profiles'])['records']),
                'character_build_content_revision': args.build_revision,
                'manifest_sha256': digest(encoded(manifest)),
                'item_source_proof_sha256': digest(encoded(item_source_proof)) if item_source_proof else None,
                'appearance_generation': {
                    'mode': 'Complete pinned-source generation' if appearance_source else 'Caller-provided exact bytes',
                    'producer_sha256': digest((NATIVE/'build_spell_appearances.py').read_bytes()) if appearance_source else None,
                    'source_revision': CANARY if appearance_source else None,
                    'additional_exact_source_revisions': [CRYSTAL] if population_proof and population_proof.get('unchanged_foreign_definitions') else []},
                'producer_sources': [
                    {'path': str(path), 'sha256': digest(path.read_bytes())}
                    for path in [NATIVE / 'creature_profiles.json', NATIVE / 'presentation_profiles.json',
                                 NATIVE / 'profile-sources.json',
                                 args.familiars / 'familiar-creature-profiles.json',
                                 args.familiars / 'familiar-presentation-profiles.json',
                                 args.familiars / 'familiar-dependency-profiles.json',
                                 args.familiars / 'familiar-profile-source-proof.json',
                                 Path(__file__) ]],
                'inputs': manifest, 'source_closure_report_sha256': digest(encoded(closure)),
                'runtime_activation': 'Not performed; independent Content issuance remains required',
                'raw_asset_redistribution': False}
    (output / 'input-proof.json').write_bytes(encoded(evidence))
    return evidence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--catalog', type=Path, default=SAMPLES / 'executable-spell-catalog.json')
    parser.add_argument('--selection', type=Path, default=SAMPLES / 'executable-spell-source-selection.json')
    parser.add_argument('--expected-spells', type=int, default=252)
    parser.add_argument('--familiars', type=Path, default=Path('/workspace/spells-r21-implemented/familiar-profiles'))
    parser.add_argument('--skeleton', type=Path, default=Path('/workspace/spells-r21-implemented/skeleton-profiles'))
    parser.add_argument('--items', type=Path, required=True)
    parser.add_argument('--item-proof', type=Path)
    parser.add_argument('--build-revision', required=True)
    parser.add_argument('--appearances', type=Path, default=NATIVE / 'spell_appearances.json')
    parser.add_argument('--missing-companions', type=Path, default=Path('/workspace/spells-r21-implemented/missing-companion-profiles'))
    parser.add_argument('--population', type=Path, default=Path('/workspace/spells-r21-implemented/companion-population-profiles'))
    parser.add_argument('--appearance-source', type=Path, default=Path('/workspace/spell-sources/canary'),
                        help='Generate the complete exact Creature appearance closure from pinned source blobs')
    parser.add_argument('--training', type=Path, default=NATIVE / 'build-training.json')
    parser.add_argument('--familiar-config', type=Path, default=NATIVE / 'familiar-config.json')
    parser.add_argument('--familiar-defenses', type=Path, default=NATIVE / 'familiar-defenses.json')
    parser.add_argument('--wheel', type=Path, default=NATIVE / 'wheel-profile.json')
    parser.add_argument('--source-world', type=Path)
    parser.add_argument('--native-map-profile', choices=('accepted-entry-r1', 'source-qualified-spell-entry-r2'), default='accepted-entry-r1')
    args = parser.parse_args()
    if args.source_world and args.native_map_profile != 'accepted-entry-r1':
        parser.error('source-world requires accepted-entry-r1 as the native inner map profile')
    evidence = build(args)
    print(json.dumps({key: evidence[key] for key in ('spell_identity_count', 'creature_count', 'item_policy_count', 'manifest_sha256')}))


if __name__ == '__main__':
    main()
