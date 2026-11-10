"""Refresh only the SOURCE compiler pin after proving all frozen cores unchanged."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

import quest_tree_authoring as tree
import source_fix_guard as guard

COMPILER = 'tools/content-schema/quest-authoring/quest_tree_authoring.py'
# QMINT3 refresh: remove the duplicate Draconia live hold; SOURCE cores stay exact.
PREVIOUS = 'cb2c59f5fc1c3ea2a549e39a8c7764c7dfd25df200986a5d11c5191b62484aae'


def verify_source_view(root):
    quests = tree.read(root, tree.SOURCE)['quests']
    gates = tree.read(root, tree.GATES)['gates']
    current = tree.build_records(
        quests, tree.claim_catalogue(root),
        {r['quest']: r for r in tree.read(root, tree.READINESS)['quests']},
        gates, tree.read(root, tree.GATE_MANIFEST)['entries'],
        tree.bound_source_data(root, quests, gates))
    index = tree.read(root, tree.DIRECTORY + 'index.json')
    frozen = [copy.deepcopy(r) for path in index['shards']
              for r in tree.read(root, path)['records']
              if r['definition'].get('definition_profile') != 'oteryn_authored_v1']
    for row in frozen:
        row['definition'].pop('oteryn_recipe', None)
    if current != tree.source_projection(root, frozen):
        raise ValueError('ITEMHOLD amendment changes frozen SOURCE cores')


def outputs(root):
    approval = guard.read_json(root / guard.APPROVAL)
    receipt = guard.read_json(root / guard.RECEIPT)
    if receipt['approval']['path'] != guard.APPROVAL:
        raise ValueError('wrong SOURCE approval path')
    guard.verify_file(root, receipt['approval'])
    digest = hashlib.sha256((root / COMPILER).read_bytes()).hexdigest()
    for category in ('immutable_inputs', 'compiler_inputs', 'proof_inputs'):
        if approval[category] != receipt[category]:
            raise ValueError('SOURCE approval inputs differ from receipt')
        for descriptor in approval[category]:
            if category == 'compiler_inputs' and descriptor['path'] == COMPILER:
                if descriptor['sha256'] not in (PREVIOUS, digest):
                    raise ValueError('unrecognized prior SOURCE compiler pin')
            else:
                guard.verify_file(root, descriptor)
    verify_source_view(root)
    for document in (approval, receipt):
        matches = [d for d in document['compiler_inputs'] if d['path'] == COMPILER]
        if len(matches) != 1:
            raise ValueError('SOURCE compiler pin missing or duplicated')
        matches[0]['sha256'] = digest
    encode = lambda value: json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + '\n'
    approval_text = encode(approval)
    receipt['approval']['sha256'] = hashlib.sha256(approval_text.encode()).hexdigest()
    return {guard.APPROVAL: approval_text, guard.RECEIPT: encode(receipt)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[3]
    for path, text in outputs(root).items():
        if args.check:
            if (root / path).read_text() != text:
                raise ValueError('stale ITEMHOLD SOURCE compiler pin: ' + path)
        else:
            (root / path).write_text(text, encoding='utf-8')
    guard.verified_receipt(root)
    print('ITEMHOLD SOURCE compiler pin current; all frozen SOURCE cores unchanged')


if __name__ == '__main__':
    main()
