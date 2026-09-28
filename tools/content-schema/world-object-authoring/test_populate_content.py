"""No-network checks over the committed WO-2 catalog under `content/world/`.

Every committed record must pass the WO-1 validator, carry the D93 key of its frozen
CW2-B1 Item key, and appear once; a held id must carry no family key; the index must
match the shards. Run with `python test_populate_content.py`.
"""

from __future__ import annotations

import hashlib
import json

import populate_content as pc
import world_objects as wo

CHECKS = 0


def check(condition, message):
    global CHECKS
    CHECKS += 1
    if not condition:
        raise AssertionError(message)


def load(path):
    return json.loads((pc.REPO_ROOT / path).read_text(encoding="utf-8"))


def committed_families():
    for family, (directory, _stem) in pc.FAMILY_DIRECTORY.items():
        if (pc.REPO_ROOT / directory / "index.json").is_file():
            yield family, directory


def test_committed_records_follow_d93_and_validate():
    bindings = pc.frozen_item_keys()
    by_item_key = {key: item_id for item_id, key in bindings.items()}
    for family, directory in committed_families():
        index = load(f"{directory}/index.json")
        held = load(f"{directory}/held.json")
        check(index["schema"] == "OTERYN_FAMILY_INDEX/v1", "index schema")
        check(index["family"] == family and held["family"] == family, "family")
        digest = hashlib.sha256()
        seen = set()
        total = 0
        expected_start = 0
        for path in index["shards"]:
            raw = (pc.REPO_ROOT / path).read_bytes()
            digest.update(raw)
            shard = json.loads(raw)
            check(shard["family"] == family, path)
            check(shard["shard"]["start"] == expected_start, f"shard gap {path}")
            check(shard["shard"]["count"] == len(shard["records"]), f"count {path}")
            check(len(shard["records"]) <= pc.SHARD_SIZE, f"shard size {path}")
            expected_start += len(shard["records"])
            for record in shard["records"]:
                total += 1
                errors = wo.validate_record(record)
                check(not errors, f"{record['identity']['key']}: {errors[:2]}")
                key = record["identity"]["key"]
                check(key not in seen, f"duplicate {key}")
                seen.add(key)
                item_key = record["provenance"]["item_pointer"]["key"]
                check(item_key in by_item_key, f"unbound Item key {item_key}")
                check(wo.family_key(item_key, family) == key, f"D93 {key}")
                source_id = record["provenance"]["source_item_id"]
                check(bindings[source_id] == item_key, f"frozen binding {key}")
        check(total == index["record_count"], "record_count")
        check(digest.hexdigest() == index["shards_sha256"], "shards_sha256")
        check(sum(index["kind_counts"].values()) == total, "kind_counts")
        check(held["held_count"] == len(held["held"]) == index["held_count"], "held")
        for row in held["held"]:
            check(set(row) == {"item_key", "reasons", "route", "source_item_id"}, row)
            check(row["reasons"] and set(row["reasons"]) <= set(pc.HOLD_REASONS), row)
            check(wo.family_key(row["item_key"], family) not in seen, "held minted")
            check(bindings[row["source_item_id"]] == row["item_key"], "held binding")


def test_family_keys_are_disjoint_from_other_families():
    keys = {}
    for family, directory in committed_families():
        for path in load(f"{directory}/index.json")["shards"]:
            for record in load(path)["records"]:
                keys.setdefault(record["identity"]["key"], family)
                check(keys[record["identity"]["key"]] == family, "cross-family key")


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_") and callable(fn):
            fn()
    print(json.dumps({"checks": CHECKS, "status": "PASS"}))
