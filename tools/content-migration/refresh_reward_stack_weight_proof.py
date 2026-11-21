"""Requalify sealed reward-stack definitions after a verified Global weight correction."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess

from import_cyclopedia_global import load_weights, only_weight_changed

ROOT = Path(__file__).resolve().parents[2]
OWNER = 'tools/content-schema/reward-claim-authoring/reward_claim_variant_authoring.py'
PROOF = 'tools/content-schema/reward-claim-authoring/reward_stack_normalization.json'
PIN = re.compile(r"(?m)^STACK_NORMALIZATION_PROOF_SHA256 = '([0-9a-f]{64})'$")


def semantic_digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False,
                                     separators=(',', ':')).encode()).hexdigest()


def qualified_digest(before, after, pinned, global_weight):
    if semantic_digest(before) != pinned:
        raise ValueError('REWARD_STACK_BASE_DEFINITION_DRIFT')
    if (not only_weight_changed(before, after)
            or after['semantics']['physical']['value']['weight']
            != {'state': 'KNOWN', 'value': global_weight}):
        raise ValueError('REWARD_STACK_NON_GLOBAL_WEIGHT_CHANGE')
    return semantic_digest(after)


def refresh(root=ROOT, base='origin/main'):
    owner = root / OWNER
    source = owner.read_text()
    match = PIN.search(source)
    raw = (root / PROOF).read_bytes()
    if match is None or hashlib.sha256(raw).hexdigest() != match[1]:
        raise ValueError('REWARD_STACK_PROOF_DRIFT')
    proof = json.loads(raw)
    weights = load_weights(root)
    selected = {case['item_identity']['key']: case for case in proof['cases']}
    current = {}
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key in selected:
                current[key] = (path, definition)
    updated = raw
    changed = []
    for case in proof['cases']:
        key = case['item_identity']['key']
        path, after = current[key]
        pinned = case['item_definition_sha256']
        if semantic_digest(after) == pinned:
            continue
        baseline = json.loads(subprocess.check_output(
            ['git', '-C', str(root), 'show', f'{base}:{path.relative_to(root)}']))
        before, = [row['definition'] for row in baseline['records']
                   if row['definition']['identity'] == case['item_identity']]
        desired = weights[int(key.rsplit('.i', 1)[1])]
        case['item_definition_sha256'] = qualified_digest(before, after, pinned, desired)
        updated = updated.replace(pinned.encode(), case['item_definition_sha256'].encode(), 1)
        changed.append(key)
    if not changed:
        return changed
    if json.loads(updated) != proof:
        raise ValueError('REWARD_STACK_PROOF_LAYOUT')
    digest = hashlib.sha256(updated).hexdigest()
    (root / PROOF).write_bytes(updated)
    owner.write_text(source[:match.start(1)] + digest + source[match.end(1):])
    Path(importlib.util.cache_from_source(str(owner))).unlink(missing_ok=True)
    return changed


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--base', default='origin/main')
    args = parser.parse_args()
    print('Reward stack weight requalification:', refresh(args.root, args.base))
