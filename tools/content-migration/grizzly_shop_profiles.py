"""Qualify only Grizzly's canonical shop keys through the native Item importer.

PR2770 Q12 authorizes this finite source qualification. Accepted repository
facts win, canonical identities/holds stay intact, and conflicting or unsupported
source rows stay rejected. The active Item provider is the trade authority.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source

PATH = 'tools/content-schema/native-gameplay/grizzly-shop-profiles.json'
CATALOGUE = 'content/quests/killing-in-the-name-of.json'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
SOURCE_HASHES = {
    'items.xml': '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
    'appearances.dat': '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
}
# These records are products, trophies, a taming item and one backpack. No
# charge, decay, transformation or custom use rule is inferred for a shop Item.
ATTRIBUTES = {'weight', 'primarytype', 'description', 'showcount',
              'containersize', 'imbuementslot', 'script'}


def encode(value):
    return (json.dumps(value, indent=2, ensure_ascii=False) + '\n').encode()


def selected(root):
    raw = (root / CATALOGUE).read_bytes()
    catalogue = json.loads(raw)
    keys = {offer['item']['key'] for offer in catalogue['shop']}
    if catalogue['source_revision'] != REVISION or len(keys) != 56:
        raise ValueError('GRIZZLY_SHOP_CATALOGUE_SET')
    return keys, hashlib.sha256(raw).hexdigest()


def known_conflict(accepted, observed):
    """Only an accepted fact can disagree; UNKNOWN never authorizes a value."""
    if isinstance(accepted, dict):
        if accepted.get('state') == 'UNKNOWN':
            return False
        if accepted.get('state') == 'NOT_APPLICABLE':
            return observed != accepted
        if not isinstance(observed, dict):
            return True
        return any(key not in observed or known_conflict(value, observed[key])
                   for key, value in accepted.items())
    if isinstance(accepted, list):
        return accepted != observed
    return accepted != observed


def accepted_first(accepted, observed):
    if isinstance(accepted, list):
        # Only the equipment pattern has an explicit stable identity for joining
        # partial records. Other accepted vectors keep their original meaning.
        if (isinstance(observed, list) and len(accepted) == len(observed)
                and all(isinstance(a, dict) and isinstance(b, dict)
                        and 'pattern_id' in a and a['pattern_id'] == b.get('pattern_id')
                        for a, b in zip(accepted, observed))):
            return [accepted_first(a, b) for a, b in zip(accepted, observed)]
        return copy.deepcopy(accepted)
    if not isinstance(accepted, dict):
        return copy.deepcopy(accepted)
    if accepted.get('state') == 'UNKNOWN':
        return copy.deepcopy(observed)
    if not isinstance(observed, dict) or accepted.get('state') == 'NOT_APPLICABLE':
        return copy.deepcopy(accepted)
    if accepted.get('state') == 'KNOWN':
        if observed.get('state') != 'KNOWN':
            return copy.deepcopy(accepted)
        return source.K(accepted_first(accepted['value'], observed['value']))
    # An ordinary semantic payload is not a fact wrapper. Never seed its fields
    # with UNKNOWN's `state`, which the native payload structs correctly reject.
    if 'state' in observed:
        observed = {}
    result = copy.deepcopy(observed)
    for key, value in accepted.items():
        result[key] = (accepted_first(value, observed[key]) if key in observed
                       else copy.deepcopy(value))
    return result


def qualify(item_id, node, obj, physical, bindings, definition):
    key = definition['identity']['key']
    binding = bindings.get(str(item_id))
    if binding is None or binding['target'] != definition['identity'] or binding['disposition'] != 'EXACT':
        raise ValueError('MISSING_EXACT_BINDING')
    attributes = source.attrs(node)
    if set(attributes) - ATTRIBUTES:
        raise ValueError('UNSUPPORTED_SOURCE_ATTRIBUTES:' + ','.join(sorted(set(attributes) - ATTRIBUTES)))
    script = node.find("attribute[@key='script']")
    if script is not None and (item_id != 9601 or
            [(a.get('key'), a.get('value')) for a in script] != [('slot', 'backpack')]):
        raise ValueError('UNSUPPORTED_SOURCE_SCRIPT')
    row = source.profile(item_id, node, obj, physical, bindings, None)
    if (row['admission']['legal_destinations'] != ['CharacterInventory', 'Ground']
            or row['semantics']['temporal'] != source.NA
            or row['semantics']['charges'] != source.NA):
        raise ValueError('UNSUPPORTED_SHOP_CUSTODY')
    for group in ('physical', 'stack', 'container'):
        if known_conflict(definition['semantics'][group], row['semantics'][group]):
            raise ValueError('CANONICAL_' + group.upper() + '_CONFLICT')
    if (definition['stack_class'] != 'Unknown'
            and definition['stack_class'] != row['admission']['stack_class']):
        raise ValueError('CANONICAL_STACK_CLASS_CONFLICT')
    # Preserve accepted presentation, equipment and imbuement facts. Unknown
    # groups remain unknown unless the existing native importer qualifies them.
    row['semantics'] = accepted_first(definition['semantics'], row['semantics'])
    result = source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1',
                                  'records': [row]}, bindings, {item_id})['records'][0]
    if result['production_definition']['production_key'] != key:
        raise ValueError('CANONICAL_IDENTITY_DRIFT')
    return result


def capture(source_root, root=ROOT):
    selected_keys, catalogue_sha = selected(root)
    raw = {name: (source_root / name).read_bytes() for name in SOURCE_HASHES}
    if any(hashlib.sha256(raw[name]).hexdigest() != digest for name, digest in SOURCE_HASHES.items()):
        raise ValueError('GRIZZLY_SHOP_SOURCE_PIN')
    xml = source.xml_index(raw['items.xml'])
    objects = source.engine_items.load_appearance_objects(raw['appearances.dat'])
    ids = {int(key.rsplit('i', 1)[1]) for key in selected_keys}
    physical = {r['server_item_id']: r for r in source.policies(raw['appearances.dat'], raw['items.xml'], ids)}
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    definitions, inputs = {}, []
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        matches = {r['definition']['identity']['key']: r['definition']
                   for r in json.loads(path.read_bytes())['records']
                   if r['definition']['identity']['key'] in selected_keys}
        if matches:
            if definitions.keys() & matches.keys():
                raise ValueError('GRIZZLY_SHOP_CANONICAL_DUPLICATE')
            definitions.update(matches)
            inputs.append({'path': path.relative_to(root).as_posix(),
                           'sha256': hashlib.sha256(path.read_bytes()).hexdigest()})
    records, rejected = [], []
    for item_id in sorted(ids):
        key = f'oteryn:item.tibia.i{item_id}'
        try:
            if key not in definitions or item_id not in xml or item_id not in objects:
                raise ValueError('MISSING_SOURCE_OR_CANONICAL_ITEM')
            record = qualify(item_id, xml[item_id], objects[item_id], physical[item_id], bindings, definitions[key])
            records.append(record)
        except ValueError as error:
            rejected.append({'key': key, 'reason': str(error)})
    return {'schema': 'OTERYN_NATIVE_GRIZZLY_SHOP_PROFILES/v1',
            'producer': 'tools/content-migration/grizzly_shop_profiles.py',
            'source_revision': REVISION,
            'sources': [{'path': 'data/items/' + name, 'sha256': digest}
                        for name, digest in SOURCE_HASHES.items()],
            'catalogue': {'path': CATALOGUE, 'sha256': catalogue_sha},
            'canonical_inputs': inputs, 'records': records, 'rejected': rejected}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    keys, catalogue_sha = selected(root)
    records = overlay['records']
    admitted = {r['production_definition']['production_key'] for r in records}
    rejected = {r['key'] for r in overlay['rejected']}
    if (overlay['schema'] != 'OTERYN_NATIVE_GRIZZLY_SHOP_PROFILES/v1'
            or overlay['source_revision'] != REVISION
            or overlay['sources'] != [{'path': 'data/items/' + name, 'sha256': digest}
                                      for name, digest in SOURCE_HASHES.items()]
            or overlay['catalogue'] != {'path': CATALOGUE, 'sha256': catalogue_sha}
            or admitted & rejected or admitted | rejected != keys
            or len(admitted) != len(records) or len(rejected) != len(overlay['rejected'])):
        raise ValueError('GRIZZLY_SHOP_PROFILE_SET')
    for descriptor in overlay['canonical_inputs']:
        if hashlib.sha256((root / descriptor['path']).read_bytes()).hexdigest() != descriptor['sha256']:
            raise ValueError('GRIZZLY_SHOP_CANONICAL_DRIFT')
    document = json.loads(generated[profile_path])
    present = {r['production_definition']['production_key']: r for r in document['records']}
    additions = []
    for record in records:
        key = record['production_definition']['production_key']
        existing = present.get(key)
        if existing is None:
            additions.append(record)
        elif (any(existing[field] != record[field] for field in
                  ('production_definition', 'production_binding', 'admission'))
              or any(known_conflict(existing['semantics'][field], record['semantics'][field])
                     for field in ('physical', 'stack', 'container'))):
            raise ValueError('GRIZZLY_SHOP_EXISTING_PROFILE_CONFLICT:' + key)
    document['records'] = sorted(document['records'] + additions, key=order)
    data = encode(document)
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = encode(capture(args.source_root))
    target = ROOT / PATH
    if args.check:
        if target.read_bytes() != raw:
            raise SystemExit('GRIZZLY_SHOP_PROFILE_DRIFT')
    else:
        target.write_bytes(raw)
    result = json.loads(raw)
    print(json.dumps({'admitted': len(result['records']), 'rejected': result['rejected']}))
