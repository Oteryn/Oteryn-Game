"""No-network checks for `world_objects.py` (WO-1), covering the WO-0 §8 WO-1 lines:
every routed id maps to exactly one family key by the D93 rule, excluded routes get
none, the key is reproducible from the frozen allocation, unknown facts stay UNKNOWN,
and a typed `routed_to` validates while a bare key or an unknown family is rejected.

Routing runs through `engine_items.convert_item` against fabricated `items`/`appearances`
with the real identity index and disposition catalog (`test_engine_items`' own
`synthetic_sources`), so no pinned checkout is needed. Run with
`python test_world_objects.py`.
"""

from __future__ import annotations

import copy
import hashlib
import json
import tempfile
from pathlib import Path

# isort: off
# world_objects puts item-authoring on sys.path, so test_engine_items imports after it.
import build_catalogue
import world_objects
from test_engine_items import synthetic_sources
# isort: on

CHECKS = 0


def check(condition, message):
    global CHECKS
    CHECKS += 1
    if not condition:
        raise AssertionError(message)


def raises(fn, *args):
    try:
        fn(*args)
    except ValueError:
        return True
    return False


META = {
    "engine": "crystal",
    "repository": "zimbadev/crystalserver",
    "revision": "ff7ede593c69d4c658b382c97443e8155926924a",
    "profile": "crystal_ff7ede5_item_definition_v1",
}


# --- D93 identity ---------------------------------------------------------------------


def test_family_key_keeps_the_tibia_id():
    item = "oteryn:item.tibia.i100"
    check(
        world_objects.family_key(item, "Terrain") == "oteryn:terrain.tibia.i100",
        "terrain key",
    )
    check(
        world_objects.family_key(item, "WorldObject")
        == "oteryn:world-object.tibia.i100",
        "world-object key",
    )
    check(raises(world_objects.family_key, item, "Item"), "Item is not a routed family")
    check(raises(world_objects.family_key, item, "LocalObject"), "LocalObject rejected")
    for bad in (
        "oteryn:item.ammunition.arrow",
        "oteryn:item.registry.i00000100",
        "oteryn:item.tibia.i0100",
        "oteryn:terrain.tibia.i100",
        "",
        None,
    ):
        check(raises(world_objects.family_key, bad, "Terrain"), f"bad item key {bad!r}")


def test_route_exclusions_d94():
    check(
        world_objects.family_for_route("Terrain", "ground_or_border") == "Terrain", "T"
    )
    check(
        world_objects.family_for_route("WorldObject", "corpse") == "WorldObject", "WO"
    )
    for owner, reason in (
        ("WorldObject", "appearance_placeholder_slot"),
        ("WorldObject", "no_client_appearance"),
        ("Fluid", "fluid_type_without_appearance"),
    ):
        check(
            world_objects.family_for_route(owner, reason) is None, f"{owner}:{reason}"
        )
    check(raises(world_objects.family_for_route, "Interaction", "x"), "unknown owner")


# --- records ------------------------------------------------------------------------


def terrain(attrs=None, flags=None, appearance=True, item_id=100):
    return world_objects.build_terrain(
        item_id,
        f"oteryn:item.tibia.i{item_id}",
        "ground_or_border",
        {"name": "t", "attrs": attrs or {}},
        {"id": item_id, "flags": flags or {}} if appearance else None,
        META,
    )


def world_object(attrs=None, flags=None, reason="immovable_unclassified", item_id=200):
    return world_objects.build_world_object(
        item_id,
        f"oteryn:item.tibia.i{item_id}",
        reason,
        {"name": "o", "attrs": attrs or {}},
        {"id": item_id, "flags": flags or {}},
        META,
    )


def test_terrain_ground_record():
    record = terrain(
        attrs={"floorchange": "down"},
        flags={"flags.bank": True, "bank.waypoints": 150, "flags.unmove": True},
    )
    check(
        world_objects.validate_record(record) == [],
        world_objects.validate_record(record),
    )
    check(record["kind"] == {"state": "KNOWN", "value": "ground"}, record["kind"])
    check(
        record["walkable"] == {"state": "KNOWN", "value": True}, "no unpass -> walkable"
    )
    check(record["ground_speed"] == {"state": "KNOWN", "value": 150}, "ground speed")
    check(record["floor_change"] == {"state": "KNOWN", "value": "down"}, "floorchange")
    check(
        record["blocks_projectile"] == {"state": "UNKNOWN"},
        "absent xml attr -> UNKNOWN",
    )
    check(record["automap"] == {"state": "KNOWN", "value": {"shown": False}}, "automap")
    check("field" not in record, "field only on kind field")
    fields = record["provenance"]["fields"]
    check(fields["ground_speed"] == ["appearance:bank.waypoints"], fields)
    check(fields["floor_change"] == ["items_xml:floorchange"], fields)


def test_terrain_kinds_in_rule_order():
    cases = [
        ({"type": "magicfield", "field": "fire"}, {"flags.bank": True}, "field"),
        ({}, {"flags.fullbank": True, "flags.clip": True}, "ground"),
        ({}, {"flags.clip": True, "flags.bottom": True}, "border"),
        ({}, {"flags.bottom": True, "flags.unpass": True}, "wall"),
        ({"primarytype": "walls"}, {}, "wall"),
        ({"primarytype": "fields"}, {}, "field"),
    ]
    for attrs, flags, kind in cases:
        record = terrain(attrs=attrs, flags=flags)
        check(record["kind"].get("value") == kind, (attrs, flags, record["kind"]))
        check(
            world_objects.validate_record(record) == [],
            world_objects.validate_record(record),
        )
    fire = terrain(attrs={"type": "magicfield", "field": "fire"})
    check(
        fire["field"] == {"state": "KNOWN", "value": {"field_type": "fire"}},
        fire["field"],
    )
    roof = world_objects.build_terrain(
        300,
        "oteryn:item.tibia.i300",
        "primarytype_world_object",
        {"name": "tiled roof", "attrs": {"primarytype": "artificial tiles"}},
        {"id": 300, "flags": {"flags.unmove": True, "flags.unpass": True}},
        META,
    )
    check(roof["kind"] == {"state": "KNOWN", "value": "roof"}, roof["kind"])
    check(
        world_objects.validate_record(roof) == [], world_objects.validate_record(roof)
    )
    swamp = terrain(attrs={"type": "trashholder"}, flags={"flags.bank": True})
    check(swamp["kind"]["value"] == "ground", swamp["kind"])
    check(
        swamp["behavior"] == {"state": "KNOWN", "value": "trash_holder"},
        swamp.get("behavior"),
    )
    check(swamp["provenance"]["fields"]["behavior"] == ["items_xml:type"], swamp)
    check(
        world_objects.validate_record(swamp) == [], world_objects.validate_record(swamp)
    )
    check("behavior" not in terrain(), "no behaviour without an items.xml type")
    tile = terrain(
        attrs={"primarytype": "artificial tiles"}, flags={"flags.unmove": True}
    )
    check(tile["kind"] == {"state": "UNKNOWN"}, "no rule -> UNKNOWN, never guessed")
    check(
        world_objects.validate_record(tile) == [], world_objects.validate_record(tile)
    )


def test_no_appearance_keeps_appearance_facts_unknown():
    record = terrain(appearance=False)
    for field in ("walkable", "blocks_sight", "automap", "client_projection"):
        check(record[field] == {"state": "UNKNOWN"}, (field, record[field]))
    check(
        world_objects.validate_record(record) == [],
        world_objects.validate_record(record),
    )


def test_world_object_kinds_and_sections():
    door = world_object(
        attrs={"type": "door", "leveldoor": "1000"}, flags={"flags.unpass": True}
    )
    check(door["kind"]["value"] == "door", door["kind"])
    check(door["door"]["level"] == {"state": "KNOWN", "value": 1000}, door["door"])
    check(door["door"]["house_guests"] == {"state": "UNKNOWN"}, door["door"])
    check(
        world_objects.validate_record(door) == [], world_objects.validate_record(door)
    )

    bed = world_object(
        attrs={
            "type": "bed",
            "bedpart": "pillow",
            "bedpartof": "741",
            "partnerdirection": "south",
            "maletransformto": "2487",
        }
    )
    check(
        bed["bed"]["part_of"] == {"state": "KNOWN", "value": {"source_item_id": 741}},
        bed,
    )
    check(bed["bed"]["female_transform_to"] == {"state": "UNKNOWN"}, bed["bed"])
    check(world_objects.validate_record(bed) == [], world_objects.validate_record(bed))

    corpse = world_object(
        attrs={"decayto": "0", "duration": "60", "containersize": "10"},
        flags={"flags.corpse": True, "flags.container": True},
        reason="corpse",
    )
    check(corpse["kind"]["value"] == "corpse", corpse["kind"])
    check(corpse["corpse"]["corpse"] == {"state": "KNOWN", "value": True}, corpse)
    text = json.dumps(corpse)
    check("decayto" not in text and "duration" not in text, "decay stays Item-owned")
    check(
        world_objects.validate_record(corpse) == [],
        world_objects.validate_record(corpse),
    )

    check(
        world_object(flags={"flags.unpass": True})["kind"]["value"] == "object",
        "object",
    )
    check(
        world_object(flags={"flags.top": True})["kind"]["value"] == "decoration",
        "decor",
    )
    for type_value, kind in (
        ("ladder", "ladder"),
        ("teleport", "teleport"),
        ("depot", "container_fixture"),
        ("carpet", "decoration"),
    ):
        check(
            world_object(attrs={"type": type_value})["kind"]["value"] == kind,
            type_value,
        )


def test_type_outside_family_is_unknown_and_reported():
    record = world_object(attrs={"type": "magicfield", "field": "fire"})
    check(record["kind"] == {"state": "UNKNOWN"}, record["kind"])
    check(
        world_objects.validate_record(record) == [],
        world_objects.validate_record(record),
    )
    check(
        world_objects.type_outside_family("WorldObject", {"type": "magicfield"})
        == "magicfield",
        "wo",
    )
    check(
        world_objects.type_outside_family("Terrain", {"type": "rewardchest"})
        == "rewardchest",
        "t",
    )
    for type_value in ("trashholder", "teleport"):
        check(
            world_objects.type_outside_family("Terrain", {"type": type_value}) is None,
            f"{type_value} is a Terrain behaviour (WO-2c)",
        )
    check(
        world_objects.type_outside_family("Terrain", {"type": "magicfield"}) is None,
        "field ok",
    )
    check(
        world_objects.type_outside_family("WorldObject", {"type": "door"}) is None,
        "door ok",
    )


def test_placement_flags_are_complete_facts():
    record = world_object(
        attrs={"rotateto": "2466", "walkstack": "0"},
        flags={"flags.hook": True, "hook.direction": 2, "flags.rotate": True},
    )
    placement = record["placement"]
    check(placement["hook_direction"] == {"state": "KNOWN", "value": "east"}, placement)
    check(
        placement["elevation"] == {"state": "KNOWN", "value": 0}, "no height flag -> 0"
    )
    check(placement["rotate_to"]["value"] == {"source_item_id": 2466}, placement)
    check(placement["walk_stack"] == {"state": "KNOWN", "value": False}, placement)
    plain = world_object()
    check(
        plain["placement"]["hook_direction"] == {"state": "KNOWN", "value": "none"},
        "no hook",
    )
    check(
        raises(world_object, {}, {"flags.hook": True, "hook.direction": 9}), "bad hook"
    )


def test_validate_record_rejects():
    base = terrain(flags={"flags.bank": True, "bank.waypoints": 100})

    def errors_after(mutate):
        record = copy.deepcopy(base)
        mutate(record)
        return world_objects.validate_record(record)

    cases = {
        "extra property": lambda r: r.update({"loot": {}}),
        "wrong family key": lambda r: r["identity"].update(
            {"key": "oteryn:world-object.tibia.i100"}
        ),
        "sequence mismatch": lambda r: r["identity"].update(
            {"key": "oteryn:terrain.tibia.i101"}
        ),
        "kind outside set": lambda r: r.update(
            {"kind": {"state": "KNOWN", "value": "door"}}
        ),
        "section on wrong kind": lambda r: r.update(
            {"field": {"state": "KNOWN", "value": {"field_type": "fire"}}}
        ),
        "missing section": lambda r: r.pop("ground_speed"),
        "KNOWN without provenance": lambda r: r["provenance"]["fields"].pop(
            "blocks_sight"
        ),
        "defaulted unknown": lambda r: r.update(
            {"blocks_projectile": {"state": "KNOWN"}}
        ),
        "route owner": lambda r: r["provenance"]["route"].update(
            {"owner": "WorldObject"}
        ),
    }
    for name, mutate in cases.items():
        check(errors_after(mutate) != [], f"{name} must be rejected")
    check(
        world_objects.validate_record({"identity": {"family": "Item"}}) != [],
        "unsupported family",
    )


# --- routed Item pointer ----------------------------------------------------------------


def test_routed_item_pointer():
    pointer = world_objects.routed_item_pointer(
        "oteryn:item.tibia.i1234", "WorldObject"
    )
    check(world_objects.validate_routed_item_pointer(pointer) == [], pointer)
    check(
        pointer["routed_to"]
        == {
            "family": "WorldObject",
            "key": "oteryn:world-object.tibia.i1234",
            "revision": "definition-r1",
        },
        pointer,
    )
    bare = copy.deepcopy(pointer)
    bare["routed_to"] = "oteryn:world-object.tibia.i1234"
    check(world_objects.validate_routed_item_pointer(bare) != [], "bare key rejected")
    unknown_family = copy.deepcopy(pointer)
    unknown_family["routed_to"]["family"] = "LocalObject"
    check(
        world_objects.validate_routed_item_pointer(unknown_family) != [],
        "family rejected",
    )
    mismatched = copy.deepcopy(pointer)
    mismatched["routed_to"]["family"] = "Terrain"
    check(
        world_objects.validate_routed_item_pointer(mismatched) != [],
        "family/key mismatch",
    )
    shifted = copy.deepcopy(pointer)
    shifted["routed_to"]["key"] = "oteryn:world-object.tibia.i1235"
    check(
        world_objects.validate_routed_item_pointer(shifted) != [], "sequence mismatch"
    )
    materializable = copy.deepcopy(pointer)
    materializable["materializable"] = True
    check(
        world_objects.validate_routed_item_pointer(materializable) != [],
        "materializable",
    )


# --- census through the real converter routing ---------------------------------------


def census_sources():
    records = {
        100: {
            "flags": {"flags.bank": True, "bank.waypoints": 150, "flags.unmove": True}
        },
        101: {"flags": {"flags.clip": True, "flags.unmove": True}},
        102: {"attrs": {"primarytype": "walls"}, "flags": {"flags.unmove": True}},
        200: {
            "attrs": {"type": "door"},
            "flags": {"flags.unmove": True, "flags.unpass": True},
        },
        201: {"flags": {"flags.corpse": True, "flags.container": True}},
        202: {"name": "reserved sprite", "flags": {}},
        203: {"flags": {"flags.unmove": True, "flags.top": True}},
    }
    sources = synthetic_sources("crystal", records)
    sources["artifact_digests"] = {"fixture": {"sha256": "0" * 64}}
    return sources


def test_census_maps_every_routed_id_once_and_is_reproducible():
    sources = census_sources()
    with tempfile.TemporaryDirectory() as tmp:
        first = world_objects.build_census(sources, records_dir=tmp)
        written = sorted(
            p.relative_to(tmp).as_posix() for p in Path(tmp).rglob("*.json")
        )
    second = world_objects.build_census(census_sources())
    check(
        world_objects.census_document_bytes(first)
        == world_objects.census_document_bytes(second),
        "census is deterministic",
    )
    check(
        first["excluded_routes"] == {"WorldObject:appearance_placeholder_slot": 1},
        first,
    )
    check(
        first["records"]["by_family"] == {"Terrain": 3, "WorldObject": 3},
        first["records"],
    )
    check(
        written
        == [
            "Terrain/100.json",
            "Terrain/101.json",
            "Terrain/102.json",
            "WorldObject/200.json",
            "WorldObject/201.json",
            "WorldObject/203.json",
        ],
        written,
    )
    index = sources["identity_index"]
    for item_id, family in ((100, "Terrain"), (200, "WorldObject")):
        item_key = index[item_id][0]
        expected = world_objects.family_key(item_key, family)
        check(expected.endswith(item_key[-8:]), (item_key, expected))
    check(
        first["records"]["by_kind"]
        == {
            "Terrain:border": 1,
            "Terrain:ground": 1,
            "Terrain:wall": 1,
            "WorldObject:corpse": 1,
            "WorldObject:decoration": 1,
            "WorldObject:door": 1,
        },
        first["records"]["by_kind"],
    )


def test_committed_census_sample_is_consistent():
    sample = json.loads(world_objects.DEFAULT_SAMPLE.read_text(encoding="utf-8"))
    routes = sample["routes"]
    excluded = sample["excluded_routes"]
    records = sample["records"]
    check(records["total"] == sum(routes.values()) - sum(excluded.values()), "total")
    check(sum(records["by_family"].values()) == records["total"], "by_family")
    check(sum(records["by_kind"].values()) == records["total"], "by_kind")
    # D149 removed the ids without a CipSoft appearance, so some D94 routes are empty.
    check(
        set(excluded)
        <= {f"{owner}:{reason}" for owner, reason in world_objects.EXCLUDED_ROUTES},
        excluded,
    )
    for name, keys in records["examples"].items():
        family = name.split(":", 1)[0]
        for key in keys:
            check(world_objects.FAMILY_KEY[family].match(key) is not None, key)


def test_catalogue_shards_each_family_and_marks_it_populated():
    files, census = build_catalogue.outputs(census_sources())
    check(
        sorted(files)
        == [
            "content/world/objects/index.json",
            "content/world/objects/objects-00000-00002.json",
            "content/world/terrain/index.json",
            "content/world/terrain/terrain-00000-00002.json",
        ],
        sorted(files),
    )
    terrain = json.loads(files["content/world/terrain/terrain-00000-00002.json"])
    check(terrain["family"] == "Terrain", terrain["family"])
    ids = [r["provenance"]["source_item_id"] for r in terrain["records"]]
    check(ids == [100, 101, 102], ids)
    for record in terrain["records"]:
        check(world_objects.validate_record(record) == [], record["identity"])
    marker = json.loads(files["content/world/objects/index.json"])
    check(marker["population_state"] == "POPULATED", marker)
    check(marker["path"] == "content/world/objects/", marker)
    check(": 3 records" in marker["notes"], marker["notes"])
    again, census_again = build_catalogue.outputs(census_sources())
    check(again == files and census_again == census, "catalogue is deterministic")


def test_committed_catalogue_matches_the_census_sample():
    sample = json.loads(world_objects.DEFAULT_SAMPLE.read_text(encoding="utf-8"))
    for family, (directory, prefix, _notes) in build_catalogue.CATALOGUES.items():
        shards = sorted(
            path
            for path in (build_catalogue.ROOT / directory).glob(f"{prefix}-*.json")
            if path
            not in (
                build_catalogue.donor_shards()
                | build_catalogue.official_shards()
                | build_catalogue.qualified_shards()
            )
        )
        count = 0
        previous = 0
        for shard in shards:
            document = json.loads(shard.read_text(encoding="utf-8"))
            check(document["family"] == family, shard.name)
            check(shard.name.startswith(f"{prefix}-{count:05d}-"), shard.name)
            for record in document["records"]:
                source_id = record["provenance"]["source_item_id"]
                check(source_id > previous, (shard.name, source_id))
                previous = source_id
            count += len(document["records"])
        check(count == sample["records"]["by_family"][family], (family, count))
        marker = json.loads(
            (build_catalogue.ROOT / directory / "index.json").read_text()
        )
        check(marker["population_state"] == "POPULATED", marker)


def test_qualified_donor_identity_and_precedence_fail_closed():
    item_id = 54335
    key = "oteryn:item.tibia.i54335"
    identity = {"family": "Item", "key": key, "revision": "definition-r1"}
    binding = {
        "disposition": "EXACT",
        "external_id": "54335",
        "identity_namespace": "ots/item_server_id",
        "source_key": "oteryn:source.crystalserver",
        "source_revision": build_catalogue.donor_census.DONOR_COMMIT,
        "target": identity,
    }
    definition = {"identity": identity, "materializable": False}
    donor = synthetic_sources(
        "crystal",
        {item_id: {"name": "fixture corpse", "flags": {"flags.corpse": True}}},
    )
    sources = synthetic_sources("crystal", {})
    sources.update(wiki_family_fallback={}, owner_family_decisions={})
    memberships = (
        {item_id: [item_id, "a" * 64, "b" * 64]},
        {item_id: [item_id, "a" * 64, "c" * 64]},
    )
    meta = dict(
        META,
        revision=build_catalogue.donor_census.DONOR_COMMIT,
        profile=build_catalogue.DONOR_PROFILE,
    )
    expected = world_objects.build_world_object(
        item_id,
        key,
        "corpse",
        donor["items"][item_id],
        donor["appearances"][item_id],
        meta,
    )
    entry = {
        "source_item_id": item_id,
        "item_key": key,
        "name": "fixture corpse",
        "owner": "WorldObject",
        "reason": "corpse",
        "identity_projection_sha256": "a" * 64,
        "donor_record_sha256": "b" * 64,
        "official_current_record_sha256": "c" * 64,
        "validated_record_sha256": hashlib.sha256(
            world_objects.canonical_bytes(expected)
        ).hexdigest(),
    }
    args = [sources, donor, entry, binding, definition, False, memberships]
    record = build_catalogue.build_qualified_donor_record(*args)
    check(record == expected, "qualified fixture retains actual donor generation")
    check(
        record["provenance"]["source"]["revision"] != META["revision"],
        "donor is not relabelled as the base generation",
    )
    for label, index, mutation in (
        ("ambiguous binding", 3, lambda value: value.update(disposition="AMBIGUOUS")),
        (
            "wrong source generation",
            3,
            lambda value: value.update(source_revision=META["revision"]),
        ),
        ("materializable Item", 4, lambda value: value.update(materializable=True)),
        ("unproven name", 2, lambda value: value.update(name="different corpse")),
        (
            "reviewed route changed",
            2,
            lambda value: value.update(reason="fixed_carpet"),
        ),
        (
            "record evolution",
            2,
            lambda value: value.update(validated_record_sha256="d" * 64),
        ),
    ):
        changed = copy.deepcopy(args)
        mutation(changed[index])
        check(raises(build_catalogue.build_qualified_donor_record, *changed), label)
    changed = copy.deepcopy(args)
    changed[5] = True
    check(
        raises(build_catalogue.build_qualified_donor_record, *changed),
        "existing Item taxonomy is retained",
    )
    changed = copy.deepcopy(args)
    changed[6][1][item_id][1] = "e" * 64
    check(
        raises(build_catalogue.build_qualified_donor_record, *changed),
        "equal numbers do not override broken appearance identity continuity",
    )
    changed = copy.deepcopy(args)
    changed[0]["items"][item_id] = donor["items"][item_id]
    check(
        raises(build_catalogue.build_qualified_donor_record, *changed),
        "base source owner classification is never replaced by donor",
    )


def test_committed_donor_qualification_and_catalogue():
    qualification = build_catalogue.qualified_donor_routes()
    expected = {row["item_key"]: row for row in qualification["records"]}
    found = {}
    for directory, prefix, _notes in build_catalogue.CATALOGUES.values():
        for path in sorted(build_catalogue.donor_shards()):
            if path.parent != build_catalogue.ROOT / directory:
                continue
            for record in json.loads(path.read_bytes())["records"]:
                key = record["provenance"]["item_pointer"]["key"]
                check(key not in found, "unique donor owner")
                check(key in expected, "no unreviewed donor id")
                check(record["identity"]["family"] == expected[key]["owner"], key)
                check(not world_objects.validate_record(record), key)
                found[key] = record
    check(set(found) == set(expected), "exact reviewed 125-id catalogue scope")
    census = json.loads(build_catalogue.DONOR_CENSUS.read_bytes())
    check(census["records"]["total"] == 125, census)
    check(census["records"]["by_family"] == {"Terrain": 30, "WorldObject": 95}, census)
    check(
        census["qualification_sha256"] == build_catalogue.DONOR_QUALIFICATION_SHA256,
        "census binds exact reviewed qualification",
    )


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_") and callable(fn):
            fn()
    print(json.dumps({"checks": CHECKS, "status": "PASS"}))
