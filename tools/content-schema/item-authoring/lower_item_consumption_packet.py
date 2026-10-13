"""Lower edible-food and restore-potion sources into pinned consumption facts and an admission packet.

ITEM-SEM-USE-2 (packet 2026-10-04 section 1.5, widened by the owner on 2026-10-13 to every edible
food with a source value and every health/mana/spirit restore potion with its flasks).

Two stages, so a run is reproducible from recorded sources:

* ``--capture`` (network) records the TibiaWiki pages (revision-pinned) and the Canary foods.lua
  and potions.lua lines into the sources snapshot. Nothing else reads the network.
* the default run lowers the snapshot, the pinned Canary ``items.xml`` and the World Project item
  records into the facts packet and the ``OTERYN_ITEM_ADMISSION/v3`` packet. ``--check`` compares
  both against the committed bytes.

Source order is TibiaWiki first, Canary 04b83b51 as fallback (OTS_HYPOTHESIS_ONLY). The conditions
that attribute potions apply have no definition in the typed model, so those rows are reported,
never lowered.
"""

import argparse
import hashlib
import json
import re
import sys
import urllib.parse
import urllib.request
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
EVIDENCE = ROOT / "docs/agents/evidence"
SNAPSHOT = EVIDENCE / "OTV2-20261004-item-consumption-sources-v1.json"
FACTS = EVIDENCE / "OTV2-20261004-item-consumption-facts-v1.json"
ADMISSION = EVIDENCE / "OTV2-20261004-consumable-item-admission.json"
WORLD_REFERENCE = ROOT / "content/world/definitions/reference.json"
ITEMS_XML = ROOT / "imports/canary/items-xml/items.xml"
ITEMS_XML_SHA256 = "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2"

TASK = "OTV2-20261004-item-sem-use-2-food-and-potion-content"
DECISION = (
    "Oteryn/Oteryn-Game#1622 ITEM-SEM-USE-2 (packet 2026-10-04 sections 1.5-1.6), widened by the "
    "owner on 2026-10-13: every edible food with a source value and every restore potion and flask"
)
CANARY_REVISION = "04b83b512114bfd888000d6e1433ed8ecaec7c5b"
CANARY_RAW = (
    "https://raw.githubusercontent.com/opentibiabr/canary/" + CANARY_REVISION + "/"
)
FOODS_LUA = "data/scripts/actions/items/foods.lua"
POTIONS_LUA = "data/scripts/actions/items/potions.lua"
# Pinned Canary sources: the whole-file sha256 at CANARY_REVISION and the digest of the extracted
# rows, so an edited or mis-refreshed snapshot cannot change gameplay data under the recorded SHA.
CANARY_PINS = {
    "foods": (
        FOODS_LUA,
        "d561ef0e3c583b7f08415e29b7da91cc6956ce2b2a99b87f27f18d0426b55cde",
        "fea7037889e67a20aabfe95a7fa4c9603d4859f0194263f08b5fcde7a3b5c289",
    ),
    "potions": (
        POTIONS_LUA,
        "d2a334ede24e3d1d22fad81a22e7cd7da12aa8bbbd067d0a68229a494f657fc8",
        "3be385e2088a88bff565997abc89a074d5f6f89ce7fd5be570671f07aae28161",
    ),
}
# Pinned TibiaWiki extraction: the digest of every page record (revision, content hash, extracted
# parameters) and of the potions page record, so one edited parameter cannot keep its recorded
# revision and content hash.
WIKI_PINS = {
    "pages": "571034a94e0abc9561a6e6b55326b1718c244b4c0175d795a557356bc2cbaacb",
    "potions_page": "9ce669654a55653ba05408e1441685c4d3e9f6fd861a490c74acb1f75a50b55d",
}
WIKI_API = "https://tibia.fandom.com/api.php"
KEY = "oteryn:item.tibia.i"
REVISION = "definition-r1"
STACK_MAX = 100
MAX_FOOD_SECONDS = 1199
CANARY_SECONDS_PER_FOOD_VALUE = 12
FLASKS = {"Small": 285, "Medium": 283, "Large": 284}
FLASK_PAGES = tuple(f"Empty Potion Flask ({size})" for size in FLASKS)
# TibiaWiki "Potions" page (revision pinned in the snapshot): the level each potion needs.
WIKI_LEVELS = {
    236: 50,
    237: 50,
    238: 80,
    239: 80,
    7642: 80,
    7643: 130,
    23373: 130,
    23374: 130,
    23375: 200,
}
VOCATIONS = {"SORCERER", "DRUID", "PALADIN", "KNIGHT", "MONK"}
PARAMS = (
    "itemid",
    "name",
    "actualname",
    "stackable",
    "regenseconds",
    "consumable",
    "primarytype",
    "secondarytype",
)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def fetch(url):
    request = urllib.request.Request(
        url, headers={"User-Agent": "oteryn-item-sem-use-2/1"}
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        return response.read()


def wiki_query(**parameters):
    parameters.update(action="query", format="json", formatversion="2")
    return json.loads(fetch(WIKI_API + "?" + urllib.parse.urlencode(parameters)))


def category_members(category):
    titles, extra = [], {}
    while True:
        data = wiki_query(
            list="categorymembers", cmtitle=category, cmlimit="500", **extra
        )
        titles += [m["title"] for m in data["query"]["categorymembers"] if m["ns"] == 0]
        if "continue" not in data:
            return titles
        extra = {"cmcontinue": data["continue"]["cmcontinue"]}


def infobox_params(content):
    found = {}
    for name in PARAMS:
        match = re.search(rf"^\|\s*{name}\s*=[ \t]*([^\n]*)$", content, re.MULTILINE)
        if match:
            found[name] = match.group(1).strip()
    return found


def flask_notes(content):
    match = re.search(
        r"^\|\s*notes\s*=(.*?)(?=^\|\s*history|\Z)", content, re.MULTILINE | re.DOTALL
    )
    return re.findall(r"\[\[([^\]|]+)", match.group(1)) if match else []


def wiki_pages(titles):
    pages = []
    for start in range(0, len(titles), 40):
        data = wiki_query(
            prop="revisions",
            rvprop="ids|timestamp|content",
            rvslots="main",
            titles="|".join(titles[start : start + 40]),
        )
        for page in data["query"]["pages"]:
            if "revisions" not in page:
                continue
            revision = page["revisions"][0]
            content = revision["slots"]["main"]["content"]
            params = infobox_params(content)
            row = {
                "title": page["title"],
                "revision_id": revision["revid"],
                "revision_timestamp": revision["timestamp"],
                "content_sha256": sha256(content.encode()),
                "params": params,
            }
            if page["title"] in FLASK_PAGES:
                row["flask_notes"] = flask_notes(content)
            pages.append(row)
    return pages


def canary_file(path):
    raw = fetch(CANARY_RAW + path)
    lines = raw.decode().split("\n")
    kept = [
        {"line": number, "text": text}
        for number, text in enumerate(lines, 1)
        if re.match(r"^\t\[\d+\] = ", text)
    ]
    return {"path": path, "sha256": sha256(raw), "rows": kept}


def capture():
    titles = sorted(
        set(category_members("Category:Food") + category_members("Category:Potions"))
    )
    titles = sorted(set(titles) | set(FLASK_PAGES) | {"Potions"})
    pages = wiki_pages(titles)
    potions_page = next(p for p in pages if p["title"] == "Potions")
    pages = [p for p in pages if p["title"] != "Potions"]
    pages.sort(key=lambda p: p["title"])
    wanted = set(FLASKS.values())
    for page in pages:
        wanted.update(ids_of(page["params"]))
    wanted.update(parse_foods(canary_file(FOODS_LUA)["rows"]))
    wanted.update(parse_potions(canary_file(POTIONS_LUA)["rows"]))
    # The tracked world already carries the admissions this tool emits, so a refresh keeps the
    # recorded pre-admission baseline of every id it already holds and reads the world only for ids
    # that are new to the snapshot.
    recorded = (
        json.loads(SNAPSHOT.read_bytes())["world_baseline"] if SNAPSHOT.exists() else {}
    )
    baseline = {
        str(i): recorded.get(str(i), v) for i, v in world_items().items() if i in wanted
    }
    for value in baseline.values():
        value.pop("consumption", None)
    snapshot = {
        "schema": "OTERYN_ITEM_CONSUMPTION_SOURCES/v1",
        "world_baseline": baseline,
        "task": TASK,
        "tibiawiki": {
            "api": WIKI_API,
            "potions_page": {
                k: potions_page[k] for k in ("title", "revision_id", "content_sha256")
            },
            "pages": pages,
        },
        "canary": {
            "repository": "opentibiabr/canary",
            "revision": CANARY_REVISION,
            "foods": canary_file(FOODS_LUA),
            "potions": canary_file(POTIONS_LUA),
        },
    }
    SNAPSHOT.write_bytes(encode(snapshot))


def encode(value):
    return (
        json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()


def world_items():
    document = json.loads(WORLD_REFERENCE.read_bytes())
    items = {}
    for record in document["records"]:
        if record.get("kind") != "Item":
            continue
        key = record["identity"]["key"]
        if not key.startswith(KEY):
            continue
        presentation = record.get("semantics", {}).get("presentation", {})
        name = presentation.get("value", {}).get("name", {}).get("value")
        stack = record.get("semantics", {}).get("stack", {})
        items[int(key[len(KEY) :])] = {
            "materializable": record["materializable"],
            "stack_class": record["stack_class"],
            "name": name,
            "stack_known": stack.get("state") == "KNOWN",
            "consumption": record.get("semantics", {}).get("consumption"),
        }
    return items


def items_xml_lines():
    data = ITEMS_XML.read_bytes()
    if sha256(data) != ITEMS_XML_SHA256:
        raise SystemExit("Canary items.xml digest drift")
    lines = data.decode().split("\n")
    found = {}
    for number, text in enumerate(lines, 1):
        match = re.match(r'\s*<item id="(\d+)"', text)
        if not match:
            continue
        end = number
        if not text.rstrip().endswith("/>"):
            while "</item>" not in lines[end - 1]:
                end += 1
        attrs = {}
        for inner in lines[number:end]:
            attr = re.search(r'<attribute key="(\w+)" value="([^"]*)"', inner)
            if attr:
                attrs[attr.group(1)] = attr.group(2)
        found[int(match.group(1))] = {
            "lines": f"{number}-{end}",
            "attributes": attrs,
        }
    return found


def parse_foods(rows):
    foods = {}
    for row in rows:
        match = re.match(r"^\t\[(\d+)\] = \{ (\d+), ", row["text"])
        if match:
            foods[int(match.group(1))] = {
                "value": int(match.group(2)),
                "line": row["line"],
            }
    return foods


def parse_potions(rows):
    potions = {}
    for row in rows:
        text = row["text"]
        match = re.match(r"^\t\[(\d+)\] = \{(.*)\},?$", text)
        if not match:
            # A multi-line entry (the monk transcendence potion) carries a condition.
            match = re.match(r"^\t\[(\d+)\] = \{$", text)
            if match:
                potions[int(match.group(1))] = {
                    "line": row["line"],
                    "unsupported": ["condition"],
                }
            continue
        body = match.group(2)
        entry = {"line": row["line"]}
        for field in ("health", "mana"):
            ranges = re.search(rf"\b{field} = \{{ (\d+), (\d+) \}}", body)
            if ranges:
                entry[field] = [int(ranges.group(1)), int(ranges.group(2))]
        level = re.search(r"\blevel = (\d+)", body)
        if level:
            entry["level"] = int(level.group(1))
        vocations = re.search(r"\bvocations = \{ ([^}]*) \}", body)
        if vocations:
            entry["vocations"] = sorted(
                {
                    v.upper()
                    for v in re.findall(r"VOCATION\.BASE_ID\.(\w+)", vocations.group(1))
                }
            )
        flask = re.search(r"\bflask = (\d+)", body)
        if flask:
            entry["flask"] = int(flask.group(1))
        unsupported = [
            name
            for name in ("transform", "condition", "combat", "func")
            if re.search(rf"\b{name} = ", body)
        ]
        if unsupported:
            entry["unsupported"] = unsupported
        potions[int(match.group(1))] = entry
    return potions


def ids_of(params):
    raw = params.get("itemid", "")
    if re.fullmatch(r"\d+(\s*,\s*\d+)*", raw):
        return [int(part) for part in raw.split(",")]
    return []


def squash(name):
    return re.sub(r"[^a-z0-9]", "", re.sub(r"^(the|an?)\s+", "", (name or "").lower()))


def target(item_id):
    return {"family": "Item", "key": f"{KEY}{item_id}", "revision": REVISION}


def verify_canary(snapshot):
    if snapshot.get("schema") != "OTERYN_ITEM_CONSUMPTION_SOURCES/v1":
        raise SystemExit("sources snapshot schema drift")
    canary = snapshot["canary"]
    if (
        canary.get("repository") != "opentibiabr/canary"
        or canary.get("revision") != CANARY_REVISION
    ):
        raise SystemExit("sources snapshot Canary repository or revision drift")
    for name, (path, file_sha256, rows_sha256) in CANARY_PINS.items():
        source = canary[name]
        if (
            source.get("path") != path
            or source.get("sha256") != file_sha256
            or sha256(encode(source["rows"])) != rows_sha256
        ):
            raise SystemExit(f"sources snapshot Canary {name} drift")


def verify_wiki(snapshot):
    wiki = snapshot["tibiawiki"]
    for name, rows_sha256 in WIKI_PINS.items():
        if sha256(encode(wiki[name])) != rows_sha256:
            raise SystemExit(f"sources snapshot TibiaWiki {name} drift")


def build(snapshot):
    verify_canary(snapshot)
    verify_wiki(snapshot)
    world = {int(i): v for i, v in snapshot["world_baseline"].items()}
    xml = items_xml_lines()
    foods = parse_foods(snapshot["canary"]["foods"]["rows"])
    potions = parse_potions(snapshot["canary"]["potions"]["rows"])
    pages = snapshot["tibiawiki"]["pages"]
    by_id = {}
    for page in pages:
        for item_id in ids_of(page["params"]):
            by_id.setdefault(item_id, []).append(page)

    reported = []
    rows = []
    admit = {}

    def report(kind, reason, item_id=None, title=None, **extra):
        entry = {"kind": kind, "reason": reason}
        if item_id is not None:
            entry["item_id"] = item_id
        if title is not None:
            entry["title"] = title
        entry.update(extra)
        reported.append(entry)

    def stack_shape(item_id, record, page):
        """Return (admission_needed, error) for the Item's stack shape against its sources."""
        wiki = (page or {}).get("params", {}).get("stackable")
        if record["materializable"]:
            if wiki in ("yes", "no") and record["stack_class"] in (
                "StackCapable",
                "NonStackable",
            ):
                same = (wiki == "yes") == (record["stack_class"] == "StackCapable")
                return False, None if same else "stack_conflict"
            return False, None
        if wiki == "yes":
            return "STACK_CAPABLE", None
        if wiki == "no":
            return "NON_STACKABLE", None
        return False, "stack_unknown"

    def want_admission(item_id, record, page, shape):
        if shape is False or not isinstance(shape, str):
            return
        xml_row = xml.get(item_id)
        evidence = {
            "tibiawiki": {
                "title": page["title"],
                "revision_id": page["revision_id"],
                "content_sha256": page["content_sha256"],
                "stackable": page["params"].get("stackable"),
            },
            "canary": {
                "repository": "opentibiabr/canary",
                "revision": CANARY_REVISION,
                "path": "data/items/items.xml",
                "sha256": ITEMS_XML_SHA256,
                "lines": xml_row["lines"] if xml_row else None,
                "attributes": xml_row["attributes"] if xml_row else None,
                "disposition": "SILENT_ON_STACKABLE" if xml_row else "NO_ITEM_ROW",
            },
        }
        admit[item_id] = {
            "item_key": f"{KEY}{item_id}",
            "materializable": True,
            "stack_class": shape,
            "stack_max": STACK_MAX if shape == "STACK_CAPABLE" else None,
            "evidence": evidence,
        }

    # Food: TibiaWiki regenseconds first, Canary foods.lua x 12 s as fallback.
    candidates = set(foods)
    for item_id, group in by_id.items():
        if any(p["params"].get("regenseconds") for p in group):
            candidates.add(item_id)
    lowered_food = Counter()
    for item_id in sorted(candidates):
        group = by_id.get(item_id, [])
        page = group[0] if group else None
        if len(group) > 1:
            report("FOOD", "multiple_wiki_pages", item_id)
            continue
        record = world.get(item_id)
        if record is None:
            report("FOOD", "no_item_record", item_id, page and page["title"])
            continue
        wiki_raw = page["params"].get("regenseconds") if page else None
        canary = foods.get(item_id)
        if wiki_raw is not None and re.fullmatch(r"\d+", wiki_raw):
            seconds, source = int(wiki_raw), "TIBIAWIKI"
        elif wiki_raw is not None:
            if canary is None:
                report(
                    "FOOD",
                    "wiki_value_not_integer",
                    item_id,
                    page["title"],
                    value=wiki_raw,
                )
                continue
            seconds, source = (
                canary["value"] * CANARY_SECONDS_PER_FOOD_VALUE,
                "OTS_HYPOTHESIS_ONLY",
            )
        elif canary is not None:
            seconds, source = (
                canary["value"] * CANARY_SECONDS_PER_FOOD_VALUE,
                "OTS_HYPOTHESIS_ONLY",
            )
        else:
            report("FOOD", "no_source_value", item_id, page and page["title"])
            continue
        if seconds <= 0:
            report("FOOD", "non_positive_value", item_id, page and page["title"])
            continue
        if seconds > MAX_FOOD_SECONDS:
            report(
                "FOOD",
                "regeneration_1200_seconds_or_more",
                item_id,
                page and page["title"],
                seconds=seconds,
            )
            continue
        if page is None:
            report("FOOD", "no_wiki_page_for_stack_shape", item_id)
            continue
        if record["name"] is None or squash(record["name"]) not in {
            squash(page["params"].get("name")),
            squash(page["params"].get("actualname")),
        }:
            report(
                "FOOD",
                "wiki_name_mismatch",
                item_id,
                page["title"],
                world_name=record["name"],
            )
            continue
        shape, error = stack_shape(item_id, record, page)
        if error:
            report("FOOD", error, item_id, page["title"])
            continue
        want_admission(item_id, record, page, shape)
        row = {
            "target": target(item_id),
            "consumption": {"kind": "FOOD", "value": {"regeneration_seconds": seconds}},
            "source": {
                "kind": source,
                "seconds": seconds,
                "tibiawiki": {
                    "title": page["title"],
                    "revision_id": page["revision_id"],
                    "regenseconds": wiki_raw,
                },
                "canary": (
                    {
                        "path": FOODS_LUA,
                        "line": canary["line"],
                        "value": canary["value"],
                    }
                    if canary
                    else None
                ),
            },
        }
        if canary and wiki_raw is not None and re.fullmatch(r"\d+", wiki_raw):
            row["source"]["canary_agrees"] = (
                canary["value"] * CANARY_SECONDS_PER_FOOD_VALUE == seconds
            )
        rows.append(row)
        lowered_food[source] += 1

    # Flasks: TibiaWiki names the potions each flask size holds; Canary corroborates.
    flask_of_title = {}
    flask_pages = {}
    for page in pages:
        if page["title"] in FLASK_PAGES:
            size = page["title"].split("(")[1].rstrip(")")
            flask_pages[FLASKS[size]] = page
            for note in page.get("flask_notes", []):
                flask_of_title[note] = FLASKS[size]
    wiki_potion_pages = {
        p["title"]: p for p in pages if p["params"].get("secondarytype") == "Potions"
    }
    admitted_flasks = set()
    flask_stack_error = {}
    for flask_id, page in sorted(flask_pages.items()):
        record = world.get(flask_id)
        if record is None:
            report("FLASK", "no_item_record", flask_id, page["title"])
            continue
        shape, error = stack_shape(flask_id, record, page)
        if error:
            flask_stack_error[flask_id] = error
            report("FLASK", error, flask_id, page["title"])
            continue
        want_admission(flask_id, record, page, shape)
        admitted_flasks.add(flask_id)

    # Potions.
    potion_count = Counter()
    for item_id, entry in sorted(potions.items()):
        group = by_id.get(item_id, [])
        page = group[0] if group else None
        title = page["title"] if page else None
        if "health" not in entry and "mana" not in entry:
            reason = (
                "attribute_or_condition_potion"
                if entry.get("unsupported")
                else "no_restore"
            )
            report(
                "POTION",
                reason,
                item_id,
                title,
                unsupported=entry.get("unsupported", []),
            )
            continue
        if entry.get("unsupported"):
            report(
                "POTION",
                "restore_with_condition",
                item_id,
                title,
                unsupported=entry["unsupported"],
            )
            continue
        record = world.get(item_id)
        if record is None:
            report("POTION", "no_item_record", item_id, title)
            continue
        if page is None:
            report("POTION", "no_wiki_page", item_id)
            continue
        if squash(record["name"]) not in {
            squash(page["params"].get("name")),
            squash(page["params"].get("actualname")),
        }:
            report(
                "POTION",
                "wiki_name_mismatch",
                item_id,
                title,
                world_name=record["name"],
            )
            continue
        flask_wiki = flask_of_title.get(title)
        flask_canary = entry.get("flask")
        if flask_wiki is None and flask_canary is None:
            report("POTION", "no_flask_source", item_id, title)
            continue
        if (
            flask_wiki is not None
            and flask_canary is not None
            and flask_wiki != flask_canary
        ):
            report(
                "POTION",
                "flask_source_conflict",
                item_id,
                title,
                wiki=flask_wiki,
                canary=flask_canary,
            )
            continue
        flask = flask_wiki if flask_wiki is not None else flask_canary
        if flask not in admitted_flasks and not (world.get(flask) or {}).get(
            "materializable"
        ):
            report("POTION", "flask_not_materializable", item_id, title, flask=flask)
            continue
        level = entry.get("level")
        if item_id in WIKI_LEVELS and level != WIKI_LEVELS[item_id]:
            report(
                "POTION",
                "level_source_conflict",
                item_id,
                title,
                wiki=WIKI_LEVELS[item_id],
                canary=level,
            )
            continue
        vocations = entry.get("vocations")
        if vocations is not None and not set(vocations) <= VOCATIONS:
            report("POTION", "unknown_vocation", item_id, title, vocations=vocations)
            continue
        shape, error = stack_shape(item_id, record, page)
        if error:
            report("POTION", error, item_id, title)
            continue
        restores = []
        for resource, field in (("HEALTH", "health"), ("MANA", "mana")):
            if field in entry:
                low, high = entry[field]
                if not 1 <= low <= high <= 10000:
                    report(
                        "POTION",
                        "restore_range_outside_model",
                        item_id,
                        title,
                        field=field,
                    )
                    restores = None
                    break
                restores.append({"resource": resource, "min": low, "max": high})
        if restores is None:
            continue
        want_admission(item_id, record, page, shape)
        row = {
            "target": target(item_id),
            "consumption": {
                "kind": "POTION",
                "value": {
                    "restores": restores,
                    "empty_flask": {
                        "state": "KNOWN",
                        "value": {"key": target(flask)["key"], "revision": REVISION},
                    },
                },
            },
            "source": {
                "kind": "OTS_HYPOTHESIS_ONLY",
                "tibiawiki": {"title": title, "revision_id": page["revision_id"]},
                "canary": {"path": POTIONS_LUA, "line": entry["line"]},
                "flask_basis": "TIBIAWIKI_FLASK_PAGE"
                if flask_wiki is not None
                else "CANARY_ONLY",
                "ranges_basis": "CANARY_ONLY",
            },
        }
        if level is not None or vocations is not None:
            row["use_requirements"] = {
                "min_level": {"state": "KNOWN", "value": level}
                if level is not None
                else {"state": "UNKNOWN"},
                "min_magic_level": {"state": "UNKNOWN"},
                "vocations": {"state": "KNOWN", "value": vocations}
                if vocations is not None
                else {"state": "UNKNOWN"},
                "enforcement_mode": "ON_USE",
            }
        rows.append(row)
        potion_count[len(restores)] += 1

    # Wiki restore potions with no range source anywhere.
    for title, page in sorted(wiki_potion_pages.items()):
        ids = ids_of(page["params"])
        if ids and not any(i in potions for i in ids):
            report("POTION", "no_range_source", ids[0], title)

    # An admission without a lowered row is only a flask.
    rows.sort(key=lambda r: int(r["target"]["key"][len(KEY) :]))
    reported.sort(key=lambda r: (r["kind"], r["reason"], r.get("item_id", 0)))
    admissions = [admit[i] for i in sorted(admit)]
    reasons = Counter((r["kind"], r["reason"]) for r in reported)
    counts = {
        "food_lowered": sum(1 for r in rows if r["consumption"]["kind"] == "FOOD"),
        "food_lowered_by_source": dict(sorted(lowered_food.items())),
        "potions_lowered": sum(1 for r in rows if r["consumption"]["kind"] == "POTION"),
        "potions_two_entries": potion_count[2],
        "flasks_admitted_or_present": len(admitted_flasks),
        "admissions": len(admissions),
        "reported": len(reported),
        "reported_by_reason": {
            "{}:{}".format(*k): v for k, v in sorted(reasons.items())
        },
    }
    facts = {
        "schema": "OTERYN_ITEM_CONSUMPTION_FACTS/v1",
        "task": TASK,
        "decision": DECISION,
        "sources": {
            "snapshot": str(SNAPSHOT.relative_to(ROOT)),
            "canary_revision": CANARY_REVISION,
            "tibiawiki_potions_page": snapshot["tibiawiki"]["potions_page"],
        },
        "counts": counts,
        "rows": rows,
        "reported": reported,
    }
    admission = {
        "schema": "OTERYN_ITEM_ADMISSION/v3",
        "task": TASK,
        "decision": DECISION,
        "policy": {
            "agreement": "PINNED_EVIDENCE_NO_CONFLICT",
            "scope": "CONSUMABLE_STACK_SHAPE_ONLY",
        },
        "admissions": admissions,
    }
    return facts, admission


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--capture", action="store_true", help="refresh the sources snapshot (network)"
    )
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.capture:
        capture()
        return
    snapshot = json.loads(SNAPSHOT.read_bytes())
    facts, admission = build(snapshot)
    outputs = ((FACTS, encode(facts)), (ADMISSION, encode(admission)))
    if args.check:
        for path, data in outputs:
            if path.read_bytes() != data:
                raise SystemExit(f"{path.name} drift")
    else:
        for path, data in outputs:
            path.write_bytes(data)
    print(json.dumps(facts["counts"], sort_keys=True, indent=1), file=sys.stderr)


if __name__ == "__main__":
    main()
