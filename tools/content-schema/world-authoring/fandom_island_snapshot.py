#!/usr/bin/env python3
"""Capture a pinned snapshot of the English TibiaWiki (Fandom) island and archipelago pages.

Evidence tooling only: TibiaWiki is a player-observed reference source (CC BY-SA). Per page
the snapshot stores the page id, exact revision id, the SHA-256 of the wikitext, the wiki's
own map coordinates, a status and event flag, and one short factual sentence; never article
prose. The committed snapshot makes `convert_islands.py` reproducible offline.

    python fandom_island_snapshot.py fetch [--out imports/tibiawiki/islands/fandom-snapshot-v1.json]

The candidate set, the pages that only lend a coordinate, and the few curated decisions
(aliases, places inside another island, the Fibula map correction, the cities the article
names) are the constants below. The tool fails closed when a curated coordinate is not in
its source page. Conventions follow `fandom_hunting_snapshot.py` (same API, throttle, User-Agent).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

import fandom_hunting_snapshot as wiki

ROOT = wiki.ROOT
SNAPSHOT_PATH = "imports/tibiawiki/islands/fandom-snapshot-v1.json"
CITIES_INDEX = "content/world/areas/cities/index.json"
SCHEMA = "OTERYN_TIBIAWIKI_FANDOM_ISLANDS_SNAPSHOT/v1"
LICENSE_NOTE = (
    "TibiaWiki (Fandom), CC BY-SA; only page identity, the wiki's map coordinates, status "
    "flags and one short factual sentence per page are recorded, never article prose."
)
MAX_EVIDENCE = 200
FLOOR_SURFACE = 7

# Candidate pages: the pages of `Template:Infobox Geography` plus the pages linked from them
# that name an island or archipelago, as reviewed for the island research. The class is the
# wiki's own wording (island or archipelago); whether the map confirms it is decided later.
CANDIDATES = {
    "Arctic Faun's Island": "island",
    "Blue Valley": "island",
    "Cake Keep Isle": "island",
    "Calcanea": "island",
    "Chazorai": "island",
    "Chyllfroest": "island",
    "Cormaya": "island",
    "Darama": "island",
    "Dawnport": "island",
    "Draconia": "island",
    "Dwacatra": "island",
    "Edron": "island",
    "Eremo's Island": "island",
    "Fenrock": "island",
    "Fibula": "island",
    "Folda": "island",
    "Forbidden Islands": "archipelago",
    "Goroma": "island",
    "Gray Island": "island",
    "Grimlund": "island",
    "Grimvale": "island",
    "Helheim": "island",
    "Hrodmir": "island",
    "Ice Islands": "archipelago",
    "Ingol": "island",
    "Island of Destiny": "island",
    "Isle of Evil": "island",
    "Isle of Merriment": "island",
    "Isle of Solitude": "island",
    "Isle of the Kings": "island",
    "Isle of the Mists": "island",
    "Kharos": "island",
    "Kilmaresh": "island",
    "Laguna Islands": "archipelago",
    "Malada": "island",
    "Marapur": "archipelago",
    "Meluna": "island",
    "Meriana": "island",
    "Mistrock": "island",
    "Nargor": "island",
    "Newhaven": "island",
    "Nibelor": "island",
    "Okolnir": "island",
    "Orcsoberfest Island": "island",
    "Oskayaat": "island",
    "Percht Island": "island",
    "Quirefang": "island",
    "Ragnir": "island",
    "Ramoa": "island",
    "Rascacoon": "island",
    "Redbone Castle": "island",
    "Robson's Isle": "island",
    "Rookgaard": "island",
    "Roshamuul": "island",
    "Schrödinger's Island": "island",
    "Senja": "island",
    "Shattered Isles": "archipelago",
    "Talahu": "island",
    "Targuna": "island",
    "Temple of Light": "island",
    "Travora": "island",
    "Treasure Hunt Island": "island",
    "Treasure Island": "island",
    "Tutorial Island": "island",
    "Tyrsung": "island",
    "Vandura": "island",
    "Vega": "island",
}
# A page with no Mapper Coords of its own borrows the coordinate of another page that states
# where the place is: (source page, x, y, floor). The tool checks the source page holds it.
DERIVED = {
    "Darama": [("Ankrahmun", 33146, 32816, 7)],
    "Eremo's Island": [("Wisdom of Solitude", 33323, 31884, 7)],
    "Forbidden Islands": [
        ("Goroma", 32095, 32583, 7),
        ("Ramoa", 31931, 32567, 7),
        ("Talahu", 31953, 32660, 7),
        ("Malada", 32016, 32713, 7),
        ("Kharos", 32121, 32686, 7),
    ],
    "Grimlund": [("Ogden Brewboiler", 32404, 31059, 7)],
    "Hrodmir": [("Svargrond", 32278, 31146, 7)],
    "Kilmaresh": [("Issavi", 33907, 31512, 7)],
    "Percht Island": [("Orcsoberfest Island", 33771, 31059, 7)],
    "Quirefang": [("Gray Beach", 33471, 31304, 7)],
    "Senja": [("Ice Islands", 32163, 31656, 7)],
    "Shattered Isles": [
        ("Liberty Bay", 32309, 32794, 7),
        ("Meriana", 32389, 32604, 7),
        ("Nargor", 31994, 32860, 7),
        ("Treasure Island", 32156, 32948, 7),
        ("Laguna Islands", 32466, 32939, 7),
    ],
    "Vandura": [("Liberty Bay", 32309, 32794, 7)],
    "Vega": [("Ice Islands", 31991, 31700, 7)],
}
# A page named like a City Area without coordinates uses that city's temple x/y on this floor.
CITY_TEMPLE = {
    "Island of Destiny": ("Island of Destiny", 7),
    "Marapur": ("Marapur", 7),
    "Newhaven": ("Newhaven", 7),
    "Rookgaard": ("Rookgaard", 7),
    "Roshamuul": ("Roshamuul", 7),
    "Targuna": ("Targuna", 7),
}
# The wiki gives the wrong place: the map coordinate that replaces it, and the page stating
# it (Meluna: "Ferryman Kamil in Fibula"). The wiki coordinate lies on the mainland.
MAP_CORRECTION = {"Fibula": ("Meluna", 32153, 32456, 7, "Ferryman Kamil in Fibula")}
# Pages that are the same place under another name; the alias merges into the target.
ALIAS_OF = {"Percht Island": "Orcsoberfest Island"}
# Pages that name a place inside another island: they never own a component.
PLACE_WITHIN = {"Chyllfroest": "Hrodmir", "Ragnir": "Hrodmir"}
# The wiki calls Darama a continent.
KIND_HINT = {"Darama": "continent"}
# Cities the article names as located on the island (resolved to City Areas on conversion).
WIKI_CITIES = {
    "Blue Valley": ["Blue Valley"],
    "Darama": ["Ankrahmun", "Darashia", "Port Hope"],
    "Dawnport": ["Dawnport"],
    "Edron": ["Edron"],
    "Hrodmir": ["Svargrond"],
    "Ice Islands": ["Svargrond"],
    "Island of Destiny": ["Island of Destiny"],
    "Kilmaresh": ["Issavi"],
    "Marapur": ["Marapur", "Moonfall", "Silvertides"],
    "Newhaven": ["Newhaven"],
    "Quirefang": ["Gray Beach"],
    "Rookgaard": ["Rookgaard"],
    "Roshamuul": ["Roshamuul"],
    "Shattered Isles": ["Liberty Bay"],
    "Targuna": ["Targuna"],
    "Vandura": ["Liberty Bay"],
}
# The wiki page that carries the island sentence instead of the page itself.
EVIDENCE_PAGE = {"Kilmaresh": "Issavi"}
# Event flag override: the wiki says the arena island only opened on a test server.
EVENT_OVERRIDE = {"Isle of Merriment"}
REMOVED = {"Dawnport"}

EVENT_TEXT = re.compile(
    r"only (be )?(accessible|reached).{0,80}(event|solstice|april|november|days a year)"
    r"|event takes place",
    re.IGNORECASE,
)
REMOVED_TEXT = re.compile(
    r"removed from the game|no longer (exists|accessible|available)", re.IGNORECASE
)
ISLAND_WORD = re.compile(r"\b(island|isle|isles|islands|archipelago)\b", re.IGNORECASE)
MAPPER = re.compile(r"\{\{\s*Mapper[ _]Coords\s*\|([^}]*)\}\}", re.IGNORECASE)
SECTOR = 256


def canonical(value) -> bytes:
    return wiki.canonical(value)


# --------------------------------------------------------------------------------------------
# Wikitext parsing
# --------------------------------------------------------------------------------------------


def lead(text: str) -> str:
    match = re.search(r"^==[^=]", text, re.MULTILINE)
    return text[: match.start()] if match else text


def strip_templates(text: str) -> str:
    previous = None
    while previous != text:
        previous, text = text, re.sub(r"\{\{[^{}]*\}\}", "", text).replace("{}", "")
    return text


def clean(text: str) -> str:
    """Plain text of wikitext: comments, references, templates, links and markup removed."""
    text = re.sub(r"__[A-Za-z]+__", "", text)
    text = re.sub(r"<ref[^>]*/>|<ref[^>]*>.*?</ref>", "", text, flags=re.DOTALL)
    text = wiki.COMMENT.sub("", text)
    text = strip_templates(text)
    text = re.sub(r"\[https?://\S+ ([^\]]*)\]", r"\1", text)
    text = re.sub(r"\[\[(?:File|Image|Category):[^\]]*\]\]", "", text)
    text = re.sub(r"\[\[([^\]|]*\|)?([^\]]*)\]\]", r"\2", text)
    text = re.sub(r"'{2,}", "", text)
    text = re.sub(r"<[^>]+>", "", text)
    return re.sub(r"\s+", " ", text).strip()


def sentences(text: str) -> list[str]:
    return re.split(r"(?<=[.!?])\s+(?=[A-Z\"'])", text)


def island_sentence(wikitext: str) -> str:
    """First sentence of the lead (else of the page) that names an island, at most 200 chars."""
    for scope in (lead(wikitext), wikitext):
        for sentence in sentences(clean(scope))[:5]:
            if ISLAND_WORD.search(sentence):
                return sentence.strip()[:MAX_EVIDENCE]
    return ""


def mapper_coordinate(raw: str) -> dict | None:
    """One `{{Mapper Coords|x.y|x.y|z|...}}` body as `{x, y, floor[, label]}`, or None."""
    parts = [part.strip() for part in raw.split("|")]
    label = next((p[5:] for p in parts if p.startswith("text=")), None)
    positional = [p for p in parts if "=" not in p or re.match(r"^\d", p)]
    if len(positional) < 3:
        return None
    axes = []
    for part in positional[:2]:
        match = re.fullmatch(r"(\d{1,3})\.(\d{1,3})", part)
        if not match or int(match.group(2)) >= SECTOR:
            return None
        axes.append(int(match.group(1)) * SECTOR + int(match.group(2)))
    if not re.fullmatch(r"\d{1,2}", positional[2]) or int(positional[2]) > 15:
        return None
    row = {"floor": int(positional[2]), "x": axes[0], "y": axes[1]}
    if label:
        row["label"] = label
    return row


def wiki_coordinates(wikitext: str) -> list[dict]:
    """Every distinct Mapper Coords of the page in text order, lead before body."""
    text = wiki.COMMENT.sub("", wikitext)
    boundary = len(lead(text))
    rows, seen = [], set()
    for match in MAPPER.finditer(text):
        row = mapper_coordinate(match.group(1))
        if row is None or (row["x"], row["y"], row["floor"]) in seen:
            continue
        seen.add((row["x"], row["y"], row["floor"]))
        rows.append(
            {**row, "origin": "wiki_lead" if match.start() < boundary else "wiki_body"}
        )
    return rows


def has_coordinate(wikitext: str, x: int, y: int, floor: int) -> bool:
    return any(
        (row["x"], row["y"], row["floor"]) == (x, y, floor)
        for row in wiki_coordinates(wikitext)
    )


def infobox_status(wikitext: str) -> list[str]:
    text = wiki.COMMENT.sub("", wikitext)
    return sorted(
        {
            value.strip().lower()
            for value in re.findall(r"\|\s*status\s*=\s*([^\n|]*)", text)
            if value.strip()
        }
    )


# --------------------------------------------------------------------------------------------
# Snapshot
# --------------------------------------------------------------------------------------------


def city_temples(root: Path = ROOT) -> dict[str, tuple[int, int]]:
    """City name -> temple (x, y) from the committed City Areas."""
    index = json.loads((root / CITIES_INDEX).read_text(encoding="utf-8"))
    temples = {}
    for shard in index["shards"]:
        for record in json.loads((root / shard).read_text(encoding="utf-8"))["records"]:
            declaration = record["declaration"]
            temple = declaration["temple"]
            temples[declaration["name"]] = (temple["x"], temple["y"])
    return temples


def page_identity(page: dict) -> dict:
    revision = page["revisions"][0]
    wikitext = revision["slots"]["main"]["content"]
    row = {
        "pageid": page["pageid"],
        "revid": revision["revid"],
        "sha256": hashlib.sha256(wikitext.encode()).hexdigest(),
        "title": page["title"],
    }
    if "timestamp" in revision:
        row["timestamp"] = revision["timestamp"]
    return row


def wikitext_of(page: dict) -> str:
    return page["revisions"][0]["slots"]["main"]["content"]


def coordinate_rows(
    title: str, wikitext: str, pages: dict[str, dict], temples: dict
) -> list[dict]:
    """The coordinates stored for a candidate, primary first; archipelagos keep every one."""
    rows = wiki_coordinates(wikitext)
    if not rows and title in DERIVED:
        rows = []
        for source, x, y, floor in DERIVED[title]:
            if source not in pages or not has_coordinate(
                wikitext_of(pages[source]), x, y, floor
            ):
                raise RuntimeError(f"{title}: {source} does not hold {x},{y},{floor}")
            rows.append(
                {
                    "floor": floor,
                    "origin": "derived",
                    "source_page": source,
                    "x": x,
                    "y": y,
                }
            )
    if not rows and title in CITY_TEMPLE:
        city, floor = CITY_TEMPLE[title]
        x, y = temples[city]
        rows = [{"city": city, "floor": floor, "origin": "city_temple", "x": x, "y": y}]
    if CANDIDATES[title] == "island":
        rows = rows[:1]
    return rows


def candidate_row(title: str, pages: dict[str, dict], temples: dict) -> dict:
    page = pages[title]
    wikitext = wikitext_of(page)
    status = infobox_status(wikitext)
    plain = clean(wikitext)
    row = {
        **page_identity(page),
        "coordinates": coordinate_rows(title, wikitext, pages, temples),
        "event_only": "event" in status
        or bool(EVENT_TEXT.search(plain))
        or title in EVENT_OVERRIDE,
        "wiki_class": CANDIDATES[title],
    }
    source = EVIDENCE_PAGE.get(title, title)
    evidence = island_sentence(wikitext_of(pages[source]))
    if not evidence:
        raise RuntimeError(f"{title}: no sentence naming an island in {source}")
    row["evidence"] = evidence
    if source != title:
        row["evidence_page"] = source
    if status:
        row["status"] = status
    if title in REMOVED:
        if not REMOVED_TEXT.search(plain):
            raise RuntimeError(f"{title}: the page does not say it was removed")
        row["removed"] = True
    if title in WIKI_CITIES:
        row["wiki_cities"] = WIKI_CITIES[title]
    if title in ALIAS_OF:
        row["alias_of"] = ALIAS_OF[title]
    if title in PLACE_WITHIN:
        row["place_within"] = PLACE_WITHIN[title]
    if title in KIND_HINT:
        row["kind_hint"] = KIND_HINT[title]
    if title in MAP_CORRECTION:
        source, x, y, floor, note = MAP_CORRECTION[title]
        if not has_coordinate(wikitext_of(pages[source]), x, y, floor):
            raise RuntimeError(f"{title}: {source} does not hold {x},{y},{floor}")
        row["map_correction"] = {
            "floor": floor,
            "note": note,
            "source_page": source,
            "x": x,
            "y": y,
        }
    return row


def needed_titles() -> list[str]:
    titles = set(CANDIDATES)
    for rows in DERIVED.values():
        titles.update(source for source, *_ in rows)
    titles.update(source for source, *_ in MAP_CORRECTION.values())
    titles.update(EVIDENCE_PAGE.values())
    return sorted(titles)


def build_snapshot(pages: list[dict], fetched_at: str, root: Path = ROOT) -> dict:
    """Snapshot of already fetched pages ({pageid, title, revisions:[...]})."""
    by_title = {page["title"]: page for page in pages}
    missing = [title for title in needed_titles() if title not in by_title]
    if missing:
        raise RuntimeError(f"pages not fetched: {missing}")
    temples = city_temples(root)
    rows = [candidate_row(t, by_title, temples) for t in CANDIDATES]
    lenders = sorted(set(needed_titles()) - set(CANDIDATES))
    return {
        "coordinate_pages": sorted(
            (page_identity(by_title[t]) for t in lenders), key=lambda r: r["pageid"]
        ),
        "fetched_at": fetched_at,
        "license": LICENSE_NOTE,
        "pages": sorted(rows, key=lambda r: r["pageid"]),
        "schema": SCHEMA,
        "source_url": "https://tibia.fandom.com",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("command", choices=["fetch"])
    parser.add_argument("--out", type=Path, default=ROOT / SNAPSHOT_PATH)
    args = parser.parse_args()
    titles = needed_titles()
    pages = wiki.fetch_pages(titles)
    if len(pages) != len(titles):
        fetched = {page["title"] for page in pages}
        print(f"FAIL not fetched: {sorted(set(titles) - fetched)}", file=sys.stderr)
        return 1
    snapshot = build_snapshot(pages, datetime.now(timezone.utc).strftime("%Y-%m-%d"))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(canonical(snapshot))
    print(
        f"wrote {args.out} ({len(snapshot['pages'])} candidate pages, "
        f"{len(snapshot['coordinate_pages'])} coordinate pages)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
