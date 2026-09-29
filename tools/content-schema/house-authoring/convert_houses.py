"""Build a House authoring candidate catalog from the staged 15.30 client observations
(imports/cipsoft-staticdata/houses/, HOUSES-1) joined with CrystalServer world-house.xml.

`extract-crystal` reads a pinned CrystalServer world-house.xml into the committed
compact sample; `extract-door-items` reads the door item ids (`type="door"`) of the
pinned CrystalServer items.xml, because the client does not mark doors; `convert`
joins them 1:1 with the client houses (engine clientid == client house id), validates
the result and writes the conversion report. Official client values win; engine
divergences are reported, never merged.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path

from validate_houses import RESTRICTION_TEXT, validate

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
STAGED = REPO / "imports" / "cipsoft-staticdata" / "houses"
CRYSTAL_REVISION = "00ce02a57ca5a12e48f32a3476e37471167e4c3f"
CRYSTAL_XML_SHA256 = "36044bf9636c5a84cac7dda6582965fba05378822fdce4a5c1f84d0dd7e6e90b"
CRYSTAL_SAMPLE = ROOT / "samples" / "crystal-world-house-00ce02a5.json"
REPORT = ROOT / "samples" / "conversion-report.json"
ITEMS_XML_SHA256 = "13a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece"
DOOR_ITEMS = ROOT / "samples" / "crystal-door-item-ids-00ce02a5.json"
# A door that is in two House layouts belongs to exactly one House. Only one exists in
# 15.30; the engine map puts it on East Lane 1a (source id 20801). Any other shared door
# stops the conversion until it is decided here.
SHARED_DOOR_OWNERS = {(32391, 31799, 6): 20801}
CRYSTAL_ATTRS = {
    "name",
    "houseid",
    "entryx",
    "entryy",
    "entryz",
    "rent",
    "townid",
    "size",
    "clientid",
    "beds",
}
EXPECTED_COUNT = 995


def dump(data) -> str:
    return json.dumps(data, indent=1, ensure_ascii=False, sort_keys=True) + "\n"


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def slug(text: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", text.lower()).strip("_")


def extract_crystal(xml_bytes: bytes) -> dict:
    if sha256(xml_bytes) != CRYSTAL_XML_SHA256:
        raise ValueError("world-house.xml digest mismatch")
    records = []
    for node in ET.fromstring(xml_bytes):
        attrs = dict(node.attrib)
        guildhall = attrs.pop("guildhall", "false")
        if (
            node.tag != "house"
            or set(attrs) != CRYSTAL_ATTRS
            or guildhall not in ("true", "false")
        ):
            raise ValueError(f"unexpected house node {node.tag} {sorted(node.attrib)}")
        records.append(
            {
                "name": attrs["name"],
                "house_id": int(attrs["houseid"]),
                "client_id": int(attrs["clientid"]),
                "entry": {
                    "x": int(attrs["entryx"]),
                    "y": int(attrs["entryy"]),
                    "z": int(attrs["entryz"]),
                },
                "rent": int(attrs["rent"]),
                "town_id": int(attrs["townid"]),
                "size": int(attrs["size"]),
                "beds": int(attrs["beds"]),
                "guildhall": guildhall == "true",
            }
        )
    records.sort(key=lambda r: r["client_id"])
    return {
        "authority": "OtsHypothesisOnly; engine evidence, never Oteryn truth",
        "engine": "crystalserver",
        "path": "data-global/world/world-house.xml",
        "records": records,
        "revision": CRYSTAL_REVISION,
        "sha256": CRYSTAL_XML_SHA256,
    }


def extract_door_items(xml_bytes: bytes) -> dict:
    if sha256(xml_bytes) != ITEMS_XML_SHA256:
        raise ValueError("items.xml digest mismatch")
    ids = set()
    for node in ET.fromstring(xml_bytes).iter("item"):
        kinds = [
            a.get("value") for a in node.findall("attribute") if a.get("key") == "type"
        ]
        if kinds != ["door"]:
            continue
        if node.get("id"):
            ids.add(int(node.get("id")))
        else:
            ids.update(range(int(node.get("fromid")), int(node.get("toid")) + 1))
    return {
        "authority": "OtsHypothesisOnly; engine item classification, never Oteryn truth",
        "engine": "crystalserver",
        "path": "data/items/items.xml",
        "revision": CRYSTAL_REVISION,
        "sha256": ITEMS_XML_SHA256,
        "door_item_ids": sorted(ids),
    }


def layout_cells(layout: dict):
    """(position, items) of each non-empty staticmapdata cell.

    Cell order is floors (ascending z), then x, then y; each cell's `skip` counts the
    empty positions that follow it. Derived by testing every axis order against the
    House tiles of the pinned CrystalServer map (see samples/otbm-tile-check.json).
    """
    origin, dims = layout["origin"], layout["dimensions"]
    width, height, floors = dims["width"], dims["height"], dims["floors"]
    index = 0
    for cell in layout["cells"]:
        if index >= width * height * floors:
            raise ValueError("layout cells exceed the footprint")
        if cell["items"]:
            z, rest = divmod(index, width * height)
            x, y = divmod(rest, height)
            yield (origin["x"] + x, origin["y"] + y, origin["z"] + z), cell["items"]
        index += 1 + cell["skip"]
    if index != width * height * floors:
        raise ValueError("layout cells do not fill the footprint")


def layout_tiles(layout: dict) -> list[list[int]]:
    """Positions of the non-empty staticmapdata cells, sorted."""
    return sorted(list(p) for p, _ in layout_cells(layout))


def layout_doors(layout: dict, door_items: set[int]) -> list[list[int]]:
    """Positions of the cells that hold a door item, sorted."""
    return sorted(
        list(p) for p, items in layout_cells(layout) if door_items & set(items)
    )


def load_staged() -> list[dict]:
    manifest = json.loads((STAGED / "manifest.json").read_text(encoding="utf-8"))
    records = []
    for entry in manifest["files"]:
        raw = (STAGED / entry["path"]).read_bytes()
        if sha256(raw) != entry["sha256"]:
            raise ValueError(f"{entry['path']}: digest mismatch with manifest.json")
        records += json.loads(raw)["records"]
    return records


def convert(
    staged: list[dict], crystal: dict, door_items: set[int]
) -> tuple[dict, dict]:
    engine = {r["client_id"]: r for r in crystal["records"]}
    if len(engine) != len(crystal["records"]) or len(staged) != EXPECTED_COUNT:
        raise ValueError("duplicate engine clientid or unexpected house count")
    if set(engine) != {r["source_id"] for r in staged}:
        raise ValueError("client and engine house id sets differ")
    houses, divergence, towns = [], Counter(), {}
    examples: dict[str, list] = {}
    for record in sorted(staged, key=lambda r: r["source_id"]):
        e = engine[record["source_id"]]
        if towns.setdefault(e["town_id"], record["town"]) != record["town"]:
            raise ValueError(f"engine town {e['town_id']} maps to several client towns")
        name = " ".join(record["name"].split())
        layout = record["layout"]
        footprint = {"origin": layout["origin"], **layout["dimensions"]}
        house = {
            "identity": {
                "key": f"oteryn:content.house.{slug(name)}",
                "revision": "definition-r1",
            },
            "name": name,
            "kind": "guildhall"
            if record["guildhall"]
            else "shop"
            if record["shop"]
            else "private_house",
            "town": {
                "family": "Area",
                "key": f"oteryn:content.area.city.{slug(record['town'])}",
            },
            "entrance": e["entry"],
            "map_marker": record["entrance"],
            "size_sqm": record["size_sqm"],
            "beds": record["beds"],
            "rent_gold": record["rent_gold"],
            "entry_restriction": RESTRICTION_TEXT.get(record["restrictions"]),
            "footprint": footprint,
            "tiles": layout_tiles(layout),
            "doors": [
                door
                for door in layout_doors(layout, door_items)
                if SHARED_DOOR_OWNERS.get(tuple(door), record["source_id"])
                == record["source_id"]
            ],
            "provenance": {
                "source": "cipsoft/staticdata/house_id",
                "client_version": "15.30",
                "source_id": record["source_id"],
                "source_name": record["name"],
                "staticdata_sha256": record["staticdata_sha256"],
                "staticmapdata_sha256": layout["staticmapdata_sha256"],
                "restrictions_text": record["restrictions"],
                "engine_house": {
                    "engine": "crystalserver",
                    "revision": CRYSTAL_REVISION,
                    "house_id": e["house_id"],
                },
            },
        }
        houses.append(house)
        for field, official, observed in (
            ("name", name, e["name"]),
            ("rent_gold", record["rent_gold"], e["rent"]),
            ("beds", record["beds"], e["beds"]),
            ("guildhall", record["guildhall"], e["guildhall"]),
            ("size_sqm", record["size_sqm"], e["size"]),
        ):
            if official != observed:
                divergence[field] += 1
                examples.setdefault(field, []).append(
                    [record["source_id"], official, observed]
                )
        if not any(
            z == e["entry"]["z"]
            and max(abs(x - e["entry"]["x"]), abs(y - e["entry"]["y"])) == 1
            for x, y, z in house["doors"]
        ):
            divergence["entrance_not_next_to_a_door"] += 1
            examples.setdefault("entrance_not_next_to_a_door", []).append(
                record["source_id"]
            )
        ex, ey, ez = (e["entry"][a] for a in "xyz")
        if not any(
            z == ez and max(abs(x - ex), abs(y - ey)) == 1 for x, y, z in house["tiles"]
        ):
            divergence["entrance_not_next_to_house_tile"] += 1
            examples.setdefault("entrance_not_next_to_house_tile", []).append(
                record["source_id"]
            )
    door_houses = Counter(tuple(d) for h in houses for d in h["doors"])
    shared = sorted(d for d, n in door_houses.items() if n > 1)
    if shared:
        raise ValueError(f"doors in two House layouts need an owner: {shared}")
    catalog = {"schema": "OTERYN_HOUSE_AUTHORING/candidate-1", "houses": houses}
    report = {
        "houses": len(houses),
        "joined_by": "crystal clientid == client house id",
        "tiles": sum(len(h["tiles"]) for h in houses),
        "tiles_in_two_house_layouts": sum(
            n > 1
            for n in Counter(tuple(t) for h in houses for t in h["tiles"]).values()
        ),
        "doors": sum(len(h["doors"]) for h in houses),
        "door_owners_decided": {
            ",".join(map(str, p)): o for p, o in sorted(SHARED_DOOR_OWNERS.items())
        },
        "kinds": dict(sorted(Counter(h["kind"] for h in houses).items())),
        "engine_divergence_counts": dict(sorted(divergence.items())),
        "engine_divergence_examples": {k: v[:5] for k, v in sorted(examples.items())},
        "engine_town_ids": {str(k): v for k, v in sorted(towns.items())},
        "towns": dict(sorted(Counter(h["town"]["key"] for h in houses).items())),
    }
    return catalog, report


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    extract = sub.add_parser("extract-crystal")
    extract.add_argument(
        "--xml",
        type=Path,
        required=True,
        help="crystalserver@00ce02a5 data-global/world/world-house.xml",
    )
    extract.add_argument("--check", action="store_true")
    doors = sub.add_parser("extract-door-items")
    doors.add_argument(
        "--items-xml",
        type=Path,
        required=True,
        help="crystalserver@00ce02a5 data/items/items.xml",
    )
    doors.add_argument("--check", action="store_true")
    run = sub.add_parser("convert")
    run.add_argument("--out", type=Path, help="write the candidate catalog here")
    run.add_argument(
        "--check",
        action="store_true",
        help="compare the report with the committed sample",
    )
    args = parser.parse_args(argv)

    if args.command == "extract-crystal":
        text = dump(extract_crystal(args.xml.read_bytes()))
        if args.check:
            ok = CRYSTAL_SAMPLE.read_text(encoding="utf-8") == text
            print(f"{CRYSTAL_SAMPLE.name}: {'ok' if ok else 'differs'}")
            return 0 if ok else 1
        CRYSTAL_SAMPLE.parent.mkdir(exist_ok=True)
        CRYSTAL_SAMPLE.write_text(text, encoding="utf-8")
        return 0

    if args.command == "extract-door-items":
        text = dump(extract_door_items(args.items_xml.read_bytes()))
        if args.check:
            ok = DOOR_ITEMS.read_text(encoding="utf-8") == text
            print(f"{DOOR_ITEMS.name}: {'ok' if ok else 'differs'}")
            return 0 if ok else 1
        DOOR_ITEMS.write_text(text, encoding="utf-8")
        return 0

    crystal = json.loads(CRYSTAL_SAMPLE.read_text(encoding="utf-8"))
    if (
        crystal["revision"] != CRYSTAL_REVISION
        or crystal["sha256"] != CRYSTAL_XML_SHA256
    ):
        raise ValueError("crystal sample is not the pinned revision")
    door_sample = json.loads(DOOR_ITEMS.read_text(encoding="utf-8"))
    if door_sample["sha256"] != ITEMS_XML_SHA256:
        raise ValueError("door item sample is not the pinned items.xml")
    catalog, report = convert(load_staged(), crystal, set(door_sample["door_item_ids"]))
    errors = validate(catalog)
    for error in errors[:50]:
        print(error, file=sys.stderr)
    if errors:
        return 1
    if args.out:
        args.out.write_text(dump(catalog), encoding="utf-8")
    text = dump(report)
    if args.check:
        ok = REPORT.read_text(encoding="utf-8") == text
        print(
            f"{len(catalog['houses'])} houses valid; {REPORT.name}: {'ok' if ok else 'differs'}"
        )
        return 0 if ok else 1
    REPORT.write_text(text, encoding="utf-8")
    print(f"{len(catalog['houses'])} houses valid")
    return 0


if __name__ == "__main__":
    sys.exit(main())
