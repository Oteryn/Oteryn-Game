"""Retained source facts for six Rotten Blood Items; no runtime/use admission."""
import hashlib, json, re, sys, tarfile
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'content-migration'))
import encounter_loot_item_admissions as shared

DIRECTORY = 'tools/content-schema/quest-authoring/samples/donor-source'
OUTPUT = 'tools/content-schema/quest-authoring/samples/questpc2-rotten-item-facts.json'
IDS = frozenset({43501, 43502, 43503, 43504, 43899, 43900})
REVISION = '00ce02a57ca5a12e48f32a3476e37471167e4c3f'
XML_SHA = '13a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece'
GROUPS = ('physical', 'stack', 'container', 'charges', 'temporal')

def capture(root, *, profiles_only=False):
    declaration = json.loads((root / DIRECTORY / 'corpus-input.json').read_bytes())
    archive = root / DIRECTORY / declaration['path']
    if hashlib.sha256(archive.read_bytes()).hexdigest() != declaration['sha256']:
        raise ValueError('Rotten Blood donor archive pin differs')
    with tarfile.open(archive) as stream:
        manifest = json.load(stream.extractfile('corpus-manifest.json'))
        row = next(r for r in manifest['files'] if r['revision'] == REVISION and r['path'] == 'data/items/items.xml')
        xml_raw = stream.extractfile(row['cache_path']).read()
    if row['sha256'] != XML_SHA or hashlib.sha256(xml_raw).hexdigest() != XML_SHA:
        raise ValueError('Rotten Blood source XML pin differs')
    inputs = {}
    for path, sha in [(shared.XML, shared.XML_SHA), (shared.CLIENT, shared.CLIENT_SHA), (shared.BINDINGS, shared.BINDINGS_SHA)]:
        inputs[path] = (root / path).read_bytes()
        if hashlib.sha256(inputs[path]).hexdigest() != sha:
            raise ValueError('Rotten Blood existing identity/client input pin differs')
    source = shared.source
    xml = source.xml_index(xml_raw)
    objects = source.engine_items.load_appearance_objects(inputs[shared.CLIENT])
    physical = {r['server_item_id']: r for r in source.policies(inputs[shared.CLIENT], xml_raw, IDS)}
    bindings = source.binding_index(inputs[shared.BINDINGS])
    profiles = [source.profile(i, xml[i], objects[i], physical[i], bindings, None) for i in sorted(IDS)]
    bound = source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': profiles}, bindings, IDS)['records']
    if profiles_only:
        return bound
    records = []
    for i, profile in zip(sorted(IDS), bound, strict=True):
        start = re.search(rb'<item\b[^>]*\bid="' + str(i).encode() + rb'"', xml_raw).start()
        records.append(dict(source_item_id=i, name=xml[i].get('name'), source_line=xml_raw[:start].count(b'\n') + 1,
            weight_evidence='xml_attribute' if 'weight' in source.attrs(xml[i]) else 'loader_default_zero',
            production_definition=profile['production_definition'], identity_binding=profile['production_binding'],
            candidate_admission=profile['admission'], semantics={g: profile['semantics'][g] for g in GROUPS}))
    return dict(schema='OTERYN_ROTTEN_BLOOD_ITEM_SOURCE_FACTS/v1', native_admission=False, use_effects_admitted=False,
        source={k: row[k] for k in ('repository', 'revision', 'path', 'sha256', 'git_blob_sha1')}, items=records)

if __name__ == '__main__':
    root = Path(__file__).resolve().parents[3]
    (root / OUTPUT).write_text(json.dumps(capture(root), sort_keys=True, indent=2) + '\n')
