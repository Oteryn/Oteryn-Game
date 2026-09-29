#!/usr/bin/env python3
"""Capture a pinned snapshot of the English TibiaWiki (Fandom) pages of the City Areas.

Evidence tooling only: TibiaWiki is a player-observed reference source (CC BY-SA). Per city
page, matched by exact title, it stores the page id, exact revision id, the SHA-256 of the
wikitext and the raw values of a few short factual `Infobox Geography` fields, plus the NPC
names of the page's `<City> NPCs` category. Article prose is never stored. Cities without a
page, or with an ambiguous one, are listed apart with a reason. The committed snapshot makes
`convert_city_facts.py` reproducible offline.

    python fandom_city_snapshot.py fetch [--out imports/tibiawiki/cities/fandom-snapshot-v1.json]

The API conventions (User-Agent, <= 2 requests/s, bounded retries) are those of
`fandom_hunting_snapshot.py`, whose parsing helpers this module reuses.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import time
from datetime import datetime, timezone
from pathlib import Path

import fandom_hunting_snapshot as wiki

ROOT = wiki.ROOT
CITIES_INDEX = "content/world/areas/cities/index.json"
SNAPSHOT_PATH = "imports/tibiawiki/cities/fandom-snapshot-v1.json"
SCHEMA = "OTERYN_TIBIAWIKI_FANDOM_CITIES_SNAPSHOT/v1"
LICENSE_NOTE = (
    "TibiaWiki (Fandom), CC BY-SA; only raw values of a few short factual infobox fields "
    "(implemented version, ruler, neighbouring places) and the NPC names of the city's NPC "
    "category are recorded, never article prose."
)
FIELDS = ("implemented", "ruler", "near")
INFOBOX = re.compile(r"\{\{\s*Infobox[ _]Geography\s*(?=[|\n}])", re.IGNORECASE)
# Exact-title pages that must not be bound to the City Area of the same name.
AMBIGUOUS = {
    "Targuna": (
        "the City Area shares its temple position with Dawnport Tutorial, so it is a "
        "CrystalServer placeholder town; the wiki island page is not proven to be it"
    ),
}
NO_PAGE = "no page with this exact title"
NO_INFOBOX = "the page has no Infobox Geography (it is not a city page)"


def raw_facts(wikitext: str) -> dict | None:
    """Raw factual field values of the page's `Infobox Geography`, or None when it has none."""
    text = wiki.COMMENT.sub("", wikitext)
    match = INFOBOX.search(text)
    if not match:
        return None
    body = wiki.template_body(text, match.start())
    if body is None:
        return None
    fields = wiki.named_fields(body)
    return {name: fields[name] for name in FIELDS if fields.get(name)}


def page_record(page: dict, npc_names: list[str]) -> dict | None:
    """Snapshot row for one fetched page, or None when it has no city infobox."""
    revision = page["revisions"][0]
    wikitext = revision["slots"]["main"]["content"]
    facts = raw_facts(wikitext)
    if facts is None:
        return None
    return {
        "facts": {**facts, "npc_names": sorted(set(npc_names))},
        "pageid": page["pageid"],
        "revid": revision["revid"],
        "sha256": hashlib.sha256(wikitext.encode()).hexdigest(),
        "timestamp": revision["timestamp"],
        "title": page["title"],
    }


def build_snapshot(
    names: list[str],
    pages: dict[str, dict],
    npcs: dict[str, list[str]],
    fetched_at: str,
) -> dict:
    """`pages` maps a City Area name to its fetched page (absent: no page)."""
    rows, unmatched = [], []
    for name in sorted(names):
        row = None
        if name in AMBIGUOUS:
            unmatched.append({"name": name, "reason": AMBIGUOUS[name]})
        elif name not in pages:
            unmatched.append({"name": name, "reason": NO_PAGE})
        elif (row := page_record(pages[name], npcs.get(name, []))) is None:
            unmatched.append({"name": name, "reason": NO_INFOBOX})
        if row:
            rows.append(row)
    return {
        "fetched_at": fetched_at,
        "license": LICENSE_NOTE,
        "pages": sorted(rows, key=lambda r: r["pageid"]),
        "schema": SCHEMA,
        "source_url": "https://tibia.fandom.com",
        "unmatched": unmatched,
    }


def city_names(root: Path) -> list[str]:
    names = []
    for shard in json.loads((root / CITIES_INDEX).read_text(encoding="utf-8"))[
        "shards"
    ]:
        for record in json.loads((root / shard).read_text(encoding="utf-8"))["records"]:
            names.append(record["declaration"]["name"])
    return names


def category_npcs(title: str) -> list[str]:
    """NPC pages of `Category:<title> NPCs`, without those also in `Category:Deprecated`."""
    members: dict[int, str] = {}
    deprecated: set[int] = set()
    params = {
        "action": "query",
        "generator": "categorymembers",
        "gcmtitle": f"Category:{title} NPCs",
        "gcmnamespace": 0,
        "gcmlimit": 500,
        "prop": "categories",
        "clcategories": "Category:Deprecated",
        "cllimit": 500,
    }
    while True:
        data = wiki.api(params)
        time.sleep(wiki.THROTTLE_SECONDS)
        for page in data.get("query", {}).get("pages", []):
            members[page["pageid"]] = page["title"]
            if page.get("categories"):
                deprecated.add(page["pageid"])
        if "continue" not in data:
            return sorted(t for i, t in members.items() if i not in deprecated)
        params.update(data["continue"])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("command", choices=["fetch"])
    parser.add_argument("--out", type=Path, default=ROOT / SNAPSHOT_PATH)
    args = parser.parse_args()
    names = city_names(ROOT)
    titles = [name for name in names if name not in AMBIGUOUS]
    pages = {page["title"]: page for page in wiki.fetch_pages(titles)}
    npcs = {name: category_npcs(name) for name in pages}
    snapshot = build_snapshot(
        names, pages, npcs, datetime.now(timezone.utc).strftime("%Y-%m-%d")
    )
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(wiki.canonical(snapshot))
    print(
        f"wrote {args.out} ({len(snapshot['pages'])} city pages, "
        f"{len(snapshot['unmatched'])} unmatched)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
