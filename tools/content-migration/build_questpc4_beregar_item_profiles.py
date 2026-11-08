"""Pinned Beregar custody with exact plain-item and equipment source contracts."""

import argparse
from pathlib import Path

import build_questpc4_stag_item_profiles as capture_owner
import grizzly_shop_profiles as owner
import questpc4_beregar_legs_profile as legs
import questpc4_plain_item_profiles as plain

PLAIN_IDS = (8774, 8777)
IDS = (3398,) + PLAIN_IDS
SCHEMA = "OTERYN_QUESTPC4_BEREGAR_ITEM_PROFILES/v2"
PROOF = "tools/content-schema/native-gameplay/questpc4-beregar-item-profiles.json"
PRODUCER = "tools/content-migration/build_questpc4_beregar_item_profiles.py"


def qualify(item_id, node, appearance, physical, bindings, definition):
    if item_id == 3398:
        return legs.qualify(item_id, node, appearance, physical, bindings, definition)
    return plain.qualify(
        PLAIN_IDS, item_id, node, appearance, physical, bindings, definition
    )


def capture(source_root, root=capture_owner.ROOT):
    return capture_owner.capture(
        source_root,
        root,
        items=IDS,
        qualify=qualify,
        schema=SCHEMA,
        producer=PRODUCER,
        scope="Physical custody of two plain Beregar NPC items and source-qualified dwarven legs; no fluid or custom use activation.",
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
        equipment=(3398,),
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
            raise SystemExit("Beregar item capture is stale")
    else:
        target.write_bytes(raw)
