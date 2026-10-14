"""No-network checks for `lower_ranged_packet.py` (RANGED-CONTENT-1, RANGED-0 section 7).

Synthetic wiki and Canary data cover the kind rules, the on_use rules, wiki-first with the Canary
fallback, the parity and conflict reports and the holds; the committed packet must rebuild byte
for byte.
"""

from __future__ import annotations

import json

import lower_ranged_packet as lower
from lower_equip_abilities_packet import canary_pin
from lower_wiki_stats_packet import SNAPSHOT, content_item_ids


def key(item_id):
    return f"oteryn:item.tibia.i{item_id}"


def snapshot(**items):
    """`items`: item id (as `iN`) -> list of field maps, one per wiki page observation."""
    records = {}
    for name, pages in items.items():
        item_id = int(name[1:])
        records[key(item_id)] = {
            "item_id": item_id,
            "observations": [
                {"page_id": 100 + n, "revision_id": 1, "fields": fields}
                for n, fields in enumerate(pages)
            ],
        }
    return {"records": records}


def run(wiki, canary, ids):
    return lower.build(wiki, canary, set(ids))


def row(promotions, item_id):
    (found,) = [p for p in promotions if p["item_key"] == key(item_id)]
    return found


SPEAR = {
    "top": {
        "weapontype": "distance",
        "shoottype": "spear",
        "range": "3",
        "maxhitchance": "76",
    },
    "nested": {"weapontype": "missile", "breakchance": "3"},
}
SNOWBALL = {
    "top": {"weapontype": "distance", "shoottype": "snowball", "range": "6"},
    "nested": {"weapontype": "missile", "action": "removecount"},
}
BOW = {
    "top": {"weapontype": "distance", "ammotype": "arrow", "range": "6"},
    "nested": {"weapontype": "distance"},
}
ARROW = {
    "top": {"weapontype": "ammunition", "ammotype": "arrow", "shoottype": "arrow"},
    "nested": {"weapontype": "ammunition", "action": "removecount"},
}
WAND = {
    "top": {"weapontype": "wand", "shoottype": "energy", "range": "3"},
    "nested": {
        "weapontype": "wand",
        "mana": "5",
        "fromdamage": "10",
        "todamage": "20",
        "wandtype": "energy",
        "vocation": "Sorcerer;true, Master Sorcerer",
        "level": "7",
    },
}


def test_kinds_and_on_use():
    canary = {1: SPEAR, 2: SNOWBALL, 3: BOW, 4: ARROW, 5: WAND}
    promotions, holds, conflicts, counts = run(snapshot(), canary, canary)
    assert (holds, conflicts) == ([], [])
    spear = row(promotions, 1)
    assert (spear["weapon_kind"], spear["on_use"], spear["break_chance_pct"]) == (
        "THROWING",
        "DROP",
        3,
    )
    assert (
        spear["range_tiles"] == 3
        and spear["max_hit_chance"] == 76
        and spear["missile"] == "spear"
    )
    assert row(promotions, 2)["on_use"] == "CONSUME"
    bow = row(promotions, 3)
    assert (bow["weapon_kind"], bow["ammo_type"], bow["on_use"]) == (
        "DISTANCE",
        "ARROW",
        "NONE",
    )
    assert "missile" not in bow
    arrow = row(promotions, 4)
    assert (arrow["weapon_kind"], arrow["on_use"], arrow["break_chance_pct"]) == (
        "AMMUNITION",
        "CONSUME",
        0,
    )
    wand = row(promotions, 5)
    assert (wand["mana_per_shot"], wand["damage_min"], wand["damage_max"]) == (
        5,
        10,
        20,
    )
    assert wand["element"] == "ENERGY" and wand["on_use"] == "NONE"
    assert wand["requirements"] == {"min_level": 7, "vocations": ["SORCERER"]}
    assert counts["kind_THROWING"] == 2 and counts["kind_WAND"] == 1


def test_wiki_first_with_canary_fallback_and_parity():
    wiki = snapshot(i3=[{"range": "7", "hit_chance": "90%"}])
    promotions, _, _, _ = run(wiki, {3: BOW}, {3})
    bow = row(promotions, 3)
    assert bow["range_tiles"] == 7 and bow["sources"]["range_tiles"] == "WIKI"
    assert bow["max_hit_chance"] == 90 and bow["sources"]["max_hit_chance"] == "WIKI"
    assert "hit_chance" not in bow
    assert bow["parity_pending"] == ["range_tiles"]
    same = snapshot(i3=[{"range": "6"}])
    bow = row(run(same, {3: BOW}, {3})[0], 3)
    assert "parity_pending" not in bow
    bare = row(run(snapshot(), {3: BOW}, {3})[0], 3)
    assert bare["sources"]["range_tiles"] == "CANARY"


def test_negative_canary_hit_chance_is_kept():
    devileye = {
        "top": {
            "weapontype": "distance",
            "ammotype": "bolt",
            "range": "6",
            "hitchance": "-20",
        },
        "nested": {"weapontype": "distance"},
    }
    assert row(run(snapshot(), {3: devileye}, {3})[0], 3)["hit_chance"] == -20
    too_low = {**devileye, "top": {**devileye["top"], "hitchance": "-101"}}
    assert "hit_chance" not in row(run(snapshot(), {3: too_low}, {3})[0], 3)


def test_requirements_carry_provenance_and_parity():
    wiki = snapshot(i3=[{"levelrequired": "0"}])
    canary = {3: {**BOW, "nested": {**BOW["nested"], "level": "1"}}}
    promotions, _, _, _ = run(wiki, canary, {3})
    found = row(promotions, 3)
    assert found["sources"]["min_level"] == "WIKI"
    assert "min_level" in found["parity_pending"]
    only_canary = row(run(snapshot(), canary, {3})[0], 3)
    assert only_canary["sources"]["min_level"] == "CANARY"
    assert only_canary["requirements"]["min_level"] == 1


def test_quiver_source_follows_the_wiki_type():
    quiver = {"top": {"primarytype": "quivers", "containersize": "10"}, "nested": {}}
    from_canary = row(run(snapshot(), {3: quiver}, {3})[0], 3)
    assert from_canary["sources"] == {"quiver": "CANARY", "container_size": "CANARY"}
    wiki = snapshot(i3=[{"primarytype": "Quivers"}])
    assert row(run(wiki, {3: quiver}, {3})[0], 3)["sources"]["quiver"] == "WIKI"


def test_wiki_hit_modifier_and_quiver_volume_come_first():
    assert [lower._parse_signed(x) for x in ("+3", "6", "-20", "x")] == [
        3,
        6,
        -20,
        None,
    ]
    bow = {"top": dict(BOW["top"], hitchance="5"), "nested": BOW["nested"]}
    found = row(run(snapshot(i3=[{"hit_mod": "+3"}]), {3: bow}, {3})[0], 3)
    assert found["hit_chance"] == 3
    assert found["sources"]["hit_chance"] == "WIKI"
    assert "hit_chance" in found["parity_pending"]
    quiver = {"top": {"primarytype": "quivers", "containersize": "10"}, "nested": {}}
    wiki = snapshot(i3=[{"primarytype": "Quivers", "volume": "8"}])
    found = row(run(wiki, {3: quiver}, {3})[0], 3)
    assert found["container_size"] == 8
    assert found["sources"]["container_size"] == "WIKI"
    assert found["parity_pending"] == ["container_size"]


def test_wiki_damage_with_average_prefix():
    assert lower._parse_damage("107 (98-116)") == (98, 116)
    assert lower._parse_damage("98-116") == (98, 116)
    assert lower._parse_damage("107 (98-116") is None


def test_wiki_mana_cost_decorations():
    assert lower._parse_mana("+3 mana per attack") == 3
    assert lower._parse_mana("+10") == 10
    assert lower._parse_mana("21") == 21
    assert lower._parse_mana("+7-8 mana per attack") is None
    assert lower._parse_mana("~2+~6+ mana per attack") is None


def test_ammunition_break_chance_is_derived():
    found = row(run(snapshot(), {3: ARROW}, {3})[0], 3)
    assert found["break_chance_pct"] == 0
    assert found["sources"]["break_chance_pct"] == "DERIVED"


def test_quiver_requirements_have_provenance_and_parity():
    quiver = {
        "top": {"primarytype": "quivers", "containersize": "10"},
        "nested": {"level": "1"},
    }
    wiki = snapshot(
        i3=[{"primarytype": "Quivers", "levelrequired": "0", "vocrequired": "Paladins"}]
    )
    found = row(run(wiki, {3: quiver}, {3})[0], 3)
    assert found["requirements"] == {"vocations": ["PALADIN"]}
    assert found["sources"]["min_level"] == "WIKI"
    assert found["sources"]["vocations"] == "WIKI"
    assert found["parity_pending"] == ["min_level"]


def test_zero_minimum_damage_and_derived_throwing_defaults():
    assert lower._parse_damage("0-10") == (0, 10)
    assert lower._parse_damage("10-0") is None
    assert lower._parse_damage("5-70000") is None
    found = row(run(snapshot(), {1: SNOWBALL}, {1})[0], 1)
    assert found["break_chance_pct"] == 0
    assert found["sources"]["break_chance_pct"] == "DERIVED"
    assert found["sources"]["on_use"] == "CANARY"
    spear = row(run(snapshot(), {1: SPEAR}, {1})[0], 1)
    assert spear["sources"]["break_chance_pct"] == "CANARY"


def test_snapshot_digest_is_verified():
    tampered = {"records": {}, "snapshot_sha256": "0" * 64}
    try:
        lower.packet_bytes(tampered, {}, set(), lower.canary_pin())
    except ValueError as error:
        assert "digest mismatch" in str(error)
    else:
        raise AssertionError("tampered wiki snapshot accepted")


def test_literal_none_vocation_is_no_restriction():
    wiki = snapshot(i1=[{"vocrequired": "None"}])
    promotions, _, conflicts, _ = run(wiki, {1: SPEAR}, {1})
    assert "vocations" not in row(promotions, 1).get("requirements", {})
    assert not [c for c in conflicts if c["field"] == "vocrequired"]


def test_conflicts_use_canary_and_are_reported():
    wiki = snapshot(i3=[{"range": "?"}], i1=[{"range": "4"}, {"range": "5"}])
    promotions, _, conflicts, _ = run(wiki, {1: SPEAR, 3: BOW}, {1, 3})
    assert row(promotions, 3)["range_tiles"] == 6
    assert row(promotions, 1)["range_tiles"] == 3
    assert {(c["item_key"], c["reason"]) for c in conflicts} == {
        (key(3), "wiki_unparseable"),
        (key(1), "wiki_pages_disagree"),
    }


def test_holds():
    unregistered = {
        "top": {"weapontype": "distance", "ammotype": "arrow", "range": "6"},
        "nested": None,
    }
    promotions, holds, _, _ = run(snapshot(), {1: unregistered, 2: SPEAR}, {1})
    assert promotions == []
    assert holds[0] == {"item_key": key(1), "reasons": ["no_weapon_registration"]}
    assert holds[1] == {"item_key": key(2), "reasons": ["no_content_definition"]}


def test_wiki_vocation_parsing():
    parse = lower._parse_vocations
    assert parse("sorcerers and druids") == ["SORCERER", "DRUID"]
    assert parse("paladins") == ["PALADIN"]
    assert parse("without") == ["NONE"]
    assert parse("paladins and without") == ["PALADIN", "NONE"]
    assert parse("hunters") is None


def test_committed_packet_rebuilds():
    wiki = json.loads(SNAPSHOT.read_text(encoding="utf-8"))
    data = lower.packet_bytes(
        wiki,
        lower.load_canary_weapons(),
        content_item_ids(),
        canary_pin(),
    )
    assert lower.OUTPUT.read_bytes() == data


if __name__ == "__main__":
    for name, test in sorted(globals().items()):
        if name.startswith("test_") and callable(test):
            test()
    print("lower_ranged_packet tests: PASS")


def test_wiki_physical_damage_type_is_recognized():
    assert lower._parse_element("Physical") == "PHYSICAL"
    wand = {"top": WAND["top"], "nested": dict(WAND["nested"], wandtype="")}
    wiki = snapshot(i3=[{"primarytype": "Wands", "damagetype": "Physical"}])
    found = row(run(wiki, {3: wand}, {3})[0], 3)
    assert found["element"] == "PHYSICAL"
    assert found["sources"]["element"] == "WIKI"


def test_proof_owned_rows_stay_held_but_keep_parity():
    bow = {"top": dict(BOW["top"], hitchance="5"), "nested": BOW["nested"]}
    wiki = snapshot(i3=[{"range": "7"}])
    promotions, holds, _, _ = lower.build(wiki, {3: bow}, {3}, {key(3)})
    assert promotions == []
    assert holds == [
        {
            "item_key": key(3),
            "reasons": ["owned_by_weapon_metadata_103"],
            "parity_pending": ["range_tiles"],
        }
    ]
