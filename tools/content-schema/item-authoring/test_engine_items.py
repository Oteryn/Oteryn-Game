"""Focused checks for engine_items/population_census: digest portability (F-02) and
Delivery Task evidence/decision separation (F-01). Builds tiny fixture checkouts in a
temp dir; no network and no dependency on any real pinned upstream checkout."""

from __future__ import annotations

import tempfile
from pathlib import Path

import engine_items
import population_census

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
    """Write one engine's fixture checkout under `root`; return its digests map."""
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
    return digests


def load_fixture_sources(root, engine, newline="\n", appearances_bytes=None):
    digests = write_fixture_checkout(
        root, engine, newline=newline, appearances_bytes=appearances_bytes
    )
    return engine_items.load_engine_sources(engine, root, digests=digests)


CHECKS = 0


def check(condition, message):
    global CHECKS
    CHECKS += 1
    if not condition:
        raise AssertionError(message)


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


def test_bundles_never_carry_delivery_task_eligible():
    with tempfile.TemporaryDirectory() as directory:
        sources = load_fixture_sources(Path(directory), "canary")
        _result, bundles = population_census.build_census(sources, "canary")
        check(bundles, "fixture census produced no bundles")
        for key, bundle in bundles.items():
            check(
                "delivery_task_eligible" not in bundle["item"],
                f"bundle {key} must never carry delivery_task_eligible",
            )
        for item_id in FIXTURE_ITEM_IDS:
            item, _dependencies, report = engine_items.convert_item(sources, item_id)
            check(item is not None, f"fixture item {item_id} failed to convert")
            check("delivery_task_eligible" not in item, item_id)
            dt = report["delivery_task"]
            check(dt["source"] == "canary_task_board_weekly_items", dt)
            check(dt["decision"] == "unresolved", dt)
            check(dt["proposal"] == dt["member"], dt)
            check("delivery_task_decision_not_admitted" in report["blockers"], report)


def test_census_counts_pending_author_decision():
    with tempfile.TemporaryDirectory() as directory:
        sources = load_fixture_sources(Path(directory), "canary")
        result, _bundles = population_census.build_census(sources, "canary")
        check(
            result["outcome"].get("pending_author_decision", 0) > 0,
            f"expected pending_author_decision > 0, got {result['outcome']}",
        )
        check(
            result["outcome"].get("structure_invalid", 0) == 0,
            f"fixture items should validate cleanly: {result['outcome']}",
        )
        check(
            result["delivery_task"]["source"] == "canary_task_board_weekly_items",
            result["delivery_task"],
        )
        check(result["delivery_task"]["member"] == 1, result["delivery_task"])
        check(result["delivery_task"]["not_member"] == 2, result["delivery_task"])


def main():
    tests = [
        test_lf_and_crlf_text_fixtures_byte_identical,
        test_binary_crlf_bytes_are_not_normalized,
        test_canary_pool_membership_reads_weekly_items_table_only,
        test_item_only_in_crystal_list_is_not_canary_member,
        test_missing_pool_file_is_a_hard_error,
        test_bundles_never_carry_delivery_task_eligible,
        test_census_counts_pending_author_decision,
    ]
    for test in tests:
        test()
    print(f"PASS {CHECKS} checks")


if __name__ == "__main__":
    main()
