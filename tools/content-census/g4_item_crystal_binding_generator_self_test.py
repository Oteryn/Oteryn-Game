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


def candidate(base_id: int, matched: list[str], contradicted: list[str]) -> dict:
    return {
        "base_source_item_id": base_id,
        "matched": matched,
        "contradicted": contradicted,
    }


def epoch2_alias_gate_and_allocation() -> None:
    """Alias gate truth table, `NO_MATCH`-only allocation, ordering and fail-closed checks."""
    gate = MODULE.resolve_alias_gate
    both = ["article_plural", "attributes"]
    full = [*both, "visual"]

    # Names only discover candidates: no same-name item, or only name-level agreement.
    assert gate([])[:2] == ("NO_MATCH", "NO_SAME_NAME_BASE_ITEM")
    assert gate([candidate(1, ["article_plural"], ["attributes"])])[0] == "NO_MATCH"
    assert gate([candidate(1, [], both)])[0] == "NO_MATCH"
    assert MODULE.counterpart(candidate(1, both, ["visual"]))
    assert not MODULE.counterpart(
        candidate(1, ["visual", "attributes"], ["article_plural"])
    )
    # Presentation is never an identity signal: a candidate that agrees on article/plural
    # and the full attribute set but differs only in sprite is held, not minted (a sprite
    # change never remints an identity). Its differing sprite is not evidence of a distinct
    # identity, so it is neither NO_MATCH nor an alias.
    assert gate([candidate(1, both, ["visual"])])[:2] == (
        "PROBABLE_MATCH",
        "SPRITE_ONLY_DIFFERENCE_HELD",
    )
    assert gate([candidate(1, both, ["visual"]), candidate(2, both, ["visual"])])[
        0
    ] == ("AMBIGUOUS")
    # Only non-presentation contradictions leave NO_MATCH, whatever the sprite says.
    assert gate([candidate(1, ["article_plural"], ["attributes", "visual"])])[:2] == (
        "NO_MATCH",
        "SAME_NAME_CANDIDATES_CONTRADICTED_BY_NON_PRESENTATION_FACTS",
    )
    assert gate([candidate(1, ["visual", "attributes"], ["article_plural"])])[0] == (
        "CONFLICT"
    )
    assert (
        gate(
            [
                candidate(1, ["article_plural"], ["attributes", "visual"]),
                candidate(2, both, ["visual"]),
            ]
        )[0]
        == "PROBABLE_MATCH"
    )
    # Proven counterpart: all three non-name signals agree, and it is unique.
    assert gate([candidate(1, full, [])])[0] == "ACCEPTED_ALIAS"
    assert (
        gate(
            [
                candidate(1, full, []),
                candidate(2, ["article_plural"], ["attributes", "visual"]),
            ]
        )[0]
        == "ACCEPTED_ALIAS"
    )
    # A second counterpart that differs only in sprite competes with the alias target.
    assert (
        gate([candidate(1, full, []), candidate(2, both, ["visual"])])[0] == "AMBIGUOUS"
    )
    # Two counterparts cannot both be the alias target.
    assert gate([candidate(1, full, []), candidate(2, full, [])])[0] == "AMBIGUOUS"
    # No visual signal: two agreeing signals are only a probable match, never a binding.
    assert gate([candidate(1, both, [])])[0] == "PROBABLE_MATCH"
    assert gate([candidate(1, both, []), candidate(2, both, [])])[0] == "AMBIGUOUS"

    signals = {"article": "a", "plural": None, "attrs": {"weight": "1"}, "visual": "v"}
    assert MODULE.compare_alias_signals(signals, dict(signals)) == (
        ["article_plural", "attributes", "visual"],
        [],
    )
    assert MODULE.compare_alias_signals(
        signals, {**signals, "attrs": {}, "visual": None}
    ) == (["article_plural"], ["attributes"])
    assert MODULE.normalize_name("  Magic   Portal ") == "magic portal"

    # Allocation: only NO_MATCH mints, in ascending source id, from 38,094 upward, and the
    # key is the rank among the minted ids (an alias or unresolved id consumes no number).
    namespace = "oteryn:item.registry"
    rows = [
        {"source_item_id": 100, "state": "NO_MATCH"},
        {"source_item_id": 200, "state": "ACCEPTED_ALIAS"},
        {"source_item_id": 300, "state": "NO_MATCH"},
        {"source_item_id": 400, "state": "AMBIGUOUS"},
        {"source_item_id": 500, "state": "PROBABLE_MATCH"},
        {"source_item_id": 600, "state": "CONFLICT"},
        {"source_item_id": 700, "state": "NO_MATCH"},
    ]
    assert MODULE.EPOCH1_HIGHEST_SEQUENCE == 38_093
    assert MODULE.allocate_epoch2(rows, namespace) == [
        (100, "oteryn:item.registry.i00038094"),
        (300, "oteryn:item.registry.i00038095"),
        (700, "oteryn:item.registry.i00038096"),
    ]
    expect_error(
        "CROSSWALK_SOURCE_ID_ORDER_VIOLATION",
        MODULE.allocate_epoch2,
        [rows[2], rows[0]],
        namespace,
    )
    expect_error(
        "CROSSWALK_SOURCE_ID_ORDER_VIOLATION",
        MODULE.allocate_epoch2,
        [rows[0], rows[0]],
        namespace,
    )
    expect_error(
        "CROSSWALK_STATE_INVALID",
        MODULE.allocate_epoch2,
        [{"source_item_id": 1, "state": "EXACTISH"}],
        namespace,
    )

    # Bindings: EXACT for minted ids, ACCEPTED_ALIAS to the existing key, nothing otherwise.
    alias_rows = [
        {**row, "alias_target_key": "oteryn:item.registry.i00000007"}
        if row["state"] == "ACCEPTED_ALIAS"
        else row
        for row in rows
    ]
    allocations = MODULE.allocate_epoch2(alias_rows, namespace)
    bindings = MODULE.epoch2_bindings(
        allocations, {"rows": alias_rows}, "donor-rev", "definition-r1"
    )
    assert [
        (row["external_id"], row["disposition"], row["target"]["key"])
        for row in bindings
    ] == [
        ("100", "EXACT", "oteryn:item.registry.i00038094"),
        ("200", "ACCEPTED_ALIAS", "oteryn:item.registry.i00000007"),
        ("300", "EXACT", "oteryn:item.registry.i00038095"),
        ("700", "EXACT", "oteryn:item.registry.i00038096"),
    ], bindings
    assert all(row["source_revision"] == "donor-rev" for row in bindings)

    # Same construction as the frozen import: `id NUL key LF`, SHA-256.
    assert MODULE.allocation_digest([(1, "k")]) == MODULE.sha256_hex(b"1\x00k\n")


def epoch2_committed_output() -> None:
    """The committed inputs and output: range, count, ordering, determinism, epoch 1 intact."""
    text = MODULE.read_text(MODULE.RUST_SOURCE)
    pins = MODULE.parse_epoch2_pins(text)
    namespace = MODULE.parse_opaque_namespace(text)
    census, census_payload = MODULE.load_census()
    crosswalk = MODULE.load_alias_crosswalk(pins)
    ids = MODULE.census_ids(census)
    assert ids == sorted(ids) and len(ids) == pins["census_id_count"] == 412

    epoch1_allocations = MODULE.apply_identity_promotions(
        MODULE.allocate_keys(
            MODULE.load_identity_records(),
            MODULE.parse_native_batch(text),
            namespace,
        ),
        MODULE.parse_identity_promotions(text),
    )
    allocations = MODULE.epoch2_allocations(
        census, census_payload, crosswalk, pins, epoch1_allocations, namespace
    )

    # Key range and count: NO_MATCH ids only, ascending by source id, 38,094..38,505.
    states = [row["state"] for row in crosswalk["rows"]]
    assert set(states) <= set(MODULE.EPOCH2_STATES)
    no_match = [
        row["source_item_id"] for row in crosswalk["rows"] if row["state"] == "NO_MATCH"
    ]
    assert [source_id for source_id, _ in allocations] == no_match
    assert no_match == sorted(no_match)
    assert len(allocations) == pins["minted_count"] == states.count("NO_MATCH")
    assert [key for _, key in allocations] == [
        f"oteryn:item.registry.i{sequence:08d}"
        for sequence in range(38_094, 38_094 + len(allocations))
    ]
    # Eight sprite-only same-name donors are held (no key, no alias, no binding), so 404 of
    # the 412 census ids mint: the range is derived as 38,094 .. 38,497.
    held = sorted(
        row["source_item_id"] for row in crosswalk["rows"] if row["state"] != "NO_MATCH"
    )
    assert held == [35500, 53380, 54609, 54610, 54613, 54614, 54615, 54616]
    by_id = {row["source_item_id"]: row for row in crosswalk["rows"]}
    assert by_id[35500]["state"] == "PROBABLE_MATCH"
    assert by_id[35500]["reason"] == "SPRITE_ONLY_DIFFERENCE_HELD"
    assert by_id[54610]["state"] == "PROBABLE_MATCH"
    assert all(
        by_id[i]["state"] == "AMBIGUOUS" for i in held if i not in (35500, 54610)
    )
    sprite_only = [
        c
        for c in by_id[35500]["same_name_base_items"]
        if c["base_source_item_id"] == 35502
    ]
    assert sprite_only == [
        {
            "base_source_item_id": 35502,
            "matched": ["article_plural", "attributes"],
            "contradicted": ["visual"],
        }
    ]
    assert len(allocations) == 404 == len(ids) - len(held)
    assert not {source_id for source_id, _ in allocations} & set(held)
    assert allocations[0][1].endswith("i00038094")
    assert allocations[-1][1].endswith("i00038497")
    # One crosswalk row per census id; nothing minted twice.
    assert len(crosswalk["rows"]) == len(ids)
    assert len({key for _, key in allocations}) == len(allocations)

    # No collision with any epoch-1 key or source id; retired 2,921 is never reassigned.
    epoch1_keys = {key for _, key in epoch1_allocations}
    assert not epoch1_keys & {key for _, key in allocations}
    assert not {source_id for source_id, _ in epoch1_allocations} & set(ids)
    assert "oteryn:item.registry.i00002921" not in epoch1_keys
    assert "oteryn:item.registry.i00002921" not in {key for _, key in allocations}
    highest = max(
        int(key.rsplit(".i", 1)[1])
        for key in epoch1_keys
        if key.startswith(namespace + ".i")
    )
    assert highest == MODULE.EPOCH1_HIGHEST_SEQUENCE == 38_093

    # Re-run determinism: two independent generations are byte-identical.
    output, payload = MODULE.generate()
    again_output, again_payload = MODULE.generate()
    assert payload == again_payload and output == again_output
    assert MODULE.OUTPUT.read_bytes() == payload

    # History: the historical epoch-1 rows still reproduce the retired bytes exactly.
    epoch1 = MODULE.build_bindings(
        epoch1_allocations, MODULE.parse_source_revision(text)
    )
    epoch1_bytes = MODULE.canonical_bytes(
        {"schema": MODULE.SCHEMA, "family": "Item", "bindings": epoch1}
    )
    assert MODULE.sha256_hex(epoch1_bytes) == MODULE.EXPECTED_EPOCH1_OUTPUT_SHA256
    assert len(epoch1_bytes) == MODULE.EXPECTED_EPOCH1_OUTPUT_BYTES

    # A12 §4.2: every emitted row is EXACT to the Tibia key of its own external id; the
    # D149 rows (no CipSoft appearance) and the held donor ids emit no binding.
    bindings = output["bindings"]
    assert len(bindings) == MODULE.EXPECTED_BOUND == 33_971
    assert bindings == sorted(bindings, key=MODULE.canonical_bytes)
    for row in bindings:
        assert row["disposition"] == "EXACT", row
        assert row["target"]["key"] == f"oteryn:item.tibia.i{row['external_id']}", row
    aliases = MODULE.load_alias_entries()
    d149 = {
        str(entry["evidence"]["source_item_id"])
        for entry in aliases.values()
        if entry["state"] == "RETIRED_WITHOUT_SUCCESSOR"
    }
    assert len(d149) == MODULE.EXPECTED_D149_UNBOUND
    assert not {row["external_id"] for row in bindings} & (
        d149 | {str(i) for i in held}
    )

    # Epoch-2 rows: one EXACT binding per minted id at the donor commit, source id verbatim.
    epoch2 = [
        row for row in bindings if row["source_revision"] == pins["source_revision"]
    ]
    assert sorted(row["external_id"] for row in epoch2) == sorted(
        str(source_id) for source_id, _ in allocations
    )

    # Requalification fails closed on a row whose alias evidence is another row's.
    row = dict(epoch1[0])
    expect_error(
        "ALIAS_EVIDENCE_NOT_THIS_ROW",
        MODULE.requalify,
        [{**row, "external_id": "999999"}],
        aliases,
    )
    expect_error(
        "HISTORICAL_KEY_WITHOUT_ALIAS",
        MODULE.requalify,
        [{**row, "target": {**row["target"], "key": "oteryn:item.registry.i99999999"}}],
        aliases,
    )
    tampered = {
        row["target"]["key"]: {
            **aliases[row["target"]["key"]],
            "state": "ALIAS",
            "target": "oteryn:item.tibia.i1",
        }
    }
    if tampered[row["target"]["key"]]["evidence"]["source_item_id"] != 1:
        expect_error("ALIAS_TARGET_NOT_OWN_ID", MODULE.requalify, [row], tampered)

    # NO_MATCH versus alias: an alias id mints nothing and shifts every later rank down.
    mutated = [dict(row) for row in crosswalk["rows"]]
    mutated[0].update(
        state="ACCEPTED_ALIAS",
        alias_target_source_item_id=epoch1_allocations[0][0],
        alias_target_key=epoch1_allocations[0][1],
    )
    shifted = MODULE.allocate_epoch2(mutated, namespace)
    assert len(shifted) == len(allocations) - 1
    assert shifted[0] == (allocations[1][0], allocations[0][1])
    assert shifted[-1] == (allocations[-1][0], allocations[-2][1])
    # The pinned evidence rejects any change of state, allocation or census.
    expect_error(
        "EPOCH2_MINTED_COUNT_MISMATCH",
        MODULE.epoch2_allocations,
        census,
        census_payload,
        {**crosswalk, "rows": mutated},
        pins,
        epoch1_allocations,
        namespace,
    )
    expect_error(
        "CENSUS_DIGEST_MISMATCH",
        MODULE.epoch2_allocations,
        census,
        census_payload + b" ",
        crosswalk,
        pins,
        epoch1_allocations,
        namespace,
    )
    # An alias may only target an existing epoch-1 key.
    bad_alias = [dict(row) for row in crosswalk["rows"]]
    bad_alias[0].update(
        state="ACCEPTED_ALIAS",
        alias_target_source_item_id=1,
        alias_target_key="oteryn:item.registry.i00038094",
    )
    expect_error(
        "ALIAS_TARGET_NOT_EPOCH1_KEY",
        MODULE.epoch2_allocations,
        census,
        census_payload,
        {**crosswalk, "rows": bad_alias},
        pins,
        epoch1_allocations,
        namespace,
    )
    # A census id that is already an epoch-1 source id fails closed.
    shared_id = epoch1_allocations[0][0]
    shared_census = {**census, "items": {**census["items"], str(shared_id): {}}}
    expect_error(
        "CENSUS_ID_COUNT_MISMATCH",
        MODULE.epoch2_allocations,
        shared_census,
        census_payload,
        crosswalk,
        pins,
        epoch1_allocations,
        namespace,
    )
    shared_pins = {**pins, "census_id_count": pins["census_id_count"] + 1}
    shared_crosswalk = {
        **crosswalk,
        "rows": [
            {"source_item_id": shared_id, "state": "AMBIGUOUS"},
            *crosswalk["rows"],
        ],
    }
    expect_error(
        "EPOCH2_SOURCE_ID_SHARED_WITH_EPOCH1",
        MODULE.epoch2_allocations,
        shared_census,
        census_payload,
        shared_crosswalk,
        shared_pins,
        epoch1_allocations,
        namespace,
    )
    # A key collision with epoch 1 (a wrong "highest earlier sequence") fails closed.
    original = MODULE.EPOCH1_HIGHEST_SEQUENCE
    try:
        MODULE.EPOCH1_HIGHEST_SEQUENCE = original - 1
        expect_error(
            "EPOCH2_KEY_COLLISION",
            MODULE.epoch2_allocations,
            census,
            census_payload,
            crosswalk,
            pins,
            epoch1_allocations,
            namespace,
        )
    finally:
        MODULE.EPOCH1_HIGHEST_SEQUENCE = original


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
        for i in range(1, MODULE.EXPECTED_BOUND_EPOCH1 + 1)
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
    drifted_cross_check_allocations[3287] = (3288, "oteryn:item.tibia.i3288")
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
        {"oteryn:item.tibia.i3288": {"5810"}},
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

    epoch2_alias_gate_and_allocation()
    epoch2_committed_output()

    print("g4_item_crystal_binding_generator synthetic self-test: PASS")


if __name__ == "__main__":
    main()
