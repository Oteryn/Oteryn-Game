#!/usr/bin/env python3
"""RAIDS-GLOBAL: reconcile pinned raid sources, staged evidence and native map bindings.

The staged apply-plan is checked byte for byte before any write. Crystal is the
primary executable source. The archived wiki identities and unresolved observations
remain in the catalogue; a missing binding or rule is an inactive reason, never a
replacement coordinate, creature, schedule or broadcast. Run `content --check` for
the focused validator; `content` also registers the family in the content manifests.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import sys
from pathlib import Path
from xml.parsers import expat

ROOT = Path(__file__).resolve().parents[2]
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
REPOSITORY = "zimbadev/crystalserver"
CAPTURE_SCHEMA = "OTERYN_CRYSTAL_RAID_SOURCE_CAPTURE/v1"
SCHEMA = "OTERYN_RAID_CATALOGUE/v1"
FAMILY = "Encounter.Raid"
FRAME = "global-target-2026-09-27"
CAPTURE = "imports/crystalserver/raids-201189f.json"
REGISTRY = "data-global/raids/raids.xml"
EVIDENCE_DIR = "docs/reference/content-prep/raid-content-1"
APPLY_PLAN = f"{EVIDENCE_DIR}/content-apply-plan.json"
INDEX = "content/encounters/raids/index.json"
CATALOGUE = "content/encounters/raids/catalogue.json"
EVIDENCE = "content/encounters/raids/evidence-00000-00121.json"
PLACEMENTS = "content/world/placements/index.json"
CITY_NAMES = {"ab'dendriel", "ankrahmun", "carlin", "darashia", "edron", "kazordoon",
              "liberty bay", "port hope", "rookgaard", "svargrond", "thais", "venore", "yalahar"}
MAX_CREATURES = 500  # Crystal yalahar/vampires.xml:6-12; RAIDS-GLOBAL 2026-11-01.
MAX_WAVES = 50  # Crystal oramond/walkers.xml:4-53, 50 separately scheduled spawns.


def encoded(value: object) -> bytes:
    return (json.dumps(value, indent=2, ensure_ascii=False, sort_keys=True) + "\n").encode()


def slug(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", value.lower()).strip("_")


def load(root: Path, path: str) -> dict:
    return json.loads((root / path).read_bytes())


def verified_apply_plan(root: Path) -> dict[str, bytes]:
    result = {}
    for row in load(root, APPLY_PLAN)["target_files"]:
        target, staged = row["target_path"], row["staged_path"]
        if (not target.startswith("content/encounters/raids/")
                or not staged.startswith(EVIDENCE_DIR + "/")
                or ".." in Path(target).parts or ".." in Path(staged).parts or target in result):
            raise ValueError("RAID_APPLY_PATH")
        raw = (root / staged).read_bytes()
        if hashlib.sha256(raw).hexdigest() != row["sha256"]:
            raise ValueError(f"RAID_APPLY_SHA256 {staged}")
        result[target] = raw
    return result


def verified_sources(snapshot: dict) -> dict[str, dict]:
    if (snapshot.get("schema") != CAPTURE_SCHEMA or snapshot.get("repository") != REPOSITORY
            or snapshot.get("revision") != REVISION):
        raise ValueError("RAID_SOURCE_REVISION")
    result = {}
    for row in snapshot["files"]:
        raw = row["text"].encode()
        if hashlib.sha256(raw).hexdigest() != row["sha256"]:
            raise ValueError(f"RAID_SOURCE_SHA256 {row['path']}")
        if hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest() != row["git_blob"]:
            raise ValueError(f"RAID_SOURCE_GIT_BLOB {row['path']}")
        if row["path"] in result:
            raise ValueError("RAID_SOURCE_DUPLICATE")
        result[row["path"]] = row
    if REGISTRY not in result:
        raise ValueError("RAID_SOURCE_REGISTRY")
    return result


def xml(text: str) -> dict:
    parser = expat.ParserCreate()
    stack, roots = [], []
    def start(tag, attributes):
        node = {"tag": tag, "attributes": attributes, "line": parser.CurrentLineNumber, "children": []}
        (stack[-1]["children"] if stack else roots).append(node)
        stack.append(node)
    parser.StartElementHandler = start
    parser.EndElementHandler = lambda _: stack.pop()
    parser.Parse(text, True)
    if len(roots) != 1:
        raise ValueError("RAID_XML_ROOT")
    return roots[0]


def citation(source: dict, line: int) -> dict:
    return {"repository": REPOSITORY, "revision": REVISION, "path": source["path"],
            "line": line, "sha256": source["sha256"], "evidence": "OTS_HYPOTHESIS_ONLY"}


def rectangle(attributes: dict) -> dict:
    if "x" in attributes:
        first = {"x": int(attributes["x"]), "y": int(attributes["y"]), "floor": int(attributes["z"])}
        last = dict(first)
    elif "radius" in attributes:
        radius = int(attributes["radius"])
        first = {"x": max(0, int(attributes["centerx"]) - radius),
                 "y": max(0, int(attributes["centery"]) - radius), "floor": int(attributes["centerz"])}
        last = {"x": min(65535, int(attributes["centerx"]) + radius),
                "y": min(65535, int(attributes["centery"]) + radius), "floor": first["floor"]}
    else:
        first = {"x": int(attributes["fromx"]), "y": int(attributes["fromy"]), "floor": int(attributes["fromz"])}
        last = {"x": int(attributes["tox"]), "y": int(attributes["toy"]), "floor": int(attributes["toz"])}
    # uniform_random sorts reversed bounds (Crystal src/utils/tools.cpp:431-439); this is the
    # same rectangle, not a relocated position or an inferred native coordinate.
    return {"from": {k: min(first[k], last[k]) for k in first},
            "to": {k: max(first[k], last[k]) for k in first}}


def area_anchor(area: dict) -> str:
    return "raid-anchor:" + "-".join(
        "_".join(str(area[end][key]) for key in ("x", "y", "floor")) for end in ("from", "to"))


def bind_area(area: dict, placements: dict) -> bool:
    area["coordinate_frame"] = placements.get("coordinate_frame")
    area["regions"] = []
    if placements.get("coordinate_frame") != FRAME or placements.get("region_size") != 256:
        return False
    low, high = area["from"], area["to"]
    if any(not 0 <= low[k] <= high[k] <= (15 if k == "floor" else 65535) for k in low):
        return False
    for region in placements["regions"]:
        match = re.search(r"region-z(\d+)-x(\d+)-y(\d+)\.b3$", region["path"])
        if not match:
            continue
        floor, x, y = map(int, match.groups())
        if (low["floor"] <= floor <= high["floor"] and low["x"] // 256 <= x <= high["x"] // 256
                and low["y"] // 256 <= y <= high["y"] // 256 and region["tiles"] > 0):
            area["regions"].append({"path": region["path"], "sha256": region["sha256"]})
    area["binding_source"] = PLACEMENTS
    area["transform_source"] = "tools/world-bundle-compiler/convert_spawns.py:88-104"
    return bool(area["regions"])


def source_bosses(sources: dict[str, dict]) -> dict[str, dict]:
    result = {}
    for path, source in sources.items():
        if not path.startswith("data-global/monster/"):
            continue
        name = re.search(r'Game.createMonsterType\("([^\"]+)"\)', source["text"])
        boss = re.search(r"monster\.bosstiary\s*=|rewardBoss\s*=\s*true", source["text"])
        if name and boss:
            result[slug(name.group(1))] = citation(source, source["text"].count("\n", 0, boss.start()) + 1)
    return result


def xml_variant(source: dict, creatures: dict[str, bool], wiki_bosses: set[str],
                donor_bosses: dict[str, dict] | None = None) -> dict:
    events = xml(source["text"])["children"]
    events = sorted(events, key=lambda e: int(e["attributes"].get("delay", 0)))
    announcements, waves, unknown = [], [], []
    previous_source, previous_actual = 0, 0
    for index, event in enumerate(events):
        attrs, tag = event["attributes"], event["tag"]
        if tag not in {"announce", "areaspawn", "singlespawn"}:
            unknown.append(f"unsupported_donor_event:{tag}:{source['path']}:{event['line']}")
            continue
        delay = max(1000, int(attrs["delay"]))
        actual = delay if index == 0 else previous_actual + max(1000, delay - previous_source)
        previous_source, previous_actual = delay, actual
        if tag == "announce":
            kind = {"warning": "warning", "event": "advance", "default": "advance"}.get(attrs.get("type", "event").lower())
            if kind is None:
                unknown.append(f"unsupported_message_type:{attrs['type']}")
                kind = "unknown"
            announcements.append({"delay_ms": actual, "raw_delay_ms": int(attrs["delay"]),
                                  "text": attrs["message"], "kind": kind, "source": citation(source, event["line"])})
            continue
        area = rectangle(attrs)
        entries = []
        for monster in ([event] if tag == "singlespawn" else event["children"]):
            values = monster["attributes"]
            creature = "oteryn:creature." + slug(values["name"])
            low = high = 1
            if tag == "areaspawn":
                low, high = int(values.get("minamount", 0)), int(values.get("maxamount", 0))
                if low == high == 0:
                    low = high = int(values["amount"])
            boss_source = (donor_bosses or {}).get(slug(values["name"]))
            entries.append({"creature": creature, "source_name": values["name"], "amount": {"min": low, "max": high},
                            "area_anchor": area_anchor(area), "area": copy.deepcopy(area),
                            "spawn_mode": "single" if tag == "singlespawn" else "area",
                            "boss": creatures.get(creature, False) or slug(values["name"]) in wiki_bosses or boss_source is not None,
                            "boss_source": boss_source,
                            "source": citation(source, monster["line"])})
        waves.append({"delay_ms": actual, "raw_delay_ms": int(attrs["delay"]), "entries": entries})
    return {"key": source["path"], "announcements": announcements, "waves": waves,
            "sources": [citation(source, 1)], "unresolved": unknown}


def raid_kind(record: dict) -> str:
    if record["schedule"]["kind"] == "calendar":
        return "calendar"
    if len(record["variants"]) > 1 or record.get("variant_group"):
        return "multi_variant"
    if record.get("boss_names") or any(entry["boss"] for v in record["variants"] for w in v["waves"] for entry in w["entries"]):
        return "boss"
    area = record.get("wiki_area", "").lower().removesuffix(" city").removesuffix(" centre")
    return "city" if area in CITY_NAMES else "area"


def qualify(record: dict, creatures: dict[str, bool], placements: dict) -> None:
    reasons = record["inactive_reasons"]
    if record["schedule"]["kind"] == "unknown":
        reasons.append("schedule_unknown")
    if not record["variants"] or any(not v["waves"] for v in record["variants"]):
        reasons.append("waves_unknown")
    for variant in record["variants"]:
        reasons.extend(variant.get("unresolved", []))
        if len(variant["waves"]) > MAX_WAVES:
            reasons.append(f"wave_limit:{len(variant['waves'])}>{MAX_WAVES}")
        requested = sum(e["amount"]["max"] for w in variant["waves"] for e in w["entries"])
        if requested > MAX_CREATURES:
            reasons.append(f"creature_limit:{requested}>{MAX_CREATURES}")
        for wave in variant["waves"]:
            for entry in wave["entries"]:
                if entry["creature"] not in creatures:
                    reasons.append("creature_unbound:" + entry["source_name"])
                if not bind_area(entry["area"], placements):
                    reasons.append("area_unbound:" + entry["area_anchor"])
                if not 0 < entry["amount"]["min"] <= entry["amount"]["max"] <= MAX_CREATURES:
                    reasons.append("amount_unknown_or_invalid:" + entry["source_name"])
    record["inactive_reasons"] = sorted(set(reasons))
    record["active"] = not record["inactive_reasons"]
    record["kind"] = raid_kind(record)


def creature_catalogue(root: Path) -> dict[str, bool]:
    result = {}
    for path in sorted((root / "content/creatures/definitions").glob("creatures-*.json")):
        for record in json.loads(path.read_bytes())["records"]:
            details = record["authoring"]["profile"]["details"]
            result[record["definition"]["identity"]["key"]] = (
                details.get("bosstiary") is not None or details["system_eligibility"]["reward_boss"])
    if not result:
        raise ValueError("RAID_CREATURE_CATALOGUE_EMPTY")
    return result


def normalized_messages(messages: list[str]) -> set[str]:
    return {" ".join(text.lower().split()) for text in messages if text}


def wiki_links(record: dict) -> list[str]:
    return sorted({row["source"] for fact in record["facts"].values() if isinstance(fact, dict)
                   for row in fact.get("sources", []) if row["source"].startswith("wiki:")})


def wiki_milliseconds(offset: str) -> int | None:
    """Read explicit hh:mm:ss; preserve ambiguous two-field 00:MM observations.

    The archived Ferumbras/Morgaroth clocks use nonzero MM:SS. Other two-field
    values such as 00:04 do not establish whether the unit was minutes/seconds.
    Their interpretation stays unresolved instead of silently multiplying by 60.
    """
    parts = offset.split(":")
    if not all(part.isdigit() for part in parts):
        return None
    numbers = list(map(int, parts))
    if not any(numbers):
        return 0
    if len(numbers) == 3 and all(number < 60 for number in numbers[1:]):
        return (numbers[0] * 3600 + numbers[1] * 60 + numbers[2]) * 1000
    if len(numbers) == 2 and numbers[0] > 0 and numbers[1] < 60:
        return (numbers[0] * 60 + numbers[1]) * 1000
    return None


def apply_global_timeline(variant: dict, timeline: list[dict], refs: list[dict]) -> bool:
    """Override exact known clocks only; never interpolate an unanchored wave.

    A source spawn at precisely an announcement's source timestamp retains that
    relationship when Global corrects the announcement clock. Source timestamps
    and citations remain alongside the resolved Global values for auditing.
    """
    known = [event for event in timeline if event.get("message")]
    announcements = variant["announcements"]
    times = [wiki_milliseconds(event["offset"]) for event in known]
    if len(known) != len(announcements) or not known or any(at is None for at in times):
        return False
    anchors = {event["raw_delay_ms"]: at for event, at in zip(announcements, times, strict=True)}
    if len(anchors) != len(announcements) or any(w["raw_delay_ms"] not in anchors for w in variant["waves"]):
        return False
    for event, observed, at in zip(announcements, known, times, strict=True):
        event["delay_ms"], event["text"] = at, observed["message"]
        event["global_sources"] = refs
    for wave in variant["waves"]:
        wave["delay_ms"] = anchors[wave["raw_delay_ms"]]
        wave["timing_basis"] = "source_same_timestamp_as_global_announcement"
        wave["global_sources"] = refs
    variant["timeline_basis"] = "global_exact_announcements_and_direct_source_wave_anchors"
    return True


def reconcile_global(record: dict, observed: dict) -> None:
    facts, variant = observed["facts"], record["variants"][0]
    record.setdefault("global_observations", []).append({"key": observed["key"], "facts": {
        key: facts[key] for key in ("broadcast_timeline", "schedule", "wave_count_observations", "wave_timing")
        if key in facts}})
    timeline = facts["broadcast_timeline"]["value"]
    known = [event for event in timeline if event.get("message")]
    announcements = variant["announcements"]
    refs = facts["broadcast_timeline"]["sources"]
    if known:
        source_messages = [" ".join(a["text"].lower().split()) for a in announcements]
        global_messages = [" ".join(a["message"].lower().split()) for a in known]
        # The exact unique boss identity resolves these donor spelling mistakes;
        # the Global source, rather than a string-similarity guess, supplies text.
        corrected_identity = record["key"] in {"raid:edron.orshabaal", "raid:libertybay.ferumbras"}
        if source_messages != global_messages and not corrected_identity:
            record["inactive_reasons"].append("global_broadcast_sequence_conflict:" + observed["key"])
        else:
            times = [wiki_milliseconds(event["offset"]) for event in known]
            if any(at is None for at in times):
                record["inactive_reasons"].append("global_broadcast_time_unit_unresolved:" + observed["key"])
            elif corrected_identity or any(max(1000, at) != event["raw_delay_ms"]
                                           for at, event in zip(times, announcements, strict=True)):
                if not apply_global_timeline(variant, timeline, refs):
                    record["inactive_reasons"].append("global_timeline_conflict_unanchored_source_waves:" + observed["key"])
    # These explicit Global observations contradict the source event counts or
    # require a separate native access/state owner. Approximate prose counts are
    # retained as evidence; they never become fabricated exact spawn entries.
    conflicts = {
        "raid:abdendriel.dragons": "global_counts_conflict:two_dragon_lords_per_floor_vs_donor_one",
        "raid:abdendriel.warwolves": "global_counts_conflict:about_70_wolves_35_warwolves_vs_donor_55_70",
        "raid:ankrahmun.gargoyles": "global_counts_conflict:about_70_gargoyles_vs_donor_15",
        "raid:ankrahmun.terramites": "global_waves_conflict:30_then_70_terramites_vs_donor_30_once",
        "raid:carlin.trolls": "global_counts_conflict:about_20_22_7_vs_donor_75_3_25",
        "raid:venore.hunters": "global_creatures_conflict:10_bears_15_hunters_vs_donor_15_hunters_only",
        "raid:darashia.thepalecount": "global_access_end_rule_unbound:15_minute_room_exit_and_access_gate",
        "raid:roshamuul.gaz_haragoth": "global_schedule_state_unbound:before_first_defeat_server_save_then_rare_prison_raid",
    }
    if record["key"] in conflicts:
        record["inactive_reasons"].append(conflicts[record["key"]])


def lua_flower(source: dict, placements: dict) -> dict:
    text = source["text"]
    amount = re.search(r"local DRYADS_PER_WAVE = (\d+)", text)
    interval = re.search(r"local WAVE_INTERVAL = (\d+) \* 60 \* 1000", text)
    pattern = (r'message = "([^\"]+)",\s*from = \{ x = (\d+), y = (\d+), z = (\d+) \},'
               r'\s*to = \{ x = (\d+), y = (\d+), z = (\d+) \}')
    matches = list(re.finditer(pattern, text))
    if amount is None or interval is None or len(matches) != 4:
        raise ValueError("RAID_FLOWER_SOURCE_SHAPE")
    areas = []
    for match in matches:
        area = {"from": dict(zip(("x", "y", "floor"), map(int, match.groups()[1:4]), strict=True)),
                "to": dict(zip(("x", "y", "floor"), map(int, match.groups()[4:7]), strict=True))}
        bind_area(area, placements)
        areas.append({"message": match.group(1), "kind": "advance", "area": area,
                      "source": citation(source, text.count("\n", 0, match.start()) + 1)})
    return {"requested_per_wave": int(amount.group(1)), "shared_attempt_budget": int(amount.group(1)) * 10,
            "interval_ms": int(interval.group(1)) * 60_000, "month": 6,
            "start": "startup", "repeat": "indefinite_four_area_cycle", "areas": areas,
            "creature": "oteryn:creature.dryad", "sources": [citation(source, 1), citation(source, 28), citation(source, 49)]}


def calendar_schedule(observation: dict) -> dict:
    # Inclusive World reset dates: the source end server-save belongs to the next
    # inactive reset date. Null times/frequency remain unknown, not midnight.
    result = {"kind": "calendar", "window": None, "interval_ms": None, "observations": observation}
    windows = {"October 31": (10, 31, 10, 31), "October 31 server save": (10, 31, 11, 2),
               "December 20 server save": (12, 20, 12, 27), "April 1 server save": (4, 1, 4, 14),
               "June": (6, 1, 6, 30)}
    label = observation.get("start", observation.get("date", observation.get("season")))
    if label in windows:
        result["window"] = dict(zip(("start_month", "start_day", "end_month", "end_day"), windows[label], strict=True))
    if "interval_hours" in observation:
        result["interval_ms"] = observation["interval_hours"] * 3_600_000
    return result


def lua_sugar(source: dict, creatures: dict[str, bool]) -> dict:
    text = source["text"]
    bounds = re.findall(r"Position\((\d+),\s*(\d+),\s*(\d+)\)", text)
    if len(bounds) != 2:
        raise ValueError("RAID_SUGAR_AREA")
    area = {end: dict(zip(("x", "y", "floor"), map(int, point), strict=True))
            for end, point in zip(("from", "to"), bounds, strict=True)}
    def number(name):
        match = re.search(rf"\b{name}\s*=\s*([\d.]+)", text)
        if not match:
            raise ValueError("RAID_SUGAR_CONFIG:" + name)
        return match.group(1)
    from decimal import Decimal
    days = re.search(r"allowedDays\s*=\s*\{([^}]+)\}", text)
    timeout = re.search(r'autoAdvance\("(\d+)h"\)', text)
    if days is None or timeout is None:
        raise ValueError("RAID_SUGAR_SCHEDULE")
    weekdays = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"]
    schedule = {"kind": "lua", "allowed_weekdays": [weekdays.index(day) + 1 for day in re.findall(r'"([^\"]+)"', days.group(1))],
                "min_active_players": int(number("minActivePlayers")), "check_interval_ms": 60_000,
                "active_idle_ms": 300_000, "checks_per_day": 1380, "min_gap_ms": 0, "max_checks_per_day": None,
                "initial_chance_milli_percent": int(Decimal(number("initialChance")) * 1000),
                "target_chance_per_day_milli_percent": int(Decimal(number("targetChancePerDay")) * 1000),
                "max_chance_per_check_milli_percent": int(Decimal(number("maxChancePerCheck")) * 1000)}
    creature = "oteryn:creature.sugar_mommy"
    return {"key": "raid:candia.sugarmommy", "name": "Sugar Mommy", "wiki_area": "Candia", "wiki_sources": [],
            "schedule": schedule, "variant_group": None, "inactive_reasons": [],
            "end": {"kind": "boss_death_or_timeout", "despawn_after_ms": int(timeout.group(1)) * 3_600_000},
            "sources": [citation(source, 1)], "wiki_records": [],
            "variants": [{"key": "candia.sugarmommy", "announcements": [], "unresolved": [],
                          "sources": [citation(source, 1)], "waves": [{"delay_ms": 0, "raw_delay_ms": 0,
                          "entries": [{"creature": creature, "source_name": "Sugar Mommy", "amount": {"min": 1, "max": 1},
                                       "area_anchor": area_anchor(area), "area": area, "spawn_mode": "zone", "boss": True, "source": citation(source, 14)}]}]}]}


def build_records(sources: dict[str, dict], evidence: dict, reconciliation: dict,
                  creatures: dict[str, bool], placements: dict) -> list[dict]:
    wiki = {r["key"]: r for r in evidence["records"]}
    wiki_bosses = {slug(name) for r in wiki.values() for name in r["facts"]["boss"]["value"]}
    donor_bosses = source_bosses(sources)
    variants = {path: xml_variant(source, creatures, wiki_bosses, donor_bosses) for path, source in sources.items()
                if path.startswith("data-global/raids/") and path.endswith(".xml") and path != REGISTRY}
    matched, candidates = set(), {}
    definitions = reconciliation["source_raid_definitions"]
    for match in reconciliation["records"]:
        if match["identification_basis"] != "FULL_BROADCAST_SET":
            continue
        for index in match["source_indexes"]:
            source = definitions[index]["source"]
            if source["repository"] == REPOSITORY and source["path"].startswith("data-global/"):
                candidates.setdefault(source["path"], []).append(match["key"])
    # Join unique exact named bosses so a conflicting donor definition cannot
    # evade its Global evidence by appearing as a separate, apparently complete
    # raid. This join identifies the subject, not proof of its wave composition.
    boss_identities, boss_definitions = {}, {}
    for key, observed in wiki.items():
        for name in observed["facts"]["boss"]["value"]:
            boss_identities.setdefault(slug(name), set()).add(key)
    for path, variant in variants.items():
        for wave in variant["waves"]:
            for entry in wave["entries"]:
                boss_definitions.setdefault(slug(entry["source_name"]), set()).add(path)
    exact_boss = {}
    for boss, identities in boss_identities.items():
        paths = boss_definitions.get(boss, set())
        if len(identities) == len(paths) == 1:
            path, identity = next(iter(paths)), next(iter(identities))
            exact_boss.setdefault(path, set()).add(identity)
            candidates.setdefault(path, []).append(identity)
    # The prompt expressly preserves the already admitted Badger identity.
    badger = "oteryn:raid.ab_dendriel_raids.badger_raid_in_ab_dendriel"
    if badger in wiki:
        candidates["data-global/raids/abdendriel/badgers.xml"] = [badger]
    registered = {"data-global/raids/" + row["attributes"]["file"]: (i, row)
                  for i, row in enumerate(xml(sources[REGISTRY]["text"])["children"])}
    absent = set(registered) - sources.keys()
    if absent:
        raise ValueError("RAID_REGISTRY_MISSING_FILES:" + ",".join(sorted(absent)))
    records = []
    for path, source in sorted(sources.items()):
        if not path.startswith("data-global/raids/") or not path.endswith(".xml") or path == REGISTRY:
            continue
        variant = variants[path]
        messages = normalized_messages([a["text"] for a in variant["announcements"]])
        identities = []
        for key in sorted(set(candidates.get(path, []))):
            known = normalized_messages([v["message"] for v in wiki[key]["facts"]["broadcast_timeline"]["value"]])
            if key == badger or key in exact_boss.get(path, set()) or (known and known <= messages):
                identities.append(key)
                matched.add(key)
        facts = wiki[identities[0]]["facts"] if identities else {}
        entry = registered.get(path)
        if entry:
            ordinal, node = entry
            attrs = node["attributes"]
            schedule = {"kind": "legacy_xml", "interval_s": int(attrs["interval2"]) * 60,
                        "margin_ms": int(attrs.get("margin", 0)) * 60_000,
                        "repeat": attrs.get("repeat", "false").lower() in {"true", "1", "yes"}, "source_order": ordinal}
            key = "raid:badger" if path.endswith("/abdendriel/badgers.xml") else "raid:" + re.sub(r"[^a-zA-Z0-9:._/-]", "_", attrs["name"])
            refs = [citation(source, 1), citation(sources[REGISTRY], node["line"])]
        else:
            key, schedule, refs = "raid:" + path.removeprefix("data-global/raids/").removesuffix(".xml").replace("/", "."), {"kind": "unknown"}, [citation(source, 1)]
        stem = Path(path).stem
        group = re.sub(r"\d+$", "", stem) if re.search(r"\d+$", stem) else None
        record = {"key": key, "name": facts.get("name", {}).get("value", entry[1]["attributes"]["name"] if entry else stem),
                  "wiki_area": facts.get("area", {}).get("value", ""),
                  "boss_names": facts.get("boss", {}).get("value", []),
                  "wiki_sources": sorted({s for identity in identities for s in wiki_links(wiki[identity])}),
                  "wiki_records": identities, "sources": refs, "schedule": schedule,
                  "variant_group": f"raid-group:{Path(path).parent.name}.{group}" if group else None,
                  "variants": [variant], "end": {"kind": "until_reset", "despawn_after_ms": None}, "inactive_reasons": []}
        for identity in identities:
            reconcile_global(record, wiki[identity])
        if key == "raid:abdendriel.chayenne":
            record["inactive_reasons"].append("global_raid_membership_unproven:donor_chayenne_has_no_archived_global_identity")
        qualify(record, creatures, placements)
        records.append(record)
    for key, observed in sorted(wiki.items()):
        if key in matched:
            continue
        facts = observed["facts"]
        observation = facts["schedule"]["value"]
        calendar = isinstance(observation, dict) and any(k in observation for k in ("start", "date", "season"))
        record = {"key": key, "name": facts["name"]["value"], "wiki_area": facts["area"]["value"],
                  "boss_names": facts["boss"]["value"],
                  "wiki_records": [key], "wiki_sources": wiki_links(observed), "sources": [],
                  "schedule": calendar_schedule(observation) if calendar else {"kind": "unknown"},
                  "variants": [], "variant_group": None, "end": {"kind": "unknown", "despawn_after_ms": None},
                  "inactive_reasons": ["wiki_donor_identity_unresolved", "native_wave_area_unknown", "message_kind_or_timing_unresolved"]}
        if calendar:
            record["inactive_reasons"].append("calendar_firing_rule_unresolved")
        if key.endswith("dryad_invasion"):
            source = sources["data-global/scripts/raids/world_changes/flower_month.lua"]
            record["sources"] = [citation(source, 1)]
            record["donor_cycle"] = lua_flower(source, placements)
            record["inactive_reasons"].remove("native_wave_area_unknown")
            if any(not entry["area"]["regions"] for entry in record["donor_cycle"]["areas"]):
                record["inactive_reasons"].append("donor_flower_area_unbound")
            record["inactive_reasons"].append("dryad_wave_conflict:wiki_five_waves_in_one_hour_vs_donor_indefinite_four_area_cycle")
        qualify(record, creatures, placements)
        records.append(record)
    sugar = lua_sugar(sources["data-global/scripts/raids/bosses/sugar_mommy.lua"], creatures)
    sugar["sources"].append(citation(sources["data/libs/systems/raids.lua"], 90))
    qualify(sugar, creatures, placements)
    records.append(sugar)
    records.sort(key=lambda r: r["key"])
    if len({r["key"] for r in records}) != len(records):
        raise ValueError("RAID_DUPLICATE_KEY")
    return records


def registered(project: dict, manifest: dict, lock: dict, count: int) -> tuple:
    project, manifest, lock = map(copy.deepcopy, (project, manifest, lock))
    if FAMILY not in project["migrated_families"]:
        project["migrated_families"].append(FAMILY)
    project["next_population_families"] = [f for f in project["next_population_families"] if f != FAMILY]
    manifest["families"][FAMILY] = {"records": count, "index": INDEX}
    paths = {row["path"] for row in manifest["managed_files"]} | {INDEX, CATALOGUE, EVIDENCE}
    manifest["managed_files"] = [{"path": path} for path in sorted(paths)]
    lock["family_counts"][FAMILY] = count
    return project, manifest, lock


def outputs(root: Path, *, register: bool = True) -> dict[str, bytes]:
    staged = verified_apply_plan(root)
    sources = verified_sources(load(root, CAPTURE))
    evidence = json.loads(staged[EVIDENCE])
    records = build_records(sources, evidence, load(root, f"{EVIDENCE_DIR}/raid-source-reconciliation.json"),
                            creature_catalogue(root), load(root, PLACEMENTS))
    totals = {kind: {"total": sum(r["kind"] == kind for r in records),
                     "active": sum(r["kind"] == kind and r["active"] for r in records)}
              for kind in ("city", "area", "boss", "multi_variant", "calendar")}
    document = {"schema": SCHEMA, "generator": "tools/content-migration/raid_catalogue.py",
                "source_revision": REVISION, "coordinate_frame": FRAME, "records": records, "totals": totals}
    result = {EVIDENCE: staged[EVIDENCE], CATALOGUE: encoded(document)}
    result[INDEX] = encoded({"schema": "OTERYN_FAMILY_INDEX/v1", "family": FAMILY,
                             "record_count": len(records), "active_count": sum(r["active"] for r in records),
                             "population_state": "POPULATED", "shards": [CATALOGUE], "evidence_shards": [EVIDENCE],
                             "evidence_record_count": len(evidence["records"]), "totals": totals,
                             "generator": "tools/content-migration/raid_catalogue.py",
                             "catalogue_sha256": hashlib.sha256(result[CATALOGUE]).hexdigest(),
                             "apply_plan_sha256": hashlib.sha256((root / APPLY_PLAN).read_bytes()).hexdigest(),
                             "source_capture_sha256": hashlib.sha256((root / CAPTURE).read_bytes()).hexdigest()})
    if register:
        names = ("project", "manifest", "content.lock")
        for name, value in zip(names, registered(*(load(root, f"content/{n}.json") for n in names), len(records)), strict=True):
            result[f"content/{name}.json"] = encoded(value)
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", nargs="?", choices=["content"], default="content")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--no-register", action="store_true")
    args = parser.parse_args()
    generated = outputs(args.root, register=not args.no_register)
    stale = [path for path, raw in generated.items() if not (args.root / path).is_file() or (args.root / path).read_bytes() != raw]
    if args.check:
        print("raid catalogue check: " + ("FAIL " + ", ".join(stale) if stale else "ok"))
        return int(bool(stale))
    for path, raw in generated.items():
        destination = args.root / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(raw)
    index = json.loads(generated[INDEX])
    print(f"raid catalogue: total={index['record_count']} active={index['active_count']} kinds={index['totals']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
