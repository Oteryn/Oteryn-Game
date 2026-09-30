"""Focused checks for engine_items/population_census: digest portability (F-02) and
Delivery Task evidence/decision separation (F-01). Builds tiny fixture checkouts in a
temp dir; no network and no dependency on any real pinned upstream checkout."""

from __future__ import annotations

import hashlib
import json
import tempfile
from pathlib import Path

import engine_items
import population_census
import source_field_catalogs

# --- minimal hand-rolled protobuf encoder (mirrors engine_items' decoder) -----------


def encode_varint(value):
    out = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if value:
            out.append(byte | 0x80)
        else:
            out.append(byte)
            return bytes(out)


def encode_tag(field_number, wire_type):
    return encode_varint((field_number << 3) | wire_type)


def encode_varint_field(field_number, value):
    return encode_tag(field_number, 0) + encode_varint(value)


def encode_bytes_field(field_number, data):
    return encode_tag(field_number, 2) + encode_varint(len(data)) + data


def encode_string_field(field_number, text):
    return encode_bytes_field(field_number, text.encode("utf-8"))


def encode_sprite_info(width, height, sprite_ids):
    parts = [encode_varint_field(1, width), encode_varint_field(2, height)]
    for sprite_id in sprite_ids:
        parts.append(encode_varint_field(5, sprite_id))
    return b"".join(parts)


def encode_frame_group(sprite_info_bytes, fixed=2):
    return encode_varint_field(1, fixed) + encode_bytes_field(3, sprite_info_bytes)


def encode_appearance_object(object_id, name, description, sprite_ids):
    frame_group = encode_frame_group(encode_sprite_info(1, 1, sprite_ids))
    parts = [
        encode_varint_field(1, object_id),
        encode_bytes_field(2, frame_group),
        encode_string_field(4, name),
        encode_string_field(5, description),
    ]
    return b"".join(parts)


def encode_appearances_dat(objects):
    return b"".join(
        encode_bytes_field(1, encode_appearance_object(**obj)) for obj in objects
    )


# --- fixture item ids: real Content World B1 identity-catalog entries, chosen only so
# build_identity_index() (which reads the real committed catalog) resolves them. -----

FIXTURE_ITEM_IDS = (100, 101, 102)


def fixture_items_xml(newline="\n"):
    # "valuables" -> family_profile "material_valuable": simple, non-weapon fields only,
    # so the only residual blockers are the two Delivery Task/sprite ones under test.
    rows = []
    for item_id, weight in ((100, 1000), (101, 1100), (102, 1200)):
        rows.append(
            f'\t<item id="{item_id}" article="a" name="fixture trinket {item_id}">\n'
            f'\t\t<attribute key="primarytype" value="valuables"/>\n'
            f'\t\t<attribute key="description" value="A plain fixture trinket."/>\n'
            f'\t\t<attribute key="weight" value="{weight}"/>\n'
            f"\t</item>"
        )
    text = "<items>\n" + "\n".join(rows) + "\n</items>\n"
    if newline != "\n":
        text = text.replace("\n", newline)
    return text


def fixture_appearances_dat():
    # Each description embeds a literal CRLF (0d 0a) so the binary fixture genuinely
    # contains that byte sequence, proving it must never be treated as normalizable text.
    return encode_appearances_dat(
        [
            {
                "object_id": item_id,
                "name": f"fixture sword {item_id}",
                "description": "line one\r\nline two",
                "sprite_ids": [4000 + item_id],
            }
            for item_id in FIXTURE_ITEM_IDS
        ]
    )


def fixture_crystal_delivery_lua(newline="\n"):
    # 101 is a Crystal delivery-list member; 100/102 are not. 999 is a decoy id absent
    # from the fixture items.xml, matching the shape of the real pinned pool file.
    text = (
        "--[[ fixture Crystal delivery list ]]\n"
        "return {\n"
        "\t{ itemId = 101, count = 1 },\n"
        "\t{ itemId = 999, count = 2 },\n"
        "}\n"
    )
    if newline != "\n":
        text = text.replace("\n", newline)
    return text


def fixture_canary_weekly_lua(newline="\n"):
    # 100 is a weeklyItems member; 999 is a decoy also present in the weeklyItems table.
    # The `id = 555` inside `shopOffers` is a decoy OUTSIDE weeklyItems and must never be
    # parsed as pool membership.
    text = (
        "local M = {}\n"
        "M.config = {\n"
        "\tweekly = {\n"
        "\t\titemSlots = 6,\n"
        "\t},\n"
        "\tweeklyItems = {\n"
        "\t\t{ id = 100, min = 1, max = 2 },\n"
        "\t\t{ id = 999, min = 1, max = 2 },\n"
        "\t},\n"
        "\tshopOffers = {\n"
        '\t\t{ kind = 0, id = 555, name = "decoy" },\n'
        "\t},\n"
        "}\n"
        "return M\n"
    )
    if newline != "\n":
        text = text.replace("\n", newline)
    return text


def sha256_hex(data):
    import hashlib

    return hashlib.sha256(data).hexdigest()


def write_fixture_checkout(root, engine, newline="\n", appearances_bytes=None):
    """Write one engine's fixture checkout under `root`; return its digests map.

    A canary checkout also gets its own copy of the fixture Crystal delivery list at
    the real `DELIVERY_LIST_PATH`, so the same fixture `root` doubles as the
    `--rule-source` checkout the Delivery Task rule requires for canary runs.
    """
    items_text = fixture_items_xml(newline)
    appearances_bytes = (
        fixture_appearances_dat() if appearances_bytes is None else appearances_bytes
    )
    items_path = root / "data/items/items.xml"
    appearances_path = root / "data/items/appearances.dat"
    items_path.parent.mkdir(parents=True, exist_ok=True)
    items_path.write_bytes(items_text.encode("utf-8"))
    appearances_path.write_bytes(appearances_bytes)

    if engine == "crystal":
        pool_relative = engine_items.DELIVERY_LIST_PATH
        pool_text = fixture_crystal_delivery_lua(newline)
    else:
        pool_relative = engine_items.CANARY_WEEKLY_ITEMS_PATH
        pool_text = fixture_canary_weekly_lua(newline)
    pool_path = root / pool_relative
    pool_path.parent.mkdir(parents=True, exist_ok=True)
    pool_bytes = pool_text.encode("utf-8")
    pool_path.write_bytes(pool_bytes)

    lf_items = items_text.replace("\r\n", "\n").encode("utf-8")
    lf_pool = pool_text.replace("\r\n", "\n").encode("utf-8")
    digests = {
        "data/items/items.xml": sha256_hex(lf_items),
        "data/items/appearances.dat": sha256_hex(appearances_bytes),
        pool_relative: sha256_hex(lf_pool),
    }

    if engine == "canary":
        crystal_list_text = fixture_crystal_delivery_lua(newline)
        crystal_list_path = root / engine_items.DELIVERY_LIST_PATH
        crystal_list_path.parent.mkdir(parents=True, exist_ok=True)
        crystal_list_path.write_bytes(crystal_list_text.encode("utf-8"))
        lf_crystal_list = crystal_list_text.replace("\r\n", "\n").encode("utf-8")
        digests[engine_items.DELIVERY_LIST_PATH] = sha256_hex(lf_crystal_list)

    return digests


def load_fixture_sources(
    root, engine, newline="\n", appearances_bytes=None, overrides_path=None
):
    digests = write_fixture_checkout(
        root, engine, newline=newline, appearances_bytes=appearances_bytes
    )
    rule_source = root if engine == "canary" else None
    rule_source_digest = (
        digests[engine_items.DELIVERY_LIST_PATH] if engine == "canary" else None
    )
    return engine_items.load_engine_sources(
        engine,
        root,
        digests=digests,
        rule_source=rule_source,
        rule_source_digest=rule_source_digest,
        overrides_path=overrides_path,
    )


# Real Content World B1 identity-catalog keys for FIXTURE_ITEM_IDS (100, 101, 102), used
# by the override tests below. The fixture Crystal delivery list (`itemId = 101`) makes
# 101 the only fixture item the rule admits without an override.
FIXTURE_ITEM_KEYS = {
    100: "oteryn:item.tibia.i100",
    101: "oteryn:item.tibia.i101",
    102: "oteryn:item.tibia.i102",
}


def write_overrides_file(path, overrides):
    path.write_text(
        json.dumps(
            {
                "schema": engine_items.DELIVERY_OVERRIDES_SCHEMA,
                "rule": engine_items.RULE_ID,
                "overrides": overrides,
            }
        ),
        encoding="utf-8",
    )


CHECKS = 0


def check(condition, message):
    global CHECKS
    CHECKS += 1
    if not condition:
        raise AssertionError(message)


# --- synthetic sources: real identity index + real disposition catalog, fabricated
# items/appearances. This skips the items.xml/appearances.dat round trip entirely
# (no network, no engine checkout) while still exercising the real
# `engine_items.convert_item` field-mapping code against the real pinned catalogs. -----


def synthetic_sources(engine, item_records):
    """`item_records`: {item_id: {"attrs": {...}, "flags": {...}, "name": ..., ...}}."""
    profile = (
        engine_items.CRYSTAL_PROFILE
        if engine == "crystal"
        else engine_items.CANARY_PROFILE
    )
    items = {}
    appearances = {}
    for item_id, record in item_records.items():
        items[item_id] = {
            "name": record.get("name", f"synthetic item {item_id}"),
            "article": record.get("article", "a"),
            "plural": record.get("plural"),
            "attrs": record.get("attrs", {}),
        }
        if "flags" in record or "appearance_name" in record:
            appearances[item_id] = {
                "id": item_id,
                "flags": record.get("flags", {}),
                "frame_groups": record.get("frame_groups", []),
                "name": record.get("appearance_name"),
                "description": record.get("appearance_description"),
            }
    return {
        "engine": engine,
        "profile": profile,
        "repository": engine_items.ENGINES[engine]["repository"],
        "revision": engine_items.ENGINES[engine]["revision"],
        "items": items,
        "appearances": appearances,
        "identity_index": engine_items.build_identity_index(),
        "delivery_member_ids": set(),
        "delivery_source": engine_items.DELIVERY_SOURCE_NAME[engine],
        "crystal_list_ids": set(),
        "crystal_list_entries": 0,
        "delivery_overrides": {},
        "disposition": engine_items.load_disposition_catalog(profile),
    }


def convert(item_records, item_id=200, engine="crystal"):
    sources = synthetic_sources(engine, item_records)
    return engine_items.convert_item(sources, item_id)


def test_value_dependent_type_field():
    # mapped route: exact passthrough into taxonomy/item_class.
    item, _deps, report = convert(
        {200: {"attrs": {"primarytype": "valuables", "type": "key"}}}
    )
    check(item["taxonomy"]["item_class"] == "key", item)
    check(report["field_status"]["type"] == "mapped", report)
    check(
        not any(
            b.startswith(("unresolved:type", "converter_missing:type"))
            for b in report["blockers"]
        ),
        report,
    )

    # external_domain route (WorldObject/Terrain type): routed, never a blocker, and the
    # taxonomy override is not applied (no `destination_value_equals` on that route).
    item, _deps, report = convert(
        {201: {"attrs": {"primarytype": "valuables", "type": "door"}}}, item_id=201
    )
    check(report["field_status"]["type"] == "routed", report)
    check(item["taxonomy"]["item_class"] != "door", item)

    # a value the catalog does not authorize at all stays a precise, value-labeled blocker.
    item, _deps, report = convert(
        {202: {"attrs": {"primarytype": "valuables", "type": "not-a-real-value"}}},
        item_id=202,
    )
    check(report["field_status"]["type"] == "unresolved", report)
    check("unresolved:type=not-a-real-value" in report["blockers"], report)


def test_value_dependent_weapontype_field():
    # mapped route on a non-weapon-profile item (shield): weapon_type is still applied.
    item, _deps, report = convert(
        {
            210: {
                "attrs": {
                    "primarytype": "shields",
                    "weapontype": "shield",
                    "defense": "10",
                }
            }
        },
        item_id=210,
    )
    check(item["family_profile"] == "equipment_offhand", item)
    check(item["weapon"]["weapon_type"] == "shield", item)
    check(report["field_status"]["weapontype"] == "mapped", report)

    # the engine-recognized value "ammunition" is mapped.
    item, _deps, report = convert(
        {
            211: {
                "attrs": {"primarytype": "distance weapons", "weapontype": "ammunition"}
            }
        },
        item_id=211,
    )
    check(item["weapon"]["weapon_type"] == "ammunition", item)
    check(report["field_status"]["weapontype"] == "mapped", report)

    # "ammo" is not a key in the engine's own WeaponTypesMap (it logs "Unknown
    # weaponType" and leaves weaponType at WEAPON_NONE), so it is routed as
    # pinned-no-effect: never a blocker, and no weapon_type is fabricated.
    item, _deps, report = convert(
        {212: {"attrs": {"primarytype": "distance weapons", "weapontype": "ammo"}}},
        item_id=212,
    )
    check(report["field_status"]["weapontype"] == "routed", report)
    check(not report["blockers"], report)
    check("weapon" not in item, item)

    # "rod" is likewise not an engine-recognized WeaponTypesMap key.
    item, _deps, report = convert(
        {213: {"attrs": {"primarytype": "distance weapons", "weapontype": "rod"}}},
        item_id=213,
    )
    check(report["field_status"]["weapontype"] == "routed", report)
    check(not report["blockers"], report)
    check("weapon" not in item, item)


def test_value_dependent_action_field():
    # mapped route applied when a weapon object already exists.
    item, _deps, report = convert(
        {
            220: {
                "attrs": {
                    "primarytype": "sword weapons",
                    "weapontype": "sword",
                    "action": "removecharge",
                }
            }
        },
        item_id=220,
    )
    check(item["weapon"]["consumption_mode"] == "consume_charge", item)
    check(report["field_status"]["action"] == "mapped", report)

    # a mapped route with nowhere to land (no weapon object) is a precise blocker rather
    # than fabricating a weapon object just to hold it.
    item, _deps, report = convert(
        {221: {"attrs": {"primarytype": "valuables", "action": "removecount"}}},
        item_id=221,
    )
    check("weapon" not in item, item)
    check(report["field_status"]["action"] == "converter_missing", report)
    check("converter_missing:action" in report["blockers"], report)


def test_value_dependent_eventtype_field():
    # mapped route ("equip" -> activation_events contains "equip") on an equippable item.
    item, _deps, report = convert(
        {
            230: {
                "attrs": {
                    "primarytype": "helmets",
                    "slot": "head",
                    "eventtype": "equip",
                }
            }
        },
        item_id=230,
    )
    check(item["equipment"]["activation_events"] == ["equip"], item)
    check(report["field_status"]["eventtype"] == "mapped", report)

    # external_domain route: routed, never a blocker.
    item, _deps, report = convert(
        {
            231: {
                "attrs": {
                    "primarytype": "helmets",
                    "slot": "head",
                    "eventtype": "stepin",
                }
            }
        },
        item_id=231,
    )
    check("activation_events" not in item.get("equipment", {}), item)
    check(report["field_status"]["eventtype"] == "routed", report)


def test_flags_unmove_boolean_invert():
    item, _deps, report = convert(
        {240: {"attrs": {"primarytype": "valuables"}, "flags": {"flags.unmove": True}}},
        item_id=240,
    )
    check(item["physical"]["movable"] is False, item)
    check(report["field_status"]["flags.unmove"] == "mapped", report)

    item, _deps, _report = convert(
        {
            241: {
                "attrs": {"primarytype": "valuables"},
                "flags": {"flags.unmove": False},
            }
        },
        item_id=241,
    )
    check(item["physical"]["movable"] is True, item)

    # an explicit engine-level immovable fact wins over a conflicting xml `movable=1`.
    item, _deps, _report = convert(
        {
            242: {
                "attrs": {"primarytype": "valuables", "movable": "1"},
                "flags": {"flags.unmove": True},
            }
        },
        item_id=242,
    )
    check(item["physical"]["movable"] is False, item)


def test_flags_container_guarded_on_capacity():
    item, _deps, report = convert(
        {
            250: {
                "attrs": {"primarytype": "containers", "containersize": "20"},
                "flags": {"flags.container": True},
            }
        },
        item_id=250,
    )
    check(item["container"] == {"capacity": 20, "content_kind": "items"}, item)
    check(report["field_status"]["flags.container"] == "mapped", report)

    # flags.container without a containersize has no admitted capacity default: a
    # precise blocker, not a fabricated capacity.
    item, _deps, report = convert(
        {
            251: {
                "attrs": {"primarytype": "valuables"},
                "flags": {"flags.container": True},
            }
        },
        item_id=251,
    )
    check("container" not in item, item)
    check(report["field_status"]["flags.container"] == "converter_missing", report)
    check("converter_missing:flags.container" in report["blockers"], report)


def test_flags_cumulative_stack():
    item, _deps, report = convert(
        {
            260: {
                "attrs": {"primarytype": "valuables"},
                "flags": {"flags.cumulative": True},
            }
        },
        item_id=260,
    )
    check(item["stack"] == {"stackable": True, "max_count": 100}, item)
    check(report["field_status"]["flags.cumulative"] == "mapped", report)

    item, _deps, _report = convert(
        {
            261: {
                "attrs": {"primarytype": "valuables"},
                "flags": {"flags.cumulative": False},
            }
        },
        item_id=261,
    )
    check(item["stack"] == {"stackable": False, "max_count": 1}, item)


def test_unproperly_level_magic_shortfall():
    item, _deps, report = convert(
        {
            270: {
                "attrs": {
                    "primarytype": "sword weapons",
                    "weapontype": "sword",
                    "unproperly": "true",
                }
            }
        },
        item_id=270,
    )
    check(
        item["requirements"]["level_magic_shortfall"]
        == {
            "policy": "allow_with_multiplicative_damage_penalty",
            "damage_multiplier_per_failed_check": {"numerator": 1, "denominator": 2},
        },
        item,
    )
    check(report["field_status"]["unproperly"] == "mapped", report)


def test_presentation_display_flags_and_assets():
    item, _deps, report = convert(
        {
            280: {
                "attrs": {
                    "primarytype": "valuables",
                    "showcount": "1",
                    "showattributes": "1",
                    "showduration": "1",
                    "effect": "bluebubble",
                    "shoottype": "arrow",
                    "meleeattackeffect": "monkstaff",
                },
                "flags": {
                    "flags.expire": True,
                    "flags.wearout": True,
                },
            }
        },
        item_id=280,
    )
    display_flags = item["presentation"]["display_flags"]
    check(display_flags["stack_count"] is True, display_flags)
    check(display_flags["attributes"] is True, display_flags)
    check(display_flags["duration"] is True, display_flags)
    check(display_flags["client_expiry_timer"] is True, display_flags)
    check(display_flags["client_wear_counter"] is True, display_flags)
    check(
        item["presentation"]["effects"] == ["crystal.appearance:effect/bluebubble"],
        item,
    )
    check(
        item["presentation"]["projectile_effect"]
        == "crystal.appearance:projectile/arrow",
        item,
    )
    check(
        item["presentation"]["attack_effect"]
        == "crystal.appearance:melee-attack-effect/monkstaff",
        item,
    )
    for field in (
        "showcount",
        "showattributes",
        "showduration",
        "effect",
        "shoottype",
        "meleeattackeffect",
        "flags.expire",
        "flags.wearout",
    ):
        check(report["field_status"][field] == "mapped", (field, report))


def test_loottype_tags():
    item, _deps, report = convert(
        {290: {"attrs": {"primarytype": "valuables", "loottype": "creature products"}}},
        item_id=290,
    )
    check(item["taxonomy"]["tags"] == ["creature_products"], item)
    check(report["field_status"]["loottype"] == "mapped", report)

    # a value that slugs to nothing usable stays a precise blocker.
    item, _deps, report = convert(
        {291: {"attrs": {"primarytype": "valuables", "loottype": "!!!"}}}, item_id=291
    )
    check("tags" not in item["taxonomy"], item)
    check(report["field_status"]["loottype"] == "converter_missing", report)


def test_stopduration_guarded_on_temporal():
    item, _deps, report = convert(
        {
            300: {
                "attrs": {
                    "primarytype": "food",
                    "decayto": "301",
                    "duration": "60",
                    "stopduration": "1",
                }
            }
        },
        item_id=300,
    )
    check(item["temporal"]["stop_duration_while_unequipped"] is True, item)
    check(report["field_status"]["stopduration"] == "mapped", report)

    # stopduration with no decayto/duration pair has no /item/temporal to attach to.
    item, _deps, report = convert(
        {302: {"attrs": {"primarytype": "food", "stopduration": "1"}}}, item_id=302
    )
    check("temporal" not in item, item)
    check(report["field_status"]["stopduration"] == "converter_missing", report)


def test_flags_liquidcontainer_fluid_role():
    item, _deps, report = convert(
        {
            310: {
                "attrs": {"primarytype": "fluid containers", "containersize": "1"},
                "flags": {"flags.liquidcontainer": True},
            }
        },
        item_id=310,
    )
    check(item["fluid"] == {"role": "container"}, item)
    check(report["field_status"]["flags.liquidcontainer"] == "mapped", report)


def test_changedtoexpire_corroborates_decayto():
    _item, _deps, report = convert(
        {
            320: {
                "attrs": {"primarytype": "food", "decayto": "104", "duration": "60"},
                "flags": {
                    "flags.changedtoexpire": True,
                    "changedtoexpire.former_object_typeid": 104,
                },
            }
        },
        item_id=320,
    )
    check(report["field_status"]["flags.changedtoexpire"] == "mapped", report)
    check(
        report["field_status"]["changedtoexpire.former_object_typeid"] == "mapped",
        report,
    )

    # a mismatched former_object_typeid cannot corroborate the decayto transform and
    # /item/temporal has no source duration of its own to build from: a precise blocker.
    _item, _deps, report = convert(
        {
            321: {
                "attrs": {"primarytype": "food", "decayto": "104", "duration": "60"},
                "flags": {
                    "flags.changedtoexpire": True,
                    "changedtoexpire.former_object_typeid": 105,
                },
            }
        },
        item_id=321,
    )
    check(
        report["field_status"]["flags.changedtoexpire"] == "converter_missing", report
    )
    check(
        report["field_status"]["changedtoexpire.former_object_typeid"]
        == "converter_missing",
        report,
    )


def test_flags_dual_wielding():
    item, _deps, report = convert(
        {
            330: {
                "attrs": {
                    "primarytype": "fist weapons",
                    "weapontype": "fist",
                    "slot": "hand",
                },
                "flags": {"flags.dual_wielding": True},
            }
        },
        item_id=330,
    )
    check(item["equipment"]["dual_wielding"] is True, item)
    check(report["field_status"]["flags.dual_wielding"] == "mapped", report)


def test_readable_write_and_write_once():
    # flags.write (appearance) proves readability only, never writability: both
    # engines' items.cpp set canReadText from has_write()/has_write_once() presence
    # alone and immediately discard the nested max_text_length(_once) value on the next
    # line, so write.max_text_length has no engine effect either.
    item, _deps, report = convert(
        {
            340: {
                "attrs": {"primarytype": "documents and papers"},
                "flags": {"flags.write": True, "write.max_text_length": 200},
            }
        },
        item_id=340,
    )
    check(item["readable"]["readable"] is True, item)
    check(item["readable"]["writable"] is False, item)
    check(item["readable"]["write_policy"] == "none", item)
    check(report["field_status"]["flags.write"] == "mapped", report)
    check(report["field_status"]["write.max_text_length"] == "routed", report)

    # writeable+maxtextlen items.xml attributes: a plain rewrite policy, no target
    # required.
    item, _deps, report = convert(
        {
            341: {
                "attrs": {
                    "primarytype": "documents and papers",
                    "writeable": "1",
                    "maxtextlen": "200",
                }
            }
        },
        item_id=341,
    )
    check(item["readable"]["writable"] is True, item)
    check(item["readable"]["write_policy"] == "rewrite", item)
    check(item["readable"]["max_characters"] == 200, item)
    check(report["field_status"]["writeable"] == "mapped", report)

    # writeable+maxtextlen+writeonceitemid items.xml attributes with a resolvable
    # target: applied in full.
    item, _deps, report = convert(
        {
            342: {
                "attrs": {
                    "primarytype": "documents and papers",
                    "writeable": "1",
                    "maxtextlen": "50",
                    "writeonceitemid": "104",
                },
                "flags": {
                    "flags.write_once": True,
                    "write_once.max_text_length_once": 50,
                },
            }
        },
        item_id=342,
    )
    check(item["readable"]["write_policy"] == "write_once", item)
    check(
        item["readable"]["write_once_target"]["key"]
        == engine_items.item_ref(engine_items.build_identity_index(), 104)["key"],
        item,
    )
    check(report["field_status"]["writeonceitemid"] == "mapped", report)
    check(report["field_status"]["flags.write_once"] == "mapped", report)
    check(report["field_status"]["write_once.max_text_length_once"] == "routed", report)

    # flags.write_once (appearance) with no XML writeable/writeonceitemid attribute at
    # all: this converter never treats the appearance flag as proof of writability or
    # of write-once intent, so the item is readable but not writable, and neither field
    # is blocked (the 54 real converter_missing:flags.write_once occurrences this
    # replaces).
    item, _deps, report = convert(
        {
            343: {
                "attrs": {"primarytype": "documents and papers"},
                "flags": {
                    "flags.write_once": True,
                    "write_once.max_text_length_once": 50,
                },
            }
        },
        item_id=343,
    )
    check(item["readable"]["readable"] is True, item)
    check(item["readable"]["writable"] is False, item)
    check(item["readable"]["write_policy"] == "none", item)
    check(report["field_status"]["flags.write_once"] == "mapped", report)
    check(report["field_status"]["write_once.max_text_length_once"] == "routed", report)
    check(not report["blockers"], report)

    # writeable=true but no positive XML maxtextlen: the schema requires
    # max_characters (>=1) whenever writable, and this converter has no value to put
    # there, so this occurrence downgrades to non-writable rather than guessing one.
    item, _deps, report = convert(
        {344: {"attrs": {"primarytype": "documents and papers", "writeable": "1"}}},
        item_id=344,
    )
    check(item["readable"]["writable"] is False, item)
    check(item["readable"]["write_policy"] == "none", item)
    check(report["field_status"]["writeable"] == "converter_missing", report)

    # writeable+maxtextlen+writeonceitemid pointing at a target with no identity
    # binding: write_policy write_once requires a resolvable write_once_target, so this
    # downgrades to non-writable rather than fabricating a target or an unlimited
    # rewrite policy.
    item, _deps, report = convert(
        {
            345: {
                "attrs": {
                    "primarytype": "documents and papers",
                    "writeable": "1",
                    "maxtextlen": "50",
                    "writeonceitemid": "99999999",
                }
            }
        },
        item_id=345,
    )
    check(item["readable"]["writable"] is False, item)
    check(item["readable"]["write_policy"] == "none", item)
    check(report["field_status"]["writeonceitemid"] == "converter_missing", report)


def test_elementalbond_and_reflectdamage():
    item, _deps, report = convert(
        {
            350: {
                "attrs": {
                    "primarytype": "fist weapons",
                    "weapontype": "fist",
                    "elementalbond": "energy",
                }
            }
        },
        item_id=350,
    )
    check(item["modifiers"]["elemental_bond"] == {"damage_type": "energy"}, item)
    check(report["field_status"]["elementalbond"] == "mapped", report)

    # a damage_type outside the schema's restricted enum has no admitted mapping.
    item, _deps, report = convert(
        {351: {"attrs": {"primarytype": "valuables", "elementalbond": "fire"}}},
        item_id=351,
    )
    check("elemental_bond" not in item.get("modifiers", {}), item)
    check(report["field_status"]["elementalbond"] == "converter_missing", report)

    item, _deps, report = convert(
        {
            352: {
                "attrs": {
                    "primarytype": "shields",
                    "weapontype": "shield",
                    "reflectdamage": "42",
                }
            }
        },
        item_id=352,
    )
    check(
        item["modifiers"]["reflection"]
        == [
            {"damage_type": "physical", "percent": {"numerator": 42, "denominator": 1}}
        ],
        item,
    )
    check(report["field_status"]["reflectdamage"] == "mapped", report)


def test_chain_weapon_coefficient():
    item, _deps, report = convert(
        {
            360: {
                "attrs": {
                    "primarytype": "axe weapons",
                    "weapontype": "axe",
                    "chain": "0.9",
                }
            }
        },
        item_id=360,
    )
    check(
        item["weapon"]["chain"]
        == {
            "mode": "override",
            "skill_formula_coefficient": {"numerator": 9, "denominator": 10},
        },
        item,
    )
    check(report["field_status"]["chain"] == "mapped", report)

    # chain with no resolvable weapon object stays a precise blocker.
    item, _deps, report = convert(
        {361: {"attrs": {"primarytype": "valuables", "chain": "0.5"}}}, item_id=361
    )
    check("weapon" not in item, item)
    check(report["field_status"]["chain"] == "converter_missing", report)


def test_proficiency_id_238_cites_both_admitted_crosswalk_entries():
    """proficiency_profiles.py already holds the admitted dual-engine Magic Sword
    crosswalk (238/version 3, both Canary and Crystal source identities), pinned and
    vetted out of band, so a single-engine converter run can cite both entries for it
    without needing the other engine's own evidence at conversion time."""
    item, deps, report = convert(
        {
            360: {
                "attrs": {"primarytype": "valuables"},
                "flags": {"proficiency.proficiency_id": 238},
            }
        },
        item_id=360,
    )
    check(item["proficiency"] == engine_items.magic_sword_proficiency(), item)
    check(report["field_status"]["proficiency.proficiency_id"] == "mapped", report)
    check(not report["blockers"], report)
    crosswalk_sources = {
        (
            crosswalk["source_profile"],
            crosswalk["external_id"],
            crosswalk["source_version"],
        )
        for crosswalk in deps["proficiency_crosswalks"]
    }
    check(
        crosswalk_sources == engine_items.MAGIC_SWORD_PROFICIENCY_SOURCE_IDENTITIES,
        deps,
    )
    for crosswalk in deps["proficiency_crosswalks"]:
        check(
            crosswalk["target"] == engine_items.MAGIC_SWORD_PROFICIENCY_REF, crosswalk
        )
    defined_keys = {ref["key"] for ref in deps["definitions"]}
    check(
        {
            engine_items.MAGIC_SWORD_PROFICIENCY_REF["key"],
            engine_items.INTENSE_WOUND_CLEANSING_REF["key"],
            engine_items.BERSERK_REF["key"],
        }
        <= defined_keys,
        deps,
    )


def test_proficiency_id_stays_a_precise_blocker_for_other_ids():
    """Every other proficiency id has no admitted canonical crosswalk at all."""
    item, _deps, report = convert(
        {
            361: {
                "attrs": {"primarytype": "valuables"},
                "flags": {"proficiency.proficiency_id": 999},
            }
        },
        item_id=361,
    )
    check("proficiency" not in item, item)
    check(
        report["field_status"]["proficiency.proficiency_id"] == "converter_missing",
        report,
    )
    check("proficiency_crosswalk_not_admitted:999" in report["blockers"], report)


def test_forge_max_tier_from_classification_table():
    # flags.upgradeclassification + the nested classification level: max_tier is the
    # highest tier key `item_tiers.lua` registers for that classification, never a
    # source fact (classification 1->1, 2->2, 3->3, 4->10).
    for classification, expected_max_tier in (1, 1), (2, 2), (3, 3), (4, 10):
        item_id = 699 + classification
        item, _deps, report = convert(
            {
                item_id: {
                    "attrs": {"primarytype": "valuables"},
                    "flags": {
                        "flags.upgradeclassification": True,
                        "upgradeclassification.upgrade_classification": classification,
                    },
                }
            },
            item_id=item_id,
        )
        check(
            item["forge"]
            == {
                "classification": classification,
                "max_tier": expected_max_tier,
            },
            item,
        )
        check(report["field_status"]["flags.upgradeclassification"] == "mapped", report)
        check(
            report["field_status"]["upgradeclassification.upgrade_classification"]
            == "mapped",
            report,
        )
        check(not report["blockers"], report)

    # a classification the pinned Lua table never registers (only 1-4 exist) has no
    # admitted max_tier: this converter has no value to invent, so it stays blocked
    # rather than guessing.
    item, _deps, report = convert(
        {
            380: {
                "attrs": {"primarytype": "valuables"},
                "flags": {
                    "flags.upgradeclassification": True,
                    "upgradeclassification.upgrade_classification": 5,
                },
            }
        },
        item_id=380,
    )
    check("forge" not in item, item)
    check(
        report["field_status"]["flags.upgradeclassification"] == "converter_missing",
        report,
    )
    check(
        report["field_status"]["upgradeclassification.upgrade_classification"]
        == "converter_missing",
        report,
    )


def test_mantra_damage_types_fixed_engine_constant():
    # damage_types is not a source fact: Crystal's parseMantra adds the raw points
    # value to mantraAbsorbValue[energy/fire/earth/ice] unconditionally, and Canary's
    # Combat::applyMantraAbsorb applies mantra only to those same four combat types.
    item, _deps, report = convert(
        {382: {"attrs": {"primarytype": "valuables", "mantra": "10"}}}, item_id=382
    )
    check(
        item["modifiers"]["mantra"]
        == {"points": 10, "damage_types": ["energy", "fire", "earth", "ice"]},
        item,
    )
    check(report["field_status"]["mantra"] == "mapped", report)
    check(not report["blockers"], report)


def test_fields_with_no_admitted_engine_data_stay_blocked():
    """Rule 1 cases this converter deliberately does not implement: the schema needs
    data (an Ability identity crosswalk, or an augment target/value kind crosswalk)
    that neither pinned engine's evidence supplies."""
    item, _deps, report = convert(
        {381: {"attrs": {"primarytype": "food", "runespellname": "adito grav"}}},
        item_id=381,
    )
    check("use" not in item or "ability" not in item.get("use", {}), item)
    check(report["field_status"]["runespellname"] == "converter_missing", report)

    # flags.forceuse is UNRESOLVED with no source_value_routes at all: unchanged.
    item, _deps, report = convert(
        {
            383: {
                "attrs": {"primarytype": "valuables"},
                "flags": {"flags.forceuse": True},
            }
        },
        item_id=383,
    )
    check(report["field_status"]["flags.forceuse"] == "unresolved", report)
    check("unresolved:flags.forceuse" in report["blockers"], report)


def test_routed_non_item_corpse_and_placeholder_and_terrain():
    # flags.corpse/flags.player_corpse: routed to WorldObject, never a converter failure.
    item, _deps, report = convert(
        {390: {"attrs": {}, "flags": {"flags.corpse": True}}}, item_id=390
    )
    check(item is None, report)
    check(report["converted"] is False, report)
    check(
        report["routed_non_item"] == {"owner": "WorldObject", "reason": "corpse"},
        report,
    )
    check(report["blockers"] == [], report)

    item, _deps, report = convert(
        {391: {"attrs": {}, "flags": {"flags.player_corpse": True}}}, item_id=391
    )
    check(report["routed_non_item"]["owner"] == "WorldObject", report)

    # a reserved sprite-sheet placeholder with no other attribute: routed, not unresolved.
    item, _deps, report = convert(
        {392: {"attrs": {}, "name": "RESERVED SPRITE"}}, item_id=392
    )
    check(
        report["routed_non_item"]
        == {"owner": "WorldObject", "reason": "appearance_placeholder_slot"},
        report,
    )

    # a Terrain-owned primarytype: routed, never family_profile_unresolved.
    item, _deps, report = convert(
        {393: {"attrs": {"primarytype": "artificial tiles"}}}, item_id=393
    )
    check(
        report["routed_non_item"]
        == {"owner": "Terrain", "reason": "primarytype_world_object"},
        report,
    )
    check("family_profile_unresolved" not in report["blockers"], report)

    # a name that merely contains "reserved sprite" as a substring, or one with other
    # attributes present, is not a placeholder slot and proceeds normally.
    item, _deps, report = convert(
        {
            394: {
                "attrs": {"primarytype": "valuables"},
                "name": "reserved sprite of doom",
            }
        },
        item_id=394,
    )
    check(report.get("routed_non_item") is None, report)
    check(report["converted"] is True, report)


def test_routed_non_item_unmove_map_geometry():
    # unmove=true + a ground/border flag, no resolvable family: Terrain/ground_or_border.
    item, _deps, report = convert(
        {1395: {"attrs": {}, "flags": {"flags.unmove": True, "flags.bank": True}}},
        item_id=1395,
    )
    check(item is None, report)
    check(report["converted"] is False, report)
    check(
        report["routed_non_item"] == {"owner": "Terrain", "reason": "ground_or_border"},
        report,
    )
    check("family_profile_unresolved" not in report["blockers"], report)

    item, _deps, report = convert(
        {396: {"attrs": {}, "flags": {"flags.unmove": True, "flags.fullbank": True}}},
        item_id=396,
    )
    check(
        report["routed_non_item"] == {"owner": "Terrain", "reason": "ground_or_border"},
        report,
    )

    item, _deps, report = convert(
        {397: {"attrs": {}, "flags": {"flags.unmove": True, "flags.clip": True}}},
        item_id=397,
    )
    check(
        report["routed_non_item"] == {"owner": "Terrain", "reason": "ground_or_border"},
        report,
    )

    # unmove=true with no ground/border flag (a wall/top decoration/door-style entry)
    # and no resolvable family: WorldObject/immovable_unclassified.
    item, _deps, report = convert(
        {398: {"attrs": {}, "flags": {"flags.unmove": True, "flags.bottom": True}}},
        item_id=398,
    )
    check(
        report["routed_non_item"]
        == {"owner": "WorldObject", "reason": "immovable_unclassified"},
        report,
    )

    # negative: unmove=true with a resolvable family still converts as an ordinary Item,
    # movable false, and is never routed away from Item.
    item, _deps, report = convert(
        {
            399: {
                "attrs": {"primarytype": "valuables"},
                "flags": {"flags.unmove": True},
            }
        },
        item_id=399,
    )
    check(report.get("routed_non_item") is None, report)
    check(report["converted"] is True, report)
    check(item["physical"]["movable"] is False, item)

    # negative: no unmove, no resolvable family, and no appearance object at all --
    # routed WorldObject/no_client_appearance (task h), never left in editorial backlog.
    item, _deps, report = convert({450: {"attrs": {}}}, item_id=450)
    check(
        report["routed_non_item"]
        == {"owner": "WorldObject", "reason": "no_client_appearance"},
        report,
    )
    check(report["converted"] is False, report)


def test_wrap_target_inheritance_resolves():
    # Owner decision 2026-09-28: an unresolved item's own `wrapableto` names another
    # items.xml id whose own `primarytype` resolves through `PRIMARYTYPE_PROFILE`.
    item, _deps, report = convert(
        {
            460: {"attrs": {"wrapableto": "90001"}},
            90001: {"attrs": {"primarytype": "furniture"}},
        },
        item_id=460,
    )
    check(item is not None, report)
    check(item["family_profile"] == "decoration", item)
    check(item["family_profile_basis"] == "engine_wrap_target", item)
    check(
        item["family_profile_evidence"]
        == {"wrap_target_id": 90001, "wrap_target_primarytype": "furniture"},
        item,
    )
    check(report["family_profile_basis"] == "engine_wrap_target", report)


def test_wrap_target_inheritance_unresolved_target_stays_unresolved():
    # wrapableto names an id absent from items.xml entirely. None of these fixtures
    # carries an appearance object, so once wrap-target inheritance itself fails to
    # resolve them, the task h catch-all routes them WorldObject/no_client_appearance
    # rather than leaving them in `family_profile_unresolved` -- the point of each
    # check below is that wrap-target inheritance never guesses a family, not the
    # exact terminal bucket.
    item, _deps, report = convert(
        {461: {"attrs": {"wrapableto": "90099"}}}, item_id=461
    )
    check(item is None, report)
    check(report["converted"] is False, report)
    check("family_profile_basis" not in report, report)

    # wrapableto names a real id whose own primarytype is not admitted.
    item, _deps, report = convert(
        {
            462: {"attrs": {"wrapableto": "90002"}},
            90002: {"attrs": {"primarytype": "others"}},
        },
        item_id=462,
    )
    check(item is None, report)
    check("family_profile_basis" not in report, report)

    # wrapableto names a real id with no primarytype attribute at all.
    item, _deps, report = convert(
        {463: {"attrs": {"wrapableto": "90003"}}, 90003: {"attrs": {}}}, item_id=463
    )
    check(item is None, report)
    check("family_profile_basis" not in report, report)


def test_wrap_target_never_overrides_already_resolved_item():
    # Engine-resolved (this item's own primarytype) wins even though its wrapableto also
    # resolves through the target's primarytype.
    item, _deps, report = convert(
        {
            464: {"attrs": {"primarytype": "valuables", "wrapableto": "90001"}},
            90001: {"attrs": {"primarytype": "furniture"}},
        },
        item_id=464,
    )
    check(item["family_profile"] == "material_valuable", item)
    check("family_profile_basis" not in item, item)
    check("family_profile_basis" not in report, report)

    # Wiki-evidence-resolved wins for the same reason: wrap-target inheritance is the
    # lowest-priority resolution, below the wiki fallback.
    key = engine_items.build_identity_index()[1465][0]
    fallback = {
        key: synthetic_wiki_fallback_entry(
            "document", ["wiki wins over wrap"], field="primarytype", value="Books"
        )
    }
    item, _deps, report = convert_with_fallback(
        {
            1465: {"name": "Wiki Wins Over Wrap", "attrs": {"wrapableto": "90001"}},
            90001: {"attrs": {"primarytype": "furniture"}},
        },
        fallback,
        item_id=1465,
    )
    check(item["family_profile"] == "document", item)
    check(item["family_profile_basis"] == "wiki_evidence_fallback", item)


def test_corpse_decoration_routes_without_take():
    # Owner decision 2026-09-28: an unresolved item named "dead ..." with no `flags.take`
    # is a non-take-able map/quest decoration corpse, routed to WorldObject, never an Item.
    item, _deps, report = convert(
        {1466: {"name": "dead dragon", "attrs": {}, "flags": {}}}, item_id=1466
    )
    check(item is None, report)
    check(report["converted"] is False, report)
    check(
        report["routed_non_item"]
        == {"owner": "WorldObject", "reason": "corpse_decoration"},
        report,
    )
    check(report["blockers"] == [], report)


def test_corpse_with_take_in_owner_table():
    # A take-able "dead ..." name in the explicit owner table resolves to its family.
    item, _deps, report = convert(
        {1467: {"name": "dead rat", "attrs": {}, "flags": {"flags.take": True}}},
        item_id=1467,
    )
    check(item is not None, report)
    check(item["family_profile"] == "material_valuable", item)
    check(item["family_profile_basis"] == "owner_name_rule", item)
    check(
        item["family_profile_evidence"]
        == {"rule": "take_able_dead_creature", "name": "dead rat"},
        item,
    )


def test_corpse_takeable_unlisted_name_stays_unresolved():
    # A take-able "dead ..." name absent from the owner table stays unresolved (fail
    # closed) rather than guessed, and is never routed away as a non-Item.
    item, _deps, report = convert(
        {
            1468: {
                "name": "dead unlisted creature",
                "attrs": {},
                "flags": {"flags.take": True},
            }
        },
        item_id=1468,
    )
    check(item is None, report)
    check(report["converted"] is False, report)
    check(report.get("routed_non_item") is None, report)
    check("family_profile_unresolved" in report["blockers"], report)


def test_real_corpse_flag_item_unchanged_by_dead_name_rule():
    # An appearance-flagged corpse whose name also starts with "dead " is routed by the
    # pre-existing flags.corpse rule in `non_item_route`, before family classification
    # (and so before the new dead-name rule) ever runs; unchanged by this owner decision.
    item, _deps, report = convert(
        {
            469: {
                "name": "dead rat",
                "attrs": {},
                "flags": {"flags.corpse": True, "flags.take": True},
            }
        },
        item_id=469,
    )
    check(item is None, report)
    check(
        report["routed_non_item"] == {"owner": "WorldObject", "reason": "corpse"},
        report,
    )


def test_fluid_type_without_appearance_routes_non_item():
    # Owner decision 2026-09-28 (task b): an items.xml-only fluid-kind name (both pinned
    # engines' `Fluids_t` enum, `src/utils/utils_definitions.hpp`) with no appearance
    # object routes non-Item, owner Fluid.
    sources = synthetic_sources("crystal", {470: {"name": "wine", "attrs": {}}})
    check(470 not in sources["appearances"], "fixture must lack an appearance")
    item, _deps, report = engine_items.convert_item(sources, 470)
    check(item is None, report)
    check(
        report["routed_non_item"]
        == {"owner": "Fluid", "reason": "fluid_type_without_appearance"},
        report,
    )
    check(report["blockers"] == [], report)

    # Every admitted fluid name routes the same way.
    for name in sorted(engine_items.FLUID_TYPE_NAMES):
        sources = synthetic_sources("crystal", {2400: {"name": name, "attrs": {}}})
        _item, _deps, report = engine_items.convert_item(sources, 2400)
        check(
            report["routed_non_item"]
            == {"owner": "Fluid", "reason": "fluid_type_without_appearance"},
            (name, report),
        )


def test_fluid_type_with_appearance_is_not_routed():
    # The exact same name WITH an appearance object is a real, physical Item candidate
    # (e.g. a "small flask of wine"-style entry): the fluid-type rule must never apply.
    # A non-empty flag keeps this id out of the unrelated empty-client-object route
    # (task #15) too, so this test stays focused on the fluid rule alone.
    sources = synthetic_sources(
        "crystal", {2401: {"name": "wine", "attrs": {}, "flags": {"flags.take": True}}}
    )
    check(2401 in sources["appearances"], "fixture must carry an appearance")
    _item, _deps, report = engine_items.convert_item(sources, 2401)
    check(report.get("routed_non_item") is None, report)
    check("family_profile_unresolved" in report["blockers"], report)


def test_fluid_type_route_requires_items_xml_record():
    # No items.xml record at all (appearance-only entry): the fluid rule never applies,
    # since it only ever recognizes an items.xml-only name.
    sources = synthetic_sources("crystal", {})
    sources["appearances"][2402] = {
        "id": 2402,
        "flags": {"flags.take": True},
        "frame_groups": [],
        "name": "water",
        "description": None,
    }
    _item, _deps, report = engine_items.convert_item(sources, 2402)
    check(report.get("routed_non_item") is None, report)


def test_appearance_less_non_fluid_name_routes_no_client_appearance():
    # A non-fluid, appearance-less name (e.g. one of the real editorial-backlog engine
    # names such as "bridge"/"hive structure") is never guessed as a fluid; owner
    # decision 2026-09-28 (task h) routes it WorldObject/no_client_appearance instead
    # of leaving it in `family_profile_unresolved` (the pinned client build simply has
    # no sprite for this id).
    sources = synthetic_sources("crystal", {2403: {"name": "bridge", "attrs": {}}})
    check(2403 not in sources["appearances"], "fixture must lack an appearance")
    item, _deps, report = engine_items.convert_item(sources, 2403)
    check(item is None, report)
    check(
        report["routed_non_item"]
        == {"owner": "WorldObject", "reason": "no_client_appearance"},
        report,
    )
    check(report["blockers"] == [], report)


def test_item_with_appearance_never_routed_no_client_appearance():
    # An item that genuinely HAS an appearances.dat object, and resolves nothing else,
    # stays plain `family_profile_unresolved` editorial backlog; task h only ever fires
    # for an id with no appearance object at all. A non-empty flag also keeps this id
    # out of the unrelated empty-client-object route (task #15).
    sources = synthetic_sources(
        "crystal",
        {
            475: {
                "name": "some unresolved name",
                "attrs": {},
                "flags": {"flags.take": True},
            }
        },
    )
    check(475 in sources["appearances"], "fixture must carry an appearance")
    item, _deps, report = engine_items.convert_item(sources, 475)
    check(item is None, report)
    check(report.get("routed_non_item") is None, report)
    check("family_profile_unresolved" in report["blockers"], report)


def test_late_placeholder_names_route_non_item():
    # Owner decision 2026-09-28 (task c): these names are only ever routed once every
    # other classifier has already failed -- unlike `PLACEHOLDER_APPEARANCE_NAMES`, this
    # never gates on "no other items.xml attribute" (see the resolved-item test below for
    # why that early gate is what makes the late rule necessary here).
    for name in sorted(engine_items.LATE_PLACEHOLDER_APPEARANCE_NAMES):
        item, _deps, report = convert(
            {475: {"name": name, "attrs": {"weight": "10"}, "flags": {}}}, item_id=475
        )
        check(item is None, report)
        check(
            report["routed_non_item"]
            == {"owner": "WorldObject", "reason": "appearance_placeholder_slot"},
            (name, report),
        )
        check(report["blockers"] == [], report)


def test_late_placeholder_name_on_already_resolved_item_is_not_rerouted():
    # An id whose name happens to be one of the late-placeholder names, but which the
    # engine's own attributes (or, in production, the wiki-evidence itemid join) already
    # resolve to a real family, must never be rerouted: the late rule only ever runs
    # once family classification has already failed for this exact item.
    item, _deps, report = convert(
        {476: {"name": "event item", "attrs": {"primarytype": "valuables"}}},
        item_id=476,
    )
    check(item is not None, report)
    check(item["family_profile"] == "material_valuable", item)
    check(report.get("routed_non_item") is None, report)

    # Same guarantee through the wiki-evidence fallback specifically (mirrors the real
    # production concern the task called out: an "old tibia item"/"event item" id the
    # snapshot resolves by itemid must never be rerouted to the placeholder owner).
    key = engine_items.build_identity_index()[477][0]
    fallback = {
        key: synthetic_wiki_fallback_entry(
            "decoration", ["old tibia item"], field="primarytype", value="Decorations"
        )
    }
    item, _deps, report = convert_with_fallback(
        {477: {"name": "old tibia item", "attrs": {}}}, fallback, item_id=477
    )
    check(item is not None, report)
    check(item["family_profile"] == "decoration", item)
    check(item["family_profile_basis"] == "wiki_evidence_fallback", item)
    check(report.get("routed_non_item") is None, report)


def test_skip_post_wiki_fallback_routes_probe_flag():
    """`sources["skip_post_wiki_fallback_routes"]` (set only by
    `item_wiki_family_capture.collect_unresolved`'s wiki-OFF probe) must disable every
    rule ranked below the wiki fallback -- wrap-target inheritance, the dead-item rules,
    and the fluid-type/late-placeholder routes -- so the probe never mistakes one of them
    for "this id doesn't need a wiki lookup" and silently excludes an id that still needs
    (and, in production, still prefers) real wiki evidence. This is the bug found and
    fixed while regenerating the wiki-evidence snapshot for this task."""
    # Wrap-target inheritance: normally resolves; with the flag, stays unresolved.
    sources = synthetic_sources(
        "crystal",
        {
            500: {"attrs": {"wrapableto": "90001"}},
            90001: {"attrs": {"primarytype": "furniture"}},
        },
    )
    item, _deps, report = engine_items.convert_item(sources, 500)
    check(item is not None and item["family_profile"] == "decoration", report)
    sources["skip_post_wiki_fallback_routes"] = True
    item, _deps, report = engine_items.convert_item(sources, 500)
    check(item is None, report)
    check("family_profile_unresolved" in report["blockers"], report)

    # Dead-item profile rule: normally resolves; with the flag, stays unresolved.
    sources = synthetic_sources(
        "crystal",
        {501: {"name": "dead rat", "attrs": {}, "flags": {"flags.take": True}}},
    )
    item, _deps, report = engine_items.convert_item(sources, 501)
    check(item is not None and item["family_profile"] == "material_valuable", report)
    sources["skip_post_wiki_fallback_routes"] = True
    item, _deps, report = engine_items.convert_item(sources, 501)
    check(item is None, report)
    check("family_profile_unresolved" in report["blockers"], report)

    # Dead-item route (corpse_decoration): normally routed; with the flag, unresolved.
    sources = synthetic_sources(
        "crystal", {502: {"name": "dead dragon", "attrs": {}, "flags": {}}}
    )
    _item, _deps, report = engine_items.convert_item(sources, 502)
    check(
        report["routed_non_item"]
        == {"owner": "WorldObject", "reason": "corpse_decoration"},
        report,
    )
    sources["skip_post_wiki_fallback_routes"] = True
    _item, _deps, report = engine_items.convert_item(sources, 502)
    check(report.get("routed_non_item") is None, report)
    check("family_profile_unresolved" in report["blockers"], report)

    # Late placeholder name: normally routed; with the flag, unresolved.
    sources = synthetic_sources(
        "crystal", {503: {"name": "old tibia item", "attrs": {}}}
    )
    _item, _deps, report = engine_items.convert_item(sources, 503)
    check(
        report["routed_non_item"]
        == {"owner": "WorldObject", "reason": "appearance_placeholder_slot"},
        report,
    )
    sources["skip_post_wiki_fallback_routes"] = True
    _item, _deps, report = engine_items.convert_item(sources, 503)
    check(report.get("routed_non_item") is None, report)
    check("family_profile_unresolved" in report["blockers"], report)

    # Fluid type without appearance: normally routed; with the flag, unresolved.
    sources = synthetic_sources("crystal", {504: {"name": "wine", "attrs": {}}})
    _item, _deps, report = engine_items.convert_item(sources, 504)
    check(
        report["routed_non_item"]
        == {"owner": "Fluid", "reason": "fluid_type_without_appearance"},
        report,
    )
    sources["skip_post_wiki_fallback_routes"] = True
    _item, _deps, report = engine_items.convert_item(sources, 504)
    check(report.get("routed_non_item") is None, report)
    check("family_profile_unresolved" in report["blockers"], report)

    # No-client-appearance catch-all: normally routed; with the flag, unresolved.
    sources = synthetic_sources("crystal", {505: {"name": "bridge", "attrs": {}}})
    _item, _deps, report = engine_items.convert_item(sources, 505)
    check(
        report["routed_non_item"]
        == {"owner": "WorldObject", "reason": "no_client_appearance"},
        report,
    )
    sources["skip_post_wiki_fallback_routes"] = True
    _item, _deps, report = engine_items.convert_item(sources, 505)
    check(report.get("routed_non_item") is None, report)
    check("family_profile_unresolved" in report["blockers"], report)


def test_family_profile_fallbacks():
    # soul cores: a new PRIMARYTYPE_PROFILE entry.
    item, _deps, report = convert(
        {400: {"attrs": {"primarytype": "soul cores"}}}, item_id=400
    )
    check(item["family_profile"] == "material_valuable", report)

    # clothes-slot fallback: no primarytype/weapontype/containersize, only an appearance
    # clothes slot. Slot 1 = head -> equipment_armor.
    item, _deps, report = convert(
        {
            401: {
                "attrs": {},
                "flags": {"flags.clothes": True, "clothes.slot": 1},
            }
        },
        item_id=401,
    )
    check(item["family_profile"] == "equipment_armor", report)
    check(item["equipment"]["slot"] == "head", item)

    # ambiguous hand slots (5/6) default to equipment_offhand.
    item, _deps, report = convert(
        {
            402: {
                "attrs": {},
                "flags": {"flags.clothes": True, "clothes.slot": 5},
            }
        },
        item_id=402,
    )
    check(item["family_profile"] == "equipment_offhand", report)

    # slot 3 (back) is deliberately left ambiguous: no fallback classification.
    item, _deps, report = convert(
        {
            403: {
                "attrs": {},
                "flags": {"flags.clothes": True, "clothes.slot": 3},
            }
        },
        item_id=403,
    )
    check(report["converted"] is False, report)
    check("family_profile_unresolved" in report["blockers"], report)

    # readable-attrs fallback: no primarytype, but a readable/writable capability names
    # a document.
    item, _deps, report = convert(
        {404: {"attrs": {"writeable": "1", "maxtextlen": "50"}}}, item_id=404
    )
    check(item["family_profile"] == "document", report)

    # the ~heterogeneous tail with no structural signal at all, and no appearance
    # object, is routed WorldObject/no_client_appearance (task h); no name-pattern
    # guess is ever made for its family.
    item, _deps, report = convert({405: {"attrs": {}}}, item_id=405)
    check(report["converted"] is False, report)
    check(
        report["routed_non_item"]
        == {"owner": "WorldObject", "reason": "no_client_appearance"},
        report,
    )


# --- wiki-evidence family fallback (items-family-fallback.json) --------------------


def sample_wiki_source(**overrides):
    base = {
        "wiki_title": "Test Page",
        "page_id": 1,
        "revision_id": 1,
        "revision_timestamp": "2026-01-01T00:00:00Z",
        "url": "https://tibia.fandom.com/wiki/Test_Page",
        "revision_sha1": "a" * 40,
        "content_sha256": "b" * 64,
        "captured_at": "2026-01-01T00:00:00Z",
    }
    base.update(overrides)
    return base


def synthetic_wiki_fallback_entry(
    profile,
    matched_names,
    field=None,
    value=None,
    candidates=None,
    match_basis=None,
    availability=None,
):
    """Build one already-resolved fallback entry, exactly the shape
    `load_wiki_family_fallback` would return, for injecting straight into
    `sources["wiki_family_fallback"]` without a round trip through a JSON file."""
    if candidates is None:
        evidence = {
            "resolution": "direct",
            "field": field,
            "value": value,
            "wiki_source": {"source_id": "fandom", **sample_wiki_source()},
        }
    else:
        evidence = {"resolution": "disambiguation", "candidates": candidates}
    entry = {
        "profile": profile,
        "matched_names": set(matched_names),
        "evidence": evidence,
        "availability": availability,
    }
    # Real fallback entries always carry a `match_basis` (the loader requires it); a
    # test that doesn't care which one it is gets the generic, high-confidence
    # "itemid" default so it still resolves ahead of wrap-target/dead-item, same as
    # every pre-(e)/(g) test always assumed.
    entry["match_basis"] = match_basis if match_basis is not None else "itemid"
    return entry


def convert_with_fallback(item_records, wiki_fallback, item_id=200, engine="crystal"):
    # The wiki fallback only applies to entries with an appearances.dat object. The
    # default flag is non-empty so a record that stays unresolved in one of these tests
    # is never coincidentally caught by the unrelated empty-client-object route (task
    # #15); a test that cares about flags overrides this default explicitly.
    item_records = {
        record_id: {"flags": {"flags.take": True}, **record}
        for record_id, record in item_records.items()
    }
    sources = synthetic_sources(engine, item_records)
    sources["wiki_family_fallback"] = wiki_fallback
    return engine_items.convert_item(sources, item_id)


def build_wiki_fallback_payload(records, schema=None):
    digest = hashlib.sha256(
        json.dumps(
            records, ensure_ascii=False, sort_keys=True, separators=(",", ":")
        ).encode("utf-8")
    ).hexdigest()
    return {
        "schema": schema or engine_items.WIKI_FAMILY_FALLBACK_SCHEMA,
        "batch_id": "test-batch",
        "family": "Item",
        "source": {"source_key": "oteryn:source.tibiawiki"},
        "captured_at": "2026-01-01T00:00:00Z",
        "records": records,
        "snapshot_sha256": digest,
    }


def test_wiki_fallback_direct_hit():
    key = engine_items.build_identity_index()[406][0]
    fallback = {
        key: synthetic_wiki_fallback_entry(
            "decoration",
            ["ancient bonelord santa"],
            field="primarytype",
            value="Decorations",
        )
    }
    item, _deps, report = convert_with_fallback(
        {406: {"name": "Ancient Bonelord Santa", "attrs": {}}}, fallback, item_id=406
    )
    check(item is not None, report)
    check(item["family_profile"] == "decoration", item)
    check(item["family_profile_basis"] == "wiki_evidence_fallback", item)
    check(item["family_profile_evidence"]["field"] == "primarytype", item)
    check(item["family_profile_evidence"]["value"] == "Decorations", item)
    check(report["family_profile_basis"] == "wiki_evidence_fallback", report)


def test_availability_wired_from_wiki_fallback():
    # Owner decision 2026-09-28 (task f): present only when the wiki fallback decided
    # this Item's family_profile and the snapshot carries an availability fact for it.
    key = engine_items.build_identity_index()[416][0]
    availability = {
        "status": "unavailable",
        "evidence": {
            "source": "tibiawiki",
            "page_id": 1,
            "revision_id": 1,
            "wiki_title": "Axe of Mayhem",
            "match_basis": "actualname",
        },
    }
    fallback = {
        key: synthetic_wiki_fallback_entry(
            "weapon_melee",
            ["axe of mayhem"],
            field="primarytype",
            value="Axe Weapons",
            availability=availability,
        )
    }
    item, _deps, report = convert_with_fallback(
        {416: {"name": "axe of mayhem", "attrs": {}}}, fallback, item_id=416
    )
    check(item is not None, report)
    check(item["availability"] == availability, item)


def test_availability_absent_when_wiki_fallback_carries_none():
    key = engine_items.build_identity_index()[417][0]
    fallback = {
        key: synthetic_wiki_fallback_entry(
            "decoration", ["no status thing"], field="primarytype", value="Decorations"
        )
    }
    item, _deps, report = convert_with_fallback(
        {417: {"name": "no status thing", "attrs": {}}}, fallback, item_id=417
    )
    check(item is not None, report)
    check("availability" not in item, item)


def test_availability_absent_when_not_wiki_resolved():
    # An engine-attribute-resolved Item never carries availability, even if its key
    # happens to have an (unused) wiki fallback entry with one.
    key = engine_items.build_identity_index()[418][0]
    availability = {
        "status": "event",
        "evidence": {
            "source": "tibiawiki",
            "page_id": 1,
            "revision_id": 1,
            "wiki_title": "Some Page",
            "match_basis": "title",
        },
    }
    fallback = {
        key: synthetic_wiki_fallback_entry(
            "decoration",
            ["engine wins"],
            field="primarytype",
            value="Decorations",
            availability=availability,
        )
    }
    item, _deps, _report = convert_with_fallback(
        {418: {"name": "Engine Wins", "attrs": {"primarytype": "valuables"}}},
        fallback,
        item_id=418,
    )
    check(item["family_profile"] == "material_valuable", item)
    check("availability" not in item, item)


def test_wiki_fallback_loader_accepts_valid_availability_status():
    key = FIXTURE_ITEM_KEYS[101]
    record = {
        "registry_key": key,
        "matched_names": ["a wall thing"],
        "resolution": "direct",
        "match_basis": "title",
        "field": "primarytype",
        "value": "Decorations",
        "availability_status": "unavailable",
        **sample_wiki_source(),
    }
    payload = build_wiki_fallback_payload({key: record})
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "items-family-fallback.json"
        path.write_text(json.dumps(payload), encoding="utf-8")
        resolved = engine_items.load_wiki_family_fallback(
            path, engine_items.build_identity_index()
        )
        check(resolved[key]["availability"]["status"] == "unavailable", resolved)
        check(
            resolved[key]["availability"]["evidence"]["source"] == "tibiawiki", resolved
        )
        check(
            resolved[key]["availability"]["evidence"]["match_basis"] == "title",
            resolved,
        )


def test_wiki_fallback_loader_rejects_unadmitted_availability_status():
    key = FIXTURE_ITEM_KEYS[101]
    record = {
        "registry_key": key,
        "matched_names": ["a"],
        "resolution": "direct",
        "match_basis": "title",
        "field": "primarytype",
        "value": "Decorations",
        "availability_status": "made_up_status",
        **sample_wiki_source(),
    }
    payload = build_wiki_fallback_payload({key: record})
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "items-family-fallback.json"
        path.write_text(json.dumps(payload), encoding="utf-8")
        raised = False
        try:
            engine_items.load_wiki_family_fallback(
                path, engine_items.build_identity_index()
            )
        except SystemExit:
            raised = True
        check(raised, "an unadmitted availability_status must be rejected")


def test_wiki_fallback_loader_null_availability_status_is_no_availability():
    key = FIXTURE_ITEM_KEYS[101]
    record = {
        "registry_key": key,
        "matched_names": ["a"],
        "resolution": "direct",
        "match_basis": "title",
        "field": "primarytype",
        "value": "Decorations",
        "availability_status": None,
        **sample_wiki_source(),
    }
    payload = build_wiki_fallback_payload({key: record})
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "items-family-fallback.json"
        path.write_text(json.dumps(payload), encoding="utf-8")
        resolved = engine_items.load_wiki_family_fallback(
            path, engine_items.build_identity_index()
        )
        check(resolved[key]["availability"] is None, resolved)


def test_wiki_fallback_name_mismatch_is_ignored():
    # The snapshot resolved a match for a *different* engine name than this item's own;
    # it must never be borrowed for a same-key-but-different-name item.
    key = engine_items.build_identity_index()[407][0]
    fallback = {
        key: synthetic_wiki_fallback_entry(
            "decoration", ["some other name"], field="primarytype", value="Decorations"
        )
    }
    item, _deps, report = convert_with_fallback(
        {407: {"name": "Completely Different Name", "attrs": {}}}, fallback, item_id=407
    )
    check(item is None, report)
    check(report["converted"] is False, report)
    check("family_profile_unresolved" in report["blockers"], report)


def test_wiki_fallback_needs_an_appearance_object():
    # An entry with no appearances.dat object at all (e.g. the fluid-kind name rows
    # 1-20 -- see `test_fluid_type_without_appearance_routes_non_item` for that specific
    # rule instead) is not a physical Item; wiki evidence for a same-named object must
    # not turn it into one -- it correctly falls through to the task h catch-all
    # instead. "gemstone slab" is not a fluid name, keeping this test isolated from the
    # dedicated fluid-type routing rule below.
    key = engine_items.build_identity_index()[409][0]
    fallback = {
        key: synthetic_wiki_fallback_entry(
            "decoration", ["gemstone slab"], field="primarytype", value="Decorations"
        )
    }
    sources = synthetic_sources(
        "crystal", {409: {"name": "gemstone slab", "attrs": {}}}
    )
    sources["wiki_family_fallback"] = fallback
    check(409 not in sources["appearances"], "fixture must lack an appearance")
    item, _deps, report = engine_items.convert_item(sources, 409)
    check(item is None, report)
    check(
        report["routed_non_item"]
        == {"owner": "WorldObject", "reason": "no_client_appearance"},
        "a resolvable wiki record must never resolve an appearance-less item; it must "
        "still fall through to the no-client-appearance catch-all",
    )
    sources = synthetic_sources(
        "crystal", {409: {"name": "gemstone slab", "attrs": {}, "flags": {}}}
    )
    sources["wiki_family_fallback"] = fallback
    check(409 in sources["appearances"], "fixture must carry an appearance")
    item, _deps, report = engine_items.convert_item(sources, 409)
    check(item["family_profile_basis"] == "wiki_evidence_fallback", report)


def test_engine_attribute_always_wins_over_wiki_fallback():
    key = engine_items.build_identity_index()[408][0]
    fallback = {
        key: synthetic_wiki_fallback_entry(
            "decoration", ["engine wins item"], field="primarytype", value="Decorations"
        )
    }
    item, _deps, report = convert_with_fallback(
        {408: {"name": "Engine Wins Item", "attrs": {"primarytype": "valuables"}}},
        fallback,
        item_id=408,
    )
    check(item["family_profile"] == "material_valuable", item)
    check("family_profile_basis" not in item, item)
    check("family_profile_basis" not in report, report)


def test_wiki_fallback_disambiguation_accepted_when_candidates_agree():
    key = engine_items.build_identity_index()[409][0]
    candidates = [
        {
            "field": "primarytype",
            "value": "Decorations",
            "wiki_source": {"source_id": "fandom", **sample_wiki_source(page_id=1)},
        },
        {
            "field": "primarytype",
            "value": "Decorations",
            "wiki_source": {"source_id": "fandom", **sample_wiki_source(page_id=2)},
        },
    ]
    fallback = {
        key: synthetic_wiki_fallback_entry(
            "decoration", ["ambiguous thing"], candidates=candidates
        )
    }
    item, _deps, _report = convert_with_fallback(
        {409: {"name": "Ambiguous Thing", "attrs": {}}}, fallback, item_id=409
    )
    check(item["family_profile"] == "decoration", item)
    check(item["family_profile_evidence"]["resolution"] == "disambiguation", item)
    check(len(item["family_profile_evidence"]["candidates"]) == 2, item)


def test_wiki_fallback_appearance_title_matches_appearance_name():
    # Owner decision 2026-09-28 (task e): an `appearance_title` record is checked
    # against this item's own appearances.dat name, not its items.xml name -- e.g. a
    # blanket items.xml range label ("weapon of mayhem") over an id whose real client
    # name is per-id ("slayer of mayhem").
    key = engine_items.build_identity_index()[410][0]
    fallback = {
        key: synthetic_wiki_fallback_entry(
            "weapon_melee",
            ["slayer of mayhem"],
            field="primarytype",
            value="Sword Weapons",
            match_basis="appearance_title",
        )
    }
    item, _deps, report = convert_with_fallback(
        {
            410: {
                "name": "weapon of mayhem",
                "appearance_name": "Slayer of Mayhem",
                "attrs": {},
            }
        },
        fallback,
        item_id=410,
    )
    check(item is not None, report)
    check(item["family_profile"] == "weapon_melee", item)
    check(item["family_profile_basis"] == "wiki_evidence_fallback", item)


def test_wiki_fallback_appearance_title_ignores_items_xml_name():
    # The same record must NOT match against the items.xml name, even though that is
    # what every other match_basis checks.
    key = engine_items.build_identity_index()[411][0]
    fallback = {
        key: synthetic_wiki_fallback_entry(
            "weapon_melee",
            ["slayer of mayhem"],
            field="primarytype",
            value="Sword Weapons",
            match_basis="appearance_title",
        )
    }
    item, _deps, report = convert_with_fallback(
        {
            411: {
                "name": "weapon of mayhem",
                "appearance_name": "weapon of mayhem",
                "attrs": {},
            }
        },
        fallback,
        item_id=411,
    )
    check(item is None, report)
    check("family_profile_unresolved" in report["blockers"], report)


def test_wiki_fallback_appearance_title_name_mismatch_is_ignored():
    # This item's own appearance name is not the one the snapshot resolved.
    key = engine_items.build_identity_index()[412][0]
    fallback = {
        key: synthetic_wiki_fallback_entry(
            "weapon_melee",
            ["slayer of mayhem"],
            field="primarytype",
            value="Sword Weapons",
            match_basis="appearance_title",
        )
    }
    item, _deps, report = convert_with_fallback(
        {
            412: {
                "name": "weapon of mayhem",
                "appearance_name": "chopper of mayhem",
                "attrs": {},
            }
        },
        fallback,
        item_id=412,
    )
    check(item is None, report)
    check("family_profile_unresolved" in report["blockers"], report)


def test_wiki_fallback_loader_accepts_valid_appearance_title_record():
    key = FIXTURE_ITEM_KEYS[101]
    record = {
        "registry_key": key,
        "matched_names": ["slayer of mayhem"],
        "resolution": "direct",
        "match_basis": "appearance_title",
        "field": "primarytype",
        "value": "Sword Weapons",
        **sample_wiki_source(),
    }
    payload = build_wiki_fallback_payload({key: record})
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "items-family-fallback.json"
        path.write_text(json.dumps(payload), encoding="utf-8")
        resolved = engine_items.load_wiki_family_fallback(
            path, engine_items.build_identity_index()
        )
        check(resolved[key]["match_basis"] == "appearance_title", resolved)
        check(resolved[key]["profile"] == "weapon_melee", resolved)


def test_wiki_fallback_loader_rejects_unknown_match_basis_value():
    key = FIXTURE_ITEM_KEYS[101]
    record = {
        "registry_key": key,
        "matched_names": ["a"],
        "resolution": "direct",
        "match_basis": "made_up_basis",
        "field": "primarytype",
        "value": "Decorations",
        **sample_wiki_source(),
    }
    payload = build_wiki_fallback_payload({key: record})
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "items-family-fallback.json"
        path.write_text(json.dumps(payload), encoding="utf-8")
        raised = False
        try:
            engine_items.load_wiki_family_fallback(
                path, engine_items.build_identity_index()
            )
        except SystemExit:
            raised = True
        check(raised, "an unrecognized match_basis value must be rejected")


def test_wiki_fallback_loader_accepts_valid_actualname_record():
    key = FIXTURE_ITEM_KEYS[101]
    record = {
        "registry_key": key,
        "matched_names": ["slayer of mayhem"],
        "resolution": "direct",
        "match_basis": "actualname",
        "field": "primarytype",
        "value": "Sword Weapons",
        **sample_wiki_source(),
    }
    payload = build_wiki_fallback_payload({key: record})
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "items-family-fallback.json"
        path.write_text(json.dumps(payload), encoding="utf-8")
        resolved = engine_items.load_wiki_family_fallback(
            path, engine_items.build_identity_index()
        )
        check(resolved[key]["match_basis"] == "actualname", resolved)
        check(resolved[key]["profile"] == "weapon_melee", resolved)


def test_wiki_fallback_actualname_matches_either_items_xml_or_appearance_name():
    # Owner decision 2026-09-28 (task g): an `actualname` record is checked against
    # EITHER this item's items.xml name or its appearance name, whichever the capture
    # tool actually matched.
    key_a = engine_items.build_identity_index()[413][0]
    fallback_a = {
        key_a: synthetic_wiki_fallback_entry(
            "weapon_melee",
            ["axe of mayhem"],
            field="primarytype",
            value="Axe Weapons",
            match_basis="actualname",
        )
    }
    item, _deps, report = convert_with_fallback(
        {413: {"name": "axe of mayhem", "attrs": {}}}, fallback_a, item_id=413
    )
    check(item is not None and item["family_profile"] == "weapon_melee", report)

    key_b = engine_items.build_identity_index()[414][0]
    fallback_b = {
        key_b: synthetic_wiki_fallback_entry(
            "weapon_melee",
            ["axe of mayhem"],
            field="primarytype",
            value="Axe Weapons",
            match_basis="actualname",
        )
    }
    item, _deps, report = convert_with_fallback(
        {
            414: {
                "name": "weapon of mayhem",
                "appearance_name": "axe of mayhem",
                "attrs": {},
            }
        },
        fallback_b,
        item_id=414,
    )
    check(item is not None and item["family_profile"] == "weapon_melee", report)


def test_wiki_fallback_actualname_mismatch_is_rejected():
    key = engine_items.build_identity_index()[415][0]
    fallback = {
        key: synthetic_wiki_fallback_entry(
            "weapon_melee",
            ["axe of mayhem"],
            field="primarytype",
            value="Axe Weapons",
            match_basis="actualname",
        )
    }
    item, _deps, report = convert_with_fallback(
        {
            415: {
                "name": "weapon of mayhem",
                "appearance_name": "some other name",
                "attrs": {},
            }
        },
        fallback,
        item_id=415,
    )
    check(item is None, report)
    check("family_profile_unresolved" in report["blockers"], report)


def test_capture_tool_actualname_index_and_disagreement():
    capture = load_capture_module()
    axe_page = _fake_wiki_page(
        1,
        "Axe of Mayhem",
        "{{Infobox Object\n|actualname = axe of mayhem\n|primarytype = Axe Weapons\n}}",
    )
    fetcher = capture.WikiFetcher()
    fetcher.cache["Axe of Mayhem"] = axe_page
    index = capture.build_actualname_index(fetcher, ["Axe of Mayhem"])
    check(index["axe of mayhem"] == [axe_page], index)
    record = capture.resolve_page_group(index["axe of mayhem"], "actualname")
    check(record is not None and record["match_basis"] == "actualname", record)
    check(record["value"] == "Axe Weapons", record)

    # Two pages sharing the same actualname but disagreeing on profile: unresolved.
    other_page = _fake_wiki_page(
        2,
        "Some Other Axe of Mayhem Page",
        "{{Infobox Object\n|actualname = axe of mayhem\n|primarytype = Others\n}}",
    )
    check(
        capture.resolve_page_group([axe_page, other_page], "actualname") is None,
        "disagreeing actualname-matched pages must not resolve",
    )


def test_wiki_fallback_loader_missing_file_is_empty():
    resolved = engine_items.load_wiki_family_fallback(
        Path("/nonexistent-does-not-exist/items-family-fallback.json"),
        engine_items.build_identity_index(),
    )
    check(resolved == {}, resolved)


def test_wiki_fallback_loader_accepts_valid_direct_record():
    key = FIXTURE_ITEM_KEYS[101]
    record = {
        "registry_key": key,
        "matched_names": ["a matched name"],
        "resolution": "direct",
        "match_basis": "title",
        "field": "primarytype",
        "value": "Decorations",
        **sample_wiki_source(),
    }
    payload = build_wiki_fallback_payload({key: record})
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "items-family-fallback.json"
        path.write_text(json.dumps(payload), encoding="utf-8")
        resolved = engine_items.load_wiki_family_fallback(
            path, engine_items.build_identity_index()
        )
        check(resolved[key]["profile"] == "decoration", resolved)
        check(resolved[key]["matched_names"] == {"a matched name"}, resolved)
        check(resolved[key]["evidence"]["wiki_source"]["page_id"] == 1, resolved[key])


def test_committed_wiki_fallback_snapshot_loads_fail_closed():
    """The committed snapshot must pass the same fail-closed loader the converter uses."""
    resolved = engine_items.load_wiki_family_fallback(
        engine_items.WIKI_FAMILY_FALLBACK_PATH, engine_items.build_identity_index()
    )
    # 1,463 pinned-engine records (1,482 less 19 D149 records, ITEM-ID-1b) plus the 8
    # epoch-2 donor records appended by B2.
    check(len(resolved) == 1471, len(resolved))
    donor_targets = {
        binding["target"]["key"]
        for binding in json.loads(
            engine_items.ROOT.parents[2]
            .joinpath("imports/crystalserver/bindings/items.json")
            .read_text(encoding="utf-8")
        )["bindings"]
        if binding["source_revision"] == "00ce02a57ca5a12e48f32a3476e37471167e4c3f"
    }
    donor_keys = sorted(key for key in resolved if key in donor_targets)
    check(len(donor_keys) == 8, donor_keys)
    check(
        all(resolved[key]["match_basis"] == "itemid" for key in donor_keys),
        "donor records join by exact itemid only",
    )
    check(
        all(
            entry["profile"] in engine_items.PROFILE_ITEM_CLASS
            for entry in resolved.values()
        ),
        "every committed record resolves to a known profile",
    )


def test_wiki_fallback_snapshot_is_registered():
    """The committed snapshot and its capture tool stay registered in imports/tibiawiki."""
    root = engine_items.WIKI_FAMILY_FALLBACK_PATH.parents[3]
    snapshot = json.loads(
        engine_items.WIKI_FAMILY_FALLBACK_PATH.read_text(encoding="utf-8")
    )
    digest = snapshot["snapshot_sha256"]
    tool = root / "tools/content-census/item_wiki_family_capture.py"
    tool_digest = hashlib.sha256(tool.read_bytes()).hexdigest()
    batch_id = "g5-item-family-fallback-tibiawiki-r1"
    batches = json.loads(
        (root / "imports/tibiawiki/batches.json").read_text(encoding="utf-8")
    )
    sources = json.loads(
        (root / "imports/tibiawiki/sources.json").read_text(encoding="utf-8")
    )
    batch = [row for row in batches["batches"] if row["batch_id"] == batch_id]
    source = [row for row in sources["sources"] if row["import_batch_id"] == batch_id]
    check(len(batch) == 1 and len(source) == 1, (len(batch), len(source)))
    check(batch[0]["source_artifact_sha256"] == digest, batch[0])
    check(batch[0]["mapper_sha256"] == tool_digest, batch[0])
    check(source[0]["sha256"] == digest, source[0])


def test_family_profile_evidence_shapes_are_mutually_exclusive():
    import jsonschema

    schema = json.loads(
        (engine_items.ROOT / "item.schema.json").read_text(encoding="utf-8")
    )
    validator = jsonschema.Draft202012Validator(
        {"$ref": "#/$defs/familyProfileEvidence", "$defs": schema["$defs"]}
    )
    resolved = engine_items.load_wiki_family_fallback(
        engine_items.WIKI_FAMILY_FALLBACK_PATH, engine_items.build_identity_index()
    )
    evidence = [entry["evidence"] for entry in resolved.values()]
    direct = next(item for item in evidence if item["resolution"] == "direct")
    disambiguation = next(
        item for item in evidence if item["resolution"] == "disambiguation"
    )
    for valid in (direct, disambiguation):
        errors = list(validator.iter_errors(valid))
        check(not errors, [error.message for error in errors])
    mixed_direct = {**direct, "candidates": disambiguation["candidates"]}
    mixed_disambiguation = {**disambiguation, "field": direct["field"]}
    for mixed in (mixed_direct, mixed_disambiguation):
        validators = sorted({error.validator for error in validator.iter_errors(mixed)})
        check(validators == ["not"], (mixed["resolution"], validators))

    # The two new (owner decision 2026-09-28) evidence shapes validate on their own...
    wrap_evidence = {"wrap_target_id": 23398, "wrap_target_primarytype": "furniture"}
    owner_name_evidence = {"rule": "take_able_dead_creature", "name": "dead rat"}
    for valid in (wrap_evidence, owner_name_evidence):
        errors = list(validator.iter_errors(valid))
        check(not errors, [error.message for error in errors])
    # ...but mixing any two families' fields together is rejected, and an evidence value
    # with none of the three families' fields is rejected too (never a silently-empty fact).
    mixed_wrap_and_wiki = {**wrap_evidence, "resolution": direct["resolution"]}
    mixed_owner_and_wrap = {**owner_name_evidence, **wrap_evidence}
    empty_evidence = {}
    for invalid in (mixed_wrap_and_wiki, mixed_owner_and_wrap, empty_evidence):
        check(list(validator.iter_errors(invalid)), invalid)


def test_wiki_fallback_loader_rejects_unadmitted_broad_bucket_value():
    key = FIXTURE_ITEM_KEYS[101]
    record = {
        "registry_key": key,
        "matched_names": ["some broad bucket item"],
        "resolution": "direct",
        "match_basis": "title",
        "field": "primarytype",
        "value": "Others",
        **sample_wiki_source(),
    }
    payload = build_wiki_fallback_payload({key: record})
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "items-family-fallback.json"
        path.write_text(json.dumps(payload), encoding="utf-8")
        try:
            engine_items.load_wiki_family_fallback(
                path, engine_items.build_identity_index()
            )
        except SystemExit as exc:
            check("not in the admitted wiki mapping" in str(exc), exc)
        else:
            raise AssertionError(
                "an unadmitted broad-bucket value must be a hard error"
            )


def test_wiki_fallback_loader_rejects_divergent_disambiguation_candidates():
    key = FIXTURE_ITEM_KEYS[101]
    record = {
        "registry_key": key,
        "matched_names": ["divergent thing"],
        "resolution": "disambiguation",
        "match_basis": "title",
        "candidates": [
            {
                "field": "primarytype",
                "value": "Decorations",
                **sample_wiki_source(page_id=1),
            },
            {
                "field": "primarytype",
                "value": "Quest Items",
                **sample_wiki_source(page_id=2),
            },
        ],
    }
    payload = build_wiki_fallback_payload({key: record})
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "items-family-fallback.json"
        path.write_text(json.dumps(payload), encoding="utf-8")
        try:
            engine_items.load_wiki_family_fallback(
                path, engine_items.build_identity_index()
            )
        except SystemExit as exc:
            check("do not all resolve to the same profile" in str(exc), exc)
        else:
            raise AssertionError(
                "divergent disambiguation candidates must be a hard error"
            )


def test_wiki_fallback_loader_rejects_unknown_registry_key():
    fake_key = "oteryn:item.registry.i99999999"
    record = {
        "registry_key": fake_key,
        "matched_names": ["ghost item"],
        "resolution": "direct",
        "match_basis": "title",
        "field": "primarytype",
        "value": "Decorations",
        **sample_wiki_source(),
    }
    payload = build_wiki_fallback_payload({fake_key: record})
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "items-family-fallback.json"
        path.write_text(json.dumps(payload), encoding="utf-8")
        try:
            engine_items.load_wiki_family_fallback(
                path, engine_items.build_identity_index()
            )
        except SystemExit as exc:
            check("not a known Item key" in str(exc), exc)
        else:
            raise AssertionError("an unknown registry_key must be a hard error")


def test_wiki_fallback_loader_rejects_digest_mismatch():
    key = FIXTURE_ITEM_KEYS[101]
    record = {
        "registry_key": key,
        "matched_names": ["digest test item"],
        "resolution": "direct",
        "match_basis": "title",
        "field": "primarytype",
        "value": "Decorations",
        **sample_wiki_source(),
    }
    payload = build_wiki_fallback_payload({key: record})
    payload["snapshot_sha256"] = "0" * 64
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "items-family-fallback.json"
        path.write_text(json.dumps(payload), encoding="utf-8")
        try:
            engine_items.load_wiki_family_fallback(
                path, engine_items.build_identity_index()
            )
        except SystemExit as exc:
            check("snapshot_sha256 mismatch" in str(exc), exc)
        else:
            raise AssertionError("a snapshot_sha256 mismatch must be a hard error")


def test_wiki_fallback_loader_rejects_unknown_top_level_key():
    key = FIXTURE_ITEM_KEYS[101]
    record = {
        "registry_key": key,
        "matched_names": ["extra key item"],
        "resolution": "direct",
        "match_basis": "title",
        "field": "primarytype",
        "value": "Decorations",
        **sample_wiki_source(),
    }
    payload = build_wiki_fallback_payload({key: record})
    payload["extra_top_level_key"] = 1
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "items-family-fallback.json"
        path.write_text(json.dumps(payload), encoding="utf-8")
        try:
            engine_items.load_wiki_family_fallback(
                path, engine_items.build_identity_index()
            )
        except SystemExit as exc:
            check("unknown top-level key" in str(exc), exc)
        else:
            raise AssertionError("an unknown top-level key must be a hard error")


def test_wiki_fallback_loader_rejects_duplicate_json_key():
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "items-family-fallback.json"
        # Built by hand: json.dumps can never emit a duplicate key.
        path.write_text(
            '{"schema": ' + json.dumps(engine_items.WIKI_FAMILY_FALLBACK_SCHEMA) + ", "
            '"schema": ' + json.dumps(engine_items.WIKI_FAMILY_FALLBACK_SCHEMA) + ", "
            '"batch_id": "x", "family": "Item", "source": {}, '
            '"captured_at": "2026-01-01T00:00:00Z", "records": {}, '
            '"snapshot_sha256": "' + "0" * 64 + '"}',
            encoding="utf-8",
        )
        try:
            engine_items.load_wiki_family_fallback(
                path, engine_items.build_identity_index()
            )
        except SystemExit as exc:
            check("duplicate JSON key" in str(exc), exc)
        else:
            raise AssertionError("a duplicate top-level JSON key must be a hard error")


def test_resolve_wiki_family_value_admitted_mapping():
    # `primarytype` "others" and the owner-decision-pending values never resolve for
    # either field. The broad `objectclass` buckets ("utilities" included: as an
    # `objectclass` bucket it is one of Fandom's broad groupings, unlike the engine's
    # own unrelated `primarytype` "utilities" -> tool entry, which is correct and
    # pre-existing) never resolve as `objectclass` either.
    for value in ("others", "fireworks", "clothing accessories", ""):
        check(
            engine_items.resolve_wiki_family_value("primarytype", value) is None,
            f"primarytype={value!r} must never resolve",
        )
        check(
            engine_items.resolve_wiki_family_value("objectclass", value) is None,
            f"objectclass={value!r} must never resolve",
        )
    # Owner decision 2026-09-28: blessing charms are progression material.
    check(
        engine_items.resolve_wiki_family_value("primarytype", "Blessing Charms")
        == "progression_material",
        "blessing charms resolve to progression_material",
    )
    check(
        engine_items.resolve_wiki_family_value("objectclass", "blessing charms")
        is None,
        "blessing charms is a primarytype value, never an objectclass",
    )
    for value in (
        "other items",
        "household items",
        "tools and other equipment",
        "utilities",
        "plants, animal products, food and drink",
        "other objects",
    ):
        check(
            engine_items.resolve_wiki_family_value("objectclass", value) is None,
            f"objectclass={value!r} (broad bucket) must never resolve",
        )
    check(
        engine_items.resolve_wiki_family_value("primarytype", "Decorations")
        == "decoration",
        "case-folded 'decorations' alias",
    )
    check(
        engine_items.resolve_wiki_family_value("primarytype", "Tools (Objects)")
        is None,
        "'tools (objects)' names map objects and must not alias portable tools",
    )
    check(
        engine_items.resolve_wiki_family_value("primarytype", "Lamps")
        == "light_source",
        "'lamps' alias (same profile as illumination)",
    )
    check(
        engine_items.resolve_wiki_family_value("objectclass", "Imbuement Scrolls")
        == "progression_material",
        "objectclass fallback admitted mapping",
    )


def test_resolve_wiki_family_value_status_field():
    # Owner decision 2026-09-28: infobox `status = event` names a time-limited Tibia
    # event drop (e.g. the 20th anniversary), which is exactly `event_collectible`.
    check(
        engine_items.resolve_wiki_family_value("status", "event")
        == "event_collectible",
        "status=event resolves to event_collectible",
    )
    check(
        engine_items.resolve_wiki_family_value("status", "Event")
        == "event_collectible",
        "status is case-folded like every other admitted field",
    )
    for value in ("deprecated", "quest reward", "unobtainable", ""):
        check(
            engine_items.resolve_wiki_family_value("status", value) is None,
            f"status={value!r} must never resolve",
        )
    # `status` is the lowest-priority admitted field: it names no field of its own
    # standing alongside an admitted `primarytype`/`objectclass` value, only a distinct
    # infobox field entirely. Resolving it never depends on, or overrides, an admitted
    # `primarytype` value for the same page -- the capture tool's field ordering
    # (`primarytype` -> `objectclass` -> `status`) is what enforces the priority; this
    # resolver itself only ever answers for the one field it is asked about.
    check(
        engine_items.resolve_wiki_family_value("primarytype", "Decorations")
        == "decoration",
        "an admitted primarytype value resolves on its own",
    )
    check(
        engine_items.resolve_wiki_family_value("status", "event") == "event_collectible"
        and engine_items.resolve_wiki_family_value("primarytype", "Decorations")
        == "decoration",
        "status and primarytype resolve independently; status never shadows primarytype",
    )


def load_capture_module():
    """Dynamically import `tools/content-census/item_wiki_family_capture.py` (a script,
    not an installed package) so its pure, network-free helpers can be unit tested
    directly, exactly as `engine_items.resolve_wiki_family_value` already is."""
    import importlib.util
    import sys as _sys

    module_path = (
        engine_items.ROOT.parents[1] / "content-census/item_wiki_family_capture.py"
    )
    spec = importlib.util.spec_from_file_location(
        "item_wiki_family_capture", module_path
    )
    module = importlib.util.module_from_spec(spec)
    _sys.modules.setdefault(spec.name, module)
    spec.loader.exec_module(module)
    return module


def _fake_wiki_page(page_id, title, content):
    """A synthetic already-fetched `WikiFetcher` cache entry, for pure/offline tests of
    capture-tool functions that only read a page's `content`/`page_id`/identity."""
    return {
        "page_id": page_id,
        "title": title,
        "revision_id": 1,
        "revision_timestamp": "2026-01-01T00:00:00Z",
        "revision_sha1": "a" * 40,
        "content_sha256": "b" * 64,
        "content": content,
    }


def test_wiki_status_event_priority_below_primarytype_in_capture_tool():
    """The capture tool's `resolve_infobox_fields` only ever consults `status` once
    both `primarytype` and `objectclass` have failed to resolve -- exactly the priority
    the owner decision requires, and the one place that ordering is actually enforced
    (`resolve_wiki_family_value` itself has no concept of field priority)."""
    capture = load_capture_module()

    # An admitted primarytype wins even when status is also present and admitted:
    # status is never consulted once primarytype already resolved.
    profile, field, value = capture.resolve_infobox_fields(
        {"primarytype": "Decorations", "status": "event"}
    )
    check(
        (profile, field, value) == ("decoration", "primarytype", "Decorations"),
        "an admitted primarytype must win over an admitted status",
    )
    # A forbidden, non-empty primarytype value must not block the status fallback.
    profile, field, value = capture.resolve_infobox_fields(
        {"primarytype": "Others", "status": "event"}
    )
    check(
        (profile, field, value) == ("event_collectible", "status", "event"),
        "a forbidden primarytype value must not block the status rule",
    )
    # status alone, with neither primarytype nor objectclass present.
    profile, field, value = capture.resolve_infobox_fields({"status": "event"})
    check(
        (profile, field, value) == ("event_collectible", "status", "event"),
        "status resolves on its own when no other admitted field is present",
    )
    # A non-admitted status value resolves nothing.
    profile, field, value = capture.resolve_infobox_fields(
        {"primarytype": "Others", "status": "deprecated"}
    )
    check(profile is None, "a non-admitted status value must not resolve")


def test_wiki_fallback_real_snapshot_old_rag_and_ivory_comb():
    """Owner decision 2026-09-28: old rag (Crystal 24415) is TibiaWiki's own
    `status = event` 20th-anniversary drop, not a creature product, so it resolves via
    the wiki fallback's status rule to `event_collectible`; ivory comb (Crystal 32773)
    stays a creature product (`material_valuable`) via its own pre-existing direct
    `primarytype = Creature Products` wiki fallback record. Both engine items carry the
    native `primarytype = clothing accessories` attribute, which is deliberately never
    an admitted engine-side alias, so both depend entirely on this wiki evidence."""
    identity_index = engine_items.build_identity_index()
    old_rag_key, _basis = identity_index[24415]
    ivory_comb_key, _basis = identity_index[32773]
    resolved = engine_items.load_wiki_family_fallback(
        engine_items.WIKI_FAMILY_FALLBACK_PATH, identity_index
    )
    check(
        resolved[old_rag_key]["profile"] == "event_collectible",
        resolved[old_rag_key],
    )
    check(
        resolved[old_rag_key]["evidence"]["field"] == "status",
        resolved[old_rag_key]["evidence"],
    )
    check(
        resolved[ivory_comb_key]["profile"] == "material_valuable",
        resolved[ivory_comb_key],
    )


def test_capture_tool_parses_last_field_before_closing_braces():
    # A page's last infobox parameter is sometimes written with no trailing newline
    # before the template's own closing `}}` (e.g. `|status=unobtainable}}`, as on the
    # real "Masterpiece of a Muse" page); that closer must never be captured as part of
    # the field's own value.
    capture = load_capture_module()
    content = "{{Infobox Object\n|itemid = 1\n|status=unobtainable}}"
    fields = capture.parse_infobox_fields(content)
    check(fields["status"] == "unobtainable", fields)
    check(capture.page_status_value(fields) == "unobtainable", fields)


def test_capture_tool_page_status_value_defaults():
    capture = load_capture_module()
    check(capture.page_status_value({"status": "  Unavailable  "}) == "unavailable", "")
    check(capture.page_status_value({"status": ""}) is None, "empty status is None")
    check(capture.page_status_value({}) is None, "absent status is None")


def test_capture_tool_itemlist_template_parsing():
    """Fandom disambiguation pages sometimes list variants inside `{{ItemList ...}}`
    instead of `[[links]]` (the owner's Kraken Buoy Lamp finding); `key=value`
    parameters (almost always the leading `type=...`) must never be read as a
    candidate title."""
    capture = load_capture_module()
    content = (
        "{{Disambig}}\n"
        "{{ItemList|type=ItemList/Sorted\n"
        " |Kraken Buoy Lamp (Lit)\n"
        " |Kraken Buoy Lamp (Unlit)\n"
        "}}"
    )
    check(
        capture.extract_itemlist_titles(content)
        == ["Kraken Buoy Lamp (Lit)", "Kraken Buoy Lamp (Unlit)"],
        capture.extract_itemlist_titles(content),
    )
    check(
        capture.extract_candidate_links(content)
        == ["Kraken Buoy Lamp (Lit)", "Kraken Buoy Lamp (Unlit)"],
        "extract_candidate_links must also read ItemList positional entries",
    )
    # A [[wikilink]] positional entry resolves to its link target, same as a bare title.
    linked = "{{ItemList|type=Foo\n |[[Green Piece of Cloth]]\n |Ivory Comb\n}}"
    check(
        capture.extract_candidate_links(linked)
        == ["Green Piece of Cloth", "Ivory Comb"],
        capture.extract_candidate_links(linked),
    )
    # No ItemList template at all: behaves exactly as before (links only).
    check(
        capture.extract_candidate_links("[[Foo]] and [[Bar]]") == ["Foo", "Bar"],
        "extract_candidate_links without any ItemList is unaffected",
    )


def test_capture_tool_itemid_join_precedence_over_name_match():
    """An exact `itemid` join match is authoritative: it is used even when the item's
    engine name would otherwise resolve to a different page/profile through the
    title-based join."""
    capture = load_capture_module()
    id_matched_page = _fake_wiki_page(
        1,
        "Kraken Buoy Lamp (Unlit)",
        "{{Infobox Object\n|itemid = 37519\n|primarytype = Decorations\n}}",
    )
    record = capture.resolve_id_matched_pages([id_matched_page])
    check(record["match_basis"] == "itemid", record)
    check(record["field"] == "primarytype" and record["value"] == "Decorations", record)
    check(record["resolution"] == "direct", record)
    # 2+ id-matched pages that agree resolve as a disambiguation-shaped record, still
    # match_basis="itemid".
    second_page = _fake_wiki_page(
        2,
        "Kraken Buoy Lamp (Lit)",
        "{{Infobox Object\n|itemid = 37187\n|primarytype = Decorations\n}}",
    )
    multi = capture.resolve_id_matched_pages([id_matched_page, second_page])
    check(multi["resolution"] == "disambiguation", multi)
    check(multi["match_basis"] == "itemid", multi)
    check(len(multi["candidates"]) == 2, multi)


def test_capture_tool_itemid_join_disagreement_or_failure_has_no_name_fallback():
    """When id-matched pages disagree, or one of them does not resolve at all, the id
    evidence is still authoritative: `resolve_id_matched_pages` returns `None` (the
    caller in `main()` then leaves the item unresolved and never tries a name-based
    lookup for it, since the id join already had an opinion)."""
    capture = load_capture_module()
    decorations_page = _fake_wiki_page(
        1, "A", "{{Infobox Object\n|itemid = 1\n|primarytype = Decorations\n}}"
    )
    weapons_page = _fake_wiki_page(
        2, "B", "{{Infobox Object\n|itemid = 2\n|primarytype = Weapons\n}}"
    )
    check(
        capture.resolve_id_matched_pages([decorations_page, weapons_page]) is None,
        "disagreeing id-matched pages must resolve to None, not pick either profile",
    )
    unresolved_page = _fake_wiki_page(
        3, "C", "{{Infobox Object\n|itemid = 3\n|primarytype = Others\n}}"
    )
    check(
        capture.resolve_id_matched_pages([unresolved_page]) is None,
        "a single id-matched page with a forbidden value must resolve to None",
    )
    check(
        capture.resolve_id_matched_pages([decorations_page, unresolved_page]) is None,
        "one id-matched page failing to resolve fails the whole id match, even when "
        "another one of the same id's pages would have resolved on its own",
    )


def test_capture_tool_diagnose_id_matched_unresolved_is_report_only():
    """`diagnose_id_matched_unresolved` reports the distinct `field=value` strings an
    id-matched-but-unresolved page carries, for a human to review; it never decides a
    routing outcome (there is no reachable routing decision to compare it against)."""
    capture = load_capture_module()
    others_page = _fake_wiki_page(
        1,
        "Some Trinket",
        "{{Infobox Object\n|itemid = 90001\n|primarytype = Others\n}}",
    )
    check(capture.resolve_id_matched_pages([others_page]) is None, others_page)
    check(
        capture.diagnose_id_matched_unresolved([others_page]) == ["primarytype=others"],
        "the diagnostic must report the value for a human to review",
    )
    # Two pages carrying different unresolved field/value pairs both get reported.
    household_page = _fake_wiki_page(
        2,
        "Some Household Thing",
        "{{Infobox Object\n|itemid = 90002\n|objectclass = Household Items\n}}",
    )
    check(
        capture.diagnose_id_matched_unresolved([others_page, household_page])
        == ["objectclass=household items", "primarytype=others"],
        "every distinct unresolved field/value pair across the group is reported",
    )


def test_wiki_fallback_loader_rejects_missing_or_unknown_match_basis():
    key = FIXTURE_ITEM_KEYS[101]
    base_record = {
        "registry_key": key,
        "matched_names": ["some item"],
        "resolution": "direct",
        "field": "primarytype",
        "value": "Decorations",
        **sample_wiki_source(),
    }
    for match_basis in (None, "name", "itemid ", "ITEMID"):
        record = dict(base_record)
        if match_basis is not None:
            record["match_basis"] = match_basis
        payload = build_wiki_fallback_payload({key: record})
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "items-family-fallback.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            try:
                engine_items.load_wiki_family_fallback(
                    path, engine_items.build_identity_index()
                )
            except SystemExit as exc:
                check("match_basis" in str(exc), exc)
            else:
                raise AssertionError(
                    f"match_basis={match_basis!r} must be rejected fail-closed"
                )
    for match_basis in ("itemid", "title"):
        record = dict(base_record)
        record["match_basis"] = match_basis
        payload = build_wiki_fallback_payload({key: record})
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "items-family-fallback.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            resolved = engine_items.load_wiki_family_fallback(
                path, engine_items.build_identity_index()
            )
        check(
            resolved[key]["evidence"]["match_basis"] == match_basis,
            resolved[key]["evidence"],
        )


def test_lf_and_crlf_text_fixtures_byte_identical():
    for engine in ("crystal", "canary"):
        with (
            tempfile.TemporaryDirectory() as lf_dir,
            tempfile.TemporaryDirectory() as crlf_dir,
        ):
            appearances_bytes = fixture_appearances_dat()
            lf_sources = load_fixture_sources(
                Path(lf_dir), engine, newline="\n", appearances_bytes=appearances_bytes
            )
            crlf_sources = load_fixture_sources(
                Path(crlf_dir),
                engine,
                newline="\r\n",
                appearances_bytes=appearances_bytes,
            )

            check(
                lf_sources["items"].keys() == crlf_sources["items"].keys(),
                f"{engine}: LF/CRLF parsed item ids differ",
            )
            check(
                lf_sources["delivery_member_ids"]
                == crlf_sources["delivery_member_ids"],
                f"{engine}: LF/CRLF pool membership differs",
            )

            lf_result, _ = population_census.build_census(lf_sources, engine)
            crlf_result, _ = population_census.build_census(crlf_sources, engine)
            lf_bytes = population_census.census_document_bytes(lf_result)
            crlf_bytes = population_census.census_document_bytes(crlf_result)
            check(
                lf_bytes == crlf_bytes,
                f"{engine}: LF/CRLF census output is not byte-identical",
            )
            check(
                lf_result["bundle_digest"] == crlf_result["bundle_digest"],
                f"{engine}: LF/CRLF bundle_digest differs",
            )
            for path, entry in lf_sources["artifact_digests"].items():
                mode = entry["digest_mode"]
                if path == "data/items/appearances.dat":
                    check(mode == "raw_bytes", f"{engine}: {path} should be raw_bytes")
                else:
                    check(
                        mode == "text_lf_normalized",
                        f"{engine}: {path} should be text_lf_normalized",
                    )


def test_binary_crlf_bytes_are_not_normalized():
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        appearances_bytes = fixture_appearances_dat()
        check(b"\r\n" in appearances_bytes, "fixture appearances.dat has no 0d0a bytes")
        digests = write_fixture_checkout(
            root, "crystal", appearances_bytes=appearances_bytes
        )

        # A correct raw read matches the pinned digest of the true raw bytes.
        payload, mode = engine_items.read_verified_artifact(
            root, "data/items/appearances.dat", digests["data/items/appearances.dat"]
        )
        check(payload == appearances_bytes, "raw appearances.dat bytes were altered")
        check(mode == "raw_bytes", "appearances.dat must be digested as raw_bytes")

        # Simulate a bad CRLF-stripping tool mangling the binary in place; digesting
        # raw bytes must reject it rather than silently treating it as equivalent text.
        mutated = appearances_bytes.replace(b"\r\n", b"\n")
        check(mutated != appearances_bytes, "mutation was a no-op; fixture is unusable")
        (root / "data/items/appearances.dat").write_bytes(mutated)
        try:
            engine_items.read_verified_artifact(
                root,
                "data/items/appearances.dat",
                digests["data/items/appearances.dat"],
            )
        except SystemExit as exc:
            check("digest mismatch" in str(exc), f"unexpected error message: {exc}")
        else:
            raise AssertionError(
                "CRLF-mutated appearances.dat must fail digest verification"
            )


def test_canary_pool_membership_reads_weekly_items_table_only():
    text = fixture_canary_weekly_lua()
    ids = engine_items.parse_canary_weekly_item_ids(text)
    check(ids == {100, 999}, f"unexpected canary weeklyItems membership: {ids}")
    check(555 not in ids, "shopOffers id must not be read as weeklyItems membership")


def test_item_only_in_crystal_list_is_not_canary_member():
    with (
        tempfile.TemporaryDirectory() as crystal_dir,
        tempfile.TemporaryDirectory() as canary_dir,
    ):
        crystal_sources = load_fixture_sources(Path(crystal_dir), "crystal")
        canary_sources = load_fixture_sources(Path(canary_dir), "canary")
        check(
            101 in crystal_sources["delivery_member_ids"],
            "fixture item 101 should be a Crystal delivery-list member",
        )
        check(
            101 not in canary_sources["delivery_member_ids"],
            "item present only in the Crystal list must not read as a Canary member",
        )
        check(
            100 in canary_sources["delivery_member_ids"]
            and 100 not in crystal_sources["delivery_member_ids"],
            "fixture item 100 should be a Canary-only weeklyItems member",
        )


def test_missing_pool_file_is_a_hard_error():
    for engine in ("crystal", "canary"):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            digests = write_fixture_checkout(root, engine)
            pool_relative = engine_items.DELIVERY_POOL_PATH[engine]
            (root / pool_relative).unlink()
            try:
                engine_items.load_engine_sources(engine, root, digests=digests)
            except SystemExit as exc:
                check(
                    "missing required source artifact" in str(exc),
                    f"unexpected error for missing {engine} pool file: {exc}",
                )
            else:
                raise AssertionError(
                    f"missing {engine} pool file must be a hard error, not an empty set"
                )


def test_bundles_carry_delivery_task_eligible_from_the_crystal_list_rule():
    """Converted bundles now carry the real `ADOPT_CRYSTAL_DELIVERY_LIST@ff7ede5`
    decision: fixture item 101 (a Crystal delivery-list member) is eligible for both
    engines; 100/102 are not. The Canary Task Board pool (its own `weeklyItems`) is
    observation only and never decides: fixture item 100 is a Canary weeklyItems member
    but not a Crystal delivery-list member, so its decision is still `false` /
    `crystal_list_non_member`, even though its observation `member` is `True`."""
    for engine in ("crystal", "canary"):
        with tempfile.TemporaryDirectory() as directory:
            sources = load_fixture_sources(Path(directory), engine)
            _result, bundles = population_census.build_census(sources, engine)
            check(bundles, f"{engine}: fixture census produced no bundles")
            for key, bundle in bundles.items():
                check(
                    "delivery_task_eligible" in bundle["item"],
                    f"{engine}: bundle {key} must carry delivery_task_eligible",
                )

            for item_id, expected_eligible, expected_basis in (
                (100, False, "crystal_list_non_member"),
                (101, True, "crystal_list_member"),
                (102, False, "crystal_list_non_member"),
            ):
                item, _dependencies, report = engine_items.convert_item(
                    sources, item_id
                )
                check(item is not None, f"{engine}: fixture item {item_id} failed")
                check(
                    item["delivery_task_eligible"] is expected_eligible,
                    (engine, item_id, item),
                )
                dt = report["delivery_task"]
                check(dt["decision"]["rule"] == engine_items.RULE_ID, dt)
                check(dt["decision"]["basis"] == expected_basis, (engine, item_id, dt))
                check(
                    dt["decision"]["eligible"] is expected_eligible,
                    (engine, item_id, dt),
                )
                check(
                    "delivery_task_decision_not_admitted" not in report["blockers"],
                    (engine, item_id, report),
                )

            # Canary-specific: the Canary pool never decides.
            if engine == "canary":
                _, _, report_100 = engine_items.convert_item(sources, 100)
                check(
                    report_100["delivery_task"]["observation"]
                    == {
                        "source": "canary_task_board_weekly_items",
                        "member": True,
                    },
                    report_100,
                )
                _, _, report_101 = engine_items.convert_item(sources, 101)
                check(
                    report_101["delivery_task"]["observation"]["member"] is False,
                    report_101,
                )


def test_census_counts_fully_resolved_and_delivery_task_block():
    for engine in ("crystal", "canary"):
        with tempfile.TemporaryDirectory() as directory:
            sources = load_fixture_sources(Path(directory), engine)
            result, _bundles = population_census.build_census(sources, engine)
            check(
                result["outcome"].get("pending_author_decision", 0) == 0,
                f"{engine}: decision is always admitted now: {result['outcome']}",
            )
            check(
                result["outcome"].get("fully_resolved", 0) == 3,
                f"{engine}: expected all 3 fixture items fully_resolved: "
                f"{result['outcome']}",
            )
            check(
                result["outcome"].get("structure_invalid", 0) == 0,
                f"{engine}: fixture items should validate cleanly: {result['outcome']}",
            )
            delivery_task = result["delivery_task"]
            check(
                delivery_task["decision"]
                == {
                    "rule": engine_items.RULE_ID,
                    "eligible_true": 1,
                    "eligible_false": 2,
                    "override": 0,
                },
                (engine, delivery_task),
            )
            check(delivery_task["crystal_list"]["entries"] == 2, delivery_task)
            check(delivery_task["crystal_list"]["unique"] == 2, delivery_task)
            check(
                delivery_task["crystal_list"]["not_in_items_xml"] == [999],
                delivery_task,
            )
            check(
                delivery_task["crystal_list"]["in_items_xml_not_converted"] == 0,
                delivery_task,
            )
            if engine == "canary":
                check(
                    delivery_task["observation"]
                    == {
                        "source": "canary_task_board_weekly_items",
                        "member": 1,
                        "not_member": 2,
                    },
                    delivery_task,
                )


def test_canary_run_without_rule_source_is_a_hard_error():
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        digests = write_fixture_checkout(root, "canary")
        try:
            engine_items.load_engine_sources("canary", root, digests=digests)
        except SystemExit as exc:
            check(
                "--rule-source" in str(exc) and "canary" in str(exc),
                f"unexpected error for missing canary rule-source: {exc}",
            )
        else:
            raise AssertionError(
                "a canary run with no --rule-source must be a hard error, not an "
                "empty (all-ineligible) rule"
            )


def test_override_flips_the_decision_with_a_reason():
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        overrides_path = root / "delivery-task-overrides.json"
        # 101 is otherwise eligible (Crystal list member); override it to ineligible.
        write_overrides_file(
            overrides_path,
            {
                FIXTURE_ITEM_KEYS[101]: {
                    "eligible": False,
                    "reason": "owner-approved manual exclusion for this test",
                }
            },
        )
        sources = load_fixture_sources(root, "crystal", overrides_path=overrides_path)
        item, _dependencies, report = engine_items.convert_item(sources, 101)
        check(item["delivery_task_eligible"] is False, item)
        check(
            report["delivery_task"]["decision"]
            == {
                "rule": engine_items.RULE_ID,
                "basis": "override",
                "eligible": False,
                "reason": "owner-approved manual exclusion for this test",
            },
            report,
        )
        # An item with no override still gets the plain rule decision.
        _item_100, _deps_100, report_100 = engine_items.convert_item(sources, 100)
        check(
            report_100["delivery_task"]["decision"]["basis"]
            == "crystal_list_non_member",
            report_100,
        )


def test_invalid_overrides_file_fails():
    key = FIXTURE_ITEM_KEYS[101]
    base_valid = {
        "schema": engine_items.DELIVERY_OVERRIDES_SCHEMA,
        "rule": engine_items.RULE_ID,
        "overrides": {},
    }

    def with_entry(entry):
        return json.dumps({**base_valid, "overrides": {key: entry}})

    cases = [
        ("not a JSON object", "[]", "must be a JSON object"),
        (
            "unknown top-level key",
            json.dumps({**base_valid, "extra": 1}),
            "unknown top-level key",
        ),
        (
            "missing 'rule'",
            json.dumps({"schema": base_valid["schema"], "overrides": {}}),
            "missing required key",
        ),
        (
            "wrong schema",
            json.dumps({**base_valid, "schema": "SOME_OTHER_SCHEMA/v1"}),
            "unexpected 'schema'",
        ),
        (
            "wrong rule",
            json.dumps({**base_valid, "rule": "SOME_OTHER_RULE@deadbeef"}),
            "unexpected 'rule'",
        ),
        (
            "overrides is not an object",
            json.dumps({**base_valid, "overrides": []}),
            "'overrides' in",
        ),
        ("override entry not an object", with_entry(True), "must be a JSON object"),
        (
            "override entry unknown key",
            with_entry({"eligible": True, "reason": "x", "extra": 1}),
            "unknown key(s) in override",
        ),
        (
            "override entry missing reason",
            with_entry({"eligible": True}),
            "missing key(s) in override",
        ),
        (
            "override entry empty reason",
            with_entry({"eligible": True, "reason": "  "}),
            "'reason' must be a non-empty string",
        ),
        (
            "override entry eligible not boolean",
            with_entry({"eligible": "true", "reason": "x"}),
            "'eligible' must be a boolean",
        ),
        (
            "duplicate override key",
            # Built by hand: json.dumps can never emit a duplicate key.
            json.dumps(base_valid)[:-3]
            + "{"
            + ", ".join(
                f"{json.dumps(key)}: "
                + json.dumps({"eligible": eligible, "reason": "r"})
                for eligible in (True, False)
            )
            + "}}",
            "duplicate JSON key",
        ),
        ("invalid JSON", "{not json", "invalid JSON"),
    ]
    with tempfile.TemporaryDirectory() as directory:
        overrides_path = Path(directory) / "overrides.json"
        for label, payload, expected in cases:
            overrides_path.write_text(payload, encoding="utf-8")
            try:
                engine_items.load_delivery_overrides(overrides_path, {key})
            except SystemExit as exc:
                check(
                    expected in str(exc),
                    f"invalid overrides file {label!r} failed for another reason: {exc}",
                )
            else:
                raise AssertionError(f"invalid overrides file must fail: {label}")


def test_override_for_unknown_key_fails():
    with tempfile.TemporaryDirectory() as directory:
        overrides_path = Path(directory) / "overrides.json"
        write_overrides_file(
            overrides_path,
            {"oteryn:item.registry.i99999999": {"eligible": True, "reason": "x"}},
        )
        try:
            engine_items.load_delivery_overrides(
                overrides_path, {FIXTURE_ITEM_KEYS[101]}
            )
        except SystemExit as exc:
            check("unknown Item key" in str(exc), exc)
        else:
            raise AssertionError("override for an unknown Item key must fail")


def test_item_without_allocator_key_keeps_the_blocker_and_no_field():
    with tempfile.TemporaryDirectory() as directory:
        sources = load_fixture_sources(Path(directory), "crystal")
        # An id absent from the identity catalog entirely (well past FIXTURE_ITEM_IDS
        # and the real committed catalog's range) never resolves an allocator key.
        unresolved_id = 999_999_999
        check(
            sources["identity_index"].get(unresolved_id) is None,
            "test id unexpectedly resolves an allocator key; pick a different one",
        )
        item, dependencies, report = engine_items.convert_item(sources, unresolved_id)
        check(item is None, report)
        check(dependencies is None, report)
        check(report["converted"] is False, report)
        check("identity_not_in_b1_catalog" in report["blockers"], report)
        check("delivery_task_decision_not_admitted" in report["blockers"], report)
        check("delivery_task" not in report, report)


def test_crystal_item_bindings_reject_duplicate_target_key():
    """_load_crystal_item_bindings fails closed when two distinct Crystal
    external_ids bind to the same Item target key; the committed
    imports/crystalserver/bindings/items.json has no such row, so this drives the
    loader against a synthetic fixture catalog instead."""

    def binding(external_id, key):
        return {
            "external_id": external_id,
            "disposition": "EXACT",
            "identity_namespace": source_field_catalogs.CRYSTAL_ITEM_BINDINGS_NAMESPACE,
            "source_key": source_field_catalogs.CRYSTAL_ITEM_BINDINGS_SOURCE_KEY,
            "target": {
                "family": "Item",
                "key": key,
                "revision": source_field_catalogs.DEFINITION_REVISION,
            },
        }

    catalog = {
        "schema": source_field_catalogs.CRYSTAL_ITEM_BINDINGS_SCHEMA,
        "family": "Item",
        "bindings": [
            binding("100", "oteryn:item.template.sample-a"),
            binding("200", "oteryn:item.template.sample-a"),
        ],
    }
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "items.json"
        path.write_text(json.dumps(catalog), encoding="utf-8")
        try:
            source_field_catalogs._load_crystal_item_bindings(path)
        except SystemExit as exc:
            check("bound by both external_id" in str(exc), f"unexpected error: {exc}")
        else:
            raise AssertionError(
                "two external ids binding to the same target key must be a hard error"
            )

    # A single external_id per target key still loads cleanly.
    catalog["bindings"][1] = binding("200", "oteryn:item.template.sample-b")
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "items.json"
        path.write_text(json.dumps(catalog), encoding="utf-8")
        index = source_field_catalogs._load_crystal_item_bindings(path)
        check(
            index
            == {
                "100": "oteryn:item.template.sample-a",
                "200": "oteryn:item.template.sample-b",
            },
            index,
        )


def write_owner_decisions_file(path, decisions):
    path.write_text(
        json.dumps(
            {
                "schema": engine_items.OWNER_FAMILY_DECISIONS_SCHEMA,
                "decisions": decisions,
            }
        ),
        encoding="utf-8",
    )


def owner_decision_entry(name, profile, reason="test reason", wiki_url=None, facts="f"):
    return {
        "name": name,
        "profile": profile,
        "reason": reason,
        "source": {"wiki_url": wiki_url, "facts": facts},
    }


def convert_with_owner_decisions(
    item_records, decisions, item_id=200, engine="crystal"
):
    # Mirrors `convert_with_fallback`'s non-empty default flag, for the same reason.
    item_records = {
        record_id: {"flags": {"flags.take": True}, **record}
        for record_id, record in item_records.items()
    }
    sources = synthetic_sources(engine, item_records)
    sources["owner_family_decisions"] = decisions
    return engine_items.convert_item(sources, item_id)


def test_owner_family_decisions_resolves_real_examples():
    """A few real ids from the committed owner leftover-family table (task #15) map to
    their approved profile through the real `convert_item` pipeline."""
    valid_keys = {key for key, _basis in engine_items.build_identity_index().values()}
    decisions = engine_items.load_owner_family_decisions(
        engine_items.OWNER_FAMILY_DECISIONS_PATH, valid_keys
    )
    reverse_index = {
        key: item_id
        for item_id, (key, _basis) in engine_items.build_identity_index().items()
    }
    cases = [
        ("oteryn:item.tibia.i4054", "red chair", "decoration"),
        ("oteryn:item.tibia.i4296", "slain ghoul", "trash"),
        ("oteryn:item.tibia.i52745", "bottle of raubritter lager", "fluid"),
    ]
    for key, name, expected_profile in cases:
        check(key in decisions, f"{key} missing from the committed owner table")
        item_id = reverse_index[key]
        sources = synthetic_sources(
            "crystal", {item_id: {"name": name, "attrs": {}, "flags": {}}}
        )
        sources["owner_family_decisions"] = decisions
        item, _deps, report = engine_items.convert_item(sources, item_id)
        check(item is not None, (key, report))
        check(item["family_profile"] == expected_profile, (key, item))
        check(item["family_profile_basis"] == "owner_name_rule", (key, item))
        check(
            item["family_profile_evidence"]
            == {"rule": "owner_leftover_review_2026_09_28", "name": name},
            (key, item),
        )


def test_owner_family_decisions_only_applies_to_unresolved_items():
    # An item an engine attribute already resolves is never overridden by a matching
    # owner-table entry for the same key/name.
    key = engine_items.build_identity_index()[508][0]
    decisions = {key: owner_decision_entry("red chair", "decoration")}
    item, _deps, report = convert_with_owner_decisions(
        {508: {"name": "red chair", "attrs": {"primarytype": "valuables"}}},
        decisions,
        item_id=508,
    )
    check(item["family_profile"] == "material_valuable", (item, report))
    check("family_profile_basis" not in item, item)


def test_owner_family_decisions_name_mismatch_is_ignored():
    # The decision's own `name` guard must equal this exact item's own lower-cased
    # engine name; a same-key decision for a different name never applies.
    key = engine_items.build_identity_index()[509][0]
    decisions = {key: owner_decision_entry("red chair", "decoration")}
    item, _deps, report = convert_with_owner_decisions(
        {509: {"name": "some other name", "attrs": {}}}, decisions, item_id=509
    )
    check(item is None, report)
    check("family_profile_unresolved" in report["blockers"], report)


def test_owner_family_decisions_never_outranks_wrap_target_or_dead_item():
    # Both wrap-target inheritance and the dead-item rules rank above the owner table
    # (see `convert_item`); a matching owner-table entry never overrides either.
    key = engine_items.build_identity_index()[510][0]
    decisions = {key: owner_decision_entry("wrapped chair", "trash")}
    item, _deps, report = convert_with_owner_decisions(
        {
            510: {"name": "wrapped chair", "attrs": {"wrapableto": "90001"}},
            90001: {"attrs": {"primarytype": "furniture"}},
        },
        decisions,
        item_id=510,
    )
    check(item["family_profile"] == "decoration", (item, report))
    check(item["family_profile_basis"] == "engine_wrap_target", item)

    key = engine_items.build_identity_index()[1467][0]
    decisions = {key: owner_decision_entry("dead rat", "trash")}
    item, _deps, report = convert_with_owner_decisions(
        {1467: {"name": "dead rat", "attrs": {}, "flags": {"flags.take": True}}},
        decisions,
        item_id=1467,
    )
    check(item["family_profile"] == "material_valuable", (item, report))
    check(item["family_profile_evidence"]["rule"] == "take_able_dead_creature", item)


def test_owner_family_decisions_loader_fails_closed():
    key = engine_items.build_identity_index()[100][0]
    base_valid = {
        "schema": engine_items.OWNER_FAMILY_DECISIONS_SCHEMA,
        "decisions": {},
    }
    valid_entry = owner_decision_entry("some name", "trash")

    def with_entry(entry):
        return json.dumps({**base_valid, "decisions": {key: entry}})

    cases = [
        ("not a JSON object", "[]", "must be a JSON object"),
        (
            "unknown top-level key",
            json.dumps({**base_valid, "extra": 1}),
            "unknown top-level key",
        ),
        (
            "missing 'decisions'",
            json.dumps({"schema": base_valid["schema"]}),
            "missing required key",
        ),
        (
            "wrong schema",
            json.dumps({**base_valid, "schema": "SOME_OTHER_SCHEMA/v1"}),
            "unexpected 'schema'",
        ),
        (
            "decisions is not an object",
            json.dumps({**base_valid, "decisions": []}),
            "'decisions' in",
        ),
        ("entry not an object", with_entry(True), "must be a JSON object"),
        (
            "entry unknown key",
            with_entry({**valid_entry, "extra": 1}),
            "unknown key(s)",
        ),
        (
            "entry missing profile",
            with_entry({k: v for k, v in valid_entry.items() if k != "profile"}),
            "missing key(s)",
        ),
        (
            "entry bad profile",
            with_entry({**valid_entry, "profile": "not_a_real_profile"}),
            "is not an admitted family_profile",
        ),
        (
            "entry empty reason",
            with_entry({**valid_entry, "reason": "   "}),
            "'reason' must be a non-empty string",
        ),
        (
            "entry non-lower-cased name",
            with_entry({**valid_entry, "name": "Some Name"}),
            "'name' must be a non-empty lower-cased string",
        ),
        (
            "entry source unknown key",
            with_entry(
                {**valid_entry, "source": {**valid_entry["source"], "extra": 1}}
            ),
            "unknown 'source' key(s)",
        ),
        (
            "entry source empty facts",
            with_entry({**valid_entry, "source": {"wiki_url": None, "facts": "  "}}),
            "'source.facts' must be a non-empty string",
        ),
        (
            "duplicate decision key",
            # Built by hand: json.dumps can never emit a duplicate key.
            json.dumps(base_valid)[:-3]
            + "{"
            + ", ".join(
                f"{json.dumps(key)}: {json.dumps(valid_entry)}" for _ in range(2)
            )
            + "}}",
            "duplicate JSON key",
        ),
        ("invalid JSON", "{not json", "invalid JSON"),
    ]
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "owner-decisions.json"
        for label, payload, expected in cases:
            path.write_text(payload, encoding="utf-8")
            try:
                engine_items.load_owner_family_decisions(path, {key})
            except SystemExit as exc:
                check(
                    expected in str(exc),
                    f"invalid owner decisions file {label!r} failed for another "
                    f"reason: {exc}",
                )
            else:
                raise AssertionError(f"invalid owner decisions file must fail: {label}")


def test_owner_family_decisions_loader_rejects_unknown_registry_key():
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "owner-decisions.json"
        write_owner_decisions_file(
            path,
            {
                "oteryn:item.registry.i99999999": owner_decision_entry(
                    "some name", "trash"
                )
            },
        )
        try:
            engine_items.load_owner_family_decisions(
                path, {engine_items.build_identity_index()[100][0]}
            )
        except SystemExit as exc:
            check("unknown Item key" in str(exc), exc)
        else:
            raise AssertionError("decision for an unknown Item key must fail")


def test_owner_family_decisions_loader_missing_file_is_hard_error():
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "does-not-exist.json"
        try:
            engine_items.load_owner_family_decisions(path, set())
        except SystemExit as exc:
            check("missing owner family decisions file" in str(exc), exc)
        else:
            raise AssertionError("a missing owner decisions file must be a hard error")


def test_committed_owner_family_decisions_loads_fail_closed():
    """The committed table must pass the same fail-closed loader the converter uses."""
    valid_keys = {key for key, _basis in engine_items.build_identity_index().values()}
    decisions = engine_items.load_owner_family_decisions(
        engine_items.OWNER_FAMILY_DECISIONS_PATH, valid_keys
    )
    # 121 leftover decisions (task #15) plus 14 Q13d rows with an engine binding (ITEM-ID-1b).
    check(len(decisions) == 138, len(decisions))
    check(
        all(
            entry["profile"] in engine_items.PROFILE_ITEM_CLASS
            for entry in decisions.values()
        ),
        "every committed decision resolves to a known profile",
    )
    check(
        all(entry["name"] == entry["name"].lower() for entry in decisions.values()),
        "every committed decision's name guard is lower-cased",
    )


def test_empty_client_object_routes_appearance_placeholder_slot():
    # An item with an appearances.dat object but zero true flags at all -- nothing the
    # converter, the wiki fallback or the owner table could ever anchor a family to --
    # routes the same way the placeholder-name checks do.
    sources = synthetic_sources(
        "crystal", {511: {"name": "some unnamed thing", "attrs": {}, "flags": {}}}
    )
    check(511 in sources["appearances"], "fixture must carry an appearance")
    check(sources["appearances"][511]["flags"] == {}, "fixture flags must be empty")
    item, _deps, report = engine_items.convert_item(sources, 511)
    check(item is None, report)
    check(
        report["routed_non_item"]
        == {"owner": "WorldObject", "reason": "appearance_placeholder_slot"},
        report,
    )
    check(report["blockers"] == [], report)


def test_frost_cannon_routes_non_pickupable_blocking_prop():
    # D165 27a: id 9132 is a quest-mechanism prop; it routes to WorldObject even though
    # its client object carries flags that would otherwise resolve a family.
    sources = synthetic_sources(
        "crystal",
        {9132: {"name": "frost cannon", "attrs": {}, "flags": {"flags.take": True}}},
    )
    item, _deps, report = engine_items.convert_item(sources, 9132)
    check(item is None, report)
    check(
        report["routed_non_item"]
        == {"owner": "WorldObject", "reason": "non_pickupable_blocking_prop"},
        report,
    )
    check(report["blockers"] == [], report)
    check(
        engine_items.non_item_route(None, {}, {"flags.take": True}, 9133) is None,
        "only the listed id routes",
    )


def test_empty_client_object_route_requires_zero_flags():
    # A single true flag (of any kind) is enough to keep this route from firing; the
    # item stays plain `family_profile_unresolved` editorial backlog instead.
    sources = synthetic_sources(
        "crystal",
        {
            512: {
                "name": "some unnamed thing",
                "attrs": {},
                "flags": {"flags.take": True},
            }
        },
    )
    item, _deps, report = engine_items.convert_item(sources, 512)
    check(item is None, report)
    check(report.get("routed_non_item") is None, report)
    check("family_profile_unresolved" in report["blockers"], report)


def test_empty_client_object_route_never_outranks_owner_table():
    # An item with zero flags that ALSO has a matching owner-table entry resolves
    # through the owner table; the empty-object route never gets a chance to fire.
    key = engine_items.build_identity_index()[513][0]
    decisions = {key: owner_decision_entry("some unnamed thing", "trash")}
    sources = synthetic_sources(
        "crystal", {513: {"name": "some unnamed thing", "attrs": {}, "flags": {}}}
    )
    sources["owner_family_decisions"] = decisions
    item, _deps, report = engine_items.convert_item(sources, 513)
    check(item is not None, report)
    check(item["family_profile"] == "trash", item)
    check(item["family_profile_basis"] == "owner_name_rule", item)


def main():
    tests = [
        test_lf_and_crlf_text_fixtures_byte_identical,
        test_binary_crlf_bytes_are_not_normalized,
        test_canary_pool_membership_reads_weekly_items_table_only,
        test_item_only_in_crystal_list_is_not_canary_member,
        test_missing_pool_file_is_a_hard_error,
        test_bundles_carry_delivery_task_eligible_from_the_crystal_list_rule,
        test_census_counts_fully_resolved_and_delivery_task_block,
        test_canary_run_without_rule_source_is_a_hard_error,
        test_override_flips_the_decision_with_a_reason,
        test_invalid_overrides_file_fails,
        test_override_for_unknown_key_fails,
        test_item_without_allocator_key_keeps_the_blocker_and_no_field,
        test_value_dependent_type_field,
        test_value_dependent_weapontype_field,
        test_value_dependent_action_field,
        test_value_dependent_eventtype_field,
        test_flags_unmove_boolean_invert,
        test_flags_container_guarded_on_capacity,
        test_flags_cumulative_stack,
        test_unproperly_level_magic_shortfall,
        test_presentation_display_flags_and_assets,
        test_loottype_tags,
        test_stopduration_guarded_on_temporal,
        test_flags_liquidcontainer_fluid_role,
        test_changedtoexpire_corroborates_decayto,
        test_flags_dual_wielding,
        test_readable_write_and_write_once,
        test_elementalbond_and_reflectdamage,
        test_chain_weapon_coefficient,
        test_proficiency_id_238_cites_both_admitted_crosswalk_entries,
        test_proficiency_id_stays_a_precise_blocker_for_other_ids,
        test_forge_max_tier_from_classification_table,
        test_mantra_damage_types_fixed_engine_constant,
        test_fields_with_no_admitted_engine_data_stay_blocked,
        test_routed_non_item_corpse_and_placeholder_and_terrain,
        test_routed_non_item_unmove_map_geometry,
        test_wrap_target_inheritance_resolves,
        test_wrap_target_inheritance_unresolved_target_stays_unresolved,
        test_wrap_target_never_overrides_already_resolved_item,
        test_corpse_decoration_routes_without_take,
        test_corpse_with_take_in_owner_table,
        test_corpse_takeable_unlisted_name_stays_unresolved,
        test_real_corpse_flag_item_unchanged_by_dead_name_rule,
        test_fluid_type_without_appearance_routes_non_item,
        test_fluid_type_with_appearance_is_not_routed,
        test_fluid_type_route_requires_items_xml_record,
        test_appearance_less_non_fluid_name_routes_no_client_appearance,
        test_item_with_appearance_never_routed_no_client_appearance,
        test_late_placeholder_names_route_non_item,
        test_late_placeholder_name_on_already_resolved_item_is_not_rerouted,
        test_skip_post_wiki_fallback_routes_probe_flag,
        test_family_profile_fallbacks,
        test_wiki_fallback_direct_hit,
        test_availability_wired_from_wiki_fallback,
        test_availability_absent_when_wiki_fallback_carries_none,
        test_availability_absent_when_not_wiki_resolved,
        test_wiki_fallback_loader_accepts_valid_availability_status,
        test_wiki_fallback_loader_rejects_unadmitted_availability_status,
        test_wiki_fallback_loader_null_availability_status_is_no_availability,
        test_wiki_fallback_name_mismatch_is_ignored,
        test_wiki_fallback_needs_an_appearance_object,
        test_engine_attribute_always_wins_over_wiki_fallback,
        test_wiki_fallback_disambiguation_accepted_when_candidates_agree,
        test_wiki_fallback_appearance_title_matches_appearance_name,
        test_wiki_fallback_appearance_title_ignores_items_xml_name,
        test_wiki_fallback_appearance_title_name_mismatch_is_ignored,
        test_wiki_fallback_loader_accepts_valid_appearance_title_record,
        test_wiki_fallback_loader_rejects_unknown_match_basis_value,
        test_wiki_fallback_loader_accepts_valid_actualname_record,
        test_wiki_fallback_actualname_matches_either_items_xml_or_appearance_name,
        test_wiki_fallback_actualname_mismatch_is_rejected,
        test_wiki_fallback_loader_missing_file_is_empty,
        test_wiki_fallback_loader_accepts_valid_direct_record,
        test_wiki_fallback_loader_rejects_unadmitted_broad_bucket_value,
        test_wiki_fallback_loader_rejects_divergent_disambiguation_candidates,
        test_wiki_fallback_loader_rejects_unknown_registry_key,
        test_wiki_fallback_loader_rejects_digest_mismatch,
        test_wiki_fallback_loader_rejects_unknown_top_level_key,
        test_wiki_fallback_loader_rejects_duplicate_json_key,
        test_resolve_wiki_family_value_admitted_mapping,
        test_resolve_wiki_family_value_status_field,
        test_wiki_status_event_priority_below_primarytype_in_capture_tool,
        test_wiki_fallback_real_snapshot_old_rag_and_ivory_comb,
        test_capture_tool_parses_last_field_before_closing_braces,
        test_capture_tool_page_status_value_defaults,
        test_capture_tool_itemlist_template_parsing,
        test_capture_tool_itemid_join_precedence_over_name_match,
        test_capture_tool_itemid_join_disagreement_or_failure_has_no_name_fallback,
        test_capture_tool_diagnose_id_matched_unresolved_is_report_only,
        test_capture_tool_actualname_index_and_disagreement,
        test_wiki_fallback_loader_rejects_missing_or_unknown_match_basis,
        test_committed_wiki_fallback_snapshot_loads_fail_closed,
        test_wiki_fallback_snapshot_is_registered,
        test_family_profile_evidence_shapes_are_mutually_exclusive,
        test_crystal_item_bindings_reject_duplicate_target_key,
        test_owner_family_decisions_resolves_real_examples,
        test_owner_family_decisions_only_applies_to_unresolved_items,
        test_owner_family_decisions_name_mismatch_is_ignored,
        test_owner_family_decisions_never_outranks_wrap_target_or_dead_item,
        test_owner_family_decisions_loader_fails_closed,
        test_owner_family_decisions_loader_rejects_unknown_registry_key,
        test_owner_family_decisions_loader_missing_file_is_hard_error,
        test_committed_owner_family_decisions_loads_fail_closed,
        test_empty_client_object_routes_appearance_placeholder_slot,
        test_empty_client_object_route_requires_zero_flags,
        test_empty_client_object_route_never_outranks_owner_table,
    ]
    for test in tests:
        test()
    print(f"PASS {CHECKS} checks")


if __name__ == "__main__":
    main()
