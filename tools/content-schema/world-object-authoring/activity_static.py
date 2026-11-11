"""Publish bounded pinned activity-map static geometry without new inventory Item aliases."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
from functools import lru_cache
from pathlib import Path

import world_objects

ROOT = Path(__file__).resolve().parents[3]
IDENTIFIERS = frozenset(
    [
        20661,
        33067,
        33068,
        34009,
        39095,
        23467,
        23468,
        33017,
        42852,
        42941,
        42944,
        42947,
        42949,
        42950,
        42952,
        42958,
        42961,
        42973,
        42974,
        42975,
        42976,
        42977,
        42978,
        42979,
        42981,
        42987,
        42991,
        42995,
        42997,
        42370,
        42371,
        42374,
        42376,
        42377,
        42380,
        42382,
        42385,
        42654,
        42655,
        42656,
        42658,
        42659,
        42660,
        42665,
        42666,
        42667,
        42753,
        42856,
        42858,
        42859,
        42865,
        43358,
        43359,
        43360,
        43361,
        43362,
        43363,
        43364,
        43365,
        43366,
        33066,
        33069,
        10649,
        10650,
        10653,
        10654,
        10728,
        10730,
        10735,
        10736,
        10737,
        10740,
        10747,
        33070,
        25181,
        42940,
        42982,
        42983,
        43308,
        43309,
        43311,
        43312,
        43313,
        43314,
        43315,
        43316,
        43317,
        32898,
        42379,
        42651,
        42653,
        43310,
        42662,
        42663,
        42664,
        42680,
        42681,
        42863,
        42864,
        43089,
        43092,
        11075,
        11077,
        11079,
        11102,
        11103,
        11104,
        11105,
        34943,
        34946,
        34951,
        34953,
        42880,
        42887,
        42892,
        42893,
        42894,
        42895,
        42896,
        42897,
        42898,
        42899,
        42900,
        42901,
        42902,
        42910,
        42923,
        42933,
        43516,
        46629,
        46630,
        46631,
        46632,
        46633,
        46634,
        46635,
        46636,
        46637,
        46638,
        46639,
        46640,
        46641,
        46643,
        46644,
        46645,
        46646,
        48236,
        32906,
        43497,
        43498,
        43507,
        43508,
        43983,
        43984,
        43985,
        43986,
        43987,
        43988,
        43989,
        42671,
        23049,
        33276,
    ]
)
# Actual M25 room/waiting/exit OTBM closure: all76 source policies are immutable.
M25_IDENTIFIERS = frozenset(
    [
        34628,
        40242,
        40251,
        40252,
        40282,
        40309,
        40310,
        40311,
        40312,
        40313,
        40314,
        40315,
        40316,
        40317,
        40318,
        40319,
        40320,
        40321,
        40322,
        40323,
        40329,
        40332,
        40333,
        40334,
        40337,
        40338,
        40339,
        40340,
        40341,
        40342,
        40343,
        40344,
        40345,
        40346,
        40347,
        40348,
        40351,
        40352,
        40353,
        40360,
        40361,
        40387,
        40388,
        40391,
        40393,
        40397,
        40400,
        40405,
        40406,
        40414,
        40437,
        40439,
        40449,
        40454,
        40456,
        40471,
        40472,
        40473,
        40474,
        40477,
        40478,
        40479,
        40480,
        40492,
        44481,
        44482,
        44483,
        44485,
        44489,
        44491,
        44493,
        44494,
        44498,
        44499,
        44500,
        44502,
    ]
)
IDENTIFIERS = IDENTIFIERS | M25_IDENTIFIERS
OFFICIAL_SHA = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
SHARDS = {
    "Terrain": "content/world/terrain/terrain-08669-08720.json",
    "WorldObject": "content/world/objects/objects-12932-13108.json",
}
REGISTRY = "content/world/source-static-definitions.json"
NOTE = " Additionally SRX-3 pinned activity static geometry: 61 Terrain and 177 WorldObject records; no Item aliases."


def encode(value):
    return (
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    ).encode()


def compose_placement_palette(placement, activity_registry, soulpit_rows, root=ROOT):
    """Retain private Activity identities and canonical public palette precedence."""
    soulpit = {row["provenance"]["source_item_id"]: row for row in soulpit_rows}
    family_aliases = 0
    for entry in activity_registry:
        incoming = soulpit.get(entry["source_item_id"])
        if incoming is None or incoming["identity"] == entry["identity"]:
            continue
        policy = entry["policy"]
        record = entry["catalogue_record"]
        if (
            entry["identity"]["family"] != "WorldObject"
            or incoming["identity"]["family"] != "Terrain"
            or policy["ground"] is not False
            or incoming["walkable"]
            != {"state": "KNOWN", "value": not policy["block_solid"]}
            or incoming["kind"]
            not in (
                {"state": "KNOWN", "value": "common"},
                {"state": "KNOWN", "value": "border"},
            )
            or record["collision"]["blocks_movement"]
            != {"state": "KNOWN", "value": policy["block_solid"]}
            or incoming["client_projection"] != record["client_projection"]
        ):
            raise ValueError(
                "joint static palette family physics conflict:"
                + str(entry["source_item_id"])
            )
        family_aliases += 1
    if family_aliases != 39:
        raise ValueError("joint static palette family set changed")
    by_id = {row["source_item_id"]: row["identity"] for row in activity_registry}
    expected_soulpit = set(soulpit)
    # The public World palette follows ruling162 Q1b: Item, Terrain,
    # WorldObject. Private Activity source records retain their own identities.
    by_id.update(
        {row["provenance"]["source_item_id"]: row["identity"] for row in soulpit_rows}
    )
    item_keys = {
        row["definition"]["identity"]["key"]
        for shard in json.loads((root / "content/items/index.json").read_bytes())[
            "shards"
        ]
        for row in json.loads((root / shard).read_bytes())["records"]
    }
    for binding in json.loads(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )["bindings"]:
        if binding["disposition"] == "EXACT" and binding["target"]["key"] in item_keys:
            by_id[int(binding["external_id"])] = binding["target"]
    selected = [
        row["source_item_id"]
        for row in placement["palette"]
        if row["source_item_id"] in expected_soulpit
    ]
    if len(selected) != len(expected_soulpit) or set(selected) != expected_soulpit:
        raise ValueError("SOULPIT_STATIC_PALETTE_SET")
    for row in placement["palette"]:
        identity = by_id.get(row["source_item_id"])
        if identity is not None:
            row.update(key=identity["key"], provisional=False)


def compose_terrain_notes(notes):
    """Both source publishers use the same two exact, idempotent suffixes."""
    import soulpit_static

    notes = re.sub(
        r" Additionally SRX-3 pinned activity static geometry: [0-9]+ Terrain and [0-9]+ WorldObject records; no Item aliases\.",
        "",
        notes,
    )
    return notes.replace(soulpit_static.NOTE, "") + NOTE + soulpit_static.NOTE


def preserved_restore_item_pointer(root=ROOT):
    """The existing public 23049 Item route shares its canonical Terrain body."""
    identity = {
        "family": "Item",
        "key": "oteryn:item.tibia.i23049",
        "revision": "definition-r1",
    }
    bindings = [
        row
        for row in json.loads(
            (root / "imports/crystalserver/bindings/items.json").read_bytes()
        )["bindings"]
        if row["identity_namespace"] == "ots/item_server_id"
        and row["external_id"] == "23049"
    ]
    if len(bindings) != 1 or bindings[0] != {
        "disposition": "EXACT",
        "external_id": "23049",
        "identity_namespace": "ots/item_server_id",
        "source_key": "oteryn:source.crystalserver",
        "source_revision": "ff7ede593c69d4c658b382c97443e8155926924a",
        "target": identity,
    }:
        raise ValueError("Anomaly restore lost its existing public Item binding")
    definitions = [
        row["definition"]
        for shard in json.loads((root / "content/items/index.json").read_bytes())[
            "shards"
        ]
        for row in json.loads((root / shard).read_bytes())["records"]
        if row["definition"]["identity"]["key"] == identity["key"]
    ]
    if (
        len(definitions) != 1
        or definitions[0]["identity"] != identity
        or definitions[0].get("materializable") is not False
    ):
        raise ValueError("Anomaly restore requires its existing nonmaterializable Item")
    return identity


@lru_cache(maxsize=4)
def source_records(root=ROOT):
    """Pure source qualification, independent of joint catalogue publication."""
    restore_pointer = preserved_restore_item_pointer(root)
    policies = {}
    captures = []
    for path in sorted((root / "content/encounters").glob("*/source-room.json")):
        document = json.loads(path.read_bytes())
        if document["source_revision"] != PIN:
            raise ValueError("activity source revision")
        captures.append(document)
        for policy in document["item_policies"]:
            identifier = policy["server_item_id"]
            if identifier not in IDENTIFIERS:
                continue
            if (
                policy["movable"]
                or policy["pickupable"]
                or policy["dynamic_kind"] is not None
            ):
                raise ValueError("static scene source has mutable Item behavior")
            if identifier in policies and policies[identifier] != policy:
                raise ValueError("inconsistent pinned source static policy")
            policies[identifier] = policy
    # These three literal replacement maps have their own source schema. Only
    # source-proven static decoration joins this publisher; no mutable Item is
    # inferred from a map stack or an appearance.
    ebb_root = root / "content/encounters/soul-war-ebb-source-maps"
    for name in ("ebb-flow-empty.json", "ebb-flow-inundate.json", "ebb-flow.json"):
        document = json.loads((ebb_root / name).read_bytes())
        if (
            document["schema"] != "OTERYN_SOUL_WAR_EBB_SOURCE_MAP/v1"
            or document["source_revision"] != PIN
        ):
            raise ValueError("Ebb static source revision")
        for policy in document["item_policies"]:
            identifier = policy["server_item_id"]
            if identifier != 33276:
                continue
            if (
                policy["movable"]
                or policy["pickupable"]
                or policy["dynamic_kind"] is not None
            ):
                raise ValueError("Ebb decoration has mutable Item behavior")
            if identifier in policies and policies[identifier] != policy:
                raise ValueError("inconsistent Ebb static policy")
            policies[identifier] = policy
    if policies.keys() != IDENTIFIERS:
        raise ValueError("closed static geometry census")
    raw = (root / f"content/assets/files/appearances-{OFFICIAL_SHA}.dat").read_bytes()
    if hashlib.sha256(raw).hexdigest() != OFFICIAL_SHA:
        raise ValueError("official appearance artifact digest")
    appearances = world_objects.engine_items.load_appearance_objects(raw)
    admitted = json.loads(
        (root / "imports/official/appearance-membership/admitted.json").read_bytes()
    )
    membership = next(
        row for row in admitted["files"] if row["label"] == "client-15.30"
    )
    members_raw = (
        root / "imports/official/appearance-membership" / membership["manifest"]
    ).read_bytes()
    if (
        hashlib.sha256(members_raw).hexdigest() != membership["manifest_sha256"]
        or membership["appearances_sha256"] != OFFICIAL_SHA
    ):
        raise ValueError("official membership digest")
    members = {row[0] for row in json.loads(members_raw)["entries"]}
    source = {
        "engine": "crystal",
        "profile": "OTERYN_ACTIVITY_STATIC_GEOMETRY_201189F/v1",
        "repository": "zimbadev/crystalserver",
        "revision": PIN,
    }
    records = {"Terrain": [], "WorldObject": []}
    registry = []
    for identifier, policy in sorted(policies.items()):
        if identifier not in members or identifier not in appearances:
            raise ValueError(
                "activity static geometry lacks official appearance membership"
            )
        family = "Terrain" if policy["ground"] else "WorldObject"
        record = world_objects.build_record(
            family,
            identifier,
            f"oteryn:item.tibia.i{identifier}",
            "ground" if policy["ground"] else "object",
            None,
            appearances[identifier],
            source,
        )
        if identifier == 23049:
            # This row replaces an existing Terrain ordinal whose public World
            # palette already names the bound Item. Keep that exact route; the
            # separate Activity binding still names the same Terrain identity.
            if record["provenance"]["item_pointer"] != restore_pointer:
                raise ValueError("Anomaly restore public route changed")
        else:
            del record["provenance"]["item_pointer"]
        # Source C++ loads projectile blocking from appearance unsight; absent is false.
        known = lambda value: {"state": "KNOWN", "value": value}
        if family == "Terrain":
            record["kind"] = known("ground")
            record["walkable"] = known(not policy["block_solid"])
            record["ground_speed"] = known(policy["ground_speed"])
            record["blocks_projectile"] = known(policy["block_projectile"])
        else:
            record["kind"] = known("object")
            record["movable"] = known(False)
            record["collision"]["blocks_movement"] = known(policy["block_solid"])
            record["collision"]["blocks_projectile"] = known(policy["block_projectile"])
        records[family].append(record)
        registry.append(
            {
                "source_item_id": identifier,
                "identity": record["identity"],
                "policy": policy,
                "catalogue_record": record,
            }
        )
    return records, registry, captures


@lru_cache(maxsize=4)
def outputs(root=ROOT):
    records, registry, captures = copy.deepcopy(source_records(root))
    # This restore-only key already has a canonical Terrain ordinal. Replace
    # exactly that row via this publisher instead of introducing a duplicate.
    restore = next(
        row
        for row in records["Terrain"]
        if row["identity"]["key"] == "oteryn:terrain.tibia.i23049"
    )
    existing_path = "content/world/terrain/terrain-05500-05999.json"
    existing = json.loads((root / existing_path).read_bytes())
    matches = [
        i
        for i, row in enumerate(existing["records"])
        if row["identity"]["key"] == restore["identity"]["key"]
    ]
    if len(matches) != 1:
        raise ValueError(
            "Anomaly restore target must retain one original canonical ordinal"
        )
    previous = existing["records"][matches[0]]
    if (
        previous["provenance"]["source"]["profile"]
        not in (
            "crystal_ff7ede5_item_definition_v1",
            "OTERYN_ACTIVITY_STATIC_GEOMETRY_201189F/v1",
        )
        or previous["provenance"]["source_item_id"] != 23049
    ):
        raise ValueError("unreviewed Anomaly canonical source body")
    existing["records"][matches[0]] = restore
    import soulpit_static

    appended = {
        family: [
            row
            for row in rows
            if row is not restore
            and not (
                family == "Terrain"
                and row["provenance"]["source_item_id"] in soulpit_static.IDS
            )
        ]
        for family, rows in records.items()
    }
    if len(appended["Terrain"]) != 52:
        raise ValueError("activity/Soulpit canonical terrain union changed")
    files = {
        SHARDS[family]: encode({"family": family, "records": rows})
        for family, rows in appended.items()
    }
    files[existing_path] = encode(existing)
    terrain_keys = set()
    for path in (root / "content/world/terrain").glob("terrain-*.json"):
        terrain_keys.update(
            row["identity"]["key"]
            for row in json.loads(path.read_bytes()).get("records", [])
        )
    terrain_keys.update(row["identity"]["key"] for row in records["Terrain"])
    terrain_keys.update(row["identity"]["key"] for row in records["WorldObject"])
    # The shared IDs retain incoming canonical Soulpit records/ordinals. The
    # registry must bind that exact joint body while keeping its original
    # source policies and qualification closure unchanged.
    soulpit_rows = soulpit_static.records(root)
    canonical = {row["identity"]["key"]: row for row in soulpit_rows}
    shared = 0
    for entry in registry:
        row = canonical.get(entry["identity"]["key"])
        if entry["identity"]["family"] != "Terrain" or row is None:
            continue
        if row["identity"] != entry["identity"]:
            raise ValueError("joint static source identity changed")
        entry["catalogue_record"] = row
        shared += 1
    if shared != 8:
        raise ValueError("joint static registry shared set changed")
    files[REGISTRY] = encode(
        {
            "schema": "OTERYN_ACTIVITY_STATIC_DEFINITIONS/v1",
            "technical_terrain_keys": sorted(terrain_keys),
            "source_revision": PIN,
            "official_appearances_sha256": OFFICIAL_SHA,
            "source_closure": captures[0]["source_closure"],
            "records": registry,
        }
    )
    files["crates/gameplay/content/src/content/activity_static_surface_pin.rs"] = (
        "// Generated only by tools/content-schema/world-object-authoring/activity_static.py.\n"
        'pub(super) const REGISTRY_SHA256: &str =\n    "'
        + hashlib.sha256(files[REGISTRY]).hexdigest()
        + '";\n'
        "#[cfg(test)]\n"
        "pub(super) const M25_SOURCE_IDS: [u16; 76] = [\n"
        + "".join(
            "    "
            + ", ".join(
                str(value) for value in sorted(M25_IDENTIFIERS)[start : start + 13]
            )
            + ",\n"
            for start in range(0, len(M25_IDENTIFIERS), 13)
        )
        + "];\n"
    ).encode()
    placement_path = "content/world/placements/index.json"
    placement = json.loads((root / placement_path).read_bytes())
    compose_placement_palette(placement, registry, soulpit_rows, root)
    files[placement_path] = encode(placement)
    for family, directory in [("Terrain", "terrain"), ("WorldObject", "objects")]:
        name = f"content/world/{directory}/index.json"
        marker = json.loads((root / name).read_bytes())
        marker["notes"] = (
            re.sub(
                r" Additionally SRX-3 pinned activity static geometry: [0-9]+ Terrain and [0-9]+ WorldObject records; no Item aliases\.",
                "",
                marker["notes"],
            )
            + NOTE
        )
        if family == "Terrain":
            marker["notes"] = compose_terrain_notes(marker["notes"])
        files[name] = (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode()
    return files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    planned = outputs()
    if not args.check:
        previous_registry = ROOT / "content/encounters/source-static-definitions.json"
        if previous_registry.exists():
            previous = json.loads(previous_registry.read_bytes())
            current = json.loads(planned[REGISTRY])
            old_keys = set(previous["technical_terrain_keys"])
            new_keys = set(current["technical_terrain_keys"])
            additions = {
                f"oteryn:terrain.tibia.i{i}" for i in (7748, 13841, 17970, 53439)
            }
            # The legacy materializer retains the pre-lane terrain inventory.
            # All source records must still match; only the four independently
            # qualified lane grounds may extend that generated inventory.
            if (
                previous | {"technical_terrain_keys": current["technical_terrain_keys"]}
                != current
                or not old_keys <= new_keys
                or new_keys - old_keys not in (set(), additions)
            ):
                raise ValueError("refusing to replace changed activity static registry")
            previous_registry.unlink()
        previous_terrain = ROOT / "content/world/terrain/terrain-08614-08650.json"
        if not previous_terrain.exists():
            previous_terrain = ROOT / "content/world/terrain/terrain-08614-08651.json"
        if previous_terrain.exists():
            old = json.loads(previous_terrain.read_bytes())
            if any(
                row["provenance"]["source"]["profile"]
                != "OTERYN_ACTIVITY_STATIC_GEOMETRY_201189F/v1"
                for row in old["records"]
            ):
                raise ValueError("refusing to replace unrelated static terrain shard")
            previous_terrain.unlink()
        old_terrain = ROOT / "content/world/terrain/terrain-08669-08697.json"
        if old_terrain.exists():
            old = json.loads(old_terrain.read_bytes())
            if any(
                row["provenance"]["source"]["profile"]
                != "OTERYN_ACTIVITY_STATIC_GEOMETRY_201189F/v1"
                for row in old["records"]
            ):
                raise ValueError("refusing to replace unrelated pre-M25 terrain shard")
            old_terrain.unlink()
        for old_name in (
            "objects-12932-13055.json",
            "objects-12932-13052.json",
            "objects-12932-13053.json",
            "objects-12932-13054.json",
        ):
            previous = ROOT / "content/world/objects" / old_name
            if previous.exists():
                old = json.loads(previous.read_bytes())
                if any(
                    row["provenance"]["source"]["profile"]
                    != "OTERYN_ACTIVITY_STATIC_GEOMETRY_201189F/v1"
                    for row in old["records"]
                ):
                    raise ValueError(
                        "refusing to replace unrelated static geometry shard"
                    )
                previous.unlink()
    for name, raw in planned.items():
        path = ROOT / name
        if args.check:
            if not path.exists() or path.read_bytes() != raw:
                raise SystemExit(f"stale activity static geometry: {name}")
        else:
            path.write_bytes(raw)


if __name__ == "__main__":
    main()
