"""Register exactly five source-qualified Magnolia custody profiles."""
import hashlib
import json
from pathlib import Path
import sys
PACKET = 'tools/content-schema/native-gameplay/candidate-bindings/srv7-summer-amulets.json'
IDS = {53197, 53199, 53201, 53203, 53205}
def admit(root, generated, profile_path, manifest_path, order, canonical):
    sys.path.insert(0, str(root / 'tools/content-schema/native-gameplay'))
    from build_spell_item_identities import BINDINGS, binding_index, bind_profiles
    raw = (root / PACKET).read_bytes()
    if hashlib.sha256(raw).hexdigest() != (root / Path(PACKET).with_suffix('.sha256')).read_text().strip():
        raise ValueError('SUMMER_AMULET_PACKET_DIGEST')
    packet = json.loads(raw)
    if packet['schema'] != 'OTERYN_SRV7_SUMMER_AMULET_IDENTITIES/v1' or {int(b['external_id']) for b in packet['bindings']} != IDS or len(packet['bindings']) != 5:
        raise ValueError('SUMMER_AMULET_EXACT_FIVE')
    document = json.loads(generated[profile_path])
    existing = {record['production_definition']['production_key']: record for record in document['records']}
    bound = bind_profiles({"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": packet["profiles"]},
                          binding_index(BINDINGS.read_bytes()), set(), summer_amulet_candidate=packet)
    for row in bound["records"]:
        key = row['production_definition']['production_key']
        if key in existing:
            if existing[key] != row:
                raise ValueError('SUMMER_AMULET_EXISTING_IDENTITY_CONFLICT')
        else:
            document['records'].append(row)
    document['records'].sort(key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
