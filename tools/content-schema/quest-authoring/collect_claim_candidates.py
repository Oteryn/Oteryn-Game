"""Build fail-closed RewardClaim acquisition candidates for chosen Quest collect stages.

An exact Item identity and same-Quest RewardClaim delivery are evidence that the claim
*may* be an acquisition producer for a chosen collect stage. They do not prove that
the claim occurrence advances QuestState, that current inventory is authoritative,
or that recipe occurrence counts equal item quantities.

The output is evidence-only and never promotes native/runtime binding.
"""
from __future__ import annotations

import argparse
import collections
import hashlib
import json
from pathlib import Path

PLAN = "content/quests/missions/completion-binding-plan.json"
CROSSWALK = "tools/content-schema/quest-authoring/samples/binding_packets/source/crosswalk.json"
CLAIM_INDEX = "content/interactions/reward_claims/index.json"
OUTPUT = "tools/content-schema/quest-authoring/samples/server-completion/collect-claim-candidates.json"

CLASS_EXACT = "ONE_SAME_QUEST_READY_CLAIM_COVERS_ALL_EXACT_TARGETS"
CLASS_AMBIGUOUS = "AMBIGUOUS_OR_PARTIAL_SAME_QUEST_CLAIM_MATCH"
CLASS_NO_MATCH = "NO_SAME_QUEST_REWARD_CLAIM_MATCH"
CLASS_TARGET_INCOMPLETE = "TARGET_IDENTITY_INCOMPLETE"


def read(root: Path, path: str):
    return json.loads((root / path).read_text(encoding="utf-8"))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def encoded(value) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")


def claim_rows(root: Path, crosswalk):
    donor_to_canonical = collections.defaultdict(set)
    for row in crosswalk["quests"]:
        donor_to_canonical[row["donor_quest_key"]].add(row["quest_key"])

    index = read(root, CLAIM_INDEX)
    claims = []
    inputs = [
        {"path": CLAIM_INDEX, "sha256": sha256(root / CLAIM_INDEX)},
    ]
    for shard in index["shards"]:
        inputs.append({"path": shard, "sha256": sha256(root / shard)})
        raw = read(root, shard)
        for record_index, record in enumerate(raw["records"]):
            definition = record["definition"]
            source_quest = (definition.get("quest") or {}).get("key")
            owners = set()
            if isinstance(source_quest, str):
                if source_quest.startswith("oteryn:quest."):
                    owners.add(source_quest)
                owners.update(donor_to_canonical.get(source_quest, ()))
            items = collections.Counter()
            placements = []
            for placement_index, placement in enumerate(definition.get("placements") or []):
                reward_items = []
                for item in ((placement.get("reward") or {}).get("items") or []):
                    ref = item.get("item") or {}
                    if ref.get("family") != "Item" or not ref.get("key"):
                        continue
                    count = int(item.get("count", 1))
                    items[ref["key"]] += count
                    reward_items.append(
                        {
                            "item": dict(ref),
                            "count": count,
                        }
                    )
                if reward_items:
                    placements.append(
                        {
                            "placement_index": placement_index,
                            "items": reward_items,
                            "source_binding": placement.get("source_binding"),
                        }
                    )
            claims.append(
                {
                    "key": definition["identity"]["key"],
                    "readiness": definition.get("readiness"),
                    "source_quest": source_quest,
                    "canonical_quest_candidates": sorted(owners),
                    "items": dict(sorted(items.items())),
                    "placements": placements,
                    "canonical_ref": {
                        "path": shard,
                        "json_pointer": f"/records/{record_index}/definition",
                        "record_sha256": hashlib.sha256(
                            json.dumps(
                                definition,
                                ensure_ascii=False,
                                sort_keys=True,
                                separators=(",", ":"),
                            ).encode("utf-8")
                        ).hexdigest(),
                    },
                }
            )
    return claims, inputs


def exact_item_targets(intent):
    exact = []
    incomplete = []
    for target in intent.get("targets") or []:
        if target.get("status") != "EXACT_CANONICAL_IDENTITY_ASSOCIATION":
            incomplete.append(
                {
                    "target": target.get("target"),
                    "status": target.get("status"),
                }
            )
            continue
        refs = [
            dict(ref)
            for ref in target.get("refs") or []
            if ref.get("family") == "Item" and ref.get("key")
        ]
        if len(refs) != 1:
            incomplete.append(
                {
                    "target": target.get("target"),
                    "status": "EXACT_TARGET_WITHOUT_SINGLE_ITEM_REF",
                }
            )
            continue
        exact.append(
            {
                "declared_target": target.get("target"),
                "item": refs[0],
                "materializable": target.get("materializable"),
                "stack_class": target.get("stack_class"),
            }
        )
    return exact, incomplete


def build(root: Path):
    root = root.resolve()
    plan = read(root, PLAN)
    crosswalk = read(root, CROSSWALK)
    claims, claim_inputs = claim_rows(root, crosswalk)
    claims_by_owner = collections.defaultdict(list)
    for claim in claims:
        for owner in claim["canonical_quest_candidates"]:
            claims_by_owner[owner].append(claim)

    records = []
    for quest in plan["records"]:
        owner = quest["quest"]
        local_claims = claims_by_owner.get(owner, [])
        for stage in quest["stages"]:
            intent = stage["event_identity_associations"]
            if intent.get("kind") != "collect":
                continue
            exact_targets, incomplete_targets = exact_item_targets(intent)
            exact_item_keys = sorted({row["item"]["key"] for row in exact_targets})
            matches_by_item = {}
            for item_key in exact_item_keys:
                rows = []
                for claim in local_claims:
                    if item_key not in claim["items"]:
                        continue
                    rows.append(
                        {
                            "claim": claim["key"],
                            "claim_readiness": claim["readiness"],
                            "delivered_count": claim["items"][item_key],
                            "source_quest": claim["source_quest"],
                            "canonical_ref": claim["canonical_ref"],
                        }
                    )
                matches_by_item[item_key] = sorted(rows, key=lambda row: row["claim"])

            target_set = set(exact_item_keys)
            claims_covering_all = []
            if target_set:
                for claim in local_claims:
                    if target_set <= set(claim["items"]):
                        claims_covering_all.append(
                            {
                                "claim": claim["key"],
                                "claim_readiness": claim["readiness"],
                                "delivered_counts": {
                                    item_key: claim["items"][item_key]
                                    for item_key in sorted(target_set)
                                },
                                "source_quest": claim["source_quest"],
                                "canonical_ref": claim["canonical_ref"],
                            }
                        )
            claims_covering_all.sort(key=lambda row: row["claim"])

            if incomplete_targets:
                classification = CLASS_TARGET_INCOMPLETE
            elif (
                len(claims_covering_all) == 1
                and claims_covering_all[0]["claim_readiness"] == "ready"
            ):
                classification = CLASS_EXACT
            elif any(matches_by_item.values()):
                classification = CLASS_AMBIGUOUS
            else:
                classification = CLASS_NO_MATCH

            record = {
                "quest": owner,
                "stage_key": stage["stage_key"],
                "quest_transition_key": stage["quest_transition_key"],
                "chosen_occurrence_count": intent.get("count"),
                "classification": classification,
                "exact_item_targets": exact_targets,
                "incomplete_targets": incomplete_targets,
                "same_quest_claim_matches_by_item": matches_by_item,
                "same_quest_claims_covering_all_exact_targets": claims_covering_all,
                "native_dispatch_binding": None,
                "runtime_admitted": False,
                "holds": [
                    "REWARD_CLAIM_DELIVERY_DOES_NOT_PROVE_COLLECT_PROGRESS_OCCURRENCE",
                    "INVENTORY_COUNT_CONSUMER_AND_PER_TARGET_QUANTITY_BINDING_PENDING",
                ],
            }
            if classification == CLASS_EXACT:
                record["candidate"] = {
                    "kind": "SAME_QUEST_REWARD_CLAIM_ACQUISITION_CANDIDATE",
                    "claim": claims_covering_all[0]["claim"],
                    "claim_readiness": claims_covering_all[0]["claim_readiness"],
                    "quantity_semantics_proven": False,
                    "quest_transition_cause_binding_proven": False,
                }
            records.append(record)

    records.sort(key=lambda row: (row["quest"], row["stage_key"]))
    counts = collections.Counter(row["classification"] for row in records)
    expected_counts = {
        CLASS_NO_MATCH: 143,
        CLASS_TARGET_INCOMPLETE: 130,
        CLASS_EXACT: 2,
        CLASS_AMBIGUOUS: 2,
    }
    if dict(counts) != expected_counts:
        raise ValueError(
            f"Collect/RewardClaim candidate population changed: {dict(counts)}"
        )

    exact = [row for row in records if row["classification"] == CLASS_EXACT]
    expected_exact = {
        (
            "oteryn:quest.hunter_outfits_quest",
            "s2",
            "oteryn:reward-claim.quest.u7_8.hunter_outfits.elane_crossbow",
        ),
        (
            "oteryn:quest.oriental_outfits_quest",
            "s5",
            "oteryn:reward-claim.quest.u7_8.oriental_outfits.coral_comb",
        ),
    }
    actual_exact = {
        (row["quest"], row["stage_key"], row["candidate"]["claim"]) for row in exact
    }
    if actual_exact != expected_exact:
        raise ValueError(f"Exact collect claim candidate set changed: {actual_exact}")

    return {
        "schema": "OTERYN_QUEST_COLLECT_CLAIM_CANDIDATES/v1",
        "classification": "EVIDENCE_ONLY_NO_RUNTIME_ADMISSION",
        "runtime_admitted": False,
        "native_dispatch_bindings": 0,
        "inputs": [
            {"path": PLAN, "sha256": sha256(root / PLAN)},
            {"path": CROSSWALK, "sha256": sha256(root / CROSSWALK)},
            *claim_inputs,
        ],
        "summary": {
            "collect_stages": len(records),
            "classification_counts": dict(sorted(counts.items())),
            "exact_same_quest_ready_claim_candidates": len(exact),
            "runtime_admitted": False,
            "native_dispatch_bindings": 0,
        },
        "limits": [
            "Exact Item identity does not prove acquisition occurrence ownership.",
            "A same-Quest RewardClaim that delivers the target Item is only an acquisition candidate.",
            "Chosen occurrence count is not assumed to equal RewardClaim item quantity.",
            "Current inventory state is never promoted into Quest transition authority.",
            "No candidate in this packet selects or invokes a Quest transition.",
        ],
        "records": records,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[3]
    )
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    payload = encoded(build(root))
    out = root / OUTPUT
    if args.check:
        if not out.is_file() or out.read_bytes() != payload:
            raise ValueError("Collect claim candidate packet drift: " + OUTPUT)
    else:
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_bytes(payload)
    print(json.dumps(json.loads(payload)["summary"], sort_keys=True))


if __name__ == "__main__":
    main()
