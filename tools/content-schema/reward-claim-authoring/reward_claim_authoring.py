#!/usr/bin/env python3
"""RewardClaim authoring: populate content/interactions/reward_claims/ from the chest pilot.

CHEST-CONTENT part 2b (#162 5909237761, scope ruling 5911004459, owner answer "a" 5912009064).
Tree-first, like CHARM-1 and PROF-CONTENT-1: this tool writes the RewardClaim family and its
registration in content/project.json, content/manifest.json and content/content.lock.json.
The server does not read content/ yet; MAP-BUNDLE-1 compiles it (ADR-0021).

Scope: plain `once` claims of the chest pilot
(tools/content-schema/quest-authoring/samples/chests/claims.json): repeat `once`, every
placement rewards `items` only, no achievement. Rules:
- RewardClaim is its own family; the reward sits on each placement (architect ruling
  5905746509), because claims with several chests may reward differently per chest.
- A placement never mints its own PlacementKey. It binds to its source (project-frame position
  plus the legacy unique ids of both servers), which MAP-BUNDLE-1 resolves to the compiled
  placement_key; compilation fails when it cannot (5909237761).
- The chest's appearance is source evidence only, kept as its Tibia id: the map bundle decides
  the placed object, and some chest appearances are not Items (appearance-only ids, #162
  5908569303).
- Every reward is an A12 Item key (`oteryn:item.tibia.i<id>`) that resolves in
  content/items. A claim is `ready` when every reward Item is materializable with a known stack
  class and fits it; otherwise it is `waiting_item_semantics` and the MINT fails closed on it
  (D82) until ITEM-SEM covers the Item.
- A reward that contradicts its Item's known stack facts is never guessed: it is listed in
  `source_checks` and the claim stays not ready.

`content` writes the family and registration (`--check` verifies it byte for byte);
`committed_errors()` runs the rules on the committed files.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
PILOT_DIR = "tools/content-schema/quest-authoring/samples/chests/"
CLAIMS_REL = PILOT_DIR + "claims.json"
MANIFEST_REL = PILOT_DIR + "manifest.json"
ITEMS_GLOB = "content/items/definitions/items-*.json"
FAMILY = "RewardClaim"
CONTENT_DIR = "content/interactions/reward_claims/"
INDEX_PATH = CONTENT_DIR + "index.json"
DIRECTORY_INDEX = "content/interactions/index.json"
SHARD_SIZE = 100
REVISION = "reward-claim-r1"
ITEM_REVISION = "definition-r1"
INDEX_SCHEMA = "OTERYN_FAMILY_INDEX/v1"
SHARD_SCHEMA = "OTERYN_REWARD_CLAIM_SHARD/v1"
PILOT_SCHEMA = "OTERYN_QUEST_CHEST_PILOT/claims"
A12_ITEM = re.compile(r"^oteryn:item\.tibia\.i[1-9][0-9]*$")
CLAIM_KEY = re.compile(r"^oteryn:reward-claim\.[a-z0-9_.-]+$")
SOURCE_ITEM = re.compile(r"^(canary|crystalserver):item/([1-9][0-9]*)$")
KNOWN_STACK = ("NonStackable", "StackCapable")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def compact(payload: dict) -> str:
    return (
        json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    )


def registry(payload: object) -> str:
    """One key per line, matching world_project_v2_to_tree.py, so registrations merge cleanly."""
    return json.dumps(payload, ensure_ascii=False, sort_keys=True, indent=2) + "\n"


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def load_items() -> dict[str, dict]:
    items = {}
    for path in sorted(ROOT.glob(ITEMS_GLOB)):
        for record in load_json(path)["records"]:
            definition = record["definition"]
            items[definition["identity"]["key"]] = definition
    return items


def tibia_id(source_ref: dict) -> int:
    match = SOURCE_ITEM.match(source_ref["key"])
    if not match:
        raise ValueError(f"unmapped source item {source_ref['key']}")
    return int(match.group(2))


def a12(source_ref: dict) -> str:
    return f"oteryn:item.tibia.i{tibia_id(source_ref)}"


def item_ref(key: str) -> dict:
    return {"family": "Item", "key": key, "revision": ITEM_REVISION}


def plain_once(claim: dict) -> bool:
    return claim["claim"]["repeat"]["kind"] == "once" and all(
        set(placement.get("reward", {})) == {"items"} and "achievement" not in placement
        for placement in claim["placements"]
    )


def stack_problem(definition: dict | None, count: int) -> str | None:
    """Why `count` cannot be minted from `definition`, or None when it can."""
    if not definition:
        return "Item record missing"
    if (
        not definition.get("materializable")
        or definition.get("stack_class") not in KNOWN_STACK
    ):
        return "waiting"
    if definition["stack_class"] == "NonStackable" and count != 1:
        return "NonStackable reward with count > 1"
    if definition["stack_class"] == "StackCapable":
        # D82 admits 100 when no smaller maximum is proven. Keep portable
        # ReferenceItemField encoding in sync with validate_reward_claim_count.
        semantics = definition.get("semantics", {})
        unsupported = "StackCapable Item has unsupported stack maximum fields"
        if not isinstance(semantics, dict):
            return unsupported
        stack = semantics.get("stack", {"state": "UNKNOWN"})

        def valid_field(field):
            return isinstance(field, dict) and (
                field.get("state") == "UNKNOWN" and set(field) == {"state"}
                or field.get("state") == "KNOWN" and set(field) == {"state", "value"}
            )

        if not valid_field(stack):
            return unsupported
        maximum = {"state": "UNKNOWN"}
        if stack["state"] == "KNOWN":
            if not isinstance(stack["value"], dict) or set(stack["value"]) != {"stackable", "stack_max"}:
                return unsupported
            stackable = stack["value"]["stackable"]
            if not valid_field(stackable):
                return unsupported
            if stackable["state"] == "KNOWN" and (type(stackable["value"]) is not bool or not stackable["value"]):
                return unsupported
            maximum = stack["value"]["stack_max"]
        if not valid_field(maximum):
            return unsupported
        limit = maximum["value"] if maximum["state"] == "KNOWN" else 100
        if type(limit) is not int or not 1 <= limit <= 100:
            return "StackCapable Item has unsupported stack maximum"
        if count > limit:
            return f"StackCapable reward with count > stack maximum ({limit})"
    return None


def source_subtype_problem(definition: dict | None) -> str | None:
    """A source non-stackable charge subtype cannot be lowered to quantity-only MINT.

    This is a claim admission hold, independent of Item materializability. The
    two-argument OTS addItem helper uses its count as per-instance charges,
    which the accepted RewardClaim contract cannot preserve. Stackable runes
    use quantity instead, so their definition charges do not trigger this hold.
    """
    if not definition or definition.get("stack_class") != "NonStackable":
        return None
    semantics = definition.get("semantics", {})
    if not isinstance(semantics, dict):
        return None
    charges = semantics.get("charges", {})
    if not isinstance(charges, dict) or charges.get("state") != "KNOWN":
        return None
    value = charges.get("value")
    if not isinstance(value, dict):
        return "Source charged reward has unsupported charge fields"
    count = value.get("count", {})
    if not isinstance(count, dict):
        return "Source charged reward has unsupported charge fields"
    if count.get("state") == "KNOWN":
        maximum = count.get("value")
        if type(maximum) is not int or maximum < 0:
            return "Source charged reward has unsupported charge fields"
        if maximum > 0:
            return "Source charge subtype requires explicit native lowering"
    return None


def legacy_uid_checks(records: list) -> list[dict]:
    """A legacy UID is server-scoped; positions disambiguate known collisions."""
    bindings = {}
    for row in records:
        claim = row["definition"]
        for placement in claim["placements"]:
            source = placement["source_binding"]
            position = source["project_position"]
            where = (claim["identity"]["key"], position["x"], position["y"], position["z"])
            for ref in source["legacy_unique_ids"]:
                bindings.setdefault((ref["server"], ref["unique_id"]), set()).add(where)
    return [
        {"reason": "duplicate legacy unique id", "server": server, "unique_id": uid,
         "bindings": [{"claim": claim, "project_position": {"x": x, "y": y, "z": z}}
                      for claim, x, y, z in sorted(places)]}
        for (server, uid), places in sorted(bindings.items()) if len(places) > 1
    ]


def build_records(claims: list, manifest: dict, items: dict) -> tuple[list, list]:
    bindings: dict[tuple, list] = {}
    for entry in manifest["entries"]:
        if "destination" not in entry:
            continue
        key = (entry["destination"], tuple(entry["position"]))
        bindings.setdefault(key, []).extend(
            {"server": source["source"], "unique_id": source["uid"]}
            for source in entry.get("sources", [])
        )
    records, source_checks = [], []
    for claim in sorted(filter(plain_once, claims), key=lambda c: c["identity"]["key"]):
        marker = claim["identity"]["key"].split("reward-claim/", 1)[1].replace("/", ".")
        ready = True
        placements = []
        for placement in claim["placements"]:
            position = placement["position"]
            xyz = (position["x"], position["y"], position["z"])
            rewards = []
            for reward in placement["reward"]["items"]:
                key = a12(reward["item"])
                problem = source_subtype_problem(items.get(key)) or stack_problem(
                    items.get(key), reward["count"]
                )
                if problem and problem != "waiting":
                    source_checks.append(
                        {
                            "claim": f"oteryn:reward-claim.{marker}",
                            "item": key,
                            "count": reward["count"],
                            "reason": problem,
                        }
                    )
                ready &= problem is None
                rewards.append({"count": reward["count"], "item": item_ref(key)})
            placements.append(
                {
                    "appearance_tibia_id": tibia_id(placement["appearance"]),
                    "reward": {"items": rewards},
                    "source_binding": {
                        "legacy_unique_ids": sorted(
                            bindings.get((claim["identity"]["key"], xyz), []),
                            key=lambda s: (s["server"], s["unique_id"]),
                        ),
                        "project_position": {"x": xyz[0], "y": xyz[1], "z": xyz[2]},
                    },
                }
            )
        placements.sort(
            key=lambda p: tuple(p["source_binding"]["project_position"].values())
        )
        record = {
            "claim": {"per": "character", "repeat": {"kind": "once"}},
            "identity": {"key": f"oteryn:reward-claim.{marker}", "revision": REVISION},
            "placements": placements,
            "provenance": {
                "pilot_key": claim["identity"]["key"],
                "pilot_revision": claim["identity"]["revision"],
            },
            "readiness": "ready" if ready else "waiting_item_semantics",
        }
        if claim.get("quest"):
            record["quest"] = claim["quest"]
        records.append({"definition": record})
    source_checks.extend(legacy_uid_checks(records))
    return records, source_checks


def content_files(
    claims_bytes: bytes, records: list, source_checks: list
) -> dict[str, str]:
    files, shards = {}, []
    for index, start in enumerate(range(0, len(records), SHARD_SIZE)):
        chunk = records[start : start + SHARD_SIZE]
        end = start + len(chunk) - 1
        path = f"{CONTENT_DIR}reward-claims-{start:05d}-{end:05d}.json"
        shards.append(path)
        files[path] = compact(
            {
                "family": FAMILY,
                "records": chunk,
                "schema": SHARD_SCHEMA,
                "shard": {
                    "count": len(chunk),
                    "end": end,
                    "index": index,
                    "start": start,
                },
            }
        )
    ready = sum(r["definition"]["readiness"] == "ready" for r in records)
    files[INDEX_PATH] = compact(
        {
            "authoring_source": {
                "path": CLAIMS_REL,
                "schema": PILOT_SCHEMA,
                "sha256": sha256(claims_bytes),
            },
            "family": FAMILY,
            "readiness": {
                "ready": ready,
                "waiting_item_semantics": len(records) - ready,
            },
            "record_count": len(records),
            "schema": INDEX_SCHEMA,
            "shards": shards,
            "source_checks": source_checks,
        }
    )
    return files


def directory_index(current: dict, count: int) -> dict:
    updated = dict(current)
    updated["population_state"] = "POPULATED"
    updated["notes"] = (
        f"Typed interaction definitions. reward_claims/: {count} plain once RewardClaim records "
        "from the chest pilot, built by tools/content-schema/reward-claim-authoring/"
        "reward_claim_authoring.py (CHEST-CONTENT part 2b)."
    )
    return updated


def registered(
    project: dict, manifest: dict, lock: dict, count: int, paths: list[str]
) -> tuple:
    """The three registration documents with the RewardClaim family registered."""
    project, manifest, lock = map(copy.deepcopy, (project, manifest, lock))
    if FAMILY not in project["migrated_families"]:
        project["migrated_families"] = project["migrated_families"] + [FAMILY]
    project["next_population_families"] = [
        f for f in project["next_population_families"] if f != FAMILY
    ]
    manifest["families"][FAMILY] = {"records": count, "index": INDEX_PATH}
    managed = {row["path"] for row in manifest["managed_files"]}
    managed = {p for p in managed if not p.startswith(CONTENT_DIR)} | set(paths)
    manifest["managed_files"] = [{"path": p} for p in sorted(managed)]
    lock["family_counts"][FAMILY] = count
    return project, manifest, lock


def validate(records: list, items: dict, source_checks: list | None = None) -> list[str]:
    """Rules on the generated records; each error names the claim."""
    errors = []
    keys, bindings = set(), set()
    for row in records:
        record = row["definition"]
        key = record["identity"]["key"]
        if not CLAIM_KEY.match(key):
            errors.append(f"{key}: claim key is not oteryn:reward-claim.<marker>")
        if key in keys:
            errors.append(f"{key}: duplicate claim key")
        keys.add(key)
        if record["claim"] != {"per": "character", "repeat": {"kind": "once"}}:
            errors.append(f"{key}: only per-character once claims are in scope")
        if not record["placements"]:
            errors.append(f"{key}: a claim needs at least one placement")
        ready = True
        for placement in record["placements"]:
            binding = placement["source_binding"]
            ids = binding["legacy_unique_ids"]
            if not ids:
                errors.append(f"{key}: placement has no legacy unique id to bind")
            where = tuple(binding["project_position"].values())
            if where in bindings:
                errors.append(
                    f"{key}: placement {where} is bound by another claim (D40)"
                )
            bindings.add(where)
            if (
                not isinstance(placement["appearance_tibia_id"], int)
                or placement["appearance_tibia_id"] < 1
            ):
                errors.append(f"{key}: appearance_tibia_id must be a positive Tibia id")
            for ref in (r["item"] for r in placement["reward"]["items"]):
                if ref["family"] != "Item" or not A12_ITEM.match(ref["key"]):
                    errors.append(f"{key}: {ref['key']} is not an A12 Item key")
                elif ref["key"] not in items:
                    errors.append(
                        f"{key}: {ref['key']} has no Item record in content/items"
                    )
            rewards = placement["reward"]["items"]
            if len(rewards) != 1:
                errors.append(
                    f"{key}: a placement rewards exactly one item (CHEST-1 §5.1)"
                )
            for reward in rewards:
                if reward["count"] < 1:
                    errors.append(f"{key}: reward count must be positive")
                ready &= (
                    (source_subtype_problem(items.get(reward["item"]["key"]))
                     or stack_problem(items.get(reward["item"]["key"]), reward["count"]))
                    is None
                )
        # Only a false `ready` is unsafe. A stale `waiting_item_semantics` (ITEM-SEM has since
        # covered the Items) stays valid until the next rebuild, so other lanes' content/items
        # changes never break this family.
        if record["readiness"] == "ready" and not ready:
            errors.append(f"{key}: marked ready but an Item cannot be minted as given")
        elif record["readiness"] not in ("ready", "waiting_item_semantics"):
            errors.append(f"{key}: unknown readiness {record['readiness']}")
    collisions = legacy_uid_checks(records)
    if source_checks is None:
        for collision in collisions:
            errors.append(f"duplicate legacy unique id {collision['server']}:{collision['unique_id']}")
    else:
        recorded = [check for check in source_checks if check.get("reason") == "duplicate legacy unique id"]
        if recorded != collisions:
            errors.append("legacy unique id collisions are missing or stale in source_checks")
    return errors


def committed_errors() -> list[str]:
    """The rules on the committed family, against the current content/items."""
    index = load_json(ROOT / INDEX_PATH)
    records = [
        row for shard in index["shards"] for row in load_json(ROOT / shard)["records"]
    ]
    errors = validate(records, load_items(), index["source_checks"])
    if len(records) != index["record_count"]:
        errors.append(
            f"index record_count {index['record_count']} != {len(records)} records"
        )
    return errors


def generate() -> dict[str, str]:
    claims_bytes = (ROOT / CLAIMS_REL).read_bytes()
    claims = json.loads(claims_bytes)
    claims = claims.get("claims", claims) if isinstance(claims, dict) else claims
    items = load_items()
    records, source_checks = build_records(
        claims, load_json(ROOT / MANIFEST_REL), items
    )
    errors = validate(records, items, source_checks)
    if errors:
        raise ValueError("\n".join(errors))
    outputs = content_files(claims_bytes, records, source_checks)
    names = ("project", "manifest", "content.lock")
    docs = [load_json(ROOT / f"content/{name}.json") for name in names]
    for name, doc in zip(
        names, registered(*docs, len(records), sorted(outputs)), strict=True
    ):
        outputs[f"content/{name}.json"] = registry(doc)
    outputs[DIRECTORY_INDEX] = (
        json.dumps(
            directory_index(load_json(ROOT / DIRECTORY_INDEX), len(records)),
            ensure_ascii=False,
            indent=2,
        )
        + "\n"
    )
    return outputs


def content_command(check: bool) -> int:
    try:
        outputs = generate()
    except ValueError as error:
        print(error, file=sys.stderr)
        return 1
    stale = []
    for rel, text in sorted(outputs.items()):
        path = ROOT / rel
        if path.is_file() and path.read_text(encoding="utf-8") == text:
            continue
        stale.append(rel)
        if not check:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8", newline="\n")
    extra = (
        sorted(
            p.relative_to(ROOT).as_posix()
            for p in (ROOT / CONTENT_DIR).glob("*.json")
            if p.relative_to(ROOT).as_posix() not in outputs
        )
        if (ROOT / CONTENT_DIR).is_dir()
        else []
    )
    if check:
        ok = not stale and not extra
        print(
            "reward claim content check: "
            + ("ok" if ok else f"FAIL, stale {stale} extra {extra}")
        )
        return 0 if ok else 1
    for rel in extra:
        (ROOT / rel).unlink()
    print(f"reward claim content: wrote {stale}, removed {extra}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    content = sub.add_parser(
        "content", help="write (or --check) the family and registration"
    )
    content.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.command == "content":
        return content_command(args.check)
    return 2


if __name__ == "__main__":
    sys.exit(main())
