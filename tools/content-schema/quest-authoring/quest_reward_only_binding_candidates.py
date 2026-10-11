"""Build the fail-closed reward-only Quest completion binding candidate packet.

This packet records exact Quest -> RewardClaim ownership and the unresolved completion
policy. It does not create QuestState transitions, mutate RewardClaims, or admit runtime.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path

TOOL = "tools/content-schema/quest-authoring/"
DEFINITIONS = "content/quests/definitions/index.json"
CANDIDATE = "content/quests/missions/quest-state-completion-candidate.json"
CROSSWALK = TOOL + "samples/binding_packets/source/crosswalk.json"
OUTPUT = TOOL + "samples/server-completion/reward-only-completion/candidates.json"
SCHEMA = "OTERYN_REWARD_ONLY_QUEST_COMPLETION_CANDIDATES/v1"


def read(root: Path, relative: str):
    return json.loads((root / relative).read_text(encoding="utf-8"))


def sha(raw: bytes):
    return hashlib.sha256(raw).hexdigest()


def compact(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def require(ok, message):
    if not ok:
        raise ValueError(message)


def load_definitions(root: Path):
    index = read(root, DEFINITIONS)
    rows = []
    inputs = [{"path": DEFINITIONS, "sha256": sha((root / DEFINITIONS).read_bytes())}]
    for relative in index["shards"]:
        raw = (root / relative).read_bytes()
        inputs.append({"path": relative, "sha256": sha(raw)})
        rows.extend(record["definition"] for record in json.loads(raw)["records"])
    return rows, inputs


def claim_row(binding):
    source = binding["source_record"]
    repeat = source["claim"]["repeat"]["kind"]
    require(repeat == "once", "reward-only candidate claim is not once")
    progress = source.get("progress_write")
    return {
        "source_ref": binding["source_ref"],
        "quest_link_basis": source.get("quest_link_basis"),
        "repeat": repeat,
        "placement_count": len(source.get("placements") or []),
        "progress_write": (
            {
                "marker": progress["marker"],
                "expression": progress["expression"],
                "value": progress["value"],
                "source": progress["source"],
            }
            if progress is not None
            else None
        ),
    }


def expected(root: Path):
    definitions, definition_inputs = load_definitions(root)
    candidate = read(root, CANDIDATE)
    crosswalk_obj = read(root, CROSSWALK)
    candidate_keys = {quest["quest"] for quest in candidate["quests"]}
    crosswalk = {row["quest_key"]: row for row in crosswalk_obj["quests"]}
    records = []

    for definition in definitions:
        key = definition["identity"]["key"]
        if definition.get("readiness") != "definition_ready":
            continue
        if definition.get("missing_data"):
            continue
        if key in candidate_keys:
            continue
        source = crosswalk.get(key)
        if source is None:
            continue
        counts = source["counts"]
        if counts.get("progress") != 0 or counts.get("native_reward_refs", 0) == 0:
            continue
        bindings = source.get("source_reward_bindings") or []
        native_refs = source.get("native_reward_refs") or []
        require(len(bindings) == len(native_refs) == counts["native_reward_refs"], "reward-only binding count drift")
        claims = [claim_row(binding) for binding in bindings]
        writes = sum(claim["progress_write"] is not None for claim in claims)
        claim_cardinality = "SINGLE" if len(claims) == 1 else "MULTI"
        progress_shape = "STORAGE" if writes else "KV"
        require(writes in (0, len(claims)), "mixed storage/KV reward-only quest requires explicit split")
        candidate_class = f"{claim_cardinality}_{progress_shape}"
        holds = []
        if claim_cardinality == "SINGLE":
            holds.append("CLAIM_AS_QUEST_COMPLETION_POLICY_REQUIRED")
        else:
            holds.append("MULTI_CLAIM_COMPLETION_AGGREGATION_POLICY_REQUIRED")
        if progress_shape == "STORAGE":
            holds.append("SOURCE_PROGRESS_WRITE_EXISTS_BUT_CANONICAL_QUEST_HAS_NO_PROGRESS_TRACK")
        else:
            holds.append("NO_SOURCE_PROGRESS_WRITE")
        records.append(
            {
                "quest_ref": {
                    "family": "Quest",
                    "key": key,
                    "revision": definition["identity"]["revision"],
                },
                "display_name": definition["display_name"],
                "kind": definition.get("kind"),
                "candidate_class": candidate_class,
                "native_reward_refs": native_refs,
                "source_claims": claims,
                "holds": sorted(holds),
                "native_completion_transition": None,
                "native_claim_binding": None,
                "runtime_admitted": False,
            }
        )

    records.sort(key=lambda row: row["quest_ref"]["key"])
    classes = Counter(row["candidate_class"] for row in records)
    claim_count = sum(len(row["native_reward_refs"]) for row in records)
    require(len(records) == 42, "reward-only quest candidate population changed")
    require(claim_count == 59, "reward-only claim population changed")
    require(
        classes == Counter({"SINGLE_KV": 24, "MULTI_KV": 10, "SINGLE_STORAGE": 7, "MULTI_STORAGE": 1}),
        "reward-only candidate classes changed",
    )
    require(all(not row["runtime_admitted"] for row in records), "runtime admission forbidden")
    require(all(row["native_completion_transition"] is None and row["native_claim_binding"] is None for row in records), "native binding promotion forbidden")

    inputs = definition_inputs + [
        {"path": CANDIDATE, "sha256": sha((root / CANDIDATE).read_bytes())},
        {"path": CROSSWALK, "sha256": sha((root / CROSSWALK).read_bytes())},
        {"path": TOOL + "quest_reward_only_binding_candidates.py", "sha256": sha((root / TOOL / "quest_reward_only_binding_candidates.py").read_bytes())},
    ]
    return {
        "schema": SCHEMA,
        "classification": "EXACT_BINDING_EVIDENCE_WITH_COMPLETION_POLICY_HELD",
        "runtime_admitted": False,
        "summary": {
            "quests": len(records),
            "claims": claim_count,
            "classes": dict(sorted(classes.items())),
            "native_completion_transitions": 0,
            "native_claim_bindings": 0,
        },
        "decision_required": [
            "For a SINGLE claim, decide whether successful once-claim is itself Quest completion.",
            "For MULTI claims, decide the per-Quest completion aggregation rule; source grouping alone does not prove ANY vs ALL or ordering.",
            "For STORAGE claims without canonical progress tracks, decide whether claim storage writes become QuestState tracks or remain source evidence only.",
            "For KV claims with no source storage write, decide whether an effectless completes=true transition may be bound directly to ClaimObligation.",
        ],
        "inputs": inputs,
        "records": records,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw = compact(expected(args.root))
    target = args.root / OUTPUT
    if args.check:
        if not target.is_file() or target.read_bytes() != raw:
            raise ValueError("reward-only completion candidate drift")
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(raw)
    print(json.dumps(json.loads(raw)["summary"], sort_keys=True))


if __name__ == "__main__":
    main()
