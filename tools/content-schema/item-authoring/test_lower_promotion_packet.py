"""Focused, no-network checks for `lower_promotion_packet.py`: the encode/decode
mirror of `apps/game-server/src/content/cw2_b1_import.rs`'s
`decode_item_semantic_promotion_value`, and the end-to-end
`build_packet`/`validate_packet` wiring against synthetic engine sources (the same
`synthetic_sources`/`convert` fixtures `test_engine_items.py` uses: a real identity
index and disposition catalog, fabricated `items`/`appearances`, no pinned checkout).
"""

from __future__ import annotations

import lower_promotion_packet as lpp
from test_engine_items import synthetic_sources

CHECKS = 0


def check(condition, message):
    global CHECKS
    CHECKS += 1
    if not condition:
        raise AssertionError(message)


def expect_lowering_error(callable_, message):
    try:
        callable_()
    except lpp.LoweringError:
        check(True, message)
    else:
        raise AssertionError(f"expected LoweringError: {message}")


# --- typed_value: encode direction, mirrors decode_item_semantic_promotion_value ----


def test_typed_value_text():
    source, typed = lpp.typed_value("presentation.name", "Magic Sword")
    check(source == "Magic Sword", typed)
    check(typed == {"kind": "TEXT", "value": "Magic Sword"}, typed)

    # empty and over-length names are decoder-rejected: skip (None, None), never raise.
    check(lpp.typed_value("presentation.name", "") == (None, None), "empty name")
    check(
        lpp.typed_value("presentation.name", "x" * 2049) == (None, None),
        "over-length name",
    )
    check(
        lpp.typed_value("presentation.name", "x" * 2048)[1] is not None,
        "exactly 2048 bytes is still admitted",
    )


def test_typed_value_signed_points():
    for field_path in lpp.SIGNED_POINTS_FIELDS:
        source, typed = lpp.typed_value(field_path, -7)
        check(source == -7, (field_path, typed))
        check(typed == {"kind": "SIGNED_POINTS", "value": -7}, (field_path, typed))
    # out of the Rust i32 range is skipped, not clamped or raised.
    check(
        lpp.typed_value("weapon.attack", lpp.I32_MAX + 1) == (None, None),
        "over i32::MAX attack",
    )
    check(
        lpp.typed_value("weapon.attack", lpp.I32_MIN - 1) == (None, None),
        "under i32::MIN attack",
    )


def test_typed_value_cells():
    source, typed = lpp.typed_value("weapon.range_cells", 4)
    check(source == 4 and typed == {"kind": "CELLS", "value": 4}, typed)
    check(
        lpp.typed_value("weapon.range_cells", -1) == (None, None),
        "negative range_cells",
    )
    check(
        lpp.typed_value("weapon.range_cells", lpp.U16_MAX + 1) == (None, None),
        "over u16::MAX range_cells",
    )


def test_typed_value_rational_percent():
    # 75% -> canonical 3/4, matching ReferenceRationalPercent's lowest-terms rule.
    source, typed = lpp.typed_value("weapon.hit_chance", (75, 1))
    check(source == 75, typed)
    check(
        typed
        == {
            "kind": "RATIONAL_PERCENT",
            "numerator": 3,
            "denominator": 4,
            "source_unit": "PERCENT_POINTS",
        },
        typed,
    )
    # 0% -> canonical 0/1, not 0/100.
    source, typed = lpp.typed_value("weapon.hit_chance", (0, 1))
    check(source == 0 and typed["numerator"] == 0 and typed["denominator"] == 1, typed)
    # this field is only PERCENT_POINTS-eligible when the authored ratio is exact
    # whole percent (denominator 1); anything else is skipped.
    check(
        lpp.typed_value("weapon.hit_chance", (1, 3)) == (None, None),
        "non-integer percent ratio",
    )


def test_typed_value_count_and_capacity():
    source, typed = lpp.typed_value("charges.count", 5)
    check(source == 5 and typed == {"kind": "COUNT_U32", "value": 5}, typed)
    check(
        lpp.typed_value("charges.count", -1) == (None, None), "negative charges.count"
    )
    check(
        lpp.typed_value("charges.count", lpp.U32_MAX + 1) == (None, None),
        "over u32::MAX charges.count",
    )

    source, typed = lpp.typed_value("container.capacity", 20)
    check(source == 20 and typed == {"kind": "CAPACITY_U16", "value": 20}, typed)
    check(
        lpp.typed_value("container.capacity", lpp.U16_MAX + 1) == (None, None),
        "over u16::MAX container.capacity",
    )


# --- validate_row/validate_packet: decode direction, mirrors the same Rust function -


def sample_row(**overrides):
    row = {
        "field_path": "weapon.attack",
        "native_key": "oteryn:item.registry.i00000001",
        "source_item_id": 1,
        "source_value": 20,
        "typed_value": {"kind": "SIGNED_POINTS", "value": 20},
    }
    row.update(overrides)
    return row


def test_validate_row_accepts_every_admitted_kind():
    rows = [
        sample_row(),
        sample_row(
            field_path="presentation.name",
            source_value="fixture",
            typed_value={"kind": "TEXT", "value": "fixture"},
        ),
        sample_row(
            field_path="weapon.range_cells",
            source_value=4,
            typed_value={"kind": "CELLS", "value": 4},
        ),
        sample_row(
            field_path="weapon.hit_chance",
            source_value=75,
            typed_value={
                "kind": "RATIONAL_PERCENT",
                "numerator": 3,
                "denominator": 4,
                "source_unit": "PERCENT_POINTS",
            },
        ),
        sample_row(
            field_path="charges.count",
            source_value=5,
            typed_value={"kind": "COUNT_U32", "value": 5},
        ),
        sample_row(
            field_path="container.capacity",
            source_value=20,
            typed_value={"kind": "CAPACITY_U16", "value": 20},
        ),
    ]
    for row in rows:
        lpp.validate_row(row)  # must not raise
    check(True, "every admitted kind validates")


def test_validate_row_rejects_source_drift():
    row = sample_row()
    row["typed_value"] = {"kind": "SIGNED_POINTS", "value": 21}
    expect_lowering_error(
        lambda: lpp.validate_row(row), "typed value must equal source_value"
    )


def test_validate_row_rejects_unknown_field_path():
    row = sample_row(field_path="weapon.weapon_type")
    expect_lowering_error(
        lambda: lpp.validate_row(row), "field_path outside the admitted 9"
    )


def test_validate_row_rejects_typed_value_shape_drift():
    row = sample_row()
    row["typed_value"] = {"kind": "SIGNED_POINTS", "value": 20, "extra": 1}
    expect_lowering_error(lambda: lpp.validate_row(row), "extra typed_value key")

    row = sample_row()
    del row["typed_value"]["kind"]
    expect_lowering_error(lambda: lpp.validate_row(row), "missing typed_value key")


def test_validate_row_rejects_wrong_kind_label():
    row = sample_row()
    row["typed_value"]["kind"] = "CELLS"
    expect_lowering_error(lambda: lpp.validate_row(row), "kind label must match field")


def test_validate_row_rejects_empty_and_oversized_text():
    row = sample_row(
        field_path="presentation.name",
        source_value="",
        typed_value={"kind": "TEXT", "value": ""},
    )
    expect_lowering_error(lambda: lpp.validate_row(row), "empty TEXT value")

    long_value = "x" * 2049
    row = sample_row(
        field_path="presentation.name",
        source_value=long_value,
        typed_value={"kind": "TEXT", "value": long_value},
    )
    expect_lowering_error(lambda: lpp.validate_row(row), "over-length TEXT value")


def test_validate_row_rejects_non_canonical_rational_percent():
    # 75/100 is proportional but not reduced to lowest terms: ReferenceRationalPercent
    # requires gcd(|numerator|, denominator) == 1.
    row = sample_row(
        field_path="weapon.hit_chance",
        source_value=75,
        typed_value={
            "kind": "RATIONAL_PERCENT",
            "numerator": 75,
            "denominator": 100,
            "source_unit": "PERCENT_POINTS",
        },
    )
    expect_lowering_error(lambda: lpp.validate_row(row), "non-canonical fraction")


def test_validate_row_rejects_wrong_percent_proportion():
    row = sample_row(
        field_path="weapon.hit_chance",
        source_value=75,
        typed_value={
            "kind": "RATIONAL_PERCENT",
            "numerator": 1,
            "denominator": 2,
            "source_unit": "PERCENT_POINTS",
        },
    )
    expect_lowering_error(lambda: lpp.validate_row(row), "numerator*100 != source*den")


def test_validate_row_rejects_wrong_source_unit():
    row = sample_row(
        field_path="weapon.hit_chance",
        source_value=75,
        typed_value={
            "kind": "RATIONAL_PERCENT",
            "numerator": 3,
            "denominator": 4,
            "source_unit": "PERCENT",
        },
    )
    expect_lowering_error(lambda: lpp.validate_row(row), "wrong source_unit")


def test_validate_row_rejects_out_of_range_values():
    row = sample_row(source_value=lpp.I32_MAX + 1)
    row["typed_value"]["value"] = lpp.I32_MAX + 1
    expect_lowering_error(lambda: lpp.validate_row(row), "SIGNED_POINTS over i32::MAX")

    row = sample_row(
        field_path="container.capacity",
        source_value=lpp.U16_MAX + 1,
        typed_value={"kind": "CAPACITY_U16", "value": lpp.U16_MAX + 1},
    )
    expect_lowering_error(lambda: lpp.validate_row(row), "CAPACITY_U16 over u16::MAX")


def sample_packet(rows):
    per_field = {}
    for row in rows:
        per_field[row["field_path"]] = per_field.get(row["field_path"], 0) + 1
    return {
        "counts": {
            "promoted_fields": len(rows),
            "promoted_items": len({row["native_key"] for row in rows}),
            "per_field": dict(sorted(per_field.items())),
        },
        "promotions": rows,
    }


def test_validate_packet_rejects_ordering_violation():
    rows = [
        sample_row(native_key="oteryn:item.registry.i00000002"),
        sample_row(native_key="oteryn:item.registry.i00000001"),
    ]
    expect_lowering_error(
        lambda: lpp.validate_packet(sample_packet(rows)), "rows must be sorted"
    )


def test_validate_packet_rejects_duplicate_atom():
    rows = [sample_row(), sample_row()]
    expect_lowering_error(
        lambda: lpp.validate_packet(sample_packet(rows)),
        "duplicate (native_key, field_path) atom",
    )


def test_validate_packet_rejects_count_drift():
    rows = [sample_row()]
    packet = sample_packet(rows)
    packet["counts"]["promoted_fields"] = 2
    expect_lowering_error(
        lambda: lpp.validate_packet(packet), "counts.promoted_fields must match rows"
    )


# --- end-to-end: build_packet against a tiny synthetic Crystal population ----------


def fixture_sources():
    records = {
        360: {
            "attrs": {
                "primarytype": "axe weapons",
                "weapontype": "axe",
                "attack": "20",
                "defense": "10",
                "extradef": "5",
                "range": "1",
                "hitchance": "75",
            }
        },
        250: {
            "attrs": {"primarytype": "containers", "containersize": "20"},
            "flags": {"flags.container": True},
        },
        270: {"attrs": {"primarytype": "valuables", "charges": "5"}},
        280: {
            "attrs": {"primarytype": "armors", "armor": "12"},
            "flags": {"clothes.slot": "BODY"},
        },
    }
    sources = synthetic_sources("crystal", records)
    # `load_engine_sources` always sets this; the plain `synthetic_sources` test
    # helper does not, since it skips the real items.xml/appearances.dat round trip.
    sources["artifact_digests"] = {}
    return sources


def test_build_packet_covers_every_admitted_field_path():
    packet, skipped = lpp.build_packet(fixture_sources())
    check(not skipped, skipped)
    check(packet["counts"]["promoted_items"] == 4, packet["counts"])
    check(
        packet["counts"]["per_field"]
        == {
            "charges.count": 1,
            "container.capacity": 1,
            "presentation.name": 4,
            "protection.armor": 1,
            "weapon.attack": 1,
            "weapon.defense": 1,
            "weapon.extra_defense": 1,
            "weapon.hit_chance": 1,
            "weapon.range_cells": 1,
        },
        packet["counts"],
    )
    check(packet["counts"]["promoted_fields"] == len(packet["promotions"]), packet)

    rows = {(row["native_key"], row["field_path"]): row for row in packet["promotions"]}
    weapon_key = "oteryn:item.registry.i00000277"
    check(
        rows[(weapon_key, "weapon.attack")]["typed_value"]
        == {"kind": "SIGNED_POINTS", "value": 20},
        rows,
    )
    check(
        rows[(weapon_key, "weapon.hit_chance")]["typed_value"]
        == {
            "kind": "RATIONAL_PERCENT",
            "numerator": 3,
            "denominator": 4,
            "source_unit": "PERCENT_POINTS",
        },
        rows,
    )
    # `build_packet` already runs `validate_packet` on its own output; calling it
    # again here must be a no-op (no raise), proving the emitted packet is self-valid.
    lpp.validate_packet(packet)


def test_build_packet_orders_rows_by_native_key_then_field_path_then_source_item_id():
    packet, _skipped = lpp.build_packet(fixture_sources())
    ordering_keys = [
        (row["native_key"], row["field_path"], row["source_item_id"])
        for row in packet["promotions"]
    ]
    check(ordering_keys == sorted(ordering_keys), ordering_keys)
    check(len(ordering_keys) == len(set(ordering_keys)), "no duplicate atoms")


def test_build_packet_skips_a_decoder_out_of_range_value_instead_of_smuggling_it():
    """This package's own `item.schema.json` puts no upper bound on a plain
    `weapon.attack` integer; the Rust decoder's i32 bound is narrower. `build_packet`
    must skip (and count) that one field rather than emit a row the Rust decoder
    would reject, while still promoting the item's other eligible fields."""
    sources = fixture_sources()
    sources["items"][290] = {
        "name": "synthetic item 290",
        "article": "a",
        "plural": None,
        "attrs": {
            "primarytype": "axe weapons",
            "weapontype": "axe",
            "attack": str(lpp.I32_MAX + 1),
        },
    }
    packet, skipped = lpp.build_packet(sources)
    check(skipped == {"weapon.attack": 1}, skipped)
    rows = {(row["native_key"], row["field_path"]) for row in packet["promotions"]}
    over_range_key = "oteryn:item.registry.i00000207"
    check((over_range_key, "weapon.attack") not in rows, rows)
    check((over_range_key, "presentation.name") in rows, rows)
    check(packet["counts"]["promoted_items"] == 5, packet["counts"])


def main():
    tests = [
        test_typed_value_text,
        test_typed_value_signed_points,
        test_typed_value_cells,
        test_typed_value_rational_percent,
        test_typed_value_count_and_capacity,
        test_validate_row_accepts_every_admitted_kind,
        test_validate_row_rejects_source_drift,
        test_validate_row_rejects_unknown_field_path,
        test_validate_row_rejects_typed_value_shape_drift,
        test_validate_row_rejects_wrong_kind_label,
        test_validate_row_rejects_empty_and_oversized_text,
        test_validate_row_rejects_non_canonical_rational_percent,
        test_validate_row_rejects_wrong_percent_proportion,
        test_validate_row_rejects_wrong_source_unit,
        test_validate_row_rejects_out_of_range_values,
        test_validate_packet_rejects_ordering_violation,
        test_validate_packet_rejects_duplicate_atom,
        test_validate_packet_rejects_count_drift,
        test_build_packet_covers_every_admitted_field_path,
        test_build_packet_orders_rows_by_native_key_then_field_path_then_source_item_id,
        test_build_packet_skips_a_decoder_out_of_range_value_instead_of_smuggling_it,
    ]
    for test in tests:
        test()
    print(f"PASS {CHECKS} checks")


if __name__ == "__main__":
    main()
