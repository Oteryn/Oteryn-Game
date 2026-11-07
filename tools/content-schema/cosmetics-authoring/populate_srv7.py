"""Populate source-qualified creature taming mounts in the maintained cosmetics catalogue.

The input is an existing admitted authoring document (NX-5), not the synthetic fixture.
Source observations bind client look records; Crystal supplies explicit mount identities,
premium flags and speed. No NPC recipe, quest completion or Store entitlement is inferred.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET

import cosmetics_authoring as authoring
import source_program_grants

SHA = '201189f871d23597b82caf88fd611454a2ddefb8'
SOURCE_PATH = 'data/XML/mounts.xml'
OUTFITS_SHA256 = '02a4f73b60237946109da1ec6ccb34257b246d9f3f746b5935caeacf2e9a0757'
GRANT_SOURCES = 'tools/content-schema/cosmetics-authoring/srv7-grant-sources.json'
EVIDENCE = 'docs/reference/content-prep/outfits-content-1/admission-evidence/evidence-00000-00135.json'
SOURCE_SHA256 = 'e4ad7d0f7b712a77160484f4c70ef83c66af1c798950b3853acf23f4897c397f'
ALIASES = {'foxmouse_mount': 'foxmouse', 'giant_beaver_mount': 'giant_beaver'}


def populate(root: Path, source: Path, outfit_source: Path):
    data = source.read_bytes()
    if hashlib.sha256(data).hexdigest() != SOURCE_SHA256:
        raise ValueError('SRV7 requires the pinned Crystal mount source')
    xml = ET.fromstring(data)
    slug = lambda name: re.sub('[^a-z0-9]+', '_', name.lower()).strip('_')
    rows = {slug(row.attrib['name']): row.attrib for row in xml}
    lines = data.decode().splitlines()
    manifest = json.loads((root / authoring.MANIFEST).read_bytes())
    looks = {}
    for shard in manifest['files']:
        records = json.loads((root / authoring.MANIFEST).parent.joinpath(shard['path']).read_bytes())
        looks.update({row['source_id']: row['source_record_sha256'] for row in records['records']})
    path = root / authoring.AUTHORING
    document = json.loads(path.read_bytes())
    original_rows = {family: {row['key']: row for row in document['document'][family]}
                     for family in ('outfits', 'mounts')}
    original_promotions = {}
    for promotion in document['promotions']:
        parts = promotion['path'].split('/')
        if len(parts) == 4 and parts[1] in original_rows:
            family_rows = document['document'][parts[1]]
            original_promotions[(parts[1], family_rows[int(parts[2])]['key'], parts[3])] = promotion
    mounts = {row['key']: row for row in document['document']['mounts']}
    mappings = {row['grant_key']: row for row in document['document'].get('grant_mappings', [])}
    rules = json.loads((root / 'content/cosmetics/taming.json').read_bytes())
    for rule in rules:
        key = rule['mount_key']
        name = key.rsplit('.', 1)[1]
        source_row = rows[ALIASES.get(name, name)]
        line = next(i + 1 for i, text in enumerate(lines)
                    if f'<mount id="{source_row["id"]}" ' in text and '<!--' not in text)
        citation = {'reference': f'https://github.com/zimbadev/crystalserver/blob/{SHA}/{SOURCE_PATH}#L{line}',
                    'revision': SHA, 'sha256': SOURCE_SHA256}
        look = int(source_row['clientid'])
        mounts[key] = {'key': key, 'revision': 'srv7-r1',
                       'look': {'id': look, 'source_record_sha256': looks[look]},
                       'speed_bonus': int(source_row['speed']),
                       'premium': source_row['premium'] == 'yes', 'source': citation}
        mapping = {'grant_key': rule['grant_key'],
                   'target': {'kind': 'Mount', 'mount_key': key},
                   'compatible_provenance': ['mount-taming-v1'] +
                       (['crystal-item-cosmetics-201189f-v1'] if rule.get('target_item_key') else [])}
        previous = mappings.get(rule['grant_key'])
        if previous is not None and (previous['target'] != mapping['target']
                or not set(previous['compatible_provenance']) <= set(mapping['compatible_provenance'])):
            raise ValueError('SRV7 grant identity conflicts with admitted authoring')
        mappings[rule['grant_key']] = mapping
    # Tibiadrome's existing production paid-reward owner resolves these exact looks.
    # Only its catalogue/provenance slice belongs to S06; arena rules remain BOSS2.
    source_row = rows['phant']
    line = next(i + 1 for i, text in enumerate(lines)
                if f'<mount id="{source_row["id"]}" ' in text and '<!--' not in text)
    look = int(source_row['clientid'])
    mounts['oteryn:content.mount.phant'] = {
        'key': 'oteryn:content.mount.phant', 'revision': 'srv7-r1',
        'look': {'id': look, 'source_record_sha256': looks[look]},
        'speed_bonus': int(source_row['speed']), 'premium': source_row['premium'] == 'yes',
        'source': {'reference': f'https://github.com/zimbadev/crystalserver/blob/{SHA}/{SOURCE_PATH}#L{line}',
                   'revision': SHA, 'sha256': SOURCE_SHA256}}
    for grant_key, target in [
            ('oteryn:appearance-grant/tibiadrome/phant', {'kind': 'Mount', 'mount_key': 'oteryn:content.mount.phant'}),
            ('oteryn:appearance-grant/tibiadrome/lion-of-war-base',
             {'kind': 'Outfit', 'outfit_key': 'oteryn:content.outfit.lion_of_war_outfits'})]:
        mapping = {'grant_key': grant_key, 'target': target,
                   'compatible_provenance': ['crystal-201189f-tibiadrome-v1']}
        if grant_key in mappings and mappings[grant_key] != mapping:
            raise ValueError('SRV7 arena grant conflicts with admitted authoring')
        mappings[grant_key] = mapping
    outfit_bytes = outfit_source.read_bytes()
    if hashlib.sha256(outfit_bytes).hexdigest() != OUTFITS_SHA256:
        raise ValueError('SRV7 requires the pinned Crystal outfit source')
    donor_outfits = {(int(row.attrib['type']), int(row.attrib['looktype'])): row.attrib
                     for row in ET.fromstring(outfit_bytes)}
    evidence_bytes = (root / EVIDENCE).read_bytes()
    evidence_hash = hashlib.sha256(evidence_bytes).hexdigest()
    outfits = []
    for index, source_record in enumerate(json.loads(evidence_bytes)['records']):
        facts = source_record['facts']
        female = int(facts['female_look_type']['value'])
        male = int(facts['male_look_type']['value'])
        citation = {'reference': f'{EVIDENCE}#/records/{index}',
                    'revision': 'tibiawiki-2026-10-18', 'sha256': evidence_hash}
        requirements = facts['addon_specific_requirements']['value']
        no_addons = not requirements or (isinstance(requirements, dict)
                                        and requirements.get('availability') == 'UNAVAILABLE')
        outfits.append({'key': source_record['key'], 'revision': 'srv7-r1',
                        'female': {'id': female, 'source_record_sha256': looks[female]},
                        'male': {'id': male, 'source_record_sha256': looks[male]},
                        'addon_count': 0 if no_addons else 2,
                        'starter_female': donor_outfits.get((0, female), {}).get('unlocked') == 'yes',
                        'starter_male': donor_outfits.get((1, male), {}).get('unlocked') == 'yes',
                        'starter_addons_female': 0, 'starter_addons_male': 0,
                        'premium': facts['premium']['value'],
                        'unlock_sources': [citation], 'source': citation})
    npc_source = root / 'tools/content-schema/cosmetics-authoring/srv7-npc-grants.json'
    npc = json.loads(npc_source.read_bytes())
    for mapping in npc['records']:
        previous = mappings.get(mapping['grant_key'])
        if previous is not None and previous != mapping:
            raise ValueError('SRV7 NPC grant identity conflicts with admitted authoring')
        mappings[mapping['grant_key']] = mapping
    # Add source qualification without replacing the admitted NX-5 identity or provenance.
    merged_outfits = {row['key']: row for row in outfits}
    for key, previous in original_rows['outfits'].items():
        generated = merged_outfits.get(key)
        if generated is None:
            merged_outfits[key] = previous
            continue
        for field in ('female', 'male', 'addon_count', 'starter_female', 'starter_male',
                      'starter_addons_female', 'starter_addons_male', 'premium'):
            if generated[field] != previous[field]:
                raise ValueError('SRV7 conflicts with admitted NX-5 outfit field: ' + key + '/' + field)
        merged = dict(previous)
        merged['unlock_sources'] = list(previous['unlock_sources'])
        for citation in generated['unlock_sources']:
            if citation not in merged['unlock_sources']:
                merged['unlock_sources'].append(citation)
        merged_outfits[key] = merged
    outfits = list(merged_outfits.values())
    source_program_grants.admit(root,document,outfits,mappings,mounts)
    document['document']['outfits'] = sorted(outfits, key=lambda row: row['key'])
    document['document']['mounts'] = [mounts[key] for key in sorted(mounts)]
    document['document']['grant_mappings'] = [mappings[key] for key in sorted(mappings)]
    promotions = {p['path']: p for p in document['promotions'] if not p['path'].startswith(('/mounts/', '/outfits/'))}
    for family in ['mounts', 'outfits']:
        for i, row in enumerate(document['document'][family]):
            for field, value in row.items():
                pointer = f'/{family}/{i}/{field}'
                previous = original_promotions.get((family, row['key'], field))
                if previous is not None:
                    promotions[pointer] = {**previous, 'path': pointer, 'value': value}
                    continue
                promotions[pointer] = {'path': pointer, 'value': value, 'state': 'PROMOTED',
                                       'decision': 'SRV7', 'source_kind': 'ACCEPTED_REFERENCE_EVIDENCE',
                                       'source': row['source']}
    grants = promotions['/grant_mappings']
    grants['value'] = document['document']['grant_mappings']
    grants['decision'] = 'NX-5 and SRV7; explicit grants retain their individual provenance'
    source_paths = ['tools/content-schema/native-gameplay/item-cosmetic-actions.json',
                    'crates/gameplay/durability/src/durability/character_appearance/quest_grants.rs',
                    'content/cosmetics/taming.json',
                    'content/cosmetics/npc-acquisitions.json',
                    'tools/content-schema/cosmetics-authoring/srv7-source-program-grants.json',
                    'tools/content-schema/cosmetics-authoring/srv7-npc-grants.json',
                    'crates/gameplay/content/src/content/native_gameplay/cosmetics_view.rs',
                    'crates/gameplay/durability/src/durability/tibiadrome.rs']
    sources = {'schema': 'OTERYN_SRV7_COSMETIC_GRANT_SOURCES/v1',
               'sources': [{'path': path, 'sha256': hashlib.sha256((root / path).read_bytes()).hexdigest()}
                           for path in source_paths]}
    source_bytes = (json.dumps(sources, indent=2) + '\n').encode()
    grants['source'] = {'reference': GRANT_SOURCES, 'revision': 'srv7-r1',
                        'sha256': hashlib.sha256(source_bytes).hexdigest()}
    document['promotions'] = [promotions[key] for key in sorted(promotions)]
    authoring.validate(document)
    # Verify every source look digest through the real materializer before writing.
    authoring.build(document, root / authoring.MANIFEST)
    index_path = root / 'content/cosmetics/outfits/index.json'
    index = json.loads(index_path.read_bytes())
    index['population_state'] = 'POPULATED'
    index['notes'] = (f"Canonical catalogue: {len(document['document']['outfits'])} source-qualified outfits, "
                      f"{len(document['document']['mounts'])} mounts and {len(mappings)} explicit grant mappings. "
                      f"All {len(rules)} taming pairs, native item acquisition and reachable NPC grants use production V6 provisioning. "
                      "Admission decisions NX-5 and SRV7; per-field sources retained in authoring.json.")
    return {(root / authoring.AUTHORING): (json.dumps(document, ensure_ascii=False, indent=2) + '\n').encode(),
            (root / GRANT_SOURCES): source_bytes,
            index_path: (json.dumps(index, indent=2) + '\n').encode()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=authoring.ROOT)
    parser.add_argument('--crystal-mounts', type=Path, required=True)
    parser.add_argument('--crystal-outfits', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    result = populate(args.root, args.crystal_mounts, args.crystal_outfits)
    for path, data in result.items():
        if args.check:
            if path.read_bytes() != data:
                raise ValueError('SRV7 cosmetic source population differs: ' + str(path))
        else:
            path.write_bytes(data)


if __name__ == '__main__':
    main()
