"""Pinned distinct special-item custody contracts for active owned quests."""

import argparse
from pathlib import Path

import build_questpc4_stag_item_profiles as capture_owner
import grizzly_shop_profiles as owner
import questpc4_special_item_profiles as special

IDS = special.IDS
SCHEMA = "OTERYN_QUESTPC4_SPECIAL_ITEM_PROFILES/v1"
PROOF = "tools/content-schema/native-gameplay/questpc4-special-item-profiles.json"
PRODUCER = "tools/content-migration/build_questpc4_special_item_profiles.py"


def capture(source_root, root=capture_owner.ROOT):
    return capture_owner.capture(
        source_root,
        root,
        items=IDS,
        qualify=special.qualify,
        schema=SCHEMA,
        producer=PRODUCER,
        scope="Distinct source armor/lifetime/fluid-container/partial-text/default-weight custody; no custom-use or NPC reward behavior.",
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
        equipment=(8862,),
        ground_only=(21572,),
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
            raise SystemExit("Special item capture is stale")
    else:
        target.write_bytes(raw)
