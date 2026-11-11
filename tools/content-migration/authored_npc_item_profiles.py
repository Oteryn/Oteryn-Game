"""Physical custody for explicit, pinned first-party NPC recipe inputs and rewards."""
import hashlib
import json

import authored_quest_item_custody as custody
import quest_batch3_item_profiles as references
from native_profile_composition import encode_native_profile

INTENT = 'tools/content-schema/native-gameplay/authored-npc-item-custody.json'
XML = 'imports/canary/items-xml/items.xml'


def admit(root, generated, profile_path, manifest_path, order, canonical):
    intent = json.loads((root / INTENT).read_bytes())
    if (intent.get('schema') != 'OTERYN_AUTHORED_NPC_ITEM_CUSTODY/v1'
            or not 1 <= len(intent.get('inputs', [])) <= 32):
        raise ValueError('AUTHORED_NPC_CUSTODY_INTENT')
    keys = set()
    for entry in intent['inputs']:
        path = root / entry['path']
        if not path.resolve().is_relative_to(root.resolve()):
            raise ValueError('AUTHORED_NPC_CUSTODY_PATH')
        raw = path.read_bytes()
        if hashlib.sha256(raw).hexdigest() != entry['sha256']:
            raise ValueError('AUTHORED_NPC_CUSTODY_AUTHORING_PIN')
        document = json.loads(raw)
        if document.get('quest') != entry['quest']:
            raise ValueError('AUTHORED_NPC_CUSTODY_QUEST')
        keys.update(key for key, _, _ in references.item_references(document))
    if not 1 <= len(keys) <= 128:
        raise ValueError('AUTHORED_NPC_CUSTODY_BOUNDS')
    definitions = {}
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            if definition['identity']['key'] in keys:
                if definition['identity']['key'] in definitions:
                    raise ValueError('AUTHORED_NPC_CUSTODY_DUPLICATE')
                definitions[definition['identity']['key']] = definition
    if set(definitions) != keys:
        raise ValueError('AUTHORED_NPC_CUSTODY_MISSING')
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    xml = (root / XML).read_bytes()
    document = json.loads(generated[profile_path])
    present = {row['production_definition']['production_key']: row for row in document['records']}
    new = custody.qualify_many(root, [definitions[key] for key in sorted(keys - present.keys())], xml)
    document['records'].extend(new)
    for key in sorted(keys):
        if key in present:
            custody.reuse_existing(definitions[key], present[key], bindings)
    document['records'].sort(key=order)
    raw = encode_native_profile(document)
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[profile_path] = raw
    generated[manifest_path] = canonical(manifest)
