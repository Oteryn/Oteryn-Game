"""Build the Area catalogue in content/world/areas/ from the 15.30 client map areas.

- `extract-crystal-towns` reads the towns (id, name, temple) of the pinned CrystalServer
  `world.otbm` into the committed sample. Local only: the 53 MB map is not fetched by CI.
- `fetch-wiki` snapshots from TibiaWiki (tibia.fandom.com) the hometown list
  (Template:Hometowns) and the position of every Cleric and Healer NPC. Local only; the
  snapshot is committed and is cross-check evidence, never a value.
- `build [--check]` writes the catalogue and samples/area-report.json. Every client area
  becomes one record: a region (flag 1), a subregion (flag 2) or a city (the subregions in
  CITY_AREAS). A hometown city gets its temple from the engine town, cross-checked against
  the nearest wiki Cleric/Healer NPC; OWNER_CHECKS override it. Keys are allocated once:
  a committed record keeps its key and revision, joined by `provenance.source_id`.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import re
import struct
import sys
import urllib.parse
import urllib.request
from collections import Counter
from pathlib import Path

from validate_areas import validate

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
STAGED = REPO / "imports" / "cipsoft-staticdata" / "map" / "areas"
HOUSES = REPO / "content" / "houses"
OUT = REPO / "content" / "world" / "areas"
SCHEMA = "OTERYN_AREA_AUTHORING/candidate-1"
CLIENT_VERSION = "15.30"
CRYSTAL_REVISION = "00ce02a57ca5a12e48f32a3476e37471167e4c3f"
OTBM_SHA256 = "dcb735549bd11de526c4bd441bbf62e4490efbb60ff7aa334530f1692345d8d7"
CRYSTAL_TOWNS = ROOT / "samples" / "crystal-world-towns-00ce02a5.json"
WIKI = ROOT / "samples" / "tibiawiki-hometowns-2026-09-30.json"
REPORT = ROOT / "samples" / "area-report.json"
WIKI_API = "https://tibia.fandom.com/api.php?"
# City name -> client area id. A House town is the client area of the same name or of the
# name plus " City" (checked in build); the three hometowns without Houses are listed by
# hand: Rookgaard and Roshamuul by name, Dawnport as "Dawnport Centre", whose client
# position is next to the Dawnport temple.
CITY_AREAS = {
    "Ab'Dendriel": 451,
    "Ankrahmun": 297,
    "Candia": 498,
    "Carlin": 456,
    "Darashia": 296,
    "Dawnport": 315,
    "Edron": 455,
    "Farmine": 300,
    "Gray Beach": 302,
    "Issavi": 357,
    "Kazordoon": 448,
    "Liberty Bay": 312,
    "Moonfall": 443,
    "Port Hope": 309,
    "Rathleton": 301,
    "Rookgaard": 317,
    "Roshamuul": 450,
    "Silvertides": 444,
    "Svargrond": 298,
    "Thais": 454,
    "Venore": 460,
    "Yalahar": 461,
}
# Owner in-game observations that override an engine temple coordinate.
OWNER_CHECKS = {
    "Ankrahmun": (
        {"z": 7},
        (
            "2026-09-30 owner in game: the Ankrahmun temple is on client level 0 (z 7); "
            "the engine has z 8"
        ),
    ),
    "Farmine": (
        {},
        (
            "2026-09-30 owner in game: the Farmine temple is on client level -4 (z 11), "
            "as the engine has it"
        ),
    ),
}


def dump(data) -> str:
    return json.dumps(data, indent=1, ensure_ascii=False, sort_keys=True) + "\n"


def canonical(value) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True) + "\n").encode()


def slug(text: str) -> str:
    """Same rule as house-authoring/convert_houses.slug, which made the House town keys."""
    return re.sub(r"[^a-z0-9]+", "_", text.lower()).strip("_")


def load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def load_staged() -> list[dict]:
    manifest = load(STAGED / "manifest.json")
    records = []
    for entry in manifest["files"]:
        raw = (STAGED / entry["path"]).read_bytes()
        if hashlib.sha256(raw).hexdigest() != entry["sha256"]:
            raise ValueError(f"{entry['path']}: digest mismatch with manifest.json")
        records += json.loads(raw)["records"]
    return records


def extract_towns(packed: bytes) -> dict:
    """Towns of a gzip OTBM map: OTBM node 13 is `u32 id, u16 len, name, u16 x, u16 y, u8 z`."""
    if hashlib.sha256(packed).hexdigest() != OTBM_SHA256:
        raise ValueError("world.otbm digest mismatch")
    raw = gzip.decompress(packed)
    special = re.compile(rb"[\xfd\xfe\xff]")
    towns, stack, pos = [], [], 4
    while (m := special.search(raw, pos)) is not None:
        j = m.start()
        if stack:
            stack[-1][1] += raw[pos:j]
        if raw[j] == 0xFD:
            stack[-1][1].append(raw[j + 1])
            pos = j + 2
        elif raw[j] == 0xFE:
            stack.append([raw[j + 1], bytearray()])
            pos = j + 2
        else:
            kind, props = stack.pop()
            pos = j + 1
            if kind == 13:
                town_id, size = struct.unpack_from("<IH", props)
                x, y, z = struct.unpack_from("<HHB", props, 6 + size)
                name = props[6 : 6 + size].decode("latin-1")
                towns.append(
                    {
                        "town_id": town_id,
                        "name": name,
                        "temple": {"x": x, "y": y, "z": z},
                    }
                )
    return {
        "engine": "crystalserver",
        "revision": CRYSTAL_REVISION,
        "source": "data-global/world/world.otbm",
        "otbm_sha256": OTBM_SHA256,
        "towns": sorted(towns, key=lambda t: t["town_id"]),
    }


def wiki_get(**params) -> dict:
    params["format"] = "json"
    request = urllib.request.Request(
        WIKI_API + urllib.parse.urlencode(params),
        headers={"User-Agent": "Oteryn-area-authoring/1.0"},
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        return json.load(response)


def wiki_pages(titles: list[str]) -> list[dict]:
    pages = []
    for i in range(0, len(titles), 40):
        data = wiki_get(
            action="query",
            prop="revisions",
            rvprop="content|ids",
            rvslots="main",
            titles="|".join(titles[i : i + 40]),
        )
        pages += data["query"]["pages"].values()
    return sorted(pages, key=lambda p: p["title"])


def mapper(value: str) -> int:
    """TibiaWiki `posx`/`posy` is `sector.offset`, one sector being 256 tiles."""
    sector, offset = value.split(".")
    return int(sector) * 256 + int(offset)


def fetch_wiki() -> dict:
    (template,) = wiki_pages(["Template:Hometowns"])
    text = template["revisions"][0]["slots"]["main"]["*"]
    hometowns = re.findall(r"^\*\s*\[\[([^\]|]+)\]\]", text, re.MULTILINE)
    titles = set()
    for category in ("Category:Cleric NPCs", "Category:Healer NPCs"):
        cont: dict = {}
        while True:
            data = wiki_get(
                action="query",
                list="categorymembers",
                cmtitle=category,
                cmlimit=500,
                **cont,
            )
            titles |= {
                m["title"] for m in data["query"]["categorymembers"] if m["ns"] == 0
            }
            if "continue" not in data:
                break
            cont = {"cmcontinue": data["continue"]["cmcontinue"]}
    npcs = []
    for page in wiki_pages(sorted(titles)):
        text = page["revisions"][0]["slots"]["main"]["*"]
        fields = {
            k: v.strip()
            for k, v in re.findall(r"^\|\s*(\w+)\s*=(.*)$", text, re.MULTILINE)
        }
        try:
            position = {
                "x": mapper(fields["posx"]),
                "y": mapper(fields["posy"]),
                "z": int(fields["posz"]),
            }
        except (KeyError, ValueError):
            position = None
        npcs.append(
            {
                "title": page["title"],
                "pageid": page["pageid"],
                "revid": page["revisions"][0]["revid"],
                "city": fields.get("city"),
                "position": position,
            }
        )
    return {
        "source": "tibia.fandom.com",
        "fetched": "2026-09-30",
        "hometowns": {
            "title": template["title"],
            "pageid": template["pageid"],
            "revid": template["revisions"][0]["revid"],
            "towns": hometowns,
        },
        "cleric_and_healer_npcs": npcs,
    }


def distance(a: dict, b: dict) -> int | None:
    if a["z"] != b["z"]:
        return None
    return max(abs(a["x"] - b["x"]), abs(a["y"] - b["y"]))


def nearest_npc(temple: dict, npcs: list[dict]) -> dict | None:
    placed = [n for n in npcs if n["position"]]
    if not placed:
        return None
    # One floor counts as ten tiles, so a temple NPC one floor away beats an unrelated
    # NPC on the temple floor.
    npc = min(
        placed,
        key=lambda n: (
            10 * abs(n["position"]["z"] - temple["z"])
            + max(
                abs(n["position"]["x"] - temple["x"]),
                abs(n["position"]["y"] - temple["y"]),
            )
        ),
    )
    return {
        "title": npc["title"],
        "pageid": npc["pageid"],
        "revid": npc["revid"],
        "position": npc["position"],
        "distance": distance(temple, npc["position"]),
    }


def existing_identities() -> dict[int, dict]:
    kept = {}
    for path in sorted(OUT.glob("*/areas-*.json")):
        for area in load(path)["areas"]:
            kept[area["provenance"]["source_id"]] = area["identity"]
    return kept


def house_towns() -> set[str]:
    return {
        house["town"]["key"]
        for path in sorted(HOUSES.glob("houses-*.json"))
        for house in load(path)["houses"]
    }


def build() -> tuple[list[dict], dict]:
    records = load_staged()
    by_id = {r["source_id"]: r for r in records}
    parent_of = {s: r["source_id"] for r in records for s in r["subarea_ids"]}
    city_of = {area_id: town for town, area_id in CITY_AREAS.items()}
    towns = {t["name"]: t for t in load(CRYSTAL_TOWNS)["towns"]}
    wiki = load(WIKI)
    hometowns = set(wiki["hometowns"]["towns"])
    if not hometowns <= set(CITY_AREAS):
        raise ValueError(
            f"hometowns without a city: {sorted(hometowns - set(CITY_AREAS))}"
        )
    kept = existing_identities()

    def key(record: dict) -> str:
        source_id = record["source_id"]
        if source_id in city_of:
            return f"oteryn:content.area.city.{slug(city_of[source_id])}"
        kind = "region" if record["flag"] == 1 else "subregion"
        return f"oteryn:content.area.{kind}.{slug(record['name'])}"

    identity = {
        r["source_id"]: kept.get(
            r["source_id"], {"key": key(r), "revision": "definition-r1"}
        )
        for r in records
    }
    areas, report = [], {"hometowns": {}}
    for record in records:
        source_id = record["source_id"]
        if record["flag"] not in (1, 2) or (record["flag"] == 1) == (
            source_id in parent_of
        ):
            raise ValueError(f"area {source_id}: flag and region listing disagree")
        town = city_of.get(source_id)
        hometown = evidence = None
        if town in hometowns:
            engine = towns[town]
            temple = dict(engine["temple"])
            override, note = OWNER_CHECKS.get(town, ({}, None))
            temple.update(override)
            hometown = {"temple": temple}
            npc = nearest_npc(temple, wiki["cleric_and_healer_npcs"])
            evidence = {
                "engine_town": {
                    "engine": "crystalserver",
                    "revision": CRYSTAL_REVISION,
                    "town_id": engine["town_id"],
                    "name": engine["name"],
                    "temple": engine["temple"],
                },
                "wiki_temple_npc": npc,
                "owner_check": note,
            }
            report["hometowns"][town] = {
                "temple": temple,
                "wiki_npc": npc and npc["title"],
                "wiki_distance": npc and npc["distance"],
                "owner_check": note is not None,
            }
        parent = parent_of.get(source_id)
        areas.append(
            {
                "identity": identity[source_id],
                "name": " ".join(record["name"].split()),
                "kind": "city"
                if town
                else "region"
                if record["flag"] == 1
                else "subregion",
                "parent": None
                if parent is None
                else {"family": "Area", "key": identity[parent]["key"]},
                "position": record["position"],
                "hometown": hometown,
                "provenance": {
                    "source": "cipsoft/map_area",
                    "client_version": CLIENT_VERSION,
                    "source_id": source_id,
                    "source_name": record["name"],
                    "source_record_sha256": record["source_record_sha256"],
                    "hometown": evidence,
                },
            }
        )
    for town, area_id in CITY_AREAS.items():
        name = by_id[area_id]["name"]
        if town not in hometowns and name not in (town, f"{town} City"):
            raise ValueError(f"{town}: client area {area_id} is {name!r}")
    cities = {a["identity"]["key"] for a in areas if a["kind"] == "city"}
    missing = house_towns() - cities
    if missing:
        raise ValueError(f"House towns without a city Area: {sorted(missing)}")
    gone = set(kept) - set(by_id)
    if gone:
        raise ValueError(f"catalogue areas missing from the source: {sorted(gone)}")
    areas.sort(key=lambda a: a["identity"]["key"])
    report["kinds"] = dict(sorted(Counter(a["kind"] for a in areas).items()))
    report["areas"] = len(areas)
    report["with_position"] = sum(a["position"] is not None for a in areas)
    report["house_towns"] = len(house_towns())
    report["engine_towns_not_hometowns"] = sorted(set(towns) - hometowns)
    return areas, report


def outputs(areas: list[dict]) -> dict[str, bytes]:
    groups = {
        "cities": [a for a in areas if a["kind"] == "city"],
        "regions": [a for a in areas if a["kind"] != "city"],
    }
    files = {}
    for folder, chunk in groups.items():
        name = f"{folder}/areas-00000-{len(chunk) - 1:05d}.json"
        files[name] = canonical({"schema": SCHEMA, "areas": chunk})
    return files


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    sub = parser.add_subparsers(dest="command", required=True)
    towns = sub.add_parser("extract-crystal-towns")
    towns.add_argument("--otbm", type=Path, required=True)
    towns.add_argument("--check", action="store_true")
    sub.add_parser("fetch-wiki")
    run = sub.add_parser("build")
    run.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)

    if args.command == "extract-crystal-towns":
        text = dump(extract_towns(args.otbm.read_bytes()))
        if args.check:
            ok = CRYSTAL_TOWNS.read_text(encoding="utf-8") == text
            print(f"{CRYSTAL_TOWNS.name}: {'ok' if ok else 'differs'}")
            return 0 if ok else 1
        CRYSTAL_TOWNS.write_text(text, encoding="utf-8")
        return 0

    if args.command == "fetch-wiki":
        WIKI.write_text(dump(fetch_wiki()), encoding="utf-8")
        return 0

    areas, report = build()
    errors = validate({"schema": SCHEMA, "areas": areas})
    if errors:
        print("\n".join(errors[:20]), file=sys.stderr)
        return 1
    files = outputs(areas)
    files["report"] = dump(report).encode()
    committed = {
        str(p.relative_to(OUT)): p.read_bytes() for p in OUT.glob("*/areas-*.json")
    }
    committed["report"] = REPORT.read_bytes() if REPORT.exists() else b""
    if args.check:
        if committed != files:
            print("stale or tampered catalogue or report", file=sys.stderr)
            return 1
        print(f"ok ({len(areas)} areas)")
        return 0
    for name in set(committed) - set(files):
        (OUT / name).unlink()
    for name, data in files.items():
        (REPORT if name == "report" else OUT / name).write_bytes(data)
    print(json.dumps({k: report[k] for k in ("areas", "kinds")}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
