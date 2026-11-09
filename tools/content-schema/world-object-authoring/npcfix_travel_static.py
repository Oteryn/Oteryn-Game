"""M38: close real Anna/islet/Targuna anchor geometry using exact source flags.

The three ids are absent from Crystal00ce items.xml. Its complete appearances
artifact and the admitted official15.30 artifact agree on every captured flag.
20661 is an unpassable border, not a walkability waiver. No Item/use admission.
"""
import argparse
import hashlib
import json
import sys
from functools import lru_cache
from pathlib import Path

import sugar_room

ROOT = Path(__file__).resolve().parents[3]
IDS = frozenset({20661, 34434, 51921})
PROFILE = "OTERYN_OFFICIAL_CLIENT_NPCFIX_TRAVEL_STATIC/v1"
SHARD = "content/world/terrain/terrain-08693-08695.json"
CAPTURE = "tools/content-schema/world-object-authoring/samples/npcfix-travel-source-flags.json"
CAPTURE_SHA256 = "e4a8aad823a249072f8050c4ae101f6e941ae7be430ee31f1e8512e408af458f"
MARKER_NOTE = " Additionally three exact M38 travel-anchor static appearances; total 8696 Terrain records."


@lru_cache(maxsize=4)
def outputs(root=ROOT):
    files = sugar_room.qualified_outputs(
        root, IDS, PROFILE, SHARD, CAPTURE, CAPTURE_SHA256, MARKER_NOTE,
        "appearance_only_npc_travel",
    )
    # Reuse the existing canonical placement pin generator after source promotion.
    import soulpit_static
    pin = "crates/client-assets/src/world.rs"
    files = dict(files)
    files[pin] = soulpit_static.placement_pin(
        root, files["content/world/placements/index.json"]
    )
    return files


def capture(donor_dat, donor_xml):
    """Reproduce the bounded witness from the complete pinned donor artifacts."""
    dat, xml = donor_dat.read_bytes(), donor_xml.read_bytes()
    if hashlib.sha256(dat).hexdigest() != "17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1":
        raise ValueError("NPC travel donor appearance digest")
    if hashlib.sha256(xml).hexdigest() != "13a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece":
        raise ValueError("NPC travel donor XML digest")
    sys.path.insert(0, str(ROOT / "tools/content-schema/native-gameplay"))
    import build_source_item_profiles as source
    if IDS & source.xml_index(xml).keys():
        raise ValueError("NPC travel static corpus unexpectedly has XML behavior")
    objects = source.engine_items.load_appearance_objects(dat)
    document = {
        "source_revision": "00ce02a57ca5a12e48f32a3476e37471167e4c3f",
        "appearances_sha256": hashlib.sha256(dat).hexdigest(),
        "items_xml_sha256": hashlib.sha256(xml).hexdigest(),
        "flags": {str(item): objects[item]["flags"] for item in sorted(IDS)},
    }
    raw = (json.dumps(document, sort_keys=True, separators=(",", ":")) + "\n").encode()
    if hashlib.sha256(raw).hexdigest() != CAPTURE_SHA256:
        raise ValueError("NPC travel normalized source capture digest")
    (ROOT / CAPTURE).write_bytes(raw)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--donor-dat", type=Path)
    parser.add_argument("--donor-xml", type=Path)
    args = parser.parse_args()
    if args.donor_dat or args.donor_xml:
        if not (args.donor_dat and args.donor_xml) or args.check:
            parser.error("capture needs both donor artifacts and cannot use --check")
        capture(args.donor_dat, args.donor_xml)
        return
    for name, raw in outputs().items():
        path = ROOT / name
        if args.check:
            if not path.is_file() or path.read_bytes() != raw:
                raise SystemExit(f"stale NPC travel static corpus: {name}")
        else:
            path.write_bytes(raw)


if __name__ == "__main__":
    main()
