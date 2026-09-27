#!/usr/bin/env python3
"""Synthetic fail-closed tests for the Crystal item identity binding generator."""

from __future__ import annotations

import importlib.util
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("g4_item_crystal_binding_generator.py")
SPEC = importlib.util.spec_from_file_location(
    "g4_item_crystal_binding_generator", MODULE_PATH
)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("Crystal item binding generator import failed")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def expect_error(code: str, fn, *args, **kwargs) -> None:
    try:
        fn(*args, **kwargs)
    except MODULE.GeneratorError as exc:
        assert str(exc).startswith(code), f"expected {code}, got {exc}"
    else:
        raise AssertionError(f"expected GeneratorError {code}")


def main() -> None:
    assert (
        MODULE.opaque_item_key("oteryn:item.registry", 3167)
        == "oteryn:item.registry.i00003167"
    )
    assert (
        MODULE.opaque_item_key("oteryn:item.registry", 1)
        == "oteryn:item.registry.i00000001"
    )

    native_batch = {3035: "oteryn:item.currency.platinum_coin"}
    namespace = "oteryn:item.registry"

    # allocate_keys enforces the real 38,157/64 closure counts, so exercise it
    # at small scale by monkeypatching those two constants for this block only.
    original_opaque, original_native = (
        MODULE.EXPECTED_OPAQUE,
        MODULE.EXPECTED_NATIVE_BATCH,
    )
    try:
        MODULE.EXPECTED_OPAQUE, MODULE.EXPECTED_NATIVE_BATCH = 1, 1

        # Ascending, well-formed rows: one native-batch hit, one opaque allocation.
        good_rows = [{"source_item_id": 10}, {"source_item_id": 3035}]
        allocations = MODULE.allocate_keys(good_rows, native_batch, namespace)
        assert allocations == [
            (10, "oteryn:item.registry.i00000001"),
            (3035, "oteryn:item.currency.platinum_coin"),
        ]

        # Non-ascending source_item_id must fail closed.
        expect_error(
            "SOURCE_ITEM_ID_ORDER_VIOLATION",
            MODULE.allocate_keys,
            [{"source_item_id": 5}, {"source_item_id": 5}],
            native_batch,
            namespace,
        )
        expect_error(
            "SOURCE_ITEM_ID_ORDER_VIOLATION",
            MODULE.allocate_keys,
            [{"source_item_id": 5}, {"source_item_id": 4}],
            native_batch,
            namespace,
        )

        # A non-integer source_item_id must fail closed.
        expect_error(
            "SOURCE_ITEM_ID_INVALID",
            MODULE.allocate_keys,
            [{"source_item_id": "5"}],
            native_batch,
            namespace,
        )

        # A native-batch collision with an already-opaque-allocated key must fail closed.
        colliding_batch = {1: "oteryn:item.registry.i00000001"}
        MODULE.EXPECTED_OPAQUE, MODULE.EXPECTED_NATIVE_BATCH = 1, 1
        expect_error(
            "NATIVE_KEY_DUPLICATE",
            MODULE.allocate_keys,
            [{"source_item_id": 1}, {"source_item_id": 2}],
            colliding_batch,
            namespace,
        )

        # allocate_keys closure check: wrong EXPECTED_OPAQUE/EXPECTED_NATIVE_BATCH shape.
        MODULE.EXPECTED_OPAQUE, MODULE.EXPECTED_NATIVE_BATCH = 5, 5
        expect_error(
            "ALLOCATION_CLOSURE_MISMATCH",
            MODULE.allocate_keys,
            [{"source_item_id": 1}],
            native_batch,
            namespace,
        )
    finally:
        MODULE.EXPECTED_OPAQUE, MODULE.EXPECTED_NATIVE_BATCH = (
            original_opaque,
            original_native,
        )

    # verify_allocations: missing canonical definition key must fail closed.
    fake_allocations = [
        (i, f"oteryn:item.synthetic.i{i:08d}")
        for i in range(1, MODULE.EXPECTED_TOTAL + 1)
    ]
    fake_definition_keys = {key for _, key in fake_allocations}
    fake_definition_keys.discard(fake_allocations[0][1])
    expect_error(
        "ALLOCATED_KEY_MISSING_FROM_DEFINITIONS",
        MODULE.verify_allocations,
        fake_allocations,
        fake_definition_keys,
        {},
    )

    # verify_allocations: an extra canonical definition key with no allocation must fail closed.
    extra_definition_keys = {key for _, key in fake_allocations} | {
        "oteryn:item.unallocated"
    }
    expect_error(
        "DEFINITION_KEY_WITHOUT_ALLOCATION",
        MODULE.verify_allocations,
        fake_allocations,
        extra_definition_keys,
        {},
    )

    # verify_allocations: the Magic Sword golden cross-check must fail closed on drift.
    all_keys = {key for _, key in fake_allocations}
    expect_error(
        "CROSS_CHECK_KEY_MISMATCH",
        MODULE.verify_allocations,
        fake_allocations,
        all_keys,
        {},
    )
    drifted_cross_check_allocations = list(fake_allocations)
    drifted_cross_check_allocations[3287] = (3288, "oteryn:item.registry.i00003167")
    expect_error(
        "CROSS_CHECK_TIBIAWIKI_TARGET_MISSING",
        MODULE.verify_allocations,
        drifted_cross_check_allocations,
        {key for _, key in drifted_cross_check_allocations},
        {},
    )
    MODULE.verify_allocations(
        drifted_cross_check_allocations,
        {key for _, key in drifted_cross_check_allocations},
        {"oteryn:item.registry.i00003167": {"5810"}},
    )

    # Identity promotions: parsed from `<PREFIX>_{SOURCE_ITEM_ID,OLD_KEY,KEY}` constants.
    rust = (
        "pub const R7_P04_GOLD_COIN_SOURCE_ITEM_ID: u64 = 3_031;\n"
        'pub const R7_P04_GOLD_COIN_OLD_KEY: &str = "oteryn:item.registry.i00002921";\n'
        'pub const R7_P04_GOLD_COIN_KEY: &str = "oteryn:item.currency.gold_coin";\n'
    )
    promotions = MODULE.parse_identity_promotions(rust)
    assert promotions == {
        3031: ("oteryn:item.registry.i00002921", "oteryn:item.currency.gold_coin")
    }, promotions
    assert MODULE.apply_identity_promotions(
        [
            (3030, "oteryn:item.registry.i00002920"),
            (3031, "oteryn:item.registry.i00002921"),
        ],
        promotions,
    ) == [
        (3030, "oteryn:item.registry.i00002920"),
        (3031, "oteryn:item.currency.gold_coin"),
    ]
    expect_error(
        "IDENTITY_PROMOTION_OLD_KEY_MISMATCH",
        MODULE.apply_identity_promotions,
        [(3031, "oteryn:item.registry.i00009999")],
        promotions,
    )
    expect_error(
        "IDENTITY_PROMOTION_INCOMPLETE",
        MODULE.parse_identity_promotions,
        'pub const X_OLD_KEY: &str = "a";\n',
    )

    # build_bindings: deterministic canonical-bytes ordering and exact field shape.
    bindings = MODULE.build_bindings(
        [(30, "oteryn:item.b"), (7, "oteryn:item.a")], "deadbeef"
    )
    assert [row["external_id"] for row in bindings] == ["30", "7"], bindings
    assert bindings[0] == {
        "disposition": "EXACT",
        "external_id": "30",
        "identity_namespace": "ots/item_server_id",
        "source_key": "oteryn:source.crystalserver",
        "source_revision": "deadbeef",
        "target": {
            "family": "Item",
            "key": "oteryn:item.b",
            "revision": "definition-r1",
        },
    }

    print("g4_item_crystal_binding_generator synthetic self-test: PASS")


if __name__ == "__main__":
    main()
