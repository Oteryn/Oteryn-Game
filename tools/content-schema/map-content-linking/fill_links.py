"""Source-backed local candidates; never promote content or activate a World.

Reuse the existing appearance grammar and accepted Spawn.Source core shape.
Asset identities below are explicitly proposals, not admitted public identities.
"""

from __future__ import annotations

import argparse
import bisect
import hashlib
import json
import re
import sys
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/item-authoring"))
import engine_items

OUT = ROOT / "imports/crystalserver/summer-update/map-content-linking"
PIN = "00ce02a57ca5a12e48f32a3476e37471167e4c3f"
MISSING = "minimap-32-0996-0984-02-dce27ae4b4d345201c9cc7f9d4f7576fc9144583779e8716b29032d6c1731073.bmp.zip"
ASSET_MANIFEST = "imports/official/client-assets/15.30/manifest.json"
PALETTE = "content/world/placements/index.json"
XML = "imports/crystalserver/summer-update/raw/data-global/world/world-monster.xml"
DIRECTIONS = {None: "north", "0": "north", "1": "east", "2": "south", "3": "west"}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def dump(value):
    return (
        json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()


def asset_ref(digest):
    return {"key": "oteryn:asset.official1530.sha256." + digest, "revision": "asset-r1"}


def intervals(catalog):
    rows = sorted(
        (r for r in catalog if r["type"] == "sprite"), key=lambda r: r["firstspriteid"]
    )
    for i, row in enumerate(rows):
        if row["firstspriteid"] > row["lastspriteid"]:
            raise ValueError("reversed sprite range")
        if i and rows[i - 1]["lastspriteid"] >= row["firstspriteid"]:
            raise ValueError("overlapping sprite ranges")
    return rows, [r["firstspriteid"] for r in rows]


def sheet_for(sprite, rows, starts):
    i = bisect.bisect_right(starts, sprite) - 1
    if i < 0 or sprite > rows[i]["lastspriteid"]:
        raise ValueError(f"sprite without a sheet: {sprite}")
    return rows[i]


def spawn_core(xml, known):
    if b"<!DOCTYPE" in xml.upper() or b"<!ENTITY" in xml.upper():
        raise ValueError("DTD/entity XML is forbidden")
    records, held, seen = [], [], Counter()
    groups = list(ET.fromstring(xml))
    raw_points = 0
    for ordinal, group in enumerate(groups):
        raw_points += len(group)
        reasons, points = [], []
        cx, cy, cz = (int(group.get(k)) for k in ("centerx", "centery", "centerz"))
        base = f"oteryn:spawn.x{cx}_y{cy}_z{cz}"
        seen[base] += 1
        key = base if seen[base] == 1 else f"{base}_{seen[base]}"
        if not 1 <= len(group) <= 64:
            reasons.append("POINT_COUNT_OUTSIDE_ACCEPTED_1_64")
        if not (0 <= cx <= 65535 and 0 <= cy <= 65535 and 0 <= cz <= 15):
            reasons.append("INVALID_CENTRE")
        cells = set()
        for point in group:
            name = point.get("name", "")
            target = "oteryn:creature." + re.sub(
                r"[^a-z0-9]+", "_", name.lower()
            ).strip("_")
            if target not in known:
                reasons.append("UNBOUND_CREATURE:" + name)
            if set(point.attrib) - {"name", "x", "y", "z", "direction", "spawntime"}:
                reasons.append("POINT_FIELDS_NOT_REPRESENTED_BY_ACCEPTED_CORE")
            x, y = cx + int(point.get("x")), cy + int(point.get("y"))
            z = int(point.get("z", cz))
            delay = int(point.get("spawntime", "0")) * 1000
            direction = DIRECTIONS.get(point.get("direction"))
            if z != cz or not (0 <= x <= 65535 and 0 <= y <= 65535):
                reasons.append("POINT_OFF_SOURCE_FLOOR_OR_PLANE")
            if not 1000 <= delay <= 86400000:
                reasons.append("RESPAWN_DELAY_OUTSIDE_ACCEPTED_RANGE")
            if direction is None:
                reasons.append("UNSUPPORTED_DIRECTION")
            if (x, y, z) in cells:
                reasons.append("SAME_CELL_SELECTION_REQUIRES_OWNING_CONTRACT")
            cells.add((x, y, z))
            points.append(
                {
                    "cell": {"floor": z, "x": x, "y": y},
                    "creature": target,
                    "direction": direction,
                    "respawn_ms": delay,
                }
            )
        if reasons:
            held.append(
                {
                    "source_group_ordinal": ordinal,
                    "key": key,
                    "point_count": len(group),
                    "reasons": sorted(set(reasons)),
                    "raw_attributes": dict(group.attrib),
                    "raw_points": [dict(p.attrib) for p in group],
                }
            )
        else:
            records.append(
                {
                    "declaration": {
                        "identity": {"key": key, "revision": "definition-r1"},
                        "centre": {"floor": cz, "x": cx, "y": cy},
                        "points": points,
                    }
                }
            )
    assert (
        sum(len(r["declaration"]["points"]) for r in records)
        + sum(h["point_count"] for h in held)
        == raw_points
    )
    return records, held, {"raw_groups": len(groups), "raw_points": raw_points}


def generate():
    inputs = {}

    def consume(relative, pin=None):
        p = ROOT / relative
        if not p.is_file() or any(member.is_symlink() for member in [p, *p.parents]):
            raise ValueError(f"missing/nonregular input: {relative}")
        data = p.read_bytes()
        digest = sha(data)
        if pin and digest != pin:
            raise ValueError(f"input pin mismatch: {relative}")
        inputs[relative] = {"sha256": digest, "size": len(data)}
        return data

    manifest = json.loads(
        consume(
            ASSET_MANIFEST,
            "febaff9f4bd7e0f8a029736e446a81f1626805e895e7c2268018e0a9a8493fe4",
        )
    )
    existing = {
        p.name for p in (ROOT / "content/assets/files").iterdir() if p.is_file()
    }
    expected = {row["name"] for row in manifest["files"]}
    if expected - existing != {MISSING} or existing - expected:
        raise ValueError("asset inventory differs from owner-confirmed6248-file set")
    files, assets = {}, {}
    for row in manifest["files"]:
        if row["name"] == MISSING:
            continue
        relative = "content/assets/files/" + row["name"]
        data = consume(relative, row["sha256"])
        if len(data) != row["bytes"]:
            raise ValueError("asset size mismatch: " + relative)
        ref = asset_ref(row["sha256"])
        files[row["name"]] = {"asset": ref, **inputs[relative]}
        assets[ref["key"]] = {"identity": ref, "sha256": row["sha256"]}
    catalog = json.loads(
        consume(
            "content/assets/files/catalog-content.json",
            "4921ef60464a0baea0aa25e02c5c7843a42196767951c453400e81a67cc3b424",
        )
    )
    rows, starts = intervals(catalog)
    for row in rows:
        if row["file"] not in files:
            raise ValueError("catalog sheet file missing")
    dat = next(row["file"] for row in catalog if row["type"] == "appearances")
    appearances = engine_items.load_appearance_objects(
        consume("content/assets/files/" + dat)
    )
    palette = json.loads(consume(PALETTE))
    if palette["source"]["revision"] != PIN:
        raise ValueError("map palette is not the pinned Crystal source")
    visual, nulls = [], []
    for slot, entry in enumerate(palette["palette"]):
        sid = entry["source_item_id"]
        if sid not in appearances:
            if sid not in {99, 2141}:
                raise ValueError("unexplained missing appearance")
            nulls.append({"slot": slot, "source_item_id": sid, "key": entry["key"]})
            continue
        groups = appearances[sid]["frame_groups"]
        sheets = {}
        for group in groups:
            for sprite in group["sprite_ids"]:
                row = sheet_for(sprite, rows, starts)
                sheets[row["file"]] = {**row, **files[row["file"]]}
        visual.append(
            {
                "palette_slot": slot,
                "key": entry["key"],
                "source_appearance_id": sid,
                "appearance_asset": files[dat]["asset"],
                "frame_groups": groups,
                "sheets": [sheets[k] for k in sorted(sheets)],
            }
        )
    consume("tools/content-schema/item-authoring/engine_items.py")
    consume("tools/content-schema/map-content-linking/fill_links.py")
    consume("tools/content-schema/map-content-linking/test_fill_links.py")
    presentation_joins, presentation_held = [], []
    slots_by_id = {}
    for link in visual:
        slots_by_id.setdefault(link["source_appearance_id"], []).append(
            link["palette_slot"]
        )
    for p in sorted(
        (ROOT / "content/presentations/definitions").glob("presentations-*.json")
    ):
        doc = json.loads(consume(p.relative_to(ROOT).as_posix()))
        for record in doc["records"]:
            identity = record["definition"]["identity"]
            binding = (
                record.get("authoring", {}).get("profile", {}).get("asset_binding")
            )
            match = re.fullmatch(r"canary\.appearance:object/(\d+)", binding or "")
            entry = {"presentation": identity, "existing_source_binding": binding}
            if not match:
                entry["reason"] = (
                    "OUTFIT_OUTSIDE_MAP_OBJECT_SCOPE"
                    if binding
                    else "NO_SOURCE_BINDING"
                )
                presentation_held.append(entry)
                continue
            sid = int(match.group(1))
            if sid not in appearances:
                entry["reason"] = "OBJECT_ABSENT_FROM_PINNED_1530_APPEARANCES"
                presentation_held.append(entry)
                continue
            sheets = {}
            for group in appearances[sid]["frame_groups"]:
                for sprite in group["sprite_ids"]:
                    row = sheet_for(sprite, rows, starts)
                    sheets[row["file"]] = {**row, **files[row["file"]]}
            presentation_joins.append(
                {
                    **entry,
                    "proposed_1530_object_id": sid,
                    "matching_palette_slots": slots_by_id.get(sid, []),
                    "appearance_asset": files[dat]["asset"],
                    "sheets": [sheets[k] for k in sorted(sheets)],
                    "status": "SOURCE_ID_JOIN_ONLY_CANARY_TO_1530_VERSION_EQUIVALENCE_NOT_QUALIFIED",
                }
            )
    tree = json.loads(
        consume(
            "imports/crystalserver/summer-update/source-tree.json",
            "40eeb39b4d01b0f5ed5134734ee08bdd6148fad44385c47ee05473282b718878",
        )
    )
    if tree["sha"] != PIN or tree["truncated"] is not False:
        raise ValueError("incomplete/wrong Crystal tree")
    license_bytes = consume("imports/crystalserver/summer-update/raw/LICENSE")
    license_blob = next(r["sha"] for r in tree["tree"] if r["path"] == "LICENSE")
    if (
        hashlib.sha1(
            b"blob " + str(len(license_bytes)).encode() + b"\0" + license_bytes
        ).hexdigest()
        != license_blob
    ):
        raise ValueError("source license Git blob mismatch")
    xml = consume(
        XML, "a3188bc1275fbf5bac1ff5c06cc26b1d2999e51c088a7ffa464c40a1aff81570"
    )
    blob = next(
        r["sha"]
        for r in tree["tree"]
        if r["path"] == "data-global/world/world-monster.xml"
    )
    if (
        hashlib.sha1(b"blob " + str(len(xml)).encode() + b"\0" + xml).hexdigest()
        != blob
    ):
        raise ValueError("XML Git blob mismatch")
    known = set()
    for p in sorted((ROOT / "content/creatures/definitions").glob("creatures-*.json")):
        doc = json.loads(consume(p.relative_to(ROOT).as_posix()))
        known.update(r["definition"]["identity"]["key"] for r in doc["records"])
    records, held, counts = spawn_core(xml, known)
    outputs = {
        "assets/catalog.json": {
            "schema": "OTERYN_WORLD_PROJECT_ASSETS/v2",
            "assets": [assets[k] for k in sorted(assets)],
        },
        "assets/files.json": {
            "client_version": "15.30",
            "runtime_admission": False,
            "files": files,
            "missing_owner_confirmed_file": MISSING,
            "identity_status": "LOCAL_ASSET_IDENTITY_PROPOSAL",
        },
        "graphics/map-appearance-links.json": {
            "schema": "LOCAL_MAP_APPEARANCE_FILE_LINKS/v1",
            "runtime_admission": False,
            "records": visual,
            "source_null_palette_slots": nulls,
            "limits": [
                "Source file/geometry/sprite links only; animation timing and client lowering not qualified.",
                "No Presentation definitions/bindings minted; WorldProject requires exact existing Presentation references.",
            ],
        },
        "spawns/held-groups.json": {
            "runtime_admission": False,
            "source_pin": PIN,
            "records": held,
        },
    }
    outputs["graphics/existing-presentation-joins.json"] = {
        "runtime_admission": False,
        "records": presentation_joins,
        "outside_scope_or_unbound": presentation_held,
        "limits": [
            "Existing Presentation identities only; no native bindings or definitions minted.",
            "Equal appearance IDs across source profiles are evidence joins, not proved visual equivalence.",
            "Multiple sheet assets and DAT require the owning client lowering contract.",
        ],
    }
    shards = []
    for start in range(0, len(records), 2000):
        part = records[start : start + 2000]
        name = f"spawns-{start:05d}-{start + len(part) - 1:05d}.json"
        shards.append("content/world/spawns/" + name)
        outputs["spawns/candidate/content/world/spawns/" + name] = {
            "coordinate_frame": "global-target-2026-09-27",
            "family": "Spawn.Source",
            "records": part,
        }
    outputs["spawns/candidate/content/world/spawns/index.json"] = {
        "coordinate_frame": "global-target-2026-09-27",
        "family": "Spawn.Source",
        "schema": "OTERYN_FAMILY_INDEX/v1",
        "population_state": "CANDIDATE_NOT_RUNTIME_ADMITTED",
        "shard_size": 2000,
        "shards": shards,
        "record_count": len(records),
        "point_count": sum(len(r["declaration"]["points"]) for r in records),
        "source": {
            "repository": "zimbadev/crystalserver",
            "revision": PIN,
            "ref": "summer-update",
            "source_key": "oteryn:source.crystalserver",
            "evidence": "OtsHypothesisOnly",
            "files": [
                {"path": "data-global/world/world-monster.xml", "sha256": sha(xml)}
            ],
        },
    }
    report = {
        "runtime_admission": False,
        "canonical_content_changed": False,
        "source_pin": PIN,
        "palette_identity_scope": "CURRENT_MAIN_KEYS_PROVISIONAL_DONOR_KEYS_PRESERVED",
        "provisional_palette_slots": sum(p["provisional"] for p in palette["palette"]),
        "assets_files_verified": len(files),
        "unique_asset_proposals": len(assets),
        "sprite_sheets_verified": len(rows),
        "palette_appearance_links": len(visual),
        "source_null_slots": len(nulls),
        "existing_presentation_object_source_joins": len(presentation_joins),
        "presentations_outside_scope_or_unbound": len(presentation_held),
        "primary_Crystal_spawn": {
            **counts,
            "candidate_groups": len(records),
            "candidate_points": sum(len(r["declaration"]["points"]) for r in records),
            "held_groups": len(held),
            "held_points": sum(h["point_count"] for h in held),
        },
        "limitations": [
            "World Bundle v3 currently specifies Canary source; Crystal source-profile replacement is a local proposal.",
            "Native spawn core omits Source radius/config/random redistribution; raw XML remains pinned, no source behaviour1to1 claim.",
            "Only primary data-global/world XML projected; alternate/event/archive variants remain in complete existing population staging.",
            "No Creature identities minted; no source group partially deleted to hide an unbound/unsupported point.",
            "No tile/walkability/house/boss/encounter or runtime admission qualification by this producer.",
            "Asset identities are proposals; exact Presentation family joining requires the owning binding contract.",
        ],
    }
    outputs["report.json"] = report
    for relative, expected_pin in inputs.items():
        if sha((ROOT / relative).read_bytes()) != expected_pin["sha256"]:
            raise ValueError("input changed during generation: " + relative)
    encoded = {p: dump(d) for p, d in outputs.items()}
    encoded["manifest.json"] = dump(
        {
            "schema": "LOCAL_MAP_CONTENT_LINKING_CANDIDATE/v1",
            "runtime_admission": False,
            "inputs": inputs,
            "outputs": {
                p: {"sha256": sha(b), "size": len(b)} for p, b in encoded.items()
            },
        }
    )
    return encoded, report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    encoded, report = generate()
    if any(p.is_symlink() for p in [OUT, *OUT.parents]):
        raise SystemExit("symlink output ancestor is forbidden")
    if any(p.is_symlink() for p in OUT.rglob("*")):
        raise SystemExit("symlink output member is forbidden")
    existing = {p.relative_to(OUT).as_posix(): p for p in OUT.rglob("*") if p.is_file()}
    if args.check:
        expected_dirs = {
            parent.as_posix()
            for k in encoded
            for parent in Path(k).parents
            if parent != Path(".")
        }
        actual_dirs = {
            p.relative_to(OUT).as_posix() for p in OUT.rglob("*") if p.is_dir()
        }
        if actual_dirs != expected_dirs:
            raise SystemExit("candidate directory inventory differs")
        if set(existing) != set(encoded) or any(
            p.is_symlink() or p.read_bytes() != encoded[k] for k, p in existing.items()
        ):
            raise SystemExit("candidate differs from exact source re-derivation")
    else:
        if existing:
            raise SystemExit("preserve previous candidate; refuse overwrite")
        for relative, data in encoded.items():
            p = OUT / relative
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_bytes(data)
    print(json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    main()
