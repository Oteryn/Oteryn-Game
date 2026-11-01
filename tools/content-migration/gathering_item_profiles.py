"""Admit exactly the Crystal operands reached by the FIXM09 gathering dispatcher."""
import hashlib
import json

PATH = 'tools/content-schema/native-gameplay/gathering-item-profiles.json'
IDS = {629, 630, 631, 632, 633, 634, 953, 3580, 4012, 4026, 4063, 4098,
       4138, 4325, 4598, 4599, 4600, 4601, 4602, 5876, 5877, 5878, 5905,
       5906, 5908, 5942, 5948, 5969, 5973, 5984, 5995, 6006, 6040, 7158,
       7159, 7200, 7236, 7237, 7441, 7442, 7444, 7445, 7446, 9306, 9307, 21312, 21314}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = [row['production_definition']['production_key'] for row in records]
    if (overlay['schema'] != 'OTERYN_NATIVE_GATHERING_ITEM_PROFILES/v1'
            or overlay['source_revision'] != '201189f871d23597b82caf88fd611454a2ddefb8'
            or len(keys) != len(IDS) or set(keys) != {f'oteryn:item.tibia.i{i}' for i in IDS}):
        raise ValueError('GATHERING_ITEM_PROFILE_SET')
    document = json.loads(generated[profile_path])
    if {row['production_definition']['production_key'] for row in document['records']} & set(keys):
        raise ValueError('GATHERING_ITEM_PROFILE_DUPLICATE')
    document['records'] = sorted(document['records'] + records, key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
