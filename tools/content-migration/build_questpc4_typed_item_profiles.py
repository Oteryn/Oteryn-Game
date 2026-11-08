"""Pinned stack and empty-backpack custody for active owned quest recipes."""

import argparse
from pathlib import Path

import build_questpc4_stag_item_profiles as capture_owner
import grizzly_shop_profiles as owner
import questpc4_typed_item_profiles as typed

IDS = typed.IDS
SCHEMA = "OTERYN_QUESTPC4_TYPED_ITEM_PROFILES/v1"
PROOF = "tools/content-schema/native-gameplay/questpc4-typed-item-profiles.json"
PRODUCER = "tools/content-migration/build_questpc4_typed_item_profiles.py"


def capture(source_root, root=capture_owner.ROOT):
    return capture_owner.capture(
        source_root,
        root,
        items=IDS,
        qualify=typed.qualify,
        schema=SCHEMA,
        producer=PRODUCER,
        scope="Physical custody of four stackable materials and one empty 20-slot backpack; no eating, fluid, repair or quest-use effects.",
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
        equipment=(5949,),
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
