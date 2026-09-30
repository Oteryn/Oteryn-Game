"""No-network fixture tests for `appearance_membership.py` (ITEM-ID-1a): the per-id
identity-projection and record digests, the file pin, the manifest self-check, the
admitted-set index and the committed manifests. Run with
`python test_appearance_membership.py`.
"""

from __future__ import annotations

import hashlib
import json

import appearance_membership as membership

CHECKS = 0


def check(condition, message):
    global CHECKS
    CHECKS += 1
    if not condition:
        raise AssertionError(message)


def varint(value):
    out = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if value:
            out.append(byte | 0x80)
        else:
            out.append(byte)
            return bytes(out)


def length_delimited(number, payload):
    return varint(number << 3 | 2) + varint(len(payload)) + payload


def appearance_object(object_id, name=None, extra=b""):
    body = varint(1 << 3) + varint(object_id)
    if name is not None:
        body += length_delimited(4, name.encode("utf-8"))
    return body + extra


def appearances_file(*objects):
    return b"".join(length_delimited(1, body) for body in objects)


def spec_for(data, label="fixture"):
    return {
        "label": label,
        "sha256": hashlib.sha256(data).hexdigest(),
        "bytes": len(data),
        "pinned_by": "fixture",
    }


def expect_exit(function, *args):
    try:
        function(*args)
    except SystemExit:
        return
    raise AssertionError(f"{function.__name__} accepted invalid input")


def test_entries_digest_projection_and_record():
    named = appearance_object(3031, "gold coin")
    unnamed = appearance_object(100)
    entries = membership.manifest_entries(appearances_file(named, unnamed))
    check([entry[0] for entry in entries] == [100, 3031], entries)
    projection = hashlib.sha256(b'{"class":"object","name":"gold coin"}\n').hexdigest()
    check(entries[1][1] == projection, entries[1])
    check(entries[1][2] == hashlib.sha256(named).hexdigest(), entries[1])
    null_projection = hashlib.sha256(b'{"class":"object","name":null}\n').hexdigest()
    check(entries[0][1] == null_projection, entries[0])


def test_record_evolution_keeps_the_projection():
    before = membership.manifest_entries(appearances_file(appearance_object(7, "x")))
    after = membership.manifest_entries(
        appearances_file(
            appearance_object(7, "x", extra=length_delimited(5, b"new description"))
        )
    )
    check(
        before[0][1] == after[0][1],
        "a description change moved the identity projection",
    )
    check(before[0][2] != after[0][2], "record digest ignored a changed field")


def test_non_object_families_are_not_members():
    data = appearances_file(appearance_object(5)) + length_delimited(
        2, appearance_object(6)
    )
    check([entry[0] for entry in membership.manifest_entries(data)] == [5], data)


def test_duplicate_id_and_missing_id_fail_closed():
    expect_exit(
        membership.manifest_entries,
        appearances_file(appearance_object(5), appearance_object(5)),
    )
    expect_exit(
        membership.manifest_entries, length_delimited(1, length_delimited(4, b"x"))
    )


def test_manifest_is_pinned_and_self_checked():
    data = appearances_file(appearance_object(1, "a"), appearance_object(2))
    spec = spec_for(data)
    payload = membership.manifest_bytes(spec, data)
    document = membership.verify_manifest(spec, payload)
    check(
        document["object_count"] == 2
        and document["appearances_sha256"] == spec["sha256"],
        document,
    )
    expect_exit(membership.manifest_bytes, spec, data + b"\x00")
    tampered = json.loads(payload)
    tampered["entries"][0][1] = "0" * 64
    expect_exit(membership.verify_manifest, spec, membership.canonical_bytes(tampered))


def test_index_derives_union_current_and_retired():
    old = appearances_file(appearance_object(1), appearance_object(9))
    new = appearances_file(appearance_object(1), appearance_object(2))
    specs = (spec_for(old, "old"), spec_for(new, "new"))
    original = membership.ADMITTED
    membership.ADMITTED = specs
    try:
        index = json.loads(
            membership.index_bytes(
                {
                    "old": membership.manifest_bytes(specs[0], old),
                    "new": membership.manifest_bytes(specs[1], new),
                }
            )
        )
    finally:
        membership.ADMITTED = original
    check(index["newest"] == "new" and index["union_count"] == 3, index)
    check(index["current_count"] == 2 and index["retired_ids"] == [9], index)


def test_committed_manifests_verify():
    index, manifests = membership.load_admitted()
    check(index["newest"] == "client-15.30", index["newest"])
    check(index["union_count"] == 43_517 and index["retired_ids"] == [53161], index)
    check(len(manifests) == 4, sorted(manifests))
    check(
        manifests["client-15.30"]["object_count"] == 43_516,
        manifests["client-15.30"]["object_count"],
    )
    legacy_ids = json.loads(
        (
            membership.REPO_ROOT
            / "imports/official/client-assets/15.30/appearance-ids.json"
        ).read_text(encoding="utf-8")
    )["ids"]
    check(
        [entry[0] for entry in manifests["client-15.30"]["entries"]] == legacy_ids,
        "15.30 manifest disagrees with the id-only membership file",
    )


def test_in_repo_client_manifest_regenerates():
    spec = membership.ADMITTED[-1]
    data = membership.IN_REPO_SOURCES[spec["label"]].read_bytes()
    committed = (membership.OUT_DIR / membership.manifest_name(spec)).read_bytes()
    check(membership.manifest_bytes(spec, data) == committed, "15.30 manifest drift")


def main():
    tests = [
        value for name, value in sorted(globals().items()) if name.startswith("test_")
    ]
    for test in tests:
        test()
    print(f"PASS test_appearance_membership tests={len(tests)} checks={CHECKS}")


if __name__ == "__main__":
    main()
