"""Lower explicit own-Object immobile=no witnesses into portable Item metadata."""

import argparse
import json
import re
from collections import defaultdict

import lower_wiki_stack_default_packet as base
from engine_items import load_appearance_objects

ROOT = base.ROOT
COMPILER = "tools/content-schema/item-authoring/lower_wiki_movable_packet.py"
PROOF = "docs/agents/evidence/OTV2-20261001-item-movable-source-qualification-v1.json"
PROOF_SHA = "ae0698adc7df7e25b88569b22de87a020661381a0623ea84d5333a576b12be06"
BASE_SHA = "5969b009ed326991e6db310b3e3c48640ae5a5ac361c9c34f5619740bff8ce2e"
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261001-item-movable-promotion-v1.json"
COORDS = ("page_id", "revision_id", "revision_timestamp", "content_sha256")


def leaf(definition, path):
    value = definition.get("semantics", {})
    for key in path.split("."):
        if "state" in value:
            if value["state"] != "KNOWN":
                return value
            value = value["value"]
        value = value.get(key, {"state": "UNKNOWN"})
    return value


def own_index(proof):
    """Reproduce the complete integer-mention index, including malformed own fields."""
    indexed, pages = defaultdict(set), {}
    parts = {p["name"]: p for p in proof["source_parts"]}
    for page in proof["complete_own_id_occurrence_projection"]:
        pid = page["page_id"]
        if (
            pid in pages
            or parts[page["source_part"]]["sha256"] != page["source_part_sha256"]
            or parts[page["source_part"]]["status"] != 200
            or page["revision_timestamp"] > proof["qualification_cutoff"]
        ):
            raise ValueError("global capture coordinate/part drift")
        pages[pid] = page
        for box in page["own_objects"]:
            for value in box["raw_itemid_values"]:
                for iid in re.findall(r"\d+", value):
                    indexed[int(iid)].add(pid)
    expected = proof["expected_complete_own_integer_index"]
    if (
        len(pages) != 9980
        or {str(k): sorted(v) for k, v in indexed.items()} != expected
    ):
        raise ValueError("complete own-ID projection/index drift")
    return indexed, pages


def reasons(iid, definition, binding, obj, routed, observations, pages):
    errors, sources = [], []
    if (
        not binding
        or binding["external_id"] != str(iid)
        or binding["target"] != definition["identity"]
    ):
        errors.append("EXACT_BINDING_NATIVE_IDENTITY_DRIFT")
    key = definition["identity"]["key"]
    if key in routed:
        errors.append("EXISTING_WORLD_OWNER")
    flags = obj.get("flags", {})
    if flags.get("flags.take") is not True or any(
        flags.get(k) is True
        for k in (
            "flags.clip",
            "flags.bank",
            "flags.ground",
            "flags.border",
            "flags.bottom",
            "flags.top",
            "flags.corpse",
            "flags.player_corpse",
            "flags.liquidpool",
        )
    ):
        errors.append("PORTABLE_ITEM_DOMAIN_NOT_AFFIRMED")
    if flags.get("flags.unmove") is True:
        errors.append("OFFICIAL_UNMOVE_CONFLICT")
    for path, expected in (("physical.movable", True), ("physical.pickupable", True)):
        current = leaf(definition, path)
        if current["state"] != "UNKNOWN" and current != {
            "state": "KNOWN",
            "value": expected,
        }:
            errors.append("BLOCKED_OR_CONFLICTING_" + path.upper())
    official = (obj.get("name") or "").strip().casefold()
    current_name = leaf(definition, "presentation.name")
    if (
        not official
        or current_name["state"] not in {"UNKNOWN", "KNOWN"}
        or (
            current_name["state"] == "KNOWN"
            and current_name["value"].strip().casefold() != official
        )
    ):
        errors.append("NATIVE_OFFICIAL_NAME_CONFLICT")
    for page, box, fields in pages:
        if (
            not box["balanced"]
            or box["inside_comment"]
            or not box["positive_exact_infobox_object_match"]
        ):
            errors.append("UNQUALIFIED_OWN_INFOBOX")
        if fields.get("itemid") != [str(iid)]:
            errors.append("AMBIGUOUS_OR_SHARED_OWN_ITEM_ID")
        names = fields.get("actualname", fields.get("name", []))
        if len(names) != 1 or not names[0] or names[0].strip().casefold() != official:
            errors.append("RAW_ACTUALNAME_OFFICIAL_MISMATCH")
        if "immobile" in fields and fields["immobile"] != ["no"]:
            errors.append("COMPETING_IMMOBILE_VALUE")
        if fields.get("immobile") == ["no"]:
            sources.append(page["page_id"])
    if not sources:
        errors.append("NO_EXPLICIT_IMMOBILE_NO")
    for obs in observations:
        matches = [p for p, _, _ in pages if p["page_id"] == obs["page_id"]]
        if (
            len(matches) != 1
            or any(obs.get(k) != matches[0].get(k) for k in COORDS)
            or obs.get("wiki_title") != matches[0].get("title")
        ):
            errors.append("CURRENT_IMPORTED_RAW_COORDINATE_MISMATCH")
    if not observations:
        errors.append("NO_CURRENT_IMPORTED_OBSERVATION")
    return sorted(set(errors)), sources


def build(root=ROOT):
    base.checked(root, base.COMPILER, BASE_SHA)
    base.checked(root, base.DECODER, base.DECODER_SHA)
    base.checked(root, base.PARSER, base.PARSER_SHA)
    proof = json.loads(base.checked(root, PROOF, PROOF_SHA))
    bridge = proof["identity_bridge"]
    bridge_proof = json.loads(base.checked(root, bridge["path"], bridge["sha256"]))
    for path, digest in proof["input_digests"].items():
        base.checked(root, path, digest)
    if not (
        len(proof["scope_source_item_ids"])
        == len(set(proof["scope_source_item_ids"]))
        == 5704
    ):
        raise ValueError("closed candidate scope drift")
    indexed, global_pages = own_index(proof)
    witnesses = proof["competing_own_infobox_witnesses"]
    parsed = {}
    for page in global_pages.values():
        for box in page["own_objects"]:
            token = f"{page['page_id']}:{box['box_index']}"
            if token not in witnesses:
                continue
            witness = witnesses[token]
            raw = witness["raw_own_infobox"]
            if base.sha(raw.encode()) != witness["raw_own_infobox_sha256"]:
                raise ValueError("raw own-Object witness digest drift")
            fields = base.raw_parameters(raw)
            if fields.get("itemid", []) != box["raw_itemid_values"]:
                raise ValueError("raw witness/complete own-ID projection mismatch")
            parsed[token] = (page, box, fields)
    objects = load_appearance_objects(base.checked(root, base.CLIENT, base.CLIENT_SHA))
    bindings = base.exact_bindings(
        json.loads((root / base.BINDINGS).read_text())["bindings"],
        bridge_proof["bridge"]["source_revisions"],
    )
    by_id = {}
    for key, binding in bindings.items():
        iid = int(binding["external_id"])
        if binding["external_id"] != str(iid) or iid in by_id:
            raise ValueError("noncanonical or duplicate external source identity")
        by_id[iid] = (key, binding)
    definitions = {
        r["definition"]["identity"]["key"]: r["definition"]
        for shard in json.loads((root / "content/items/index.json").read_text())[
            "shards"
        ]
        for r in json.loads((root / shard).read_text())["records"]
    }
    routed, map_inputs = set(), {}
    for family in ("objects", "terrain"):
        for path in sorted((root / f"content/world/{family}").glob("*.json")):
            map_inputs[str(path.relative_to(root))] = base.sha(path.read_bytes())
            for record in json.loads(path.read_text()).get("records", []):
                if record.get("provenance", {}).get("item_pointer"):
                    routed.add(record["provenance"]["item_pointer"]["key"])
    snapshot = json.loads((root / base.WIKI).read_text())
    observations = {
        r["item_id"]: r["observations"] for r in snapshot["records"].values()
    }
    rows, holds = [], []
    for iid in proof["scope_source_item_ids"]:
        key, binding = by_id[iid]
        own = []
        for pid in sorted(indexed[iid]):
            page = global_pages[pid]
            for box in page["own_objects"]:
                if not any(
                    str(iid) in re.findall(r"\d+", v) for v in box["raw_itemid_values"]
                ):
                    continue
                token = f"{pid}:{box['box_index']}"
                if token not in parsed:
                    raise ValueError("competing own-ID raw witness missing")
                own.append(parsed[token])
        blocked, sources = reasons(
            iid,
            definitions[key],
            binding,
            objects[iid],
            routed,
            observations.get(iid, []),
            own,
        )
        if blocked:
            holds.append({"source_item_id": iid, "item_key": key, "reasons": blocked})
        else:
            rows.append(
                {
                    "item_key": key,
                    "kind": "MOVABLE_TRUE",
                    "source_item_id": iid,
                    "definition_identity": definitions[key]["identity"],
                    "binding": binding,
                    "source_page_ids": sources,
                    "appearance_id": iid,
                }
            )
    if len(rows) != 5692 or len(holds) != 12:
        raise ValueError(f"closed movable scope drift: {len(rows)}/{len(holds)}")
    return {
        "schema": "OTERYN_ITEM_PHYSICAL_PROMOTION/v1",
        "compiler": {
            "path": COMPILER,
            "sha256": base.sha((root / COMPILER).read_bytes()),
        },
        "source_policy": "EXPLICIT_IMMOBILE_NO_PORTABLE_ITEM_COMPACT_OWN_INFOBOX_WITNESS",
        "sources": {
            "proof_path": PROOF,
            "proof_sha256": PROOF_SHA,
            "base_compiler_sha256": BASE_SHA,
            "decoder_sha256": base.DECODER_SHA,
            "client_sha256": base.CLIENT_SHA,
            "world_owner_inputs": map_inputs,
        },
        "counts": {"promotions": len(rows), "fields": len(rows), "holds": len(holds)},
        "promotions": rows,
        "holds": holds,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    packet = build()
    data = (
        json.dumps(packet, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("movable promotion packet drift")
    else:
        OUTPUT.write_bytes(data)
    print(json.dumps(packet["counts"]))


if __name__ == "__main__":
    main()
