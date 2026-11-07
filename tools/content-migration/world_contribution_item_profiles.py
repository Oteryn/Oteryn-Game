"""Admit sourced World contribution custody, corpse decay and dirty-water fishing operands.

Only native profiles are generated. Existing canonical identities and donor facts
are preserved; the fresh corpse is Ground-only, aged corpses are portable, and all
four stages keep their sourced duration and container/leaf distinction.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source

PATH = 'tools/content-schema/native-gameplay/world-contribution-item-profiles.json'
SCHEMA = 'OTERYN_NATIVE_WORLD_CONTRIBUTION_ITEM_PROFILES/v1'
IDS = (3111, 4176, 4177, 4178, 6018, 12413, 12517, 12557, 12558, 12559, 12560, 12561, 12562, 12563, 14156, 21182, 21291)
REVISION = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
XML_SHA = 'c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb'
APPEARANCES_SHA = '6adb790d1064c2d31ffb2e5ce1a7aef376942ba672edea2adb6cafc620dd18f1'
CORPUS = 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz'
XML_BLOB = 'blobs/0b1dc3ba1a49094d9c83b90ab399bd2a9dd7a17f'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def capture(appearances):
    with tarfile.open(ROOT / CORPUS) as archive:
        member = archive.extractfile(XML_BLOB)
        if member is None:
            raise ValueError('WCONTRIB_SOURCE_XML_MISSING')
        xml = member.read()
    raw_appearances = appearances.read_bytes()
    if digest(xml) != XML_SHA or digest(raw_appearances) != APPEARANCES_SHA:
        raise ValueError('WCONTRIB_SOURCE_PIN')
    index = source.xml_index(xml)
    objects = source.engine_items.load_appearance_objects(raw_appearances)
    physical = {row['server_item_id']: row for row in source.policies(raw_appearances, xml, set(IDS))}
    bindings = source.binding_index(source.BINDINGS.read_bytes())
    records = [source.profile(identifier, index[identifier], objects[identifier], physical[identifier], bindings, None)
               for identifier in IDS]
    bound = source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}, bindings, set(IDS))
    # Preserve existing qualified physical facts for medicine, shimmer swimmer
    # and fishbone. Reference-only corpse/water definitions gain only source facts.
    canonical_inputs = []
    native = {row['production_definition']['production_key']: row for row in bound['records']}
    for canonical_path in sorted((ROOT / 'content/items/definitions').glob('items-*.json')):
        canonical = json.loads(canonical_path.read_bytes())
        selected = [row['definition'] for row in canonical['records']
                    if row['definition']['identity']['key'] in native]
        if not selected:
            continue
        canonical_inputs.append({'path': canonical_path.relative_to(ROOT).as_posix(),
                                 'sha256': digest(canonical_path.read_bytes())})
        for definition in selected:
            physical = definition.get('semantics', {}).get('physical')
            if physical is not None and physical.get('state') == 'KNOWN' and physical != native[definition['identity']['key']]['semantics']['physical']:
                raise ValueError('WCONTRIB_CANONICAL_PHYSICAL_DRIFT')
    return {'schema': SCHEMA, 'producer': 'tools/content-migration/world_contribution_item_profiles.py',
            'source_revision': REVISION,
            'sources': [{'path': 'data/items/items.xml', 'sha256': XML_SHA},
                        {'path': 'data/items/appearances.dat', 'sha256': APPEARANCES_SHA}],
            'source_spans': {'3111': 'data/items/items.xml:7544-7547',
                             '4176': 'data/items/items.xml:12906-12911',
                             '4177': 'data/items/items.xml:12912-12916',
                             '4178': 'data/items/items.xml:12917-12921',
                             '6018': 'data/items/items.xml:16535-16540',
                             '12413': 'data/items/items.xml:30161-30165',
                             '12517': 'data/items/items.xml:30366-30369',
                             '12557': 'data/items/items.xml:30559-30563',
                             '12558-12560': 'data/items/items.xml:30564-30586',
                             '12561-12563': 'data/items/items.xml:30588-30592',
                             '14156': 'data/items/items.xml:32686-32690',
                             '21182': 'data/items/items.xml:42216-42220',
                             '21291': 'data/items/items.xml:42390-42393'},
            'canonical_inputs': canonical_inputs,
            'records': bound['records']}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    if (overlay['schema'] != SCHEMA or overlay['source_revision'] != REVISION
            or [row['production_definition']['production_key'] for row in records]
            != [f'oteryn:item.tibia.i{identifier}' for identifier in IDS]):
        raise ValueError('WCONTRIB_PROFILE_SET')
    for descriptor in overlay['canonical_inputs']:
        if digest((root / descriptor['path']).read_bytes()) != descriptor['sha256']:
            raise ValueError('WCONTRIB_CANONICAL_DRIFT')
    document = json.loads(generated[profile_path])
    present = {row['production_definition']['production_key'] for row in document['records']}
    if any(row['production_definition']['production_key'] in present for row in records):
        raise ValueError('WCONTRIB_PROFILE_DUPLICATE')
    document['records'] = sorted(document['records'] + records, key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = digest(raw)
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--appearances', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(capture(args.appearances), indent=2, ensure_ascii=False) + '\n').encode()
    output = ROOT / PATH
    if args.check:
        if output.read_bytes() != raw:
            raise SystemExit('WCONTRIB_PROFILE_DRIFT')
    else:
        output.write_bytes(raw)
