"""P18: carry existing Item Forge owners into the active native provider.

The materialized Item tree owns classification and maximum tier. This overlay
does not infer either from an equipment slot, create identities or admit new
equipment. It projects only already admitted native records. Where their old
equipment metadata is UNKNOWN, retain the accepted Item's explicit pattern,
including every remaining UNKNOWN restriction, so Forge can read its slot.
"""
import copy
import hashlib
import json


def owners(root):
    result = {}
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            authoring = row.get('authoring', {})
            if 'forge' not in authoring:
                continue
            identity = row['definition']['identity']
            # The materializer embeds the owner in its definition and omits the
            # redundant `item` field; retained explicit bindings must still agree.
            if (authoring.get('item', identity) != identity
                    or identity['key'] in result):
                raise ValueError('FORGE_NATIVE_OWNER_IDENTITY')
            result[identity['key']] = row
    return result


def project(document, admitted):
    for record in document['records']:
        key = record['production_definition']['production_key']
        owner = admitted.get(key)
        if owner is None:
            if 'forge' in record['authoring']:
                raise ValueError('FORGE_NATIVE_UNOWNED_PROFILE')
            continue
        identity = owner['definition']['identity']
        if record['production_binding']['target'] != identity:
            raise ValueError('FORGE_NATIVE_BINDING_CONFLICT')
        profile = owner['authoring']['forge']
        existing = record['authoring'].get('forge')
        if existing is not None and existing != profile:
            raise ValueError('FORGE_NATIVE_OWNER_CONFLICT')
        record['authoring']['forge'] = copy.deepcopy(profile)
        equipment = owner['definition']['semantics']['equipment']
        if (record['semantics']['equipment'] == {'state': 'UNKNOWN'}
                and equipment.get('state') == 'KNOWN'):
            record['semantics']['equipment'] = copy.deepcopy(equipment)


def apply_native(root, generated, path, manifest_path, canonical):
    document = json.loads(generated[path])
    project(document, owners(root))
    data = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
