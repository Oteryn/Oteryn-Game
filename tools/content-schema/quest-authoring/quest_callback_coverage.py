"""Generate the bounded quest-chest execution inventory without executing donor Lua.

This reports implementation coverage, not admission of a deployed World. A candidate
still needs the exact map binding and an admitted native reward/backpack generation.
"""
import argparse
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path

import lua_tables

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / "docs/repository/QUEST_CALLBACK_COVERAGE.json"
QUALIFIED = "oteryn:reward-claim.kv.steelhelmet1"
KEY_LOWERING = "KEY_REWARD_NATIVE_LOWERING_NOT_IMPLEMENTED"
SOURCE_SHA256 = {
    "canary": "2b130d4196afc9700eb0b7c2f5594e331edee4d25811a0d2020fb9d8757d6cda",
    "crystalserver": "0ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca",
}


def source(root, pack):
    path = f"{pack}/startup/tables/chest.lua"
    data = (root / path).read_bytes()
    fields = lua_tables.assignments(data.decode(), {"ChestUnique"})["ChestUnique"]["fields"]
    return path, hashlib.sha256(data).hexdigest(), {
        field["key"]: (field["line"], lua_tables.as_python(field["value"]))
        for field in fields
    }


def coverage(canary, crystal):
    sources = {
        "canary": source(canary, "data-otservbr-global"),
        "crystalserver": source(crystal, "data-global"),
    }
    if {server: digest for server, (_, digest, _) in sources.items()} != SOURCE_SHA256:
        raise ValueError("reference chest tables differ from the task's pinned revisions")
    quests = defaultdict(list)
    for path in sorted((ROOT / "content/interactions/reward_claims").glob("reward-claim*.json")):
        for record in json.loads(path.read_text())["records"]:
            definition = record["definition"]
            key = definition["identity"]["key"]
            variant = definition.get("source_variant", {})
            pending = variant.get("native_lowering", {}).get("pending_implementation", [])
            supported = definition.get("readiness") == "ready" or (
                pending == [KEY_LOWERING] and not definition.get("data_holds")
                and not variant.get("achievement_grants")
                and not variant.get("charge_dispositions")
                and variant.get("source_data", {}).get("status") == "COMPLETE"
                and all(p["reward"].get("key_binding", "").startswith("canary:door-key/")
                        for p in definition["placements"])
            )
            status = "supported_unqualified" if supported else "not_playable"
            reason = "requires exact World binding and native Item admission" if supported else (
                ", ".join(pending) or definition["readiness"]
            )
            if key == QUALIFIED:
                status = "dispatch_pg_qualified"
                reason = "quest_callbacks_source_chest_use_once_replay_and_second_use_refused"
            placements = []
            for placement in definition["placements"]:
                witnesses = []
                for legacy in placement["source_binding"]["legacy_unique_ids"]:
                    server = legacy["server"]
                    src_path, _, entries = sources[server]
                    found = entries.get(legacy["unique_id"])
                    witness = {"server": server, "unique_id": legacy["unique_id"]}
                    if found:
                        line, value = found
                        prefix = "canary" if server == "canary" else "crystal"
                        witness["citation"] = f"{prefix}:{src_path}:{line}"
                        authored = placement["reward"].get("items", [])
                        pairs = [[int(item["item"]["key"].rsplit(".i", 1)[1]), item["count"]]
                                 for item in authored]
                        witness["reward_matches_current_source"] = value.get("reward") == pairs
                    else:
                        witness["missing_in_current_source"] = True
                    witnesses.append(witness)
                placements.append({"position": placement["source_binding"]["project_position"],
                                   "sources": witnesses})
            quests[definition.get("quest", {}).get("key", "unlinked")].append({
                "claim": key, "status": status, "reason": reason, "placements": placements,
            })
    counts = Counter(row["status"] for rows in quests.values() for row in rows)
    return {
        "task": "PARITY-QUEST-CALLBACKS-1",
        "source_revisions": {"canary": "47dfd51f", "crystalserver": "ff7ede59"},
        "source_sha256": {server: digest for server, (_, digest, _) in sources.items()},
        "scope": "Generic once-only single-item chests; levers and step-in scripts are follow-up slices.",
        "qualification": "Only dispatch_pg_qualified has a real source-placement PostgreSQL USE test. "
                         "Other supported shapes need per-World binding, Item admission and qualification. "
                         "No deployed World or full-quest completion is certified.",
        "counts": dict(sorted(counts.items())),
        "quests": [{"quest": quest, "chests": sorted(rows, key=lambda row: row["claim"])}
                   for quest, rows in sorted(quests.items())],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--canary", type=Path, required=True)
    parser.add_argument("--crystal", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    text = json.dumps(coverage(args.canary, args.crystal), indent=2, sort_keys=True) + "\n"
    if args.check:
        if OUTPUT.read_text() != text:
            raise SystemExit("quest callback coverage is stale")
    else:
        OUTPUT.write_text(text)


if __name__ == "__main__":
    main()
