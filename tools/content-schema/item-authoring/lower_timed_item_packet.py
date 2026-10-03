"""Lower timed-item facts into the packet `apps/game-server/src/content/item_timed_promotion.rs` applies
(TIMED-CONTENT-1, TIMED-ITEM-0 brief and sections 4 and 6, TIMED-ITEM-0B sections 4 and 10.1).

Scope: rings, amulets and necklaces, soft-boots-like boots, torches and lamps. Exercise and training
weapons, helmets, quest objects and decoration are out of scope (counted in the report).

Source policy: TibiaWiki first (`imports/tibiawiki/facts/items-stats.json`, fields `charges` and
`duration`), then Canary `imports/canary/items-xml/items.xml` (OTS_HYPOTHESIS_ONLY) as the fallback and
the only source of transform pairs, stopduration and decayTo. Every row records its evidence class.
Wiki pages that disagree, unparseable values and anything not decidable go to the report, never into a
row; a field the wiki is silent on, not one it is unsure of, falls back to Canary.

Rows (one per field per Item): `charges.count`, `temporal.duration_ms`, `temporal.consumption_mode`
(`ON_EQUIP` for an equip-paired active form, `CONTINUOUS` for a lit torch or lamp; never `ON_USE`),
`temporal.stop_duration_while_unequipped`, `transform.use|equip|unequip|decay`. A decay target is the
`transform.decay` row (Canary has one `decayTo`; a `decayTo` of 0 decays to nothing and has no row).
`charges.show_count` is not a row: the Reference Item model has no field for it (reported).

Limits: TIMEDITEM0-RL-01 65,535 charges, TIMEDITEM0-RL-02 604,800,000 ms. A timed definition must be
non-stackable. Only Items in `content/items` with a source binding get rows.

`--check` rebuilds the packet in memory and fails on any byte difference.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import xml.etree.ElementTree as ET
from collections import Counter, defaultdict
from decimal import Decimal
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SNAPSHOT = ROOT / "imports" / "tibiawiki" / "facts" / "items-stats.json"
CANARY = ROOT / "imports" / "canary" / "items-xml" / "items.xml"
ITEM_INDEX = ROOT / "content" / "items" / "index.json"
SOURCE_BINDINGS = ROOT / "imports" / "crystalserver" / "bindings" / "items.json"
OUTPUT = (
    ROOT / "docs" / "agents" / "evidence" / "OTV2-20261003-timed-item-facts-v1.json"
)
COMPILER_PATH = "tools/content-schema/item-authoring/lower_timed_item_packet.py"
SCHEMA = "OTERYN_ITEM_TIMED_PROMOTION/v1"
ITEM_KEY = "oteryn:item.tibia.i{}"

MAX_CHARGES = 65_535  # TIMEDITEM0-RL-01
MAX_DURATION_MS = 604_800_000  # TIMEDITEM0-RL-02

UNSIGNED = re.compile(r"^[0-9]+$")
WIKI_DURATION = re.compile(r"^([0-9]+(?:\.[0-9]+)?) (second|minute|hour|day)s?$")
UNIT_MS = {"second": 1000, "minute": 60_000, "hour": 3_600_000, "day": 86_400_000}
WIKI_CATEGORIES = {"Rings", "Amulets and Necklaces", "Boots"}
CANARY_CATEGORIES = {"rings", "amulets and necklaces", "boots"}
CANARY_SLOTS = {"ring", "necklace", "feet"}
EXCLUDED_PRIMARYTYPES = {"exercise weapons", "training weapons"}
TIMED_KEYS = (
    "charges",
    "duration",
    "decayto",
    "stopduration",
    "transformequipto",
    "transformdeequipto",
    "transformonuse",
    "showcharges",
    "showduration",
)


def load_canary():
    """Canary item id -> (name, lower-cased attribute map); ranges expand to every id."""
    out = {}
    for item in ET.parse(CANARY).getroot().iter("item"):
        attrs = {
            node.get("key").lower(): node.get("value")
            for node in item.iter("attribute")
        }
        if item.get("id"):
            ids = [int(item.get("id"))]
        else:
            ids = range(int(item.get("fromid")), int(item.get("toid")) + 1)
        for item_id in ids:
            out[item_id] = (item.get("name") or "", attrs)
    return out


def content_items():
    """(content Item ids with a source binding, ids whose stackable flag is Known true)."""
    bindings = json.loads(SOURCE_BINDINGS.read_text(encoding="utf-8"))["bindings"]
    bound = {row["target"]["key"] for row in bindings}
    index = json.loads(ITEM_INDEX.read_text(encoding="utf-8"))
    ids, stackable = set(), set()
    for shard in index["shards"]:
        document = json.loads((ROOT / shard).read_text(encoding="utf-8"))
        for record in document["records"]:
            definition = record["definition"]
            key = definition["identity"]["key"]
            if key not in bound:
                continue
            item_id = int(key.rsplit(".i", 1)[1])
            ids.add(item_id)
            stack = definition.get("semantics", {}).get("stack", {})
            value = (
                stack.get("value", {}).get("stackable")
                if stack.get("state") == "KNOWN"
                else None
            )
            if value == {"state": "KNOWN", "value": True}:
                stackable.add(item_id)
    return ids, stackable


def wiki_unsigned(raw):
    if not UNSIGNED.match(raw):
        return None
    value = int(raw)
    return value if value >= 1 else None


def wiki_duration_ms(raw):
    match = WIKI_DURATION.match(raw)
    if not match:
        return None
    ms = Decimal(match.group(1)) * UNIT_MS[match.group(2)]
    return int(ms) if ms == ms.to_integral_value() and ms >= 1 else None


def wiki_facts(snapshot, report):
    """item id -> {"charges"|"duration": (value, [observations])}; disagreement is reported."""
    facts = defaultdict(dict)
    report["wiki_blocked"] = set()
    for record in snapshot["records"].values():
        for name, parse in (("charges", wiki_unsigned), ("duration", wiki_duration_ms)):
            seen = [
                (obs, obs["fields"][name])
                for obs in record["observations"]
                if name in obs["fields"]
            ]
            if not seen:
                continue
            values = {parse(raw) for _obs, raw in seen}
            if None in values:
                report["wiki_malformed"][name] += 1
                report["wiki_blocked"].add((record["item_id"], name))
                note(report, f"wiki_malformed:{name}", [record["item_id"], seen[0][1]])
            elif len(values) > 1:
                report["wiki_conflict"][name] += 1
                report["wiki_blocked"].add((record["item_id"], name))
                note(report, f"wiki_conflict:{name}", record["item_id"])
            else:
                facts[record["item_id"]][name] = (
                    values.pop(),
                    [obs for obs, _raw in seen],
                )
    return facts


def note(report, key, value, limit=8):
    if len(report["examples"][key]) < limit:
        report["examples"][key].append(value)


def wiki_source(observations):
    return [
        {
            "class": "TIBIAWIKI",
            "page_id": obs["page_id"],
            "revision_id": obs["revision_id"],
        }
        for obs in observations
    ]


def canary_source(item_id, attrs, names):
    return [
        {
            "class": "OTS_HYPOTHESIS_ONLY",
            "canary_item_id": item_id,
            "attributes": {name: attrs[name] for name in names if name in attrs},
        }
    ]


def canary_int(attrs, name):
    raw = attrs.get(name)
    return int(raw) if raw is not None and UNSIGNED.match(raw) else None


def categories(canary, wiki_primary):
    """Item ids in scope: ring, amulet, boots, torch and lamp forms."""
    scoped = set()
    light = {}
    for item_id, (name, attrs) in canary.items():
        primary = attrs.get("primarytype")
        if primary in EXCLUDED_PRIMARYTYPES:
            continue
        if primary in CANARY_CATEGORIES or attrs.get("slot") in CANARY_SLOTS:
            scoped.add(item_id)
        lowered = name.lower()
        if primary == "light sources" and ("torch" in lowered or "lamp" in lowered):
            light[item_id] = lowered
    light_names = set(light.values())
    for item_id, (name, attrs) in canary.items():
        lowered = name.lower()
        if item_id in light or (
            lowered.startswith("lit ") and lowered[4:] in light_names
        ):
            scoped.add(item_id)
    for item_id, kinds in wiki_primary.items():
        if kinds & WIKI_CATEGORIES:
            scoped.add(item_id)
    return scoped, set(light) | {
        i
        for i, (n, _a) in canary.items()
        if n.lower().startswith("lit ") and n.lower()[4:] in light_names
    }


def build(snapshot, canary, item_ids, stackable_ids):
    report = {
        "wiki_conflict": Counter(),
        "wiki_malformed": Counter(),
        "skipped": Counter(),
        "wiki_canary_disagree": Counter(),
        "out_of_scope_timed_canary_items": Counter(),
        "show_count_not_modelled": 0,
        "examples": defaultdict(list),
    }
    wiki = wiki_facts(snapshot, report)
    wiki_primary = defaultdict(set)
    for record in snapshot["records"].values():
        for obs in record["observations"]:
            if "primarytype" in obs["fields"]:
                wiki_primary[record["item_id"]].add(obs["fields"]["primarytype"])
    scoped, light_ids = categories(canary, wiki_primary)
    # active form -> the inactive form(s) that equip into it
    inactive_of = defaultdict(list)
    for item_id, (_name, attrs) in canary.items():
        target = canary_int(attrs, "transformequipto")
        if target:
            inactive_of[target].append(item_id)

    def skip(reason, item_id):
        report["skipped"][reason] += 1
        note(report, f"skipped:{reason}", item_id)

    timed = {}  # item id -> facts for an admitted timed definition
    for item_id in sorted(scoped & item_ids):
        _name, attrs = canary.get(item_id, ("", {}))
        if canary_int(attrs, "transformequipto") is not None:
            # An inactive form carries no timed value; its paired active form does (TIMED-ITEM-0 s4).
            continue
        charges = duration = None
        if "charges" in wiki.get(item_id, {}):
            value, observations = wiki[item_id]["charges"]
            charges = (value, wiki_source(observations))
        elif (item_id, "charges") in report["wiki_blocked"]:
            pass  # the wiki disagrees with itself: reported, never a row
        elif canary_int(attrs, "charges") is not None:
            charges = (
                canary_int(attrs, "charges"),
                canary_source(item_id, attrs, ("charges",)),
            )
        if (
            charges
            and "charges" in wiki.get(item_id, {})
            and canary_int(attrs, "charges") not in (None, charges[0])
        ):
            report["wiki_canary_disagree"]["charges"] += 1
            note(report, "wiki_canary_disagree:charges", item_id)
        canary_ms = canary_int(attrs, "duration")
        canary_ms = canary_ms * 1000 if canary_ms else None
        wiki_item = (
            item_id
            if "duration" in wiki.get(item_id, {})
            else next(
                (
                    i
                    for i in inactive_of.get(item_id, [])
                    if "duration" in wiki.get(i, {})
                ),
                None,
            )
        )
        if wiki_item is not None:
            value, observations = wiki[wiki_item]["duration"]
            duration = (value, wiki_source(observations))
            if canary_ms is not None and canary_ms != value:
                report["wiki_canary_disagree"]["duration"] += 1
                note(
                    report, "wiki_canary_disagree:duration", [item_id, value, canary_ms]
                )
        elif (item_id, "duration") in report["wiki_blocked"]:
            pass
        elif canary_ms is not None:
            duration = (canary_ms, canary_source(item_id, attrs, ("duration",)))
        if charges is None and duration is None:
            continue
        if item_id in stackable_ids or attrs.get("stackable") == "1":
            skip("STACKABLE_TIMED_DEFINITION", item_id)
            continue
        if charges and charges[0] > MAX_CHARGES:
            skip("CHARGES_ABOVE_RL_01", item_id)
            charges = None
        if duration and duration[0] > MAX_DURATION_MS:
            skip("DURATION_ABOVE_RL_02", item_id)
            duration = None
        mode = None
        if duration:
            paired = canary_int(attrs, "transformdeequipto") is not None
            if paired or attrs.get("stopduration") == "1":
                mode = "ON_EQUIP"
            elif item_id in light_ids:
                mode = "CONTINUOUS"
            else:
                skip("MODE_UNDETERMINED", item_id)
                duration = None
        if charges is None and duration is None:
            continue
        timed[item_id] = {
            "charges": charges,
            "duration": duration,
            "mode": mode,
            "attrs": attrs,
        }
    for item_id, (_name, attrs) in canary.items():
        if item_id in scoped or not any(key in attrs for key in TIMED_KEYS):
            continue
        if item_id in item_ids and ("duration" in attrs or "charges" in attrs):
            report["out_of_scope_timed_canary_items"][
                attrs.get("primarytype") or "-"
            ] += 1

    rows = []

    def add(item_id, field_path, typed_value, sources):
        rows.append(
            {
                "evidence": sources[0]["class"],
                "field_path": field_path,
                "item_key": ITEM_KEY.format(item_id),
                "sources": sources,
                "typed_value": typed_value,
            }
        )

    def transform(item_id, trigger, target, attrs, attr_name):
        if not target:
            return
        if target not in item_ids:
            skip(f"TRANSFORM_TARGET_NOT_AN_ITEM:{trigger}", item_id)
            return
        add(
            item_id,
            f"transform.{trigger}",
            {"kind": "ITEM_TARGET", "value": ITEM_KEY.format(target)},
            canary_source(item_id, attrs, (attr_name,)),
        )

    timed_ids = set(timed)
    for item_id in sorted(timed):
        facts = timed[item_id]
        attrs = facts["attrs"]
        if facts["charges"]:
            value, sources = facts["charges"]
            add(
                item_id, "charges.count", {"kind": "COUNT_U32", "value": value}, sources
            )
            if attrs.get("showcharges") == "1":
                report["show_count_not_modelled"] += 1
        if facts["duration"]:
            value, sources = facts["duration"]
            add(
                item_id,
                "temporal.duration_ms",
                {"kind": "MILLISECONDS", "value": value},
                sources,
            )
            evidence = canary_source(
                item_id,
                attrs,
                ("duration", "stopduration", "transformdeequipto", "transformequipto"),
            )
            add(
                item_id,
                "temporal.consumption_mode",
                {"kind": "CONSUMPTION_MODE", "value": facts["mode"]},
                evidence,
            )
            add(
                item_id,
                "temporal.stop_duration_while_unequipped",
                {"kind": "BOOL", "value": facts["mode"] == "ON_EQUIP"},
                evidence,
            )
        decay = canary_int(attrs, "decayto")
        if decay:
            if facts["duration"]:
                transform(item_id, "decay", decay, attrs, "decayto")
            else:
                skip("DECAY_WITHOUT_DURATION", item_id)
        transform(
            item_id,
            "unequip",
            canary_int(attrs, "transformdeequipto"),
            attrs,
            "transformdeequipto",
        )
        transform(
            item_id, "use", canary_int(attrs, "transformonuse"), attrs, "transformonuse"
        )
    # inactive forms: the equip transform into an admitted timed form
    for item_id in sorted(scoped & item_ids):
        attrs = canary.get(item_id, ("", {}))[1]
        target = canary_int(attrs, "transformequipto")
        if target and target in timed_ids:
            transform(item_id, "equip", target, attrs, "transformequipto")
        elif target and target not in timed_ids:
            skip("EQUIP_TARGET_NOT_TIMED", item_id)
        use = canary_int(attrs, "transformonuse")
        if use and use in timed_ids and item_id not in timed_ids:
            transform(item_id, "use", use, attrs, "transformonuse")
    rows.sort(key=lambda row: (row["item_key"], row["field_path"]))
    counts = Counter(row["field_path"] for row in rows)
    return rows, report, counts


def packet_bytes(snapshot, canary, item_ids, stackable_ids, compiler_sha256):
    rows, report, counts = build(snapshot, canary, item_ids, stackable_ids)
    packet = {
        "compiler": {"path": COMPILER_PATH, "sha256": compiler_sha256},
        "counts": {
            "fields": len(rows),
            "items": len({row["item_key"] for row in rows}),
            "by_field": dict(sorted(counts.items())),
        },
        "limits": {
            "TIMEDITEM0-RL-01": MAX_CHARGES,
            "TIMEDITEM0-RL-02": MAX_DURATION_MS,
        },
        "policy": {
            "precedence": "TIBIAWIKI_THEN_CANARY_HYPOTHESIS",
            "agreement": "ALL_PAGES_AGREE_ELSE_REPORT",
            "canary_duration_unit": "SECONDS",
            "consumption_mode": "ON_EQUIP_PAIRED_OR_STOPDURATION_ELSE_CONTINUOUS_LIGHT_SOURCE",
        },
        "promotions": rows,
        "report": {
            "wiki_conflict": dict(sorted(report["wiki_conflict"].items())),
            "wiki_malformed": dict(sorted(report["wiki_malformed"].items())),
            "wiki_canary_disagree": dict(
                sorted(report["wiki_canary_disagree"].items())
            ),
            "skipped": dict(sorted(report["skipped"].items())),
            "out_of_scope_timed_canary_items": dict(
                sorted(report["out_of_scope_timed_canary_items"].items())
            ),
            "show_count_not_modelled": report["show_count_not_modelled"],
            "examples": {k: v for k, v in sorted(report["examples"].items())},
        },
        "schema": SCHEMA,
        "source": {
            "canary": {
                "path": "imports/canary/items-xml/items.xml",
                "sha256": hashlib.sha256(CANARY.read_bytes()).hexdigest(),
            },
            "tibiawiki": {
                "batch_id": snapshot["batch_id"],
                "path": "imports/tibiawiki/facts/items-stats.json",
                "snapshot_sha256": snapshot["snapshot_sha256"],
            },
        },
    }
    return (
        json.dumps(packet, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args(argv)
    snapshot = json.loads(SNAPSHOT.read_text(encoding="utf-8"))
    compiler_sha256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    item_ids, stackable_ids = content_items()
    data = packet_bytes(
        snapshot, load_canary(), item_ids, stackable_ids, compiler_sha256
    )
    if args.check:
        if args.output.read_bytes() != data:
            print(f"packet drift against {args.output}", file=sys.stderr)
            return 1
        print(json.dumps({"check": "PASS", "bytes": len(data)}))
        return 0
    args.output.write_bytes(data)
    packet = json.loads(data)
    print(
        json.dumps(
            {"counts": packet["counts"], "report": packet["report"], "bytes": len(data)}
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
