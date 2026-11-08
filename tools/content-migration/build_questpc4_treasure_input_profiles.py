"""Pinned custody for six selected Treasure Hunt inputs, without use activation."""

import argparse
from pathlib import Path

import build_questpc4_stag_item_profiles as capture_owner
import grizzly_shop_profiles as owner
import questpc4_treasure_input_profiles as scoped

IDS = scoped.IDS
SCHEMA = "OTERYN_QUESTPC4_TREASURE_INPUT_PROFILES/v1"
PROOF = "tools/content-schema/native-gameplay/questpc4-treasure-input-profiles.json"
PRODUCER = "tools/content-migration/build_questpc4_treasure_input_profiles.py"


def capture(source_root, root=capture_owner.ROOT):
    return capture_owner.capture(
        source_root,
        root,
        items=IDS,
        qualify=scoped.qualify,
        schema=SCHEMA,
        producer=PRODUCER,
        scope="Physical custody of four map pieces, encrypted text and the selected3613 doll; no reading, code activation, exchange or whole-quest inference.",
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
            raise SystemExit("Treasure input capture is stale")
    else:
        target.write_bytes(raw)
