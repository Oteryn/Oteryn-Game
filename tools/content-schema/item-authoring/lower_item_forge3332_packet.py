"""New singleton September selector for an explicit retained Forge pair."""

import argparse
import json
from pathlib import Path

from engine_items import decode_appearance_object, protobuf_fields
from lower_client_market_packet import WORLD_FLAGS
from lower_wiki_stack_default_packet import checked, sha

ROOT = Path(__file__).resolve().parents[3]
COMPILER = "tools/content-schema/item-authoring/lower_item_forge3332_packet.py"
PROOF = "docs/agents/evidence/OTV2-20261002-item-forge3332-source-qualification-v2.json"
PROOF_SHA = "9132dc6266517b8246a6d575fff9ccda430a08d8c61a63673783e432dcd27084"
RECEIPT = "docs/agents/evidence/OTV2-20261002-item-forge3332-current-integration-receipt-v1.json"
RECEIPT_SHA = "73a21d75b6c32b6132b94160dcb7816fd34911bddf26a02bb5450930ecef3a4f"
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261001-item-forge3332-promotion-v1.json"

DEPENDENCIES = {
    "tools/content-schema/item-authoring/engine_items.py": "0a494fd64d7774507b4d9e493492a7ffc9d99d751dd2ed4fec73561cb06f5b06",
    "tools/content-schema/item-authoring/lower_client_market_packet.py": "850ee01d3b4831c412856b77720c0996122e4f67ed4603b0e6337637dc09457e",
    "tools/content-schema/item-authoring/lower_wiki_stack_default_packet.py": "5969b009ed326991e6db310b3e3c48640ae5a5ac361c9c34f5619740bff8ce2e",
}


def exact_binding(bindings, expected, target):
    fields = ("source_key", "source_revision", "identity_namespace", "external_id")
    actual = [
        b
        for b in bindings
        if (b["source_key"] == expected["source_key"] and b["target"] == target)
        or all(b[k] == expected[k] for k in fields)
    ]
    if actual != [expected] or expected["disposition"] != "EXACT":
        raise ValueError("Forge exact forward/reverse binding drift")
    return actual[0]


def qualify(proof, row, definition, obj, owners, routed):
    target, selector = proof["target"], proof["current_selector"]
    expected = selector["singleton_exact_source"]
    if (
        row != proof["source_pair"]["retained_source_row"]
        or any(
            row[k] != expected[k]
            for k in (
                "external_id",
                "revision_id",
                "revision_timestamp",
                "source_digest",
            )
        )
        or row["revision_timestamp"] > selector["qualification_cutoff"]
    ):
        raise ValueError("Forge singleton source/cutoff drift")
    if row["fields"]["classificacao"] != {"state": "VALUE", "value": "2"} or row[
        "fields"
    ]["max_tier"] != {"state": "VALUE", "value": "2"}:
        raise ValueError("Forge explicit whole source pair required")
    if definition["identity"] != target or target["key"] in routed:
        raise ValueError("Forge current full Item/World target drift")
    name = definition.get("semantics", {}).get("presentation", {})
    if name.get("state") != "KNOWN" or name["value"]["name"] != {
        "state": "KNOWN",
        "value": obj["name"],
    }:
        raise ValueError("Forge current native name blocked/conflict")
    if (
        obj["id"] != 3332
        or obj["name"] != "hammer of wrath"
        or obj["flags"].get("flags.take") is not True
        or any(obj["flags"].get(k) is True for k in (*WORLD_FLAGS, "flags.unmove"))
    ):
        raise ValueError("Forge official portable identity/name drift")
    existing = [o for o in owners if o["item"]["key"] == target["key"]]
    if len(existing) > 1 or any(
        o["item"] != target or o.get("forge") not in (None, proof["value"])
        for o in existing
    ):
        raise ValueError("Forge existing owner duplicate/target/conflict")
    return {"item": target, "forge": proof["value"]}


def build(root=ROOT):
    proof = json.loads(checked(root, PROOF, PROOF_SHA))
    receipt = json.loads(checked(root, RECEIPT, RECEIPT_SHA))
    historical = proof["historical_qualification"]
    checked(root, historical["path"], historical["sha256"])
    for path, digest in DEPENDENCIES.items():
        checked(root, path, digest)
    # Historical baseline native/shard hashes are observations, not current leaf pins.
    for path, digest in proof["input_digests"].items():
        if (
            path.startswith(("imports/", "docs/agents/evidence/"))
            or path == "content/items/aliases.json"
        ):
            checked(root, path, digest)
    pair, bridge, official = (
        proof["source_pair"],
        proof["identity_bridge"],
        proof["official_membership_and_name"],
    )
    rows = json.loads(checked(root, pair["artifact_path"], pair["artifact_sha256"]))[
        "rows"
    ]
    row = rows[pair["row_ordinal"]]
    canonical = json.dumps(
        row, sort_keys=True, ensure_ascii=False, separators=(",", ":")
    )
    if sha(canonical.encode()) != pair["source_row_canonical_sha256"]:
        raise ValueError("Forge retained structured row digest drift")
    bindings = []
    for path, key in (
        ("imports/tibiawiki/bindings/items.json", "br_to_full_native"),
        (
            "imports/crystalserver/bindings/items.json",
            "crystal_appearance_to_full_native",
        ),
    ):
        expected = bridge[key]
        source = json.loads(checked(root, path, expected["file_sha256"]))["bindings"]
        bindings.append(exact_binding(source, expected["row"], proof["target"]))
    aliases = json.loads(
        checked(root, "content/items/aliases.json", bridge["alias_file_sha256"])
    )["entries"]
    alias = bridge["retained_target_alias"]
    if [a for a in aliases if a["key"] == row["target"]["key"]] != [alias] or alias[
        "target"
    ] != proof["target"]["key"]:
        raise ValueError("Forge retained target alias drift")
    admitted = json.loads(
        checked(
            root, official["admitted_index_path"], official["admitted_index_sha256"]
        )
    )
    descriptor = official["newest_descriptor"]
    if (
        admitted["newest"] != descriptor["label"]
        or max(admitted["files"], key=lambda file: file["order"]) != descriptor
    ):
        raise ValueError("Forge current official descriptor drift")
    membership = json.loads(
        checked(root, official["membership_path"], official["membership_sha256"])
    )
    if [e for e in membership["entries"] if e[0] == 3332] != [official["entry"]]:
        raise ValueError("Forge current official membership drift")
    client_path = f"content/assets/files/appearances-{official['complete_official_appearance_sha256']}.dat"
    objects = [
        (decode_appearance_object(raw), sha(raw))
        for tag, raw in protobuf_fields(
            checked(root, client_path, official["complete_official_appearance_sha256"])
        )
        if tag == 1
    ]
    matches = [(obj, digest) for obj, digest in objects if obj["id"] == 3332]
    if len(matches) != 1 or matches[0][1] != official["object_sha256"]:
        raise ValueError("Forge current official object drift")
    definitions = [
        r["definition"]
        for s in json.loads((root / "content/items/index.json").read_text())["shards"]
        for r in json.loads((root / s).read_text())["records"]
        if r["definition"]["identity"]["key"] == proof["target"]["key"]
    ]
    if len(definitions) != 1:
        raise ValueError("Forge current unique Item missing")
    if (
        definitions[0] != receipt["current_definition"]
        or proof["target"] != receipt["target"]
    ):
        raise ValueError("Forge current integration definition drift")
    routed, world = set(), {}
    for family in ("terrain", "objects"):
        for path in sorted((root / f"content/world/{family}").glob("*.json")):
            world[str(path.relative_to(root))] = sha(path.read_bytes())
            routed.update(
                r["provenance"]["item_pointer"]["key"]
                for r in json.loads(path.read_text()).get("records", [])
                if r.get("provenance", {}).get("item_pointer")
            )
    owners = json.loads(
        (root / "content/world/definitions/declarations.json").read_text()
    )["item_authoring"]
    draft_bindings = json.loads(
        (root / "content/world/provenance/sources.json").read_text()
    )["source_identity_bindings"]
    current_br = exact_binding(draft_bindings, bindings[0], proof["target"])
    result = qualify(proof, row, definitions[0], matches[0][0], owners, routed)
    return {
        "schema": "OTERYN_ITEM_FORGE3332_PROMOTION/v1",
        "compiler": {"path": COMPILER, "sha256": sha((root / COMPILER).read_bytes())},
        "sources": {
            "proof_path": PROOF,
            "proof_sha256": PROOF_SHA,
            "current_receipt_path": RECEIPT,
            "current_receipt_sha256": RECEIPT_SHA,
        },
        "compiler_dependencies": DEPENDENCIES,
        "world_owner_inputs": world,
        "bindings": [current_br],
        "qualified_identity_bridge": bindings,
        "promotion": result,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = (
        json.dumps(build(), sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("Forge3332 packet drift")
    else:
        OUTPUT.write_bytes(data)
    print("Forge3332:2 authoring fields/1Item")


if __name__ == "__main__":
    main()
