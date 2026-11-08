"""One exact physical Old Treasure Map through existing source qualification."""
import hashlib
import json
import tarfile
from pathlib import Path
import qengine_reward_item_profiles as shared

ROOT = Path(__file__).resolve().parents[2]
PROOF = 'tools/content-schema/native-gameplay/questpc5-targuna-map-profile.json'
XML_SHA = 'c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb'
REVISION = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'


def capture(root=ROOT):
    corpus = root / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz'
    with tarfile.open(corpus) as archive:
        manifest = json.load(archive.extractfile('corpus-manifest.json'))
        entry, = [r for r in manifest['files'] if r['file_id'] == f'crystalserver:{REVISION}:data/items/items.xml']
        raw = archive.extractfile(entry.get('blob_path', entry.get('cache_path'))).read()
    client = (root / shared.CLIENT).read_bytes()
    if hashlib.sha256(raw).hexdigest() != XML_SHA or hashlib.sha256(client).hexdigest() != shared.CLIENT_SHA:
        raise ValueError('TARGUNA_MAP_SOURCE_PIN')
    node = shared.source.xml_index(raw)[53158]
    if shared.source.attrs(node) != {'weight': '400', 'description': 'It shows encrypted coordinates, someone related to Aragonia should take a look at it.'}:
        raise ValueError('TARGUNA_MAP_XML_DRIFT')
    physical, = shared.source.policies(client, raw, {53158})
    bindings = shared.source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    definitions = [r['definition'] for path in sorted((root / 'content/items/definitions').glob('items-*.json'))
                   for r in json.loads(path.read_bytes())['records'] if r['definition']['identity']['key'] == 'oteryn:item.tibia.i53158']
    definition, = definitions
    record = shared.accepted.qualify(53158, node, shared.source.engine_items.load_appearance_objects(client)[53158], physical, bindings, definition)
    return dict(schema='OTERYN_TARGUNA_MAP_PROFILE/v1', records=[record], sources=[
        dict(path='data/items/items.xml', revision=REVISION, sha256=XML_SHA, line=88034, corpus=str(corpus.relative_to(root))),
        dict(path=shared.CLIENT, sha256=shared.CLIENT_SHA)])


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = capture(root)
    generated[PROOF] = (json.dumps(packet, indent=2, ensure_ascii=False) + '\n').encode()
    shared.admit_records(root, generated, profile_path, manifest_path, order, canonical, packet['records'])
