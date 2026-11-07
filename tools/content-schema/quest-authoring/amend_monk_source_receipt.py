"""Rebuild the owner-approved MONKQUEST3 native annotation pair and cumulative receipt, offline.

The authored evidence is the approval input; this tool never selects new evidence
or approves arbitrary core changes. Existing immutable/proof inputs stay pinned.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path

import source_fix_guard as guard
from source_monk_native_guard import CORE, EVIDENCE, reviewed_pair

HERE = 'tools/content-schema/quest-authoring/'
COMPILERS = (
    'source_fix_guard.py', 'test_source_fix_guard.py', 'source_monk_native_guard.py',
    'test_source_monk_native_guard.py', 'amend_monk_source_receipt.py',
    'quest_tree_authoring.py', 'quest_tree.schema.json', 'test_monk_native_lowering.py',
    'test_quest_completion_authoring.py',
)


def outputs(root):
    approval = guard.read_json(root / guard.APPROVAL)
    receipt = guard.read_json(root / guard.RECEIPT)
    # No unrelated compiler or evidence refresh is permitted by this amendment.
    mutable = {HERE + name for name in COMPILERS}
    mutable_proofs = {EVIDENCE, 'content/quests/world-callbacks.json',
                      'content/quests/monk-pilgrimage.json'}
    for category in ('immutable_inputs', 'compiler_inputs', 'proof_inputs'):
        if approval[category] != receipt[category]:
            raise ValueError('existing approved inputs differ from receipt')
        for descriptor in approval[category]:
            if descriptor['path'] not in mutable | mutable_proofs:
                guard.verify_file(root, descriptor)
    current = {}
    annotation_files = {}
    for path in guard.read_json(root / 'content/quests/definitions/index.json')['shards']:
        shard = guard.read_json(root / path)
        for row in shard['records']:
            core = copy.deepcopy(row['definition'])
            core.pop('oteryn_recipe', None)
            if core['identity']['key'] == CORE:
                from quest_tree_authoring import monk_native_lowering, compact
                core['native_lowering'] = monk_native_lowering(root)
                row['definition']['native_lowering'] = copy.deepcopy(core['native_lowering'])
                annotation_files[path] = compact(shard)
            current[core['identity']['key']] = core
    packet = guard.read_json(root / EVIDENCE)
    new = current[CORE]
    from quest_tree_authoring import monk_native_lowering
    if new['native_lowering'] != monk_native_lowering(root):
        raise ValueError('Monk annotation differs from callback compilation')
    old = copy.deepcopy(new)
    old['native_lowering'] = packet['old_native_lowering']
    left, right = reviewed_pair(root, old, new)
    if left != right:
        raise ValueError('Monk amendment changes SOURCE fields')
    baseline = guard.read_json(root / (guard.DIRECTORY + 'baseline-core-digests.json'))
    if guard.digest(old) != baseline[CORE]:
        raise ValueError('Monk original core differs from immutable baseline')
    signature = {'key': CORE, 'from_digest': guard.digest(old), 'to_digest': guard.digest(new)}
    approval['approved_core_digests'] = sorted(
        [r for r in approval['approved_core_digests'] if r['key'] != CORE] + [signature],
        key=lambda r: r['key'])
    receipt['changes'] = sorted(
        [r for r in receipt['changes'] if r['key'] != CORE]
        + [dict(signature, old_core=old, new_core=new)], key=lambda r: r['key'])
    def descriptor(path):
        return {'path': path, 'sha256': hashlib.sha256((root / path).read_bytes()).hexdigest()}
    compilers = {r['path']: r for r in approval['compiler_inputs']}
    compilers.update({path: descriptor(path) for path in mutable})
    approval['compiler_inputs'] = [compilers[p] for p in sorted(compilers)]
    proofs = {r['path']: r for r in approval['proof_inputs']}
    for path in sorted(mutable_proofs):
        proofs[path] = descriptor(path)
    approval['proof_inputs'] = [proofs[p] for p in sorted(proofs)]
    for category in ('compiler_inputs', 'proof_inputs'):
        receipt[category] = copy.deepcopy(approval[category])
    encode = lambda value: json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + '\n'
    approval_text = encode(approval)
    receipt['approval'] = {'path': guard.APPROVAL,
                           'sha256': hashlib.sha256(approval_text.encode()).hexdigest()}
    return dict(annotation_files, **{guard.APPROVAL: approval_text, guard.RECEIPT: encode(receipt)})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[3]
    for path, content in outputs(root).items():
        if args.check:
            if (root / path).read_text() != content:
                raise ValueError('stale Monk amendment: ' + path)
        else:
            (root / path).write_text(content, encoding='utf-8')
    baseline = guard.read_json(root / (guard.DIRECTORY + 'baseline-core-digests.json'))
    receipt, approval = guard.verified_receipt(root)
    for change in receipt['changes']:
        guard.validate_change(change, baseline, change['new_core'], approval, root)
    print('28 exact reviewed cores; Monk native annotation ready; original SOURCE and recipe admission unchanged')


if __name__ == '__main__':
    main()
