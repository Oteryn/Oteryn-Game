"""Capture the TibiaWiki BR House pages and compare them with the House candidate catalog.

Evidence tooling only (stdlib; <= 2 requests/s; neutral User-Agent). TibiaWiki BR answers
this repository's runners but challenges agent containers, so `fetch` runs in the
`house-tibiawiki-br-capture.yml` workflow.

`fetch`: `Todas_as_casas` plus every namespace-0 page it links to, with page id, exact
revision, timestamp, SHA-256 and wikitext. It stays a CI artifact, never committed.
`facts`: per page, the first `Infobox` template's short single-line parameters (no
prose), page/revision ids and the page SHA-256. That file may be committed.
`compare`: joins the facts with the catalog built by `convert_houses.py` (by a
parameter whose values are client house ids when one exists, else by name) and, for
each catalog field, reports the best-agreeing wiki parameter and its agreement.
The parameter names are discovered, not assumed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from collections import Counter
from pathlib import Path

API = "https://www.tibiawiki.com.br/api.php"
USER_AGENT = "OterynHouseAuthoring/1.0 (+https://github.com/Oteryn/Oteryn-Game)"
LIST_PAGE = "Todas_as_casas"
MAX_PAGES = 2000
MAX_VALUE = 80
MAX_WORDS = 5  # longer values are prose, not facts
FIELDS = ("rent_gold", "size_sqm", "beds", "town")


def api(params: dict) -> dict:
    url = (
        API
        + "?"
        + urllib.parse.urlencode({**params, "format": "json", "formatversion": "2"})
    )
    delay = 1.0
    for attempt in range(4):
        try:
            request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with urllib.request.urlopen(request, timeout=60) as response:
                data = json.load(response)
            time.sleep(0.5)
            if "error" in data:
                raise SystemExit(f"API error: {data['error']}")
            return data
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            if attempt == 3:
                raise SystemExit(f"API request failed: {exc}") from exc
            time.sleep(delay)
            delay *= 2
    raise AssertionError


def fetch() -> dict:
    titles, params = (
        [LIST_PAGE],
        {
            "action": "query",
            "titles": LIST_PAGE,
            "prop": "links",
            "plnamespace": 0,
            "pllimit": 500,
        },
    )
    while True:
        data = api(params)
        for page in data["query"]["pages"]:
            titles += [link["title"] for link in page.get("links", [])]
        if "continue" not in data:
            break
        params.update(data["continue"])
    if len(titles) > MAX_PAGES:
        raise SystemExit("too many linked pages")
    pages = []
    for start in range(0, len(titles), 50):
        data = api(
            {
                "action": "query",
                "titles": "|".join(titles[start : start + 50]),
                "prop": "revisions",
                "rvprop": "ids|timestamp|content",
                "rvslots": "main",
            }
        )
        for page in data["query"]["pages"]:
            if "revisions" not in page:
                continue
            revision = page["revisions"][0]
            text = revision["slots"]["main"]["content"]
            pages.append(
                {
                    "title": page["title"],
                    "pageid": page["pageid"],
                    "revid": revision["revid"],
                    "timestamp": revision["timestamp"],
                    "sha256": hashlib.sha256(text.encode()).hexdigest(),
                    "wikitext": text,
                }
            )
    return {
        "schema": "OTERYN_HOUSE_TIBIAWIKI_BR_SNAPSHOT/v1",
        "api": API,
        "pages": sorted(pages, key=lambda p: p["pageid"]),
    }


def infobox(text: str) -> tuple[str, dict[str, str]] | None:
    """Name and short top-level parameters of the first {{Infobox...}} template."""
    start = text.find("{{Infobox")
    if start < 0:
        return None
    depth, i, parts, current = 0, start, [], []
    while i < len(text):
        pair = text[i : i + 2]
        if pair in ("{{", "[["):
            depth += 1
            current.append(pair)
            i += 2
            continue
        if pair in ("}}", "]]"):
            depth -= 1
            if depth == 0:
                parts.append("".join(current))
                break
            current.append(pair)
            i += 2
            continue
        if text[i] == "|" and depth == 1:
            parts.append("".join(current))
            current = []
        else:
            current.append(text[i])
        i += 1
    name, params = parts[0][2:].strip(), {}
    for part in parts[1:]:
        key, sep, value = part.partition("=")
        value = re.sub(r"\[\[(?:[^|\]]*\|)?([^\]]*)\]\]", r"\1", value).strip()
        short = 0 < len(value) <= MAX_VALUE and len(value.split()) <= MAX_WORDS
        if sep and "\n" not in value and short:
            params[key.strip().lower()] = value
    return name, params


def facts(snapshot: dict) -> dict:
    records = []
    for page in snapshot["pages"]:
        box = infobox(page["wikitext"])
        if box:
            records.append(
                {k: page[k] for k in ("title", "pageid", "revid", "sha256")}
                | {"template": box[0], "params": box[1]}
            )
    return {
        "schema": "OTERYN_HOUSE_TIBIAWIKI_BR_FACTS/v1",
        "api": snapshot["api"],
        "records": records,
    }


def number(value: str) -> int | None:
    digits = re.sub(r"[.,\s]|gps?$|gold$", "", value.lower())
    return int(digits) if digits.isdigit() else None


def compare(wiki: dict, catalog: dict) -> dict:
    houses = catalog["houses"]
    by_id = {h["provenance"]["source_id"]: h for h in houses}
    by_name = {h["name"].casefold(): h for h in houses}
    records = wiki["records"]
    id_keys = Counter(
        k for r in records for k, v in r["params"].items() if number(v) in by_id
    )
    id_key = (
        id_keys.most_common(1)[0][0]
        if id_keys and id_keys.most_common(1)[0][1] >= len(records) // 2
        else None
    )
    joined = []
    for r in records:
        house = by_id.get(number(r["params"].get(id_key, ""))) if id_key else None
        house = house or by_name.get(r["params"].get("name", r["title"]).casefold())
        if house:
            joined.append((r, house))
    ours = {
        "rent_gold": lambda h: h["rent_gold"],
        "size_sqm": lambda h: h["size_sqm"],
        "beds": lambda h: h["beds"],
        "town": lambda h: h["town"]["key"].rsplit(".", 1)[1],
    }
    fields = {}
    for field, get in ours.items():
        agree: Counter = Counter()
        seen: Counter = Counter()

        def wiki_value(value):
            if field == "town":
                return re.sub(r"[^a-z0-9]+", "_", value.lower()).strip("_")
            return number(value)

        for r, house in joined:
            for key, value in r["params"].items():
                seen[key] += 1
                agree[key] += wiki_value(value) == get(house)
        if agree and agree.most_common(1)[0][1]:
            key, hits = agree.most_common(1)[0]
            fields[field] = {
                "wiki_param": key,
                "agree": hits,
                "compared": seen[key],
                "disagree_examples": [
                    [house["provenance"]["source_id"], get(house), r["params"][key]]
                    for r, house in joined
                    if key in r["params"] and wiki_value(r["params"][key]) != get(house)
                ][:10],
            }
    return {
        "wiki_records": len(records),
        "join_param": id_key or "name",
        "joined": len(joined),
        "unjoined_titles": sorted(
            r["title"] for r in records if all(r is not j[0] for j in joined)
        )[:50],
        "catalog_houses_without_wiki_page": len(houses)
        - len({id(h) for _, h in joined}),
        "fields": fields,
    }


SELF_TEST_PAGE = "{{Infobox Casa|name=Example Lane 1|houseid=1|city=[[Example Town]]|size=20|beds=2|rent=50.000 gps|notes=Long\nprose}}\nText."


def self_test() -> None:
    name, params = infobox(SELF_TEST_PAGE)
    assert (
        name == "Infobox Casa"
        and params["city"] == "Example Town"
        and "notes" not in params
    ), params
    root = Path(__file__).resolve().parent
    catalog = json.loads(
        (root / "synthetic-valid-house.json").read_text(encoding="utf-8")
    )
    wiki = facts(
        {
            "api": API,
            "pages": [
                {
                    "title": "Example Lane 1",
                    "pageid": 1,
                    "revid": 1,
                    "sha256": "0",
                    "wikitext": SELF_TEST_PAGE,
                }
            ],
        }
    )
    result = compare(wiki, catalog)
    assert result["join_param"] == "houseid" and result["joined"] == 1, result
    assert result["fields"]["rent_gold"] == {
        "wiki_param": "rent",
        "agree": 1,
        "compared": 1,
        "disagree_examples": [],
    }, result
    assert result["fields"]["town"]["wiki_param"] == "city", result
    print("self-test ok")


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("fetch").add_argument("--out", type=Path, required=True)
    f = sub.add_parser("facts")
    f.add_argument("--snapshot", type=Path, required=True)
    f.add_argument("--out", type=Path, required=True)
    c = sub.add_parser("compare")
    c.add_argument("--facts", type=Path, required=True)
    c.add_argument("--catalog", type=Path, required=True)
    sub.add_parser("self-test")
    args = parser.parse_args(argv)
    if args.command == "self-test":
        self_test()
        return 0
    if args.command == "fetch":
        result = fetch()
    elif args.command == "facts":
        result = facts(json.loads(args.snapshot.read_text(encoding="utf-8")))
    else:
        result = compare(
            json.loads(args.facts.read_text(encoding="utf-8")),
            json.loads(args.catalog.read_text(encoding="utf-8")),
        )
        print(json.dumps(result, indent=1, ensure_ascii=False))
        return 0
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(
        json.dumps(result, indent=1, ensure_ascii=False, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
