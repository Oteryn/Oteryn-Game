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
import re
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


OVERLAY_DIR = 'tools/content-schema/native-gameplay'
WEIGHT_PATTERN = re.compile(rb'("weight":\s*\{"state":\s*"KNOWN",\s*"value":\s*)(\d+)(\})')


def _overlay_profiles(node):
    """Yield (item id, weight leaf) for every overlay record carrying a KNOWN physical weight."""
    if isinstance(node, dict):
        key = (node.get('production_definition') or {}).get('production_key')
        physical = (node.get('semantics') or {}).get('physical')
        if (isinstance(key, str) and key.startswith('oteryn:item.tibia.i')
                and isinstance(physical, dict) and physical.get('state') == 'KNOWN'
                and (physical.get('value') or {}).get('weight', {}).get('state') == 'KNOWN'):
            yield int(key.rsplit('.i', 1)[1]), physical['value']['weight']
        for value in node.values():
            yield from _overlay_profiles(value)
    elif isinstance(node, list):
        for value in node:
            yield from _overlay_profiles(value)


def apply_profile_weights(records, root):
    """Give profile records read from a faithful source capture the client weight."""
    weights = load_weights(root)
    for identifier, weight in _overlay_profiles(records):
        weight['value'] = weights.get(identifier, weight['value'])


def apply_overlay_weights(root, weights):
    """Give native-gameplay overlay profiles the client weight, changing only weight values.

    Each overlay keeps its own byte layout: a record's weight is replaced on the
    line that names its key, and the parsed result must equal the expected edit."""
    changed = {}
    for path in sorted((root / OVERLAY_DIR).glob('*.json')):
        raw = path.read_bytes()
        document = json.loads(raw)
        edits = {}
        for identifier, weight in _overlay_profiles(document):
            if identifier in weights and weights[identifier] != weight['value']:
                edits.setdefault(identifier, set()).add(weight['value'])
                weight['value'] = weights[identifier]
        if not edits:
            continue
        if raw == (json.dumps(json.loads(raw), indent=2, ensure_ascii=False) + '\n').encode():
            updated = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
        else:
            lines = raw.split(b'\n')
            for identifier in edits:
                needle = f'"oteryn:item.tibia.i{identifier}"'.encode()
                hits = [n for n, line in enumerate(lines) if needle in line]
                for n in hits:
                    lines[n] = WEIGHT_PATTERN.sub(
                        lambda m: m.group(1) + str(weights[identifier]).encode() + m.group(3)
                        if int(m.group(2)) in edits[identifier] else m.group(0), lines[n])
            updated = b'\n'.join(lines)
        if json.loads(updated) != document:
            raise ValueError('CLIENT_WEIGHT_OVERLAY_LAYOUT:' + path.name)
        path.write_bytes(updated)
        changed[path.name] = len(edits)
    return changed


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
