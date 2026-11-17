"""Import qready1 PG evidence, retaining unchanged proofs on a focused refresh."""

import argparse
import copy
import hashlib
import json
import re
from pathlib import Path

from quest_completion_matrix import load_reward_claims
from quest_reward_use_proofs import digest

TEST = "crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qready1_reward_claims.rs"
BINDINGS = (
    "tools/content-schema/world-authoring/samples/quest-chest-bindings/qready1.json"
)


def generate(root, text, *, selected=None, previous=None):
    source = (root / TEST).read_text()
    functions = re.findall(
        r"fn (qready1_\w+_exact_reward_once)\(\)(.*?)(?=\n#\[test\]|\Z)",
        source,
        re.DOTALL,
    )
    summaries = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed;", text)
    if len(functions) != 17:
        raise ValueError("Expected the 17 owned qready1 test functions")
    names = {name for name, _ in functions}
    if selected is not None:
        if not selected or not selected <= names or previous is None:
            raise ValueError("Focused refresh requires known tests and previous evidence")
        if (previous.get('schema') != 'OTERYN_QUEST_REWARD_USE_PROOF/v1'
                or previous.get('test_file') != TEST
                or previous.get('test_sha256') != hashlib.sha256((root / TEST).read_bytes()).hexdigest()):
            raise ValueError("Retained test evidence changed; rerun all qready1 cases")
        functions = [(name, body) for name, body in functions if name in selected]
    if summaries != [("ok", str(len(functions)), "0")]:
        raise ValueError(f"Expected one complete passing {len(functions)}-case qready1 PostgreSQL run")
    claims = load_reward_claims(root)
    matrix = json.loads(
        (
            root
            / "tools/content-schema/quest-authoring/samples/completion-matrix/all373.json"
        ).read_text()
    )
    owners = {
        key: row["wiki_title"]
        for row in matrix["records"]
        for key in (row.get("reward_claim_owner") or {}).get("claim_keys", [])
    }
    cases = []
    for name, body in functions:
        marker = name.removesuffix("_exact_reward_once")
        matches = re.findall(
            r"POSTGRES_CASE_COMPLETED "
            + re.escape(marker)
            + r" duration=([0-9.]+)s success=(true|false)\b",
            text,
        )
        if len(matches) != 1 or matches[0][1] != "true" or float(matches[0][0]) < 0.01:
            raise ValueError(
                "Missing, failed, duplicated or skipped PG case: " + marker
            )
        keys = sorted(set(re.findall(r'"(oteryn:reward-claim\.[^"]+)"', body)))
        if not keys:
            raise ValueError("PG case has no reward claim: " + name)
        cases.extend(
            {
                "wiki_title": owners[key],
                "claim_key": key,
                "test": name,
                "marker": marker,
                "duration_seconds": float(matches[0][0]),
                "claim_sha256": digest(claims[key]),
            }
            for key in keys
        )
    if selected is not None:
        replacements = {case['claim_key']: case for case in cases}
        if {case['claim_key'] for case in previous['cases'] if case['test'] in selected} != set(replacements):
            raise ValueError("Focused refresh changed the selected claim inventory")
        retained = []
        for case in previous['cases']:
            if case['test'] in selected:
                retained.append(replacements[case['claim_key']])
            else:
                if (case['test'] not in names or case['duration_seconds'] < 0.01
                        or case['claim_sha256'] != digest(claims[case['claim_key']])
                        or case['marker'] not in source):
                    raise ValueError("Unselected reward USE evidence is stale")
                retained.append(case)
        cases = retained
    expected = {
        row["claim"] for row in json.loads((root / BINDINGS).read_text())["bindings"]
    }
    if {row["claim_key"] for row in cases} != expected:
        raise ValueError("PG cases must cover exactly the owned qready1 claim bindings")
    return {
        "schema": "OTERYN_QUEST_REWARD_USE_PROOF/v1",
        "runtime_path": "bundle_chest::use_bundle_chest -> reward_claim_mint",
        "test_file": TEST,
        "test_sha256": hashlib.sha256((root / TEST).read_bytes()).hexdigest(),
        "cases": cases,
    }


def rebind_m35_completion(root, previous):
    """Retain reward USE evidence across the exact M35 completion-owner split.

    The recorded test proves the same reward, source placement, once mint and
    replay. It does not prove the legacy quest-transition side effect, which
    M35 replaced with the separately qualified mint-only claim-set owner. No
    new PG run or completion qualification is asserted by this rebind.
    """
    if (previous.get('schema') != 'OTERYN_QUEST_REWARD_USE_PROOF/v1'
            or previous.get('test_file') != TEST
            or previous.get('test_sha256') != hashlib.sha256((root / TEST).read_bytes()).hexdigest()):
        raise ValueError('M35 reward USE test evidence changed')
    rule = json.loads((root / 'content/quests/claim-sets.json').read_bytes())
    if rule.get('oteryn:quest.circle_room_quest') != {
            'groups': [['kv/circleroom1'], ['kv/circleroom2']], 'mint_only': True}:
        raise ValueError('M35 Circle Room completion owner changed')
    state = json.loads((root / "content/quests/missions/quest-state.json").read_bytes())
    transitions = {t["key"] for quest in state["quests"] for t in quest["transitions"]
                   if t["source"]["owner"] == "chest"}
    claims = load_reward_claims(root)
    output = copy.deepcopy(previous)
    rebound = []
    for case in output['cases']:
        key = case['claim_key']
        current = claims[key]
        if case['duration_seconds'] < 0.01 or case['marker'] not in (root / TEST).read_text():
            raise ValueError('M35 reward USE proof is skipped or unbound')
        if digest(current) == case['claim_sha256']:
            continue
        if key not in {'oteryn:reward-claim.kv.circleroom1', 'oteryn:reward-claim.kv.circleroom2'}:
            raise ValueError('M35 rebind cannot qualify another changed claim')
        transition = 'oteryn:quest-transition/kv/' + key.rsplit('.', 1)[1] + '/chest'
        if transition in transitions or len(current['placements']) != 1 or 'quest_transition' in current['placements'][0]:
            raise ValueError('M35 legacy completion owner remains active')
        legacy = copy.deepcopy(current)
        legacy['placements'][0]['quest_transition'] = transition
        if digest(legacy) != case['claim_sha256']:
            raise ValueError('M35 rebind changes reward USE facts')
        rebound.append({'claim_key': key, 'previous_claim_sha256': case['claim_sha256'],
                        'removed_legacy_transition': transition})
        case['claim_sha256'] = digest(current)
    if rebound:
        output['completion_owner_rebind'] = {
            'scope': 'Retained reward USE evidence only; no fresh PG run or completion qualification',
            'owner': 'M35 mint-only claim sets', 'claims': rebound}
    return output


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--log", type=Path)
    parser.add_argument("--rebind-m35-completion", action="store_true")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--test", action="append", help="Refresh only this passing test; retain unchanged output evidence")
    args = parser.parse_args()
    if args.rebind_m35_completion:
        if args.log or args.test:
            parser.error("M35 evidence rebind cannot import a PG log")
        proof = rebind_m35_completion(Path(__file__).resolve().parents[3], json.loads(args.output.read_text()))
        args.output.write_text(json.dumps(proof, indent=2) + "\n")
        print("Retained reward USE evidence after exact M35 completion-owner rebind; no new PG run")
        raise SystemExit(0)
    if args.log is None:
        parser.error("--log is required for PG qualification")
    proof = generate(Path(__file__).resolve().parents[3], args.log.read_text(),
                     selected=set(args.test) if args.test else None,
                     previous=json.loads(args.output.read_text()) if args.test else None)
    args.output.write_text(json.dumps(proof, indent=2) + "\n")
    print(
        f"Qualified {len(proof['cases'])} qready1 reward claims; refreshed {len(args.test) if args.test else 17} PostgreSQL cases"
    )
