"""Publish pinned encounter map regions through the existing native provider."""
import hashlib
import json

PROVIDER = "imports/spells/r25/source-world.json"
MANIFEST = "content/spells.manifest.json"
CHANNEL_SHA = "44f860d20712a32e6bc22aa269d3e220d39d164b7738d3c150a406579849b5f5"


def apply_native(root, generated, encode):
    channel = generated[PROVIDER]
    if hashlib.sha256(channel).hexdigest() != CHANNEL_SHA:
        raise ValueError("encounter regions require the unchanged pinned channel source")
    # Crystal 201189f the_primal_manace.lua:8-20 defines the arena and placements;
    # capture_primal_region.py verifies the map and item-loader source blobs.
    primal = (root / "tools/content-schema/native-gameplay/crystal-primal-region.json").read_bytes()
    document = json.loads(primal)
    if document["source_revision"] != "201189f871d23597b82caf88fd611454a2ddefb8" or document["map_git_blob"] != "e61658ec0ab7b70ec35025ae969426d6fc57ae75":
        raise ValueError("unqualified encounter source map")
    endgame = (root / "tools/content-schema/native-gameplay/crystal-endgame-regions.json").read_bytes()
    if json.loads(endgame)["map_git_blob"] != document["map_git_blob"]:
        raise ValueError("endgame source map mismatch")
    bundled = encode({"schema": "OTERYN_NATIVE_SOURCE_REGION_BUNDLE/v1", "channel": channel.decode(), "primal": primal.decode(), "endgame": endgame.decode()})
    generated[PROVIDER] = bundled
    manifest = json.loads(generated[MANIFEST])
    manifest["source_world"]["sha256"] = hashlib.sha256(bundled).hexdigest()
    generated[MANIFEST] = encode(manifest)
