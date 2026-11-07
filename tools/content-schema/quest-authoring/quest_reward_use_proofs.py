"""Import real-dispatch PostgreSQL completion markers for reward-only quests.

Proofs qualify the existing bundle USE / durable once-claim mint owner. They do
not assert source fidelity, map coverage, or full quest walkthrough playability.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re

PROOFS = 'tools/content-schema/quest-authoring/samples/completion-matrix/reward-use-proofs'


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def load(root, claims):
    qualified = set()
    for path in sorted((root / PROOFS).glob('*.json')):
        proof = json.loads(path.read_text())
        if proof['schema'] != 'OTERYN_QUEST_REWARD_USE_PROOF/v1':
            raise ValueError('Unknown reward USE proof: ' + str(path))
        test = root / proof['test_file']
        if hashlib.sha256(test.read_bytes()).hexdigest() != proof['test_sha256']:
            raise ValueError('Reward USE test changed; rerun PostgreSQL: ' + str(test))
        source = test.read_text()
        for case in proof['cases']:
            key = case['claim_key']
            if (case['duration_seconds'] < 0.01 or
                    case['claim_sha256'] != digest(claims[key]) or
                    'fn ' + case['test'] + '()' not in source or
                    case['marker'] not in source):
                raise ValueError('Stale or skipped reward USE proof: ' + key)
            qualified.add(key)
    return qualified


def import_log(root, packet, log, batch=None):
    from quest_completion_matrix import load_reward_claims
    claims = load_reward_claims(root)
    source = root / packet['test_file']
    text = log.read_text()
    count = len(packet['cases'])
    if not re.search(rf'test result: ok\. {count} passed; 0 failed;', text):
        raise ValueError(f'Expected all {count} QREADY2 PostgreSQL cases to pass')
    cases = []
    titles = list(dict.fromkeys(case['wiki_title'] for case in packet['cases']))
    selected = titles if batch is None else titles[([0, 6, 12][batch-1]):([6, 12, 17][batch-1])]
    for case in packet['cases']:
        if case['wiki_title'] not in selected:
            continue
        marker = 'QREADY2_' + case['test'].removeprefix('qready2_').upper()
        matches = re.findall(r'POSTGRES_CASE_COMPLETED ' + re.escape(marker) + r' duration=([0-9.]+)s', text)
        if len(matches) != 1 or float(matches[0]) < 0.01:
            raise ValueError('Missing, duplicated or skipped PostgreSQL case: ' + marker)
        cases.append({**case, 'marker': marker, 'duration_seconds': float(matches[0]),
                      'claim_sha256': digest(claims[case['claim_key']])})
    return {'schema': 'OTERYN_QUEST_REWARD_USE_PROOF/v1',
            'runtime_path': 'bundle_chest::use_bundle_chest -> reward_claim_mint',
            'test_file': packet['test_file'],
            'test_sha256': hashlib.sha256(source.read_bytes()).hexdigest(), 'cases': cases}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--packet', type=Path, required=True)
    parser.add_argument('--log', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--batch', type=int, choices=(1, 2, 3))
    args = parser.parse_args()
    proof = import_log(args.root, json.loads(args.packet.read_text()), args.log, args.batch)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(proof, indent=2) + '\n')
    print(f"Qualified {len(proof['cases'])} real-dispatch PostgreSQL reward USE cases")


if __name__ == '__main__':
    main()
