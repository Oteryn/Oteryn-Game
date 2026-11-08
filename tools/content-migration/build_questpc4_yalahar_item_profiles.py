"""Pinned physical custody for eight plain Yalahar NPC items; no use effects."""

import argparse
from pathlib import Path

import build_questpc4_stag_item_profiles as capture_owner
import grizzly_shop_profiles as owner

IDS = (8758, 8759, 8760, 8761, 8762, 8763, 8818, 8819)
SCHEMA = "OTERYN_QUESTPC4_YALAHAR_ITEM_PROFILES/v1"
PROOF = "tools/content-schema/native-gameplay/questpc4-yalahar-item-profiles.json"
PRODUCER = "tools/content-migration/build_questpc4_yalahar_item_profiles.py"


def qualify(item_id, node, appearance, physical, bindings, definition):
    if item_id not in IDS or node.get("id") != str(item_id):
        raise ValueError("YALAHAR_ITEM_IDENTITY")
    if set(owner.source.attrs(node)) - {
        "primarytype",
        "description",
        "weight",
    } or node.findall("attribute[@key='script']"):
        raise ValueError("YALAHAR_ITEM_SOURCE_ATTRIBUTES")
    if any(
        appearance["flags"].get("flags." + flag)
        for flag in ("cumulative", "container", "liquidcontainer", "liquidpool")
    ):
        raise ValueError("YALAHAR_ITEM_INSTANCE_FLAGS")
    record = owner.qualify(item_id, node, appearance, physical, bindings, definition)
    if record["semantics"]["equipment"] != owner.source.NA or record["semantics"][
        "protection"
    ] != {"state": "UNKNOWN"}:
        raise ValueError("YALAHAR_ITEM_NON_EQUIPMENT_FACTS")
    return record


def capture(source_root, root=capture_owner.ROOT):
    return capture_owner.capture(
        source_root,
        root,
        items=IDS,
        qualify=qualify,
        schema=SCHEMA,
        producer=PRODUCER,
        scope="Physical custody of eight plain Yalahar NPC items; no equipment, fluid or custom use activation.",
    )


def admit(root, generated, profile_path, manifest_path, order, canonical_bytes):
    capture_owner.admit(
        root,
        generated,
        profile_path,
        manifest_path,
        order,
        canonical_bytes,
        proof=PROOF,
        items=IDS,
        schema=SCHEMA,
        neutral=(),
        producer=PRODUCER,
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw = owner.encode(capture(args.source_root))
    target = capture_owner.ROOT / PROOF
    if args.check:
        if target.read_bytes() != raw:
            raise SystemExit("Yalahar item capture is stale")
    else:
        target.write_bytes(raw)
