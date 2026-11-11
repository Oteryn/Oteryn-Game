"""Restore pinned Crystal startup chest UIDs and their required level-door actions.

Selections live in separate source captures so quest lanes can add their own
cohort. The canonical RewardClaim must still agree with every captured selector.
Run --apply [--check]; convert_world_base also applies the same overlay.
"""

import argparse
import hashlib
import json
from pathlib import Path

import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]
CAPTURES = "tools/content-schema/world-authoring/samples/quest-chest-bindings"
INDEX = "content/world/placements/index.json"
SUMMARY = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
TABLE_SHA256 = "7c3ac1280196923e0bf5ca6f2ea932c19065bb9ac22d1559150a184f3cf06df4"
CURRENT_REVISION = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
TABLE_PINS = {
    REVISION: TABLE_SHA256,
    CURRENT_REVISION: "0ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca",
}
STARTUP_PINS = {
    CURRENT_REVISION: "897c4ea5572c6adef1d2b300fd75c141910d608a093ca15c3d3bf65fcf56826c"
}
LEVEL_SHA256 = "d8868f675aae7c541ddc3e76b3648b6a886e0ae31a4a614747d17e1784d234fc"
ITEM_SHA256 = "7f551a398529e18ccad1ba56d740a4961874bbddd4736dd2df00272ddf6ea6ad"
ITEM_ACCESS = {
    "oteryn:reward-claim.kv.naginata": ((32356, 32074, 10), 1791, 40031),
}
# Crystal door_level.lua:65-83. Restoring a reward must retain its source level gate.
LEVEL_ACCESS = {
    **{
        f"oteryn:reward-claim.kv.desert{i}": ((32673, 32100, 8), 5102, 1020)
        for i in (1, 2)
    },
    # Pinned door_level.lua:173-178; the raw B3 omits startup attributes.
    "oteryn:reward-claim.quest.u7_24.the_annihilator.reward": (
        (33214, 31671, 13),
        5111,
        1100,
    ),
    **{
        f"oteryn:reward-claim.kv.orcfortress{i}": ((32981, 31760, 9), 6264, 1040)
        for i in range(1, 4)
    },
    "oteryn:reward-claim.kv.naginata": ((32353, 32073, 11), 1646, 1040),
    "oteryn:reward-claim.kv.noblearmor1": ((32448, 32042, 8), 6264, 1035),
    "oteryn:reward-claim.kv.noblearmor2": ((32448, 32042, 8), 6264, 1035),
}


def valid_capture_pin(document):
    revision = document.get("revision")
    return (
        document.get("schema") == 1
        and revision in TABLE_PINS
        and document.get("table_sha256") == TABLE_PINS[revision]
        and document.get("level_source_sha256") == LEVEL_SHA256
        and (
            revision not in STARTUP_PINS
            or document.get("startup_sha256") == STARTUP_PINS[revision]
        )
    )


def canonical(value):
    return (
        json.dumps(value, separators=(",", ":"), sort_keys=True, ensure_ascii=False)
        + "\n"
    ).encode()


def captured_level_access(raw, selected, root=ROOT):
    """Requalify level access independently of an unchanged chest capture."""
    import sys

    sys.path.insert(0, str(root / "tools/content-schema/quest-authoring"))
    import lua_tables

    if hashlib.sha256(raw).hexdigest() != LEVEL_SHA256:
        raise ValueError("QUEST_CHEST_LEVEL_SOURCE_PIN")
    fields = lua_tables.assignments(raw.decode(), {"LevelDoorAction"})[
        "LevelDoorAction"
    ]["fields"]
    doors = {}
    for key in sorted(set(selected) & LEVEL_ACCESS.keys()):
        position, appearance, action = LEVEL_ACCESS[key]
        field = next(f for f in fields if f["key"] == action)
        value = lua_tables.as_python(field["value"])
        groups = value if isinstance(value, list) else [value]
        if not any(
            p == dict(zip("xyz", position)) and group["itemId"] in (False, appearance)
            for group in groups
            if isinstance(group, dict)
            for p in (
                group["itemPos"]
                if isinstance(group["itemPos"], list)
                else [group["itemPos"]]
            )
        ):
            raise ValueError("QUEST_CHEST_LEVEL_ACCESS_SOURCE_DRIFT")
        row = doors.setdefault(
            position,
            {
                "position": list(position),
                "appearance": appearance,
                "action": action,
                "source_line": field["line"],
                "claims": [],
            },
        )
        row["claims"].append(key)
    return [doors[p] for p in sorted(doors)]


def claim_definitions(root):
    claims = {}
    for path in sorted(
        (root / "content/interactions/reward_claims").glob("reward-claim*.json")
    ):
        for record in json.loads(path.read_bytes())["records"]:
            definition = record["definition"]
            claims[definition["identity"]["key"]] = definition
    return claims


def supported_claim(definition):
    """Match the existing native container/text gate; retain all source holds."""
    native = definition.get("native_contents", {})
    return (
        definition["readiness"] == "ready"
        or panpipe_key_claim(definition)
        or (
            definition.get("definition_profile") == "authored_variant_v1"
            and definition.get("readiness") == "waiting_implementation"
            and definition.get("native_admission") == "WAITING_IMPLEMENTATION"
            and definition.get("data_holds") == []
            and definition.get("source_variant", {})
            .get("source_data", {})
            .get("status")
            == "COMPLETE"
            and native.get("schema") == "OTERYN_NATIVE_CONTAINER_REWARD/v1"
            and native.get("revision") == REVISION
            and native.get("source_sha256")
            == "b073d546930a60adf511677f932e6e0d37df3a230313dc85c4b9253742325757"
            and len(native.get("placements", [])) == len(definition["placements"])
        )
    )


def panpipe_key_claim(definition):
    """Select the existing key-only lowering, with its exact source key number."""
    variant = definition.get("source_variant", {})
    return (
        definition["identity"]["key"] == "oteryn:reward-claim.quest.key.id4055"
        and definition.get("definition_profile") == "authored_variant_v1"
        and definition.get("readiness") == "waiting_implementation"
        and definition.get("native_admission") == "WAITING_IMPLEMENTATION"
        and definition.get("data_holds") == []
        and variant.get("achievement_grants") == []
        and variant.get("charge_dispositions") == []
        and variant.get("source_data", {}).get("status") == "COMPLETE"
        and variant.get("native_lowering")
        == {
            "status": "WAITING_IMPLEMENTATION",
            "pending_implementation": ["KEY_REWARD_NATIVE_LOWERING_NOT_IMPLEMENTED"],
        }
        and len(definition["placements"]) == 1
        and definition["placements"][0]["reward"]
        == {
            "items": [
                {
                    "item": {
                        "family": "Item",
                        "key": "oteryn:item.tibia.i2969",
                        "revision": "definition-r1",
                    },
                    "count": 1,
                }
            ],
            "key_binding": "canary:door-key/4055",
        }
    )


def capture(source_root, selected, root=ROOT):
    """Read exact pinned source facts; never execute donor Lua."""
    import sys

    sys.path.insert(0, str(ROOT / "tools/content-schema/quest-authoring"))
    import lua_tables

    sys.path.insert(0, str(ROOT / "tools/content-schema/reward-claim-authoring"))
    import container_native
    import poison_daggers
    import reward_claim_authoring

    raw = (source_root / "data-global/startup/tables/chest.lua").read_bytes()
    table_sha256 = hashlib.sha256(raw).hexdigest()
    revision = next(
        (revision for revision, pin in TABLE_PINS.items() if pin == table_sha256), None
    )
    if revision is None:
        raise ValueError("QUEST_CHEST_SOURCE_PIN")
    startup_sha256 = None
    if revision in STARTUP_PINS:
        startup = (
            source_root / "data-global/startup/others/functions.lua"
        ).read_bytes()
        startup_sha256 = hashlib.sha256(startup).hexdigest()
        if startup_sha256 != STARTUP_PINS[revision]:
            raise ValueError("QUEST_CHEST_STARTUP_SOURCE_PIN")
    entries = {
        f["key"]: (f["line"], lua_tables.as_python(f["value"]))
        for f in lua_tables.assignments(raw.decode(), {"ChestUnique"})["ChestUnique"][
            "fields"
        ]
    }
    claims, rows = claim_definitions(root), []
    needs_charges = any(
        claims[key].get("provenance", {}).get("pilot_key")
        in container_native.CHARGED_QUEST_CLAIMS
        for key in selected
    )
    items = reward_claim_authoring.load_items() if needs_charges else {}
    for key in sorted(selected):
        definition = claims[key]
        if not supported_claim(definition):
            raise ValueError("QUEST_CHEST_REWARD_NOT_READY")
        for placement in definition["placements"]:
            ids = [
                u["unique_id"]
                for u in placement["source_binding"]["legacy_unique_ids"]
                if u["server"] == "crystalserver"
            ]
            if len(ids) != 1:
                raise ValueError("QUEST_CHEST_SOURCE_UID")
            unique = ids[0]
            line, setting = entries[unique]
            position = placement["source_binding"]["project_position"]
            reward = [
                [int(item["item"]["key"].rsplit(".i", 1)[1]), item["count"]]
                for item in placement["reward"]["items"]
            ]
            container = placement["reward"].get("container")
            container_id = (
                int(container["key"].rsplit(".i", 1)[1]) if container else None
            )
            source_reward = container_native.source_charged_reward(
                definition.get("provenance", {}).get("pilot_key"),
                poison_daggers.source_reward(unique, position, setting["reward"]),
                items,
            )
            if (
                setting["itemPos"] != position
                or setting["itemId"] != placement["appearance_tibia_id"]
                or source_reward != reward
                or setting.get("container") != container_id
            ):
                raise ValueError("QUEST_CHEST_SOURCE_SELECTOR_OR_REWARD_DRIFT")
            if panpipe_key_claim(definition) and (
                setting.get("isKey") is not True
                or setting.get("storage") != {"expr": "Storage.Quest.Key.ID4055"}
            ):
                raise ValueError("QUEST_CHEST_SOURCE_KEY_DRIFT")
            rows.append(
                {
                    "claim": key,
                    "position": [position[k] for k in ("x", "y", "z")],
                    "appearance": setting["itemId"],
                    "unique": unique,
                    "source_line": line,
                }
            )
    level_raw = (source_root / "data-global/startup/tables/door_level.lua").read_bytes()
    level_access = captured_level_access(level_raw, selected, root)
    document = {
        "schema": 1,
        "classification": "OTS_HYPOTHESIS_ONLY",
        "repository": "zimbadev/crystalserver",
        "revision": revision,
        "table": "data-global/startup/tables/chest.lua",
        "table_sha256": table_sha256,
        "startup": "data-global/startup/others/functions.lua:97-115",
        "bindings": rows,
        "level_source_sha256": hashlib.sha256(level_raw).hexdigest(),
        "level_access": level_access,
    }
    if startup_sha256 is not None:
        document["startup_sha256"] = startup_sha256
        document["startup"] = "data-global/startup/others/functions.lua:97-120"
    if set(selected) & ITEM_ACCESS.keys():
        item_raw = (source_root / "data-global/startup/tables/item.lua").read_bytes()
        if hashlib.sha256(item_raw).hexdigest() != ITEM_SHA256:
            raise ValueError("QUEST_CHEST_ITEM_SOURCE_PIN")
        table = lua_tables.assignments(item_raw.decode(), {"ItemAction"})["ItemAction"]
        access = []
        for key in sorted(set(selected) & ITEM_ACCESS.keys()):
            position, appearance, action = ITEM_ACCESS[key]
            # This source repeats action40031 for an unrelated Item644. Bind the
            # exact source row and position, rather than losing the first field.
            matches = [
                f
                for f in table["fields"]
                if f["key"] == action
                and lua_tables.as_python(f["value"])["itemId"] == appearance
                and dict(zip("xyz", position))
                in lua_tables.as_python(f["value"])["itemPos"]
            ]
            if len(matches) != 1:
                raise ValueError("QUEST_CHEST_ITEM_ACCESS_SOURCE_DRIFT")
            access.append(
                {
                    "position": list(position),
                    "appearance": appearance,
                    "action": action,
                    "source_line": matches[0]["line"],
                    "claims": [key],
                }
            )
        document["item_source_sha256"] = ITEM_SHA256
        document["item_access"] = access
    return document


def selections(root=ROOT):
    claims = claim_definitions(root)
    bindings, doors, sources = {}, {}, []
    for path in sorted((root / CAPTURES).glob("*.json")):
        raw = path.read_bytes()
        document = json.loads(raw)
        if not valid_capture_pin(document):
            raise ValueError("QUEST_CHEST_SOURCE_PIN")
        sources.append(
            {
                "path": path.relative_to(root).as_posix(),
                "sha256": hashlib.sha256(raw).hexdigest(),
            }
        )
        for row in document["bindings"]:
            position = tuple(row["position"])
            definition = claims[row["claim"]]
            if not supported_claim(definition):
                raise ValueError("QUEST_CHEST_REWARD_NOT_READY")
            matches = [
                p
                for p in definition["placements"]
                if p["source_binding"]["project_position"]
                == dict(zip(("x", "y", "z"), position))
                and p["appearance_tibia_id"] == row["appearance"]
                and {"server": "crystalserver", "unique_id": row["unique"]}
                in p["source_binding"]["legacy_unique_ids"]
            ]
            if len(matches) != 1 or position in bindings:
                raise ValueError("QUEST_CHEST_CANONICAL_SELECTOR_DRIFT")
            bindings[position] = row
        captured_claims = {r["claim"] for r in document["bindings"]}
        for row in document.get("level_access", []):
            position = tuple(row["position"])
            if (
                not row["claims"]
                or not set(row["claims"]) <= captured_claims
                or any(
                    LEVEL_ACCESS.get(key)
                    != (position, row["appearance"], row["action"])
                    for key in row["claims"]
                )
            ):
                raise ValueError("QUEST_CHEST_LEVEL_ACCESS_DRIFT")
            if position in doors and (
                doors[position]["action"],
                doors[position]["appearance"],
            ) != (row["action"], row["appearance"]):
                raise ValueError("QUEST_CHEST_LEVEL_ACCESS_CONFLICT")
            doors[position] = row
        for key in captured_claims & LEVEL_ACCESS.keys():
            if LEVEL_ACCESS[key][0] not in doors:
                raise ValueError("QUEST_CHEST_LEVEL_ACCESS_MISSING")
        for row in document.get("item_access", []):
            position = tuple(row["position"])
            if (
                document.get("item_source_sha256") != ITEM_SHA256
                or not row["claims"]
                or not set(row["claims"]) <= captured_claims
                or any(
                    ITEM_ACCESS.get(key) != (position, row["appearance"], row["action"])
                    for key in row["claims"]
                )
                or position in doors
            ):
                raise ValueError("QUEST_CHEST_ITEM_ACCESS_DRIFT")
            doors[position] = row
        for key in captured_claims & ITEM_ACCESS.keys():
            if ITEM_ACCESS[key][0] not in doors:
                raise ValueError("QUEST_CHEST_ITEM_ACCESS_MISSING")
    if any(
        row["claim"] == "oteryn:reward-claim.kv.panpipe" for row in bindings.values()
    ):
        # Already captured from both donors: door_key.lua lines137/158. The
        # numeric storage is4055 despite the donors' stale "key3791" label.
        path = "tools/content-schema/quest-authoring/samples/doors/gates.json"
        raw = (root / path).read_bytes()
        if (
            hashlib.sha256(raw).hexdigest()
            != "dfcd8f8424d01c393fc6e0a25dbf26aeab902ec5cc691bd21ddbe336ddad7e87"
        ):
            raise ValueError("PANPIPE_KEY_DOOR_SOURCE_PIN")
        gate = [
            r
            for r in json.loads(raw)["gates"]
            if r["identity"]["key"] == "canary:door-gate/key/4055"
        ]
        if (
            len(gate) != 1
            or gate[0]["condition"]["key_binding"] != "canary:door-key/4055"
            or gate[0]["placements"]
            != [{"position": {"x": 32643, "y": 32128, "z": 8}, "appearance": None}]
        ):
            raise ValueError("PANPIPE_KEY_DOOR_SOURCE_DRIFT")
        position = (32643, 32128, 8)
        if position in doors:
            raise ValueError("PANPIPE_KEY_DOOR_ACCESS_CONFLICT")
        doors[position] = {
            "position": list(position),
            "appearance": 1631,
            "action": 4055,
            "claims": ["oteryn:reward-claim.kv.panpipe"],
        }
        sources.append({"path": path, "sha256": hashlib.sha256(raw).hexdigest()})
    return {"unique": bindings, "action": doors}, sources


def overlay_region(raw, palette, bindings, attribute="unique"):
    """Only a single top-level source appearance may receive its captured attribute."""
    z, rx, ry, table = codec.parse_region(raw)
    selected = {
        p: r
        for p, r in bindings.items()
        if p[2] == z and p[0] // 256 == rx and p[1] // 256 == ry
    }
    if not selected:
        return raw, 0, 0
    sectors, found, added = {}, set(), 0
    for local, offset, length in table:
        payload = codec.zstandard.ZstdDecompressor().decompress(
            raw[offset : offset + length], max_output_size=codec.MAX_SECTOR_BYTES
        )
        sx, sy = rx * 8 + local % 8, ry * 8 + local // 8
        tiles = codec.decode_sector(payload, sx, sy)
        changed = False
        for x, y, _, house, _, items in tiles:
            position = (x, y, z)
            row = selected.get(position)
            if row is None:
                continue
            slots = [
                slot
                for slot, (index, depth, _) in enumerate(items)
                if depth == 0 and palette[index]["source_item_id"] == row["appearance"]
            ]
            if house or len(slots) != 1:
                raise ValueError(f"QUEST_CHEST_AMBIGUOUS_OR_MISSING: {position}")
            slot = slots[0]
            index, depth, attrs = items[slot]
            previous = (attrs or {}).get(attribute)
            if previous is not None and previous != row[attribute]:
                raise ValueError(f"QUEST_CHEST_UID_CONFLICT: {position}")
            if previous is None:
                items[slot] = (
                    index,
                    depth,
                    {**(attrs or {}), attribute: row[attribute]},
                )
                added += 1
                changed = True
            found.add(position)
        sectors[local] = (
            codec.encode_sector_tiles(sx, sy, tiles) if changed else payload
        )
    if found != set(selected):
        raise ValueError(f"QUEST_CHEST_TILE_MISSING: {sorted(set(selected) - found)}")
    return (
        (codec.encode_region(z, rx, ry, sectors) if added else raw),
        len(found),
        added,
    )


def overlay_outputs(outputs, root=ROOT, require_all=False):
    selections_by_attribute, sources = selections(root)
    index = json.loads(outputs[INDEX])
    imported, byte_delta = 0, 0
    added = {attribute: 0 for attribute in selections_by_attribute}
    bindings = {
        p: row for rows in selections_by_attribute.values() for p, row in rows.items()
    }
    paths = {
        f"content/world/placements/{codec.region_name(z, x // 256, y // 256)}"
        for x, y, z in bindings
    }
    regions = {r["path"]: r for r in index["regions"]}
    if require_all and paths - regions.keys():
        raise ValueError("QUEST_CHEST_REGION_MISSING")
    # The converter also builds small independent World fixtures. Apply only
    # selectors whose regions belong to that World; the repository apply/check
    # route requires the entire selected cohort before publication.
    paths &= regions.keys()
    if not paths:
        return 0
    for path in sorted(paths):
        previous = outputs[path]
        raw = previous
        for attribute, rows in selections_by_attribute.items():
            raw, count, delta = overlay_region(raw, index["palette"], rows, attribute)
            imported += count
            added[attribute] += delta
        byte_delta += len(raw) - len(previous)
        outputs[path] = raw
        regions[path]["sha256"] = hashlib.sha256(raw).hexdigest()
    if require_all and imported != len(bindings):
        raise ValueError("QUEST_CHEST_INCOMPLETE_OVERLAY")
    index["source"]["quest_chests"] = {"captures": sources, "imported": imported}
    outputs[INDEX] = canonical(index)
    summary = json.loads(outputs[SUMMARY])
    summary["source"] = index["source"]
    for attribute, count in added.items():
        if count:
            summary["item_attributes"][attribute] = (
                summary["item_attributes"].get(attribute, 0) + count
            )
    summary["bytes_on_disk"] += byte_delta
    outputs[SUMMARY] = canonical(summary)
    return imported


def apply(check):
    selected, _ = selections()
    bindings = {p: row for rows in selected.values() for p, row in rows.items()}
    paths = {INDEX, SUMMARY} | {
        f"content/world/placements/{codec.region_name(z, x // 256, y // 256)}"
        for x, y, z in bindings
    }
    outputs = {p: (ROOT / p).read_bytes() for p in paths}
    count = overlay_outputs(outputs, require_all=True)
    for path, raw in outputs.items():
        if (ROOT / path).read_bytes() != raw:
            if check:
                raise ValueError(f"QUEST_CHEST_BINDING_DRIFT: {path}")
            (ROOT / path).write_bytes(raw)
    print(f"quest chest/access startup attributes verified: {count}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--apply", action="store_true")
    mode.add_argument(
        "--capture", type=Path, help="pinned Crystal checkout or reference root"
    )
    mode.add_argument(
        "--capture-level-access",
        type=Path,
        help="retain chest capture and requalify its level access",
    )
    parser.add_argument("--level-source", type=Path)
    parser.add_argument("--claims", nargs="+")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.capture_level_access:
        if not args.level_source:
            parser.error("--capture-level-access requires --level-source")
        document = json.loads(args.capture_level_access.read_bytes())
        if not valid_capture_pin(document):
            raise ValueError("QUEST_CHEST_SOURCE_PIN")
        document["level_access"] = captured_level_access(
            args.level_source.read_bytes(), {r["claim"] for r in document["bindings"]}
        )
        raw = (json.dumps(document, indent=2) + "\n").encode()
        if args.check:
            if args.capture_level_access.read_bytes() != raw:
                raise ValueError("QUEST_CHEST_LEVEL_CAPTURE_DRIFT")
        else:
            args.capture_level_access.write_bytes(raw)
    elif args.capture:
        if not args.claims or not args.output:
            parser.error("--capture requires --claims and --output")
        raw = (json.dumps(capture(args.capture, args.claims), indent=2) + "\n").encode()
        if args.check:
            if args.output.read_bytes() != raw:
                raise ValueError("QUEST_CHEST_CAPTURE_DRIFT")
        else:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_bytes(raw)
    else:
        apply(args.check)
