#!/usr/bin/env python3
"""Import captured Global weights into canonical and cooking Item semantics.

Weights are hundredths of an ounce. Alias records resolve through the capture's
explicit details target; constructor declarations follow the same client facts.
"""
import argparse
from collections import Counter
from decimal import Decimal
import hashlib
import json
from pathlib import Path


def load_weights(root):
    """Return client weights (hundredths of an ounce) by Item id from the verified capture."""
    capture = root / 'docs/reference/content-prep/item-cyclopedia-capture-1'
    raw = (capture / 'items.jsonl').read_bytes()
    manifest = json.loads((capture / 'manifest.json').read_bytes())
    if hashlib.sha256(raw).hexdigest() != manifest['items_jsonl_sha256']:
        raise ValueError('Cyclopedia capture digest mismatch')
    records = [json.loads(line) for line in raw.splitlines()]
    rows = {row['id']: row for row in records}
    if len(rows) != len(records) or len(rows) != manifest['records']:
        raise ValueError('Cyclopedia record count or duplicate ID')
    weights = {}
    for identifier, row in rows.items():
        seen = set()
        while 'details_from_alias_id' in row['compatibility']:
            target = row['compatibility']['details_from_alias_id']
            if target in seen:
                raise ValueError('Cyclopedia alias cycle')
            seen.add(target)
            row = rows[target]
        if row['compatibility']['target'] != 'tibia-global':
            raise ValueError('Unexpected compatibility target')
        value = Decimal(row['basic']['Weight'].removesuffix(' oz')) * 100
        if value != int(value) or not 0 <= value <= 0xffffffff:
            raise ValueError('Unrepresentable weight')
        weights[identifier] = int(value)
    return weights


def apply_weight(definition, weights):
    """Set a KNOWN physical weight to the client value; return True when it changed."""
    identity = definition.get('identity', definition.get('authoring', {}).get('item', {}))
    key = identity.get('key', '')
    prefix = 'oteryn:item.tibia.i' if key.startswith('oteryn:item.tibia.i') else 'candidate:item/'
    if not key.startswith(prefix) or not key[len(prefix):].isdigit():
        return False
    identifier = int(key[len(prefix):])
    physical = definition.get('semantics', {}).get('physical', {})
    if identifier not in weights or physical.get('state') != 'KNOWN':
        return False
    desired = {'state': 'KNOWN', 'value': weights[identifier]}
    if physical['value'].get('weight') == desired:
        return False
    physical['value']['weight'] = desired
    return True


def only_weight_changed(before, after):
    """True when after differs from before at most in a KNOWN physical weight leaf."""
    try:
        weight = after['semantics']['physical']['value']['weight']
        if (after['semantics']['physical']['state'] != 'KNOWN' or weight.get('state') != 'KNOWN'
                or type(weight.get('value')) is not int or not 0 <= weight['value'] <= 0xffffffff
                or before['semantics']['physical']['state'] != 'KNOWN'):
            return False
        expected = json.loads(json.dumps(before))
        expected['semantics']['physical']['value']['weight'] = weight
        return expected == after
    except (KeyError, TypeError, AttributeError):
        return False


def apply_admissions(item_records, root):
    """Client weights are tree-first; retain them across legacy regeneration."""
    weights = load_weights(root)
    for definition in item_records:
        apply_weight(definition, weights)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()
    root = args.root
    weights = load_weights(root)
    counts = Counter()
    constructors = []

    def write(path, document):
        previous = path.read_text()
        # Preserve the canonical compact shard representation.
        compact = '\n' not in previous.rstrip('\n')
        options = {'separators': (',', ':'), 'sort_keys': True} if compact else {'indent': 2}
        path.write_text(json.dumps(document, ensure_ascii=False, **options) + '\n')

    paths = sorted((root / 'content/items/definitions').glob('items-*.json'))
    paths.append(root / 'content/craft/runtime-item-profiles.json')
    for path in paths:
        document = json.loads(path.read_bytes())
        changed = False
        for record in document['records']:
            if apply_weight(record.get('definition', record), weights):
                counts[str(path.relative_to(root))] += 1
                changed = True
        if changed:
            write(path, document)
    path = root / 'tools/content-schema/native-gameplay/authored-static-item-constructors.json'
    document = json.loads(path.read_bytes())
    for declaration in document['constructors']:
        identifier = int(declaration['item'].removeprefix('oteryn:item.tibia.i'))
        if identifier in weights and declaration['weight'] != weights[identifier]:
            constructors.append([identifier, declaration['weight'], weights[identifier]])
            declaration['weight'] = weights[identifier]
    if constructors:
        write(path, document)
    print(json.dumps({'weight_changes_by_file': counts, 'constructor_weights': constructors}, indent=2))


if __name__ == '__main__':
    main()
