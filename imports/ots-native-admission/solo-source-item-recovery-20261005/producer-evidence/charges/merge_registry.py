"""Measured closed registry successor; never writes a repository or Native docs."""

import argparse
import copy
import json
from pathlib import Path

from produce import file_sha, pinned, produce, require

OLD_PATH = "apps/game-server/src/content/source-definition-registry.json"
OLD_SHA = "dc1283b321a0de953eb97acb7c7875b72127b763a26df53f11391d04580d4c17"
KIND = "CHARGES_AND_LEVEL_DOOR"


def merge(old, packet):
    require(packet["input_rows"] == 45721, "WHOLE_SOURCE_ROW_SCOPE")
    count = sum(len(r["observations"]) for r in packet["records"])
    require(count + len(packet["held"]) == 45721, "WHOLE_ROW_ACCOUNTING")
    require(not packet["held"], "HELD_ROWS_REQUIRE_EXPLICIT_DISPOSITION")
    require(len(packet["records"]) == 33975, "WHOLE_SOURCE_TARGET_SCOPE")
    require(len(old["targets"]) == 4475, "OLD_TARGET_SCOPE")
    require(
        sum(len(v) for rs in old["targets"].values() for v in rs.values()) == 9291,
        "OLD_OBSERVATION_SCOPE",
    )
    result = copy.deepcopy(old)
    additions = {}
    for record in packet["records"]:
        target, entries = record["target"], record["observations"]
        require(
            target["family"] == "Item" and target["key"] != "oteryn:item.tibia.i901",
            "ITEM_SOURCE_STORAGE_PROTECTED_CORE",
        )
        require(
            len({e["source_cut"] for e in entries}) == len(entries), "UNIQUE_OWN_CUT"
        )
        require(all(e["parameter"]["kind"] == KIND for e in entries), "CLOSED_NEW_KIND")
        entries = sorted(entries, key=lambda e: e["source_cut"])
        revisions = result["targets"].setdefault(target["key"], {})
        vector = revisions.setdefault(target["revision"], [])
        require(
            not any(e["parameter"]["kind"] == KIND for e in vector),
            "NEW_KIND_ALREADY_PRESENT",
        )
        vector.extend(copy.deepcopy(entries))
        additions[(target["key"], target["revision"])] = entries
    inverse = copy.deepcopy(result)
    for (key, revision), entries in additions.items():
        vector = inverse["targets"][key][revision]
        require(vector[-len(entries) :] == entries, "EXACT_SUFFIX")
        del vector[-len(entries) :]
        if not vector:
            del inverse["targets"][key][revision]
            if not inverse["targets"][key]:
                del inverse["targets"][key]
    require(inverse == old, "WHOLE_OLD_REGISTRY_INVERSE")
    vectors = [v for rs in result["targets"].values() for v in rs.values()]
    return result, {
        "targets": len(result["targets"]),
        "observations": sum(map(len, vectors)),
        "max_observations": max(map(len, vectors)),
        "max_assignments": max(
            len(e["ordered_assignments"]) for v in vectors for e in v
        ),
        "new_observations": count,
        "old_registry_inverse": "EXACT",
        "actual_cpp_execution": "UNKNOWN",
        "runtime_artifact": "HELD",
        "bounds_installation": "REQUIRES_INDEPENDENT_MEASURED_SUCCESSOR_REVIEW",
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--law", type=Path, required=True)
    parser.add_argument("--output-directory", type=Path, required=True)
    parser.add_argument("--root-released-heavy-slot", action="store_true")
    args = parser.parse_args()
    require(
        args.root_released_heavy_slot, "ROOT_EXCLUSIVE_SOURCE_REPLAY_RELEASE_REQUIRED"
    )
    out = args.output_directory.resolve()
    require(not out.is_relative_to(args.root.resolve()), "EXTERNAL_OUTPUT_ONLY")
    require(not out.exists(), "FRESH_OUTPUT_DIRECTORY_REQUIRED")
    old = json.loads(pinned(args.root, OLD_PATH, OLD_SHA).read_bytes())
    packet = produce(args.root, args.law, heavy_release=True)
    registry, census = merge(old, packet)
    out.mkdir(parents=True)
    for name, value in (
        ("observations.json", packet),
        ("registry.json", registry),
        ("census.json", census),
    ):
        path = out / name
        path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")
        census[name + "_sha256"] = file_sha(path)
    (out / "output-hashes.json").write_text(json.dumps(census, indent=2) + "\n")


if __name__ == "__main__":
    main()
