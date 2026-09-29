#!/usr/bin/env python3
"""Charm authoring candidate v1: source capture, catalogue build and validation.

capture   reads the 25 TibiaWiki (tibia.fandom.com) Charm pages over the MediaWiki API
          and the pinned Canary bestiary_charms.lua, and writes source facts only
          (numbers, versions, page revisions and digests; no wiki prose, no engine code).
build     turns the committed source facts into the candidate catalogue and the
          wiki/Canary comparison report; --check regenerates in memory and diffs.
validate  checks a catalogue against charm.schema.json and the semantic rules.

content    writes (or, with --check, verifies) the populated content/charms/ family and its
          registration in content/project.json, content/manifest.json and content/content.lock.json.

capture/build/validate write only under this directory; see README.md.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import itertools
import json
import re
import sys
import urllib.parse
import urllib.request
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
SCHEMA = ROOT / "charm.schema.json"
SOURCES = ROOT / "samples" / "charm-sources-2026-09-29.json"
CATALOGUE = ROOT / "samples" / "charms-candidate.json"
REPORT = ROOT / "samples" / "charm-source-comparison.json"

WIKI_API = "https://tibia.fandom.com/api.php"
CANARY_REVISION = "47dfd51f45280a59a1d3e50ba7edd573d7234446"
CANARY_PATH = "data/scripts/systems/bestiary_charms.lua"
CANARY_SHA256 = "19350ba311a04797705d0e82f608d61080924ac95f8b0d592ae93fe0bb8debd6"
CATALOGUE_SCHEMA = "OTERYN_CHARM_AUTHORING_CATALOGUE/v1"
SOURCES_SCHEMA = "OTERYN_CHARM_SOURCE_FACTS/v1"
REPORT_SCHEMA = "OTERYN_CHARM_SOURCE_COMPARISON/v1"

FAMILY = "Charm"
CONTENT_DIR = "content/charms/"
INDEX_PATH = CONTENT_DIR + "index.json"
SHARD_PATH = CONTENT_DIR + "charms-00000-00024.json"
REVISION = "definition-r1"
CATALOGUE_REL = "tools/content-schema/charm-authoring/samples/charms-candidate.json"
INDEX_SCHEMA = "OTERYN_FAMILY_INDEX/v1"
SHARD_SCHEMA = "OTERYN_CHARM_SHARD/v1"
# content.lock.json family_counts is asserted verbatim by tools/content-migration, which
# does not know Charm yet, so the static Charm count lives beside it.
LOCK_COUNT_KEY = "static_family_counts"

CURRENCY = {"major": "charm_points", "minor": "minor_charm_echoes"}
CHANCE, EFFECT = "trigger_chance_percent", "effect_percent"
STAGE_VALUE = {
    "attack_proc_damage": CHANCE,
    "attack_proc_resource_damage": CHANCE,
    "kill_area_damage": CHANCE,
    "paralyse_creature_on_attack": CHANCE,
    "paralyse_creature_after_its_attack": CHANCE,
    "haste_after_hit": CHANCE,
    "prevent_creature_flee": CHANCE,
    "reflect_damage_taken": CHANCE,
    "dodge_attack": CHANCE,
    "cleanse_after_hit": CHANCE,
    "mana_drain_inversion": CHANCE,
    "death_loss_reduction": EFFECT,
    "skinning_chance_bonus": EFFECT,
    "creature_product_bonus": EFFECT,
    "critical_hit_chance": EFFECT,
    "critical_extra_damage": EFFECT,
    "life_leech": EFFECT,
    "mana_leech": EFFECT,
}


def proc(element: str) -> tuple[dict, list[str]]:
    effect = {
        "type": "attack_proc_damage",
        "element": element,
        "percent_of_creature_max_health": 5,
        "damage_cap_level_multiplier": 2,
    }
    phrases = [
        f"5% of its maximum hit points as {element} damage",
        "damage is limited to 2 times the character's level",
    ]
    return effect, phrases


def resource(kind: str, percent: float, word: str) -> tuple[dict, list[str]]:
    effect = {
        "type": "attack_proc_resource_damage",
        "element": "physical",
        "resource": kind,
        "percent_of_own_maximum": percent,
        "damage_cap_percent_of_creature_max_health": 8,
    }
    phrases = [
        f"physical damage equal to {percent}% of your maximum {word}",
        "limited up to 8% of the creature's maximum health",
    ]
    return effect, phrases


def plain(
    kind: str, *phrases: str, duration_ms: int | None = None
) -> tuple[dict, list[str]]:
    effect: dict = {"type": kind}
    if duration_ms is not None:
        effect["duration_ms"] = duration_ms
    return effect, list(phrases)


# Wiki page title -> (effect, phrases the page text must contain, lowercase, links unwrapped).
# The phrases are what ties each hand-written parameter to the captured page revision.
CHARMS: dict[str, tuple[dict, list[str]]] = {
    "Wound": proc("physical"),
    "Enflame": proc("fire"),
    "Poison": proc("earth"),
    "Freeze": proc("ice"),
    "Zap": proc("energy"),
    "Curse (Charm)": proc("death"),
    "Divine Wrath": proc("holy"),
    "Overpower": resource("health", 5, "health"),
    "Overflux": resource("mana", 2.5, "mana"),
    "Carnage": (
        {
            "type": "kill_area_damage",
            "element": "physical",
            "percent_of_creature_max_health": 15,
        },
        [
            "killing a monster",
            "physical damage equal to 15% of its maximum health to all monsters",
        ],
    ),
    "Cripple": plain(
        "paralyse_creature_on_attack", "paralyses it for 10 seconds", duration_ms=10000
    ),
    "Numb": plain(
        "paralyse_creature_after_its_attack",
        "after its attack",
        "paralyses the creature for 10 seconds",
        duration_ms=10000,
    ),
    "Adrenaline Burst": plain(
        "haste_after_hit",
        "after getting hit",
        "move faster for 10 seconds",
        duration_ms=10000,
    ),
    "Fatal Hold": plain(
        "prevent_creature_flee",
        "prevent creatures from fleeing",
        "for 30 seconds",
        duration_ms=30000,
    ),
    "Parry": plain("reflect_damage_taken", "reflected to the aggressor"),
    "Dodge": plain("dodge_attack", "dodges an attack", "taking no damage"),
    "Cleanse": plain(
        "cleanse_after_hit", "after you get hit", "removes one random active negative"
    ),
    "Void Inversion": plain(
        "mana_drain_inversion",
        "gain mana instead of losing it when taking mana drain damage",
    ),
    "Bless": plain("death_loss_reduction", "reduces skill and xp loss by"),
    "Scavenge": plain(
        "skinning_chance_bonus", "skin/dust a skinnable/dustable creature"
    ),
    "Gut": plain("creature_product_bonus", "more creature products"),
    "Low Blow": plain(
        "critical_hit_chance",
        "critical hit chance to attacks with critical hit weapons",
    ),
    "Savage Blow": plain(
        "critical_extra_damage",
        "critical extra damage to attacks with critical hit weapons",
    ),
    "Vampiric Embrace": plain("life_leech", "life leech to attacks"),
    "Void's Call": plain("mana_leech", "mana leech to attacks"),
}
COMBAT = {
    "PHYSICAL": "physical",
    "FIRE": "fire",
    "EARTH": "earth",
    "ICE": "ice",
    "ENERGY": "energy",
    "DEATH": "death",
    "HOLY": "holy",
    "NEUTRAL": "neutral",
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def dumps(payload: object) -> str:
    return json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def number(text: str) -> int | float:
    value = float(text.replace(",", ""))
    return int(value) if value.is_integer() else value


def unwrap(wikitext: str) -> str:
    """Lowercase page text with [[target|label]] and [[target]] links reduced to their label."""
    text = re.sub(r"\[\[(?:[^|\]]*\|)?([^\]]*)\]\]", r"\1", wikitext)
    return re.sub(r"\s+", " ", text).lower()


def wiki_facts(
    title: str, pageid: int, revid: int, timestamp: str, wikitext: str
) -> dict:
    """Infobox Charm facts of one page; rejects any page whose shape or phrases differ."""
    fields = dict(re.findall(r"^\|\s*(\w+)\s*=\s*(.*?)\s*$", wikitext, re.MULTILINE))
    if not re.search(r"\{\{Infobox[ _]Charm\|", wikitext):
        raise ValueError(f"{title}: no Infobox Charm")
    for field in ("name", "type", "cost", "effect", "implemented"):
        if not fields.get(field):
            raise ValueError(f"{title}: missing infobox field {field}")
    if fields["type"] not in ("Major", "Minor"):
        raise ValueError(f"{title}: unexpected type {fields['type']!r}")
    cost_parts = (
        re.split(r"\s*/\s*", fields["cost"])
        if "/" in fields["cost"]
        else re.split(r",\s+", fields["cost"])
    )
    if len(cost_parts) != 3 or not all(
        re.fullmatch(r"\d{1,3}(,\d{3})*|\d+", p) for p in cost_parts
    ):
        raise ValueError(f"{title}: unexpected cost {fields['cost']!r}")
    values = re.search(
        r"(\d+(?:\.\d+)?)%\s*/\s*(\d+(?:\.\d+)?)%\s*/\s*(\d+(?:\.\d+)?)%",
        fields["effect"],
    )
    if values is None:
        raise ValueError(f"{title}: no per-stage percent triple in effect")
    text = unwrap(wikitext)
    phrases = CHARMS[title][1]
    missing = [p for p in phrases if p not in text]
    if missing:
        raise ValueError(f"{title}: page text lacks {missing}")
    status = fields.get("status") or None
    if status not in (None, "active"):
        raise ValueError(f"{title}: status {status!r}")
    return {
        "title": title,
        "pageid": pageid,
        "revid": revid,
        "timestamp": timestamp,
        "wikitext_sha256": sha256(wikitext.encode("utf-8")),
        "name": fields.get("actualname") or fields["name"],
        "type": fields["type"].lower(),
        "cost": [number(p) for p in cost_parts],
        "stage_values": [number(v) for v in values.groups()],
        "implemented": fields["implemented"],
        "confirmed_phrases": phrases,
    }


def canary_facts(source: bytes) -> list[dict]:
    """The charms table of the pinned bestiary_charms.lua, read with a strict pattern parser."""
    if sha256(source) != CANARY_SHA256:
        raise ValueError("canary: bestiary_charms.lua digest mismatch")
    text = source.decode("utf-8")
    blocks = re.findall(
        r"^\t\[(\d+)\] = \{\n(.*?)^\t\},$", text, re.MULTILINE | re.DOTALL
    )
    rows = []
    for position, (index, body) in enumerate(blocks, 1):
        if int(index) != position:
            raise ValueError(f"canary: charm index {index} out of order")

        def one(
            pattern: str, *, required: bool = True, body: str = body, index: str = index
        ) -> str | None:
            found = re.findall(pattern, body, re.MULTILINE)
            if len(found) > 1 or (required and not found):
                raise ValueError(
                    f"canary charm {index}: {pattern!r} matched {len(found)} times"
                )
            return found[0] if found else None

        def triple(
            field: str, body: str = body, index: str = index
        ) -> list[int | float]:
            found = re.findall(
                rf"^\t\t{field} = \{{ ([\d.]+), ([\d.]+), ([\d.]+) \}},$",
                body,
                re.MULTILINE,
            )
            if len(found) != 1:
                raise ValueError(f"canary charm {index}: {field} triple")
            return [number(v) for v in found[0]]

        damage = one(r"damageType = COMBAT_(\w+)DAMAGE,", required=False)
        percent = one(r"percent = ([\d.]+),", required=False)
        rows.append(
            {
                "charm_id": position - 1,
                "name": one(r'^\t\tname = "([^"]+)",$'),
                "category": one(r"category = CHARM_(MAJOR|MINOR),").lower(),
                "kind": one(r"type = CHARM_(OFFENSIVE|DEFENSIVE|PASSIVE),").lower(),
                "damage_type": COMBAT[damage] if damage else None,
                "percent": number(percent) if percent else None,
                "chance": triple("chance"),
                "points": triple("points"),
            }
        )
    if len(rows) != len(CHARMS) or text.count("\t[") != len(rows):
        raise ValueError(f"canary: expected {len(CHARMS)} charms, found {len(rows)}")
    return rows


def fetch_wiki(titles: list[str]) -> list[dict]:
    query = urllib.parse.urlencode(
        {
            "action": "query",
            "prop": "revisions",
            "rvprop": "ids|timestamp|content",
            "rvslots": "main",
            "titles": "|".join(titles),
            "format": "json",
            "formatversion": "2",
        }
    )
    request = urllib.request.Request(
        f"{WIKI_API}?{query}", headers={"User-Agent": "Oteryn charm source capture"}
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        pages = json.load(response)["query"]["pages"]
    if sorted(p["title"] for p in pages) != sorted(titles) or any(
        "missing" in p for p in pages
    ):
        raise ValueError("wiki: page set differs from the expected Charm pages")
    facts = []
    for page in sorted(pages, key=lambda p: p["title"]):
        revision = page["revisions"][0]
        facts.append(
            wiki_facts(
                page["title"],
                page["pageid"],
                revision["revid"],
                revision["timestamp"],
                revision["slots"]["main"]["content"],
            )
        )
    return facts


def capture(canary_root: Path) -> dict:
    return {
        "schema": SOURCES_SCHEMA,
        "tibiawiki": {"api": WIKI_API, "pages": fetch_wiki(sorted(CHARMS))},
        "canary": {
            "revision": CANARY_REVISION,
            "path": CANARY_PATH,
            "file_sha256": CANARY_SHA256,
            "charms": canary_facts((canary_root / CANARY_PATH).read_bytes()),
        },
    }


def charm_key(name: str) -> str:
    return "oteryn:charm." + re.sub(
        r"[^a-z0-9]+", "_", name.lower().replace("'", "")
    ).strip("_")


def build(sources: dict) -> tuple[dict, dict]:
    """Candidate catalogue (wiki values, Canary kind) and the field-by-field comparison report."""
    if sources.get("schema") != SOURCES_SCHEMA:
        raise ValueError("sources: unexpected schema")
    canary = {row["name"]: row for row in sources["canary"]["charms"]}
    charms, comparisons = [], []
    for page in sources["tibiawiki"]["pages"]:
        effect, _ = CHARMS[page["title"]]
        engine = canary.pop(page["name"], None)
        if engine is None:
            raise ValueError(f"{page['name']}: no Canary charm with this name")
        charms.append(
            {
                "key": charm_key(page["name"]),
                "name": page["name"],
                "category": page["type"],
                "kind": engine["kind"],
                "cost_currency": CURRENCY[page["type"]],
                "stage_value": STAGE_VALUE[effect["type"]],
                "stages": [
                    {"stage": i + 1, "cost": cost, "value": value}
                    for i, (cost, value) in enumerate(
                        zip(page["cost"], page["stage_values"], strict=True)
                    )
                ],
                "effect": effect,
                "implemented_version": page["implemented"],
                "sources": {
                    "tibiawiki": {
                        k: page[k]
                        for k in ("title", "pageid", "revid", "wikitext_sha256")
                    },
                    "canary": {
                        "revision": sources["canary"]["revision"],
                        "path": sources["canary"]["path"],
                        "file_sha256": sources["canary"]["file_sha256"],
                        "charm_id": engine["charm_id"],
                    },
                },
            }
        )
        differences = []
        if engine["category"] != page["type"]:
            differences.append(
                {
                    "field": "category",
                    "tibiawiki": page["type"],
                    "canary": engine["category"],
                }
            )
        if engine["points"] != page["cost"]:
            differences.append(
                {
                    "field": "stage_cost",
                    "tibiawiki": page["cost"],
                    "canary": engine["points"],
                }
            )
        if engine["chance"] != page["stage_values"]:
            differences.append(
                {
                    "field": "stage_value",
                    "tibiawiki": page["stage_values"],
                    "canary": engine["chance"],
                }
            )
        wiki_element = effect.get("element")
        if engine["damage_type"] is not None and engine["damage_type"] != wiki_element:
            differences.append(
                {
                    "field": "element",
                    "tibiawiki": wiki_element,
                    "canary": engine["damage_type"],
                }
            )
        wiki_percent = effect.get(
            "percent_of_creature_max_health", effect.get("percent_of_own_maximum")
        )
        if engine["percent"] is not None and engine["percent"] != wiki_percent:
            differences.append(
                {
                    "field": "effect_percent",
                    "tibiawiki": wiki_percent,
                    "canary": engine["percent"],
                }
            )
        comparisons.append({"name": page["name"], "differences": differences})
    if canary:
        raise ValueError(f"Canary charms without a wiki page: {sorted(canary)}")
    catalogue = {
        "schema": CATALOGUE_SCHEMA,
        "charms": sorted(charms, key=lambda c: c["key"]),
    }
    report = {
        "schema": REPORT_SCHEMA,
        "charms": len(comparisons),
        "agreeing": sum(not c["differences"] for c in comparisons),
        "differing": [
            c for c in sorted(comparisons, key=lambda c: c["name"]) if c["differences"]
        ],
    }
    return catalogue, report


def validate(catalogue: dict) -> list[str]:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    errors = [
        f"{'/'.join(map(str, e.absolute_path)) or '<root>'}: {e.message}"
        for e in Draft202012Validator(schema).iter_errors(catalogue)
    ]
    if errors:
        return errors
    seen: dict[str, set] = {"key": set(), "name": set(), "charm_id": set()}
    for charm in catalogue["charms"]:
        where = charm["key"]
        for field, value in (
            ("key", charm["key"]),
            ("name", charm["name"]),
            ("charm_id", charm["sources"]["canary"]["charm_id"]),
        ):
            if value in seen[field]:
                errors.append(f"{where}: duplicate {field} {value!r}")
            seen[field].add(value)
        if charm["key"] != charm_key(charm["name"]):
            errors.append(
                f"{where}: key does not follow the name ({charm_key(charm['name'])})"
            )
        if charm["cost_currency"] != CURRENCY[charm["category"]]:
            errors.append(
                f"{where}: {charm['category']} charm must cost {CURRENCY[charm['category']]}"
            )
        if charm["stage_value"] != STAGE_VALUE[charm["effect"]["type"]]:
            errors.append(
                f"{where}: {charm['effect']['type']} stages carry {STAGE_VALUE[charm['effect']['type']]}"
            )
        stages = charm["stages"]
        if [s["stage"] for s in stages] != [1, 2, 3]:
            errors.append(f"{where}: stages must be 1, 2, 3 in order")
        if not all(a["cost"] < b["cost"] for a, b in itertools.pairwise(stages)):
            errors.append(f"{where}: stage costs must increase")
        if not all(a["value"] < b["value"] for a, b in itertools.pairwise(stages)):
            errors.append(f"{where}: stage values must increase")
        if charm["stage_value"] == CHANCE and stages[-1]["value"] > 100:
            errors.append(f"{where}: a trigger chance cannot exceed 100%")
    return errors


def compact(payload: object) -> str:
    return (
        json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    )


def content_files(catalogue: dict) -> dict[str, str]:
    """The populated family: one shard of definitions and the family index."""
    charms = sorted(catalogue["charms"], key=lambda c: c["key"])
    records = []
    for charm in charms:
        definition = {k: v for k, v in charm.items() if k != "key"}
        definition["identity"] = {"key": charm["key"], "revision": REVISION}
        records.append({"definition": definition})
    last = len(records) - 1
    shard = {
        "family": FAMILY,
        "records": records,
        "schema": SHARD_SCHEMA,
        "shard": {"count": len(records), "end": last, "index": 0, "start": 0},
    }
    index = {
        "authoring_source": {
            "path": CATALOGUE_REL,
            "schema": CATALOGUE_SCHEMA,
            "sha256": sha256(dumps(catalogue).encode()),
        },
        "family": FAMILY,
        "record_count": len(records),
        "schema": INDEX_SCHEMA,
        "shards": [SHARD_PATH],
    }
    return {INDEX_PATH: compact(index), SHARD_PATH: compact(shard)}


def registered(project: dict, manifest: dict, lock: dict, count: int) -> tuple:
    """The three registration documents with the Charm family registered."""
    project, manifest, lock = map(copy.deepcopy, (project, manifest, lock))
    project["migrated_families"] = [
        f for f in project["migrated_families"] if f != FAMILY
    ] + [FAMILY]
    project["next_population_families"] = [
        f for f in project["next_population_families"] if f != FAMILY
    ]
    manifest["families"][FAMILY] = {"records": count, "index": INDEX_PATH}
    paths = {row["path"] for row in manifest["managed_files"]} | {
        INDEX_PATH,
        SHARD_PATH,
    }
    manifest["managed_files"] = [{"path": p} for p in sorted(paths)]
    lock[LOCK_COUNT_KEY] = {**lock.get(LOCK_COUNT_KEY, {}), FAMILY: count}
    return project, manifest, lock


def content_command(check: bool) -> int:
    catalogue = json.loads(CATALOGUE.read_text(encoding="utf-8"))
    errors = validate(catalogue)
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        return 1
    names = ("project", "manifest", "content.lock")
    docs = [
        json.loads((REPO / f"content/{n}.json").read_text(encoding="utf-8"))
        for n in names
    ]
    outputs = dict(content_files(catalogue))
    for name, doc in zip(
        names, registered(*docs, len(catalogue["charms"])), strict=True
    ):
        outputs[f"content/{name}.json"] = compact(doc)
    stale = []
    for rel, text in sorted(outputs.items()):
        path = REPO / rel
        if path.is_file() and path.read_text(encoding="utf-8") == text:
            continue
        stale.append(rel)
        if not check:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8", newline="\n")
    if check:
        print("charm content check: " + (f"FAIL, stale {stale}" if stale else "ok"))
        return 1 if stale else 0
    print(f"charm content: wrote {stale}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    sub = parser.add_subparsers(dest="command", required=True)
    cap = sub.add_parser(
        "capture", help="fetch the wiki pages and read Canary; write the source facts"
    )
    cap.add_argument(
        "--canary",
        type=Path,
        required=True,
        help=f"Canary checkout at {CANARY_REVISION}",
    )
    bld = sub.add_parser(
        "build", help="write the candidate catalogue and comparison report"
    )
    bld.add_argument(
        "--check",
        action="store_true",
        help="diff an in-memory build against the committed samples",
    )
    cnt = sub.add_parser(
        "content", help="write the content/charms/ family and its registration"
    )
    cnt.add_argument(
        "--check",
        action="store_true",
        help="fail if the committed content tree differs from the candidate catalogue",
    )
    val = sub.add_parser("validate", help="validate catalogue files")
    val.add_argument("files", nargs="+", type=Path)
    args = parser.parse_args(argv)

    if args.command == "capture":
        SOURCES.parent.mkdir(exist_ok=True)
        SOURCES.write_text(dumps(capture(args.canary)), encoding="utf-8", newline="\n")
        return 0
    if args.command == "build":
        catalogue, report = build(json.loads(SOURCES.read_text(encoding="utf-8")))
        errors = validate(catalogue)
        for error in errors:
            print(error, file=sys.stderr)
        if errors:
            return 1
        outputs = {CATALOGUE: dumps(catalogue), REPORT: dumps(report)}
        if args.check:
            stale = [
                p.name
                for p, text in outputs.items()
                if not p.is_file() or p.read_text(encoding="utf-8") != text
            ]
            print(
                "charm build check: "
                + (
                    f"FAIL, stale {stale}"
                    if stale
                    else f"ok ({len(catalogue['charms'])} charms)"
                )
            )
            return 1 if stale else 0
        for path, text in outputs.items():
            path.write_text(text, encoding="utf-8", newline="\n")
        return 0
    if args.command == "content":
        return content_command(args.check)
    failed = False
    for path in args.files:
        errors = validate(json.loads(path.read_text(encoding="utf-8")))
        for error in errors:
            print(f"{path}: {error}", file=sys.stderr)
        failed |= bool(errors)
        print(f"{path}: {'INVALID' if errors else 'ok'}")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
