"""Lower the EQUIP-CONTENT-1 Equipment ability record (EQUIP-0 §3.1-§3.2, architect bundle §2.9).

The six abilities are a derived typed view over the Item `skill_modifiers` and `protection`
groups (`apps/game-server/src/content/item_abilities.rs`; control-plane ruling a: no codec or
schema change). TibiaWiki comes first, through the committed stats packet that
`lower_wiki_stats_packet.py` writes. Where every wiki page of an Item is silent on a group, this
tool emits a Canary `items.xml` fallback row (D384 pin, OTS_HYPOTHESIS_ONLY), and the materializer
writes it only into an Unknown leaf. Canary speed is already in displayed units: it agrees 1:1
with the wiki on every Item that states both, and any disagreement fails the run. An Item whose
Canary group holds a key outside the mapped abilities is held, because a Known list never drops
an observed fact. STAT_BOOST and LIGHT have no source in either input.

The record lists every source per Item and the derived `timed` flag (`charges.count` or
`temporal.duration` Known), which the materializer re-derives and compares. It is a separate tool
so that the stats packet, whose bytes embed its compiler digest, stays unchanged.

`--check` rebuilds the record in memory and fails on any byte difference.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path

from lower_wiki_stats_packet import (
    INTEGER,
    ITEM_KEY,
    MODIFIER_PERCENTS,
    OUTPUT,
    RESISTANCE_KINDS,
    ROOT,
    SNAPSHOT,
    content_item_ids,
    known,
    physical_field_inputs,
)

ABILITIES = (
    ROOT / "docs" / "agents" / "evidence" / "OTV2-20261003-equip-abilities-v1.json"
)
ABILITIES_SCHEMA = "OTERYN_EQUIP_ABILITIES/v1"
CANARY = ROOT / "imports" / "canary" / "items-xml" / "items.xml"
CANARY_MANIFEST = ROOT / "imports" / "canary" / "items-xml" / "manifest.json"
# Every wiki parameter the stats lowering reads into each group: one present means not silent.
WIKI_MODIFIER_FIELDS = ("attrib", *MODIFIER_PERCENTS, "mantra", "elementalbond")
WIKI_RESISTANCE_FIELDS = ("resist",)
CANARY_MODIFIER_POINTS = {
    "magiclevelpoints": "MAGIC_LEVEL_POINTS",
    "skillaxe": "SKILL_AXE",
    "skillclub": "SKILL_CLUB",
    "skilldist": "SKILL_DISTANCE",
    "skillfist": "SKILL_FIST",
    "skillshield": "SKILL_SHIELD",
    "skillsword": "SKILL_SWORD",
    "speed": "SPEED",
}
CANARY_SUPPRESSIONS = {
    "suppressdrown": "SUPPRESS_DROWN",
    "suppressdrunk": "SUPPRESS_DRUNK",
}
# Canary parses `absorbpercentpoison` and `absorbpercentearth` as the same earth damage.
CANARY_ABSORB = {
    "absorbpercentdeath": "DEATH",
    "absorbpercentdrown": "DROWN",
    "absorbpercentearth": "EARTH",
    "absorbpercentenergy": "ENERGY",
    "absorbpercentfire": "FIRE",
    "absorbpercentholy": "HOLY",
    "absorbpercentice": "ICE",
    "absorbpercentlifedrain": "LIFE_DRAIN",
    "absorbpercentmanadrain": "MANA_DRAIN",
    "absorbpercentphysical": "PHYSICAL",
    "absorbpercentpoison": "EARTH",
    "fieldabsorbpercentfire": "FIRE_FIELD",
}
# Other top-level Canary keys of the same two groups. A Known list never drops an observed fact
# of its group, so an Item carrying one of these is held, not written.
CANARY_MODIFIER_OTHER = re.compile(
    r"^(?:skill|suppress|speed|magicshield|manashield$|invisible$|criticalhit|reflect"
    r"|perfectshot|cleave|lifeleech|manaleech|(?:health|mana)(?:gain|ticks)$)"
    r"|magiclevelpoints$|^mantra$|^elementalbond$"
)
CANARY_ABSORB_OTHER = re.compile(r"absorbpercent")
NO_SOURCE = (
    "no source in the TibiaWiki stats snapshot nor in Canary items.xml (D384 pin); "
    "a follow-up question for EQUIP-RT-1"
)


def load_canary_top_level():
    """Canary Item id -> top-level attribute map (lower-cased keys); ranges expand.

    Nested attributes (imbuement slots, script requirements) are not Item abilities.
    """
    import xml.etree.ElementTree as ET

    out = {}
    for item in ET.parse(CANARY).getroot().iter("item"):
        attrs = {
            node.get("key").lower(): node.get("value")
            for node in item.findall("attribute")
        }
        if item.get("id"):
            ids = [int(item.get("id"))]
        else:
            ids = range(int(item.get("fromid")), int(item.get("toid")) + 1)
        for item_id in ids:
            out[item_id] = attrs
    return out


def canary_pin():
    manifest = json.loads(CANARY_MANIFEST.read_text(encoding="utf-8"))
    pinned = next(row for row in manifest["files"] if row["path"] == "items.xml")
    actual = hashlib.sha256(CANARY.read_bytes()).hexdigest()
    if actual != pinned["sha256"]:
        raise ValueError("Canary items.xml digest differs from its D384 pin")
    return {
        "path": str(CANARY.relative_to(ROOT)),
        "sha256": actual,
        "revision": manifest["source"]["revision"],
        "evidence": "OTS_HYPOTHESIS_ONLY",
    }


def canary_points(raw):
    if raw is None or not INTEGER.match(raw):
        return None
    value = int(raw)
    return value if value != 0 and -(2**31) <= value < 2**31 else None


def canary_modifiers(attrs):
    """(observed keys, typed modifier list | "MALFORMED" | "UNMAPPED" | None)."""
    mapped = {
        k: attrs[k]
        for k in (*CANARY_MODIFIER_POINTS, *CANARY_SUPPRESSIONS)
        if k in attrs
    }
    other = sorted(
        k
        for k in attrs
        if CANARY_MODIFIER_OTHER.search(k)
        and k not in CANARY_MODIFIER_POINTS
        and k not in CANARY_SUPPRESSIONS
    )
    if not mapped and not other:
        return None, None
    observed = {k: attrs[k] for k in sorted({*mapped, *other})}
    if other:
        return observed, "UNMAPPED"
    values = {}
    for key, kind in CANARY_MODIFIER_POINTS.items():
        if key in mapped:
            points = canary_points(mapped[key])
            if points is None:
                return observed, "MALFORMED"
            # The pinned items.xml states speed in displayed units (ability_sources speed_unit).
            values[kind] = {"kind": "SIGNED_POINTS", "value": points}
    for key, kind in CANARY_SUPPRESSIONS.items():
        if key in mapped:
            if mapped[key] != "1":
                return observed, "MALFORMED"
            values[kind] = {"kind": "BOOLEAN", "value": True}
    unknown = {"state": "UNKNOWN"}
    return observed, [
        {
            "evaluation_phase": unknown,
            "kind": kind,
            "parameter": known(values[kind]),
            "priority": unknown,
            "target_domain": unknown,
        }
        for kind in sorted(values)
    ]


def canary_resistances(attrs):
    """(observed keys, typed resistance list | "MALFORMED" | "UNMAPPED" | None)."""
    observed = {k: attrs[k] for k in sorted(attrs) if CANARY_ABSORB_OTHER.search(k)}
    if not observed:
        return None, None
    if any(k not in CANARY_ABSORB for k in observed):
        return observed, "UNMAPPED"
    values = {}
    for key, raw in observed.items():
        kind = CANARY_ABSORB[key]
        if raw is None or not INTEGER.match(raw) or not -100 <= int(raw) <= 100:
            return observed, "MALFORMED"
        if int(raw) == 0 or values.get(kind, int(raw)) != int(raw):
            return observed, "MALFORMED"
        values[kind] = int(raw)
    return observed, [
        {
            "kind": kind,
            "percent": known({"denominator": 1, "numerator": values[kind]}),
        }
        for kind in RESISTANCE_KINDS
        if kind in values
    ]


def wiki_speed(fields):
    match = re.search(
        r"(?:^|,)\s*speed ([+-]?[0-9]+)\s*(?:,|$)", fields.get("attrib", "").lower()
    )
    return int(match[1]) if match else None


def timed(definition):
    """EQUIP-0 §3.2: `charges.count` or `temporal.duration` present."""
    semantics = definition.get("semantics", {})
    for group, leaf in (("charges", "count"), ("temporal", "duration")):
        source = semantics.get(group, {})
        if source.get("state") == "KNOWN":
            if source["value"].get(leaf, {}).get("state") == "KNOWN":
                return True
    return False


def ability_sources(snapshot, canary, item_ids, definitions):
    """The EQUIP-CONTENT-1 record: every source per Item, the timed flag and fallback rows."""
    wiki = {
        record["item_id"]: record["observations"]
        for record in snapshot["records"].values()
    }
    items, holds, speed_pairs = [], [], []
    for item_id in sorted(item_ids):
        key = ITEM_KEY.format(item_id)
        observations = wiki.get(item_id, [])
        attrs = canary.get(item_id, {})
        groups = (
            (
                "skill_modifiers.modifiers",
                WIKI_MODIFIER_FIELDS,
                canary_modifiers(attrs),
            ),
            (
                "protection.resistances",
                WIKI_RESISTANCE_FIELDS,
                canary_resistances(attrs),
            ),
        )
        wiki_sources = [
            {
                "class": "TIBIAWIKI",
                "page_id": obs["page_id"],
                "revision_id": obs["revision_id"],
                "values": {
                    name: obs["fields"][name]
                    for name in (*WIKI_MODIFIER_FIELDS, *WIKI_RESISTANCE_FIELDS)
                    if name in obs["fields"]
                },
            }
            for obs in observations
            if any(
                name in obs["fields"]
                for name in (*WIKI_MODIFIER_FIELDS, *WIKI_RESISTANCE_FIELDS)
            )
        ]
        canary_observed = {}
        fallback = []
        for field_path, wiki_fields, (observed, typed) in groups:
            if observed is None:
                continue
            canary_observed |= observed
            if any(
                name in obs["fields"] for obs in observations for name in wiki_fields
            ):
                continue
            reason = None
            if "semantics" not in definitions.get(key, {}):
                reason = "NO_CANONICAL_ITEM_SEMANTICS"
            elif typed in ("MALFORMED", "UNMAPPED"):
                reason = f"{typed}_CANARY_VALUE"
            if reason:
                holds.append(
                    {
                        "item_key": key,
                        "field_path": field_path,
                        "reason": reason,
                        "canary_attributes": observed,
                    }
                )
                continue
            fallback.append({"field_path": field_path, "value": typed})
        for obs in observations:
            speed = wiki_speed(obs["fields"])
            if speed is not None and "speed" in attrs:
                speed_pairs.append((key, speed, attrs["speed"]))
        if not wiki_sources and not canary_observed:
            continue
        if "semantics" not in definitions.get(key, {}):
            continue
        sources = list(wiki_sources)
        if canary_observed:
            sources.append(
                {
                    "class": "OTS_HYPOTHESIS_ONLY",
                    "canary_item_id": item_id,
                    "attributes": canary_observed,
                }
            )
        items.append(
            {
                "item_key": key,
                "timed": timed(definitions[key]),
                "sources": sources,
                "fallback": fallback,
            }
        )
    # EQUIP-0 §2 expected Canary speed in a doubled unit; the pinned items.xml agrees 1:1 with
    # the wiki on every Item that has both, so the conversion factor is 1. Any other pair fails.
    disagree = [pair for pair in speed_pairs if str(pair[1]) != pair[2].lstrip("+")]
    if disagree or not speed_pairs:
        raise ValueError(f"Canary speed unit is not the displayed unit: {disagree[:5]}")
    return items, holds, len(speed_pairs)


def abilities_bytes(snapshot, item_ids, definitions, packet_data, canary=None):
    """The record bytes; `packet_data` is the committed stats packet the sources sit beside."""
    canary = load_canary_top_level() if canary is None else canary
    items, holds, speed_pairs = ability_sources(snapshot, canary, item_ids, definitions)
    by_field = Counter(row["field_path"] for item in items for row in item["fallback"])
    return (
        json.dumps(
            {
                "schema": ABILITIES_SCHEMA,
                "policy": {
                    "precedence": "TIBIAWIKI_THEN_CANARY_WHERE_EVERY_WIKI_PAGE_IS_SILENT_ON_THE_GROUP",
                    "derived_view": "apps/game-server/src/content/item_abilities.rs",
                    "speed_unit": {
                        "canary_to_displayed": "1:1",
                        "agreeing_wiki_canary_items": speed_pairs,
                    },
                    "timed": "charges.count or temporal.duration KNOWN (EQUIP-0 §3.2)",
                    "abilities_without_source": {
                        "LIGHT": NO_SOURCE,
                        "STAT_BOOST": NO_SOURCE,
                    },
                },
                "source": {
                    "canary": canary_pin(),
                    "stats_packet": {
                        "path": str(OUTPUT.relative_to(ROOT)),
                        "sha256": hashlib.sha256(packet_data).hexdigest(),
                    },
                    "wiki_snapshot_sha256": snapshot["snapshot_sha256"],
                },
                "counts": {
                    "items": len(items),
                    "timed_items": sum(item["timed"] for item in items),
                    "fallback_items": sum(bool(item["fallback"]) for item in items),
                    "fallback_by_field": dict(sorted(by_field.items())),
                    "holds": len(holds),
                },
                "items": items,
                "holds": holds,
            },
            sort_keys=True,
            ensure_ascii=False,
            indent=1,
        )
        + "\n"
    ).encode("utf-8")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--output", type=Path, default=ABILITIES)
    args = parser.parse_args(argv)
    snapshot = json.loads(SNAPSHOT.read_text(encoding="utf-8"))
    data = abilities_bytes(
        snapshot, content_item_ids(), physical_field_inputs()[0], OUTPUT.read_bytes()
    )
    if args.check:
        if args.output.read_bytes() != data:
            print(f"record drift against {args.output}", file=sys.stderr)
            return 1
        print(json.dumps({"check": "PASS", "bytes": len(data)}))
        return 0
    args.output.write_bytes(data)
    print(json.dumps({"counts": json.loads(data)["counts"], "bytes": len(data)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
