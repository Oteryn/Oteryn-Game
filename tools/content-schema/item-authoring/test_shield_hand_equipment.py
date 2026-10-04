"""Explicit single Shield Hand claims retain their source-backed pattern."""

import lower_wiki_stats_packet as lower


def test_single_shield_hand():
    for vocation in ("sorcerers", "druids"):
        fields = {
            "primarytype": "Spellbooks",
            "slot": "Shield Hand",
            "hands": "One",
            "levelrequired": "350",
            "vocrequired": vocation,
        }
        _, result = lower.equipment(fields)
        assert isinstance(result, dict), result
        pattern = result["value"][0]
        assert pattern["primary_slot"] == lower.known("SHIELD")
        assert pattern["additional_reserved_slots"] == lower.known([])
        assert pattern["level"] == lower.known(350)
        assert pattern["vocations"] == lower.known([vocation[:-1].upper()])
        assert lower.equipment(fields | {"hands": "Two"})[1] == "MALFORMED"
        extra = lower.equipment(fields | {"slot": "Extra Slot"})[1]["value"][0]
        assert extra["primary_slot"] == lower.known("EXTRA")
        assert extra["level"] == extra["vocations"] == {"state": "UNKNOWN"}
    _, absent = lower.equipment({"primarytype": "Spellbooks", "slot": "Shield Hand"})
    assert absent["value"][0]["additional_reserved_slots"] == lower.known([])


if __name__ == "__main__":
    test_single_shield_hand()
    print("Shield Hand equipment regression passed")
