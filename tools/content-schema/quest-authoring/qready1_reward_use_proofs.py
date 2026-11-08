"""Import the 17 owned qready1 PG cases into the existing reward USE proof format."""

import argparse
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


def generate(root, text):
    source = (root / TEST).read_text()
    functions = re.findall(
        r"fn (qready1_\w+_exact_reward_once)\(\)(.*?)(?=\n#\[test\]|\Z)",
        source,
        re.DOTALL,
    )
    summaries = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed;", text)
    if len(functions) != 17 or summaries != [("ok", "17", "0")]:
        raise ValueError("Expected one complete passing 17-case qready1 PostgreSQL run")
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


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--log", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    proof = generate(Path(__file__).resolve().parents[3], args.log.read_text())
    args.output.write_text(json.dumps(proof, indent=2) + "\n")
    print(
        f"Qualified {len(proof['cases'])} qready1 reward claims from 17 PostgreSQL cases"
    )
