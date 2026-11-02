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
QUALIFIED = {
    "oteryn:reward-claim.kv.steelhelmet1": "quest_callbacks_source_chest_use_once_replay_and_second_use_refused",
    "oteryn:reward-claim.kv.bear_chest1": "quest_container_chests_bear_bag_once_and_replay",
    "oteryn:reward-claim.quest.u6_1.emperors_cookies.rewards.cookies": "quest_container_chests_key_attributes_are_atomic",
    "oteryn:reward-claim.quest.u7_24.the_annihilator.reward": "quest_container_chests_annihilator_choices_share_storage_and_achievement",
    "oteryn:reward-claim.quest.u7_9.dreamers_challenge.bplong_sword": "quest_container_chests_authored_book_text_survives_replay",
    "oteryn:reward-claim.kv.powerbolt2": "quest_container_chests_leaf_text_is_frozen_and_replayed",
    "oteryn:reward-claim.quest.key.id3002": "quest_container_chests_crystal_key3002_is_minted_once",
    "oteryn:reward-claim.kv.madmageroom3": "quest_container_chests_full_definition_charges_mint_one_instance",
}
KEY_LOWERING = "KEY_REWARD_NATIVE_LOWERING_NOT_IMPLEMENTED"
SOURCE_SHA256 = {
    "canary": "2b130d4196afc9700eb0b7c2f5594e331edee4d25811a0d2020fb9d8757d6cda",
    "crystalserver": "7c3ac1280196923e0bf5ca6f2ea932c19065bb9ac22d1559150a184f3cf06df4",
}
CONTAINER_SOURCE = "b073d546930a60adf511677f932e6e0d37df3a230313dc85c4b9253742325757"
ACCEPTED_OVERRIDES = {
    6078: "PR2770 Q5/A: retain accepted Item12510 over donor3208",
    6218: "PR2770 Q4/A: retain accepted two100stacks over donor Container:addItem clamp",
    6226: "PR2770 Q4/A: retain accepted two100stacks over donor Container:addItem clamp",
    6335: "PR2770 Q6/A: retain Item9238 at32994,32405,12 over donor9237 at32629,32446,10",
    **{uid: "PR2770 Q3/A: retain accepted24hour repeat despite absent donor timerStorage"
       for uid in range(6105, 6111)},
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
            native = definition.get("native_contents", {})
            supported = definition.get("readiness") == "ready" or (
                pending == [KEY_LOWERING] and not definition.get("data_holds")
                and not variant.get("achievement_grants")
                and not variant.get("charge_dispositions")
                and variant.get("source_data", {}).get("status") == "COMPLETE"
                and all(p["reward"].get("key_binding", "").startswith("canary:door-key/")
                        for p in definition["placements"])
            ) or (
                native.get("schema") == "OTERYN_NATIVE_CONTAINER_REWARD/v1"
                and native.get("source_sha256") == CONTAINER_SOURCE
                and native.get("revision") == "201189f871d23597b82caf88fd611454a2ddefb8"
                and not definition.get("data_holds")
                and len(native.get("placements", [])) == len(definition["placements"])
            )
            status = "supported_unqualified" if supported else "not_playable"
            reason = "requires exact World binding and native Item admission" if supported else (
                ", ".join(pending) or definition["readiness"]
            )
            if key in QUALIFIED:
                status = "dispatch_pg_qualified"
                reason = QUALIFIED[key]
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
                        if server == "crystalserver" and legacy["unique_id"] in ACCEPTED_OVERRIDES:
                            witness["accepted_override"] = ACCEPTED_OVERRIDES[legacy["unique_id"]]
                    else:
                        witness["missing_in_current_source"] = True
                    witnesses.append(witness)
                placements.append({"position": placement["source_binding"]["project_position"],
                                   "sources": witnesses})
            inventory = {
                "claim": key, "status": status, "reason": reason, "placements": placements,
            }
            if native and any(p['reward'].get('written_text') for p in definition['placements']):
                inventory["qualification_limit"] = (
                    "Reward-grant support includes durable authored book text; "
                    "ordinary carried-book READ/TextWindow transport is not implemented. "
                    "LOOK and distance-reading admission are not changed. "
                    "Text retains the established source author's outer-LF normalization."
                )
            quests[definition.get("quest", {}).get("key", "unlinked")].append(inventory)
    counts = Counter(row["status"] for rows in quests.values() for row in rows)
    return {
        "task": "PARITY-QUEST-CALLBACKS-1+W15b",
        "source_revisions": {"canary": "47dfd51f", "crystalserver": "201189f871d23597b82caf88fd611454a2ddefb8"},
        "source_sha256": {server: digest for server, (_, digest, _) in sources.items()},
        "scope": "Once-only leaf/key/text chests and exact filled-container/shared-storage-choice recipes; levers and step-in scripts are separate owners.",
        "qualification": "Only dispatch_pg_qualified has a real source-placement PostgreSQL USE test. "
                         "That status certifies reward granting, not every later Item consumer. "
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
