#!/usr/bin/env python3
"""Derive the closed Hare2096/Golem6393 presentation projection from real immutable index evidence."""
import argparse
import hashlib
import json
from pathlib import Path

ITEM_IDS = (2096, 6393)

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def load(path, maximum=8 * 1024 * 1024):
    raw = path.read_bytes()
    if len(raw) > maximum:
        raise ValueError('projection source size bound')
    return json.loads(raw), raw

def build(root, export, client_artifact, golem_export):
    packet, packet_raw = load(export)
    source = packet.get('source', packet)
    golem, _ = load(golem_export)
    body = golem['source']
    if golem.get('schema') != 'OTERYN_SEASONAL_GOLEM_NATIVE_CAPTURE/v1' or body.get('production_source_world_preserved') is not True:
        raise ValueError('actual production Golem body export')
    if body['client_content_generation'] != source['client_content_generation'] or body['item_keys_sha256'] != source['item_keys_sha256']:
        raise ValueError('seasonal body exports must use the same independently qualified client/key generation')
    # The exporter must use runtime.content_pin().client_artifact_digest(),
    # never the unrelated NativeGameplayState::source_digest().
    generation = source['client_content_generation']
    if not isinstance(generation, str) or len(generation) != 64 or bytes.fromhex(generation).hex() != generation:
        raise ValueError('actual client content generation')
    artifact = client_artifact.read_bytes()
    if not 80 < len(artifact) <= 8 * 1024 * 1024 or not artifact.startswith(b'OTNCW03\0'):
        raise ValueError('actual bounded production source-world client artifact')
    # Native source-world carrier3: exactly two independently hashed public
    # sections. Its generation is SHA256 of the WHOLE carrier, not a reference
    # artifact trailer or a native authoritative policy digest.
    cursor = 8
    for _ in range(2):
        if cursor + 36 > len(artifact):
            raise ValueError('truncated compiled public carrier')
        size = int.from_bytes(artifact[cursor:cursor + 4], 'big')
        digest = artifact[cursor + 4:cursor + 36]
        cursor += 36
        data = artifact[cursor:cursor + size]
        if size == 0 or len(data) != size or hashlib.sha256(data).digest() != digest:
            raise ValueError('compiled public section integrity')
        cursor += size
    if cursor != len(artifact) or sha(artifact) != generation:
        raise ValueError('actual compiled client artifact does not prove the captured generation')
    keys_path = root / 'tools/content-schema/native-gameplay/item-keys.json'
    keys, keys_raw = load(keys_path)
    if keys['schema'] != 'OTERYN_NATIVE_ITEM_KEYS/v1' or sha(keys_raw) != source['item_keys_sha256']:
        raise ValueError('actual Native artifact Item-key pin')
    rows = keys['records']
    if any(len(row) != 2 or not all(isinstance(s, str) for s in row) for row in rows):
        raise ValueError('typed Item key shape')
    if any(rows[i][0].encode() >= rows[i + 1][0].encode() for i in range(len(rows) - 1)):
        raise ValueError('MAP-ITEM-REF-1 requires strictly ascending unique keys')
    admitted_path = root / 'imports/official/appearance-membership/admitted.json'
    admitted, admitted_raw = load(admitted_path)
    newest = next(row for row in admitted['files'] if row['label'] == admitted['newest'])
    membership_path = admitted_path.parent / newest['manifest']
    membership, membership_raw = load(membership_path)
    if sha(membership_raw) != newest['manifest_sha256'] or membership['entry_columns'] != ['id', 'identity_projection_sha256', 'record_sha256']:
        raise ValueError('official appearance membership pin')
    aliases, aliases_raw = load(root / 'content/items/aliases.json', 32 * 1024 * 1024)
    items = []
    exported_refs = {}
    for item_id in ITEM_IDS:
        key = f'oteryn:item.tibia.i{item_id}'
        found = [(index + 1, row[1]) for index, row in enumerate(rows) if row[0] == key]
        if len(found) != 1:
            raise ValueError('closed seasonal Item absent from pinned Native keyset')
        reference, revision = found[0]
        exported = (source if item_id == 2096 else body).get(f'item_{item_id}_definition_ref')
        if exported is None or reference != exported:
            raise ValueError('actual exported indexed reference disagrees with immutable key set')
        exported_refs[str(item_id)] = reference
        appearances = [row for row in membership['entries'] if row[0] == item_id]
        if len(appearances) != 1:
            raise ValueError('closed seasonal appearance unavailable in current official client')
        evidence = [row for row in aliases['entries'] if row.get('state') == 'ALIAS' and row.get('target') == key
                    and row.get('evidence', {}).get('source_item_id') == item_id
                    and row.get('evidence', {}).get('identity_projection_sha256') == appearances[0][1]]
        if not evidence:
            raise ValueError('closed donor/current-client identity binding missing')
        items.append({'definition_ref': reference, 'definition_key': key,
                      'definition_revision': revision, 'appearance_id': appearances[0][0],
                      'appearance_identity_sha256': appearances[0][1]})
    items.sort(key=lambda item: item['definition_ref'])
    result = {
        'schema': 'OTERYN_SEASONAL_ITEM_APPEARANCE_PROJECTION/v1',
        'client_content_generation': generation,
        'items': items,
        'sources': {
            'export_facts_sha256': sha(json.dumps({'client_content_generation': generation, 'item_keys_sha256': source['item_keys_sha256'], 'item_definition_refs': exported_refs}, sort_keys=True, separators=(',', ':')).encode()),
            'golem_export_facts_sha256': sha(json.dumps({'client_content_generation': body['client_content_generation'], 'item_keys_sha256': body['item_keys_sha256'], 'item_6393_definition_ref': body['item_6393_definition_ref'], 'producer_test': body['producer_test']}, sort_keys=True, separators=(',', ':')).encode()),
            'client_artifact_file_sha256': sha(artifact),
            'item_keys_sha256': sha(keys_raw),
            'appearance_admission_sha256': sha(admitted_raw),
            'current_membership_sha256': sha(membership_raw),
            'aliases_sha256': sha(aliases_raw),
            'client_release': admitted['newest'],
        },
    }
    return (json.dumps(result, indent=2, sort_keys=True) + '\n').encode()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('export', type=Path, help='actual runtime/compiled-client projection export')
    parser.add_argument('output', type=Path)
    parser.add_argument('--golem-export', type=Path, required=True, help='actual independent production Golem body export')
    parser.add_argument('--client-artifact', type=Path, required=True, help='independently qualified actual production client artifact')
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = build(args.root, args.export, args.client_artifact, args.golem_export)
    if args.check:
        if args.output.read_bytes() != raw:
            raise ValueError('closed Hare projection drift')
        if args.output.with_suffix(args.output.suffix + '.sha256').read_text().strip() != sha(raw):
            raise ValueError('closed seasonal projection SHA pin drift')
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(raw)
        args.output.with_suffix(args.output.suffix + '.sha256').write_text(sha(raw) + '\n')
    print('closed artifact-derived seasonal item projection ' + ('verified' if args.check else 'generated'))
if __name__ == '__main__':
    main()
