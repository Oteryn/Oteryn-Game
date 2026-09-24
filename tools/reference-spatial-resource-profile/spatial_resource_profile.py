"""Non-shipping, synthetic one-destination spatial carrier sizing candidate.

No Reference cell admission is performed here. All bytes are an isolated versioned
sidecar candidate; they do not change Content/Item/Movement production codecs.
"""
import argparse
import hashlib
import json
import struct

MAGIC = b"OTSPC001"
HEADER = struct.Struct("<8sHBBIII32s8x")  # 64 bytes
ENTRY = struct.Struct("<iiiII32s")  # 52 bytes
HEADER_BYTES = HEADER.size
MAX_CELLS = 1  # deliberately scoped one destination per generation
MAX_ATOM_BYTES = 512
MAX_MANIFEST_BYTES = 7500
MAX_INDEX_BYTES = 4 + MAX_CELLS * ENTRY.size
MAX_SERVER_BODY_BYTES = MAX_CELLS * 33
MAX_CLIENT_BODY_BYTES = MAX_CELLS
MAX_SERVER_ARTIFACT_BYTES = HEADER_BYTES + MAX_MANIFEST_BYTES + MAX_INDEX_BYTES + MAX_SERVER_BODY_BYTES
MAX_CLIENT_ARTIFACT_BYTES = HEADER_BYTES + MAX_MANIFEST_BYTES + MAX_INDEX_BYTES + MAX_CLIENT_BODY_BYTES
MAX_PAIR_BYTES = MAX_SERVER_ARTIFACT_BYTES + MAX_CLIENT_ARTIFACT_BYTES
U32 = 2**32 - 1
U64 = 2**64 - 1
KEYS = ("world_id", "coordinate_frame_ref", "map_revision_ref", "content_generation")
CELL_KEYS = ("x", "y", "z", "collision", "evidence_binding_sha256")


class Rejected(ValueError):
    pass


def require(ok, message):
    if not ok:
        raise Rejected(message)


def checked_add(a, b, limit=U64):
    require(type(a) is int and type(b) is int and 0 <= a <= limit and 0 <= b <= limit - a, "addition overflow")
    return a + b


def checked_mul(a, b, limit=U64):
    require(type(a) is int and type(b) is int and 0 <= a <= limit and 0 <= b <= limit, "multiplication overflow")
    require(b == 0 or a <= limit // b, "multiplication overflow")
    return a * b


def hex_digest(value):
    require(type(value) is str and len(value) == 64, "digest shape")
    try:
        raw = bytes.fromhex(value)
    except ValueError as exc:
        raise Rejected("digest hex") from exc
    require(raw.hex() == value and raw != bytes(32), "digest canonical/nonzero")
    return raw


def manifest_bytes(binding):
    require(type(binding) is dict and set(binding) == set(KEYS), "binding fields")
    for key in KEYS[:3]:
        value = binding[key]
        require(type(value) is str and 1 <= len(value.encode("utf-8")) <= MAX_ATOM_BYTES, "binding atom")
        require(all(ord(c) >= 32 for c in value), "binding control")
    hex_digest(binding["content_generation"])
    raw = json.dumps(binding, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    require(len(raw) <= MAX_MANIFEST_BYTES, "manifest limit")
    return raw


def cell_fields(cell):
    require(type(cell) is dict and set(cell) == set(CELL_KEYS), "cell fields / unsupported placement or footprint")
    coords = tuple(cell[k] for k in ("x", "y", "z"))
    require(all(type(n) is int and -(2**31) <= n < 2**31 for n in coords), "signed coordinate")
    require(type(cell["collision"]) is bool, "unresolved collision")
    evidence = hex_digest(cell["evidence_binding_sha256"])
    return coords, evidence


def bounds(projection):
    require(projection in (1, 2), "projection")
    body = MAX_SERVER_BODY_BYTES if projection == 1 else MAX_CLIENT_BODY_BYTES
    artifact = MAX_SERVER_ARTIFACT_BYTES if projection == 1 else MAX_CLIENT_ARTIFACT_BYTES
    return body, artifact


def encode(binding, cells, projection):
    body_cap, artifact_cap = bounds(projection)
    require(type(cells) is list and len(cells) == MAX_CELLS, "single destination generation")
    manifest = manifest_bytes(binding)
    index = bytearray(struct.pack("<I", len(cells)))
    body = bytearray()
    seen = set()
    for cell in cells:
        coords, evidence = cell_fields(cell)
        require(coords not in seen, "duplicate address")
        seen.add(coords)
        record = (bytes([int(cell["collision"])]) + evidence) if projection == 1 else b"\x00"
        offset = len(body)
        index.extend(ENTRY.pack(*coords, offset, len(record), hashlib.sha256(record).digest()))
        body.extend(record)
    require(len(index) <= MAX_INDEX_BYTES and len(body) <= body_cap, "section limit")
    length = checked_add(HEADER_BYTES, checked_add(len(manifest), checked_add(len(index), len(body))))
    require(length <= artifact_cap, "artifact limit")
    header = HEADER.pack(MAGIC, 1, projection, 0, len(manifest), len(index), len(body),
                         hashlib.sha256(manifest + index + body).digest())
    return header + manifest + index + body


def unique_pairs(pairs):
    result = {}
    for k, v in pairs:
        require(k not in result, "duplicate manifest key")
        result[k] = v
    return result


def decode(data, expected_binding, expected_coords, projection, expected_evidence=None):
    body_cap, artifact_cap = bounds(projection)
    require(type(data) is bytes and len(data) <= artifact_cap and len(data) >= HEADER_BYTES, "artifact length")
    magic, version, view, reserved, ml, il, bl, digest = HEADER.unpack_from(data)
    require((magic, version, view, reserved) == (MAGIC, 1, projection, 0) and data[56:64] == bytes(8), "version/projection/padding")
    require(ml <= MAX_MANIFEST_BYTES and il <= MAX_INDEX_BYTES and bl <= body_cap, "section length")
    require(checked_add(HEADER_BYTES, checked_add(ml, checked_add(il, bl))) == len(data), "length mismatch")
    manifest = data[HEADER_BYTES:HEADER_BYTES + ml]
    index = data[HEADER_BYTES + ml:HEADER_BYTES + ml + il]
    body = data[HEADER_BYTES + ml + il:]
    require(hashlib.sha256(manifest + index + body).digest() == digest, "artifact digest")
    try:
        parsed = json.loads(manifest.decode("utf-8"), object_pairs_hook=unique_pairs)
    except (UnicodeError, json.JSONDecodeError) as exc:
        raise Rejected("manifest parse") from exc
    require(manifest_bytes(parsed) == manifest and parsed == expected_binding, "manifest/caller binding mismatch")
    require(len(index) >= 4, "index count")
    count = struct.unpack_from("<I", index)[0]
    require(count == MAX_CELLS and il == checked_add(4, checked_mul(count, ENTRY.size)), "index dimensions")
    require(type(expected_coords) is tuple and len(expected_coords) == 3, "expected coordinates")
    seen = set()
    previous_end = 0
    result = []
    for i in range(count):
        x, y, z, offset, length, record_hash = ENTRY.unpack_from(index, 4 + i * ENTRY.size)
        coords = (x, y, z)
        require(coords not in seen and coords == expected_coords, "duplicate/address mismatch")
        seen.add(coords)
        require(offset == previous_end and length == (33 if projection == 1 else 1), "record offset/length")
        previous_end = checked_add(offset, length)
        require(previous_end <= len(body), "record bounds")
        record = body[offset:previous_end]
        require(hashlib.sha256(record).digest() == record_hash, "record digest")
        if projection == 1:
            require(record[0] in (0, 1) and record[1:] != bytes(32), "unresolved record")
            require(expected_evidence is not None and record[1:] == hex_digest(expected_evidence), "evidence binding mismatch")
            result.append(bool(record[0]))
        else:
            require(record == b"\x00" and expected_evidence is None, "client authority leakage")
            result.append(None)
    require(previous_end == len(body), "trailing body")
    return result


def encode_pair(binding, cells):
    server = encode(binding, cells, 1)
    client = encode(binding, cells, 2)
    require(checked_add(len(server), len(client)) <= MAX_PAIR_BYTES, "pair limit")
    return server, client


def measure():
    binding = dict(world_id="synthetic:world", coordinate_frame_ref="synthetic:frame",
                   map_revision_ref="synthetic:map-revision", content_generation="11" * 32)
    cell = dict(x=2**31 - 1, y=-(2**31), z=0, collision=True, evidence_binding_sha256="22" * 32)
    server, client = encode_pair(binding, [cell])
    assert decode(server, binding, (cell["x"], cell["y"], cell["z"]), 1, cell["evidence_binding_sha256"]) == [True]
    assert decode(client, binding, (cell["x"], cell["y"], cell["z"]), 2) == [None]
    worst = dict(world_id="w" * 512, coordinate_frame_ref="f" * 512,
                 map_revision_ref="m" * 512, content_generation="11" * 32)
    worst_server = encode(worst, [cell], 1)
    worst_client = encode(worst, [cell], 2)
    return dict(profile="REFERENCE_SPATIAL_SINGLE_DESTINATION_CANDIDATE/v1",
                authority="NON_SHIPPING_SYNTHETIC_ONLY", dimensions=dict(cells_per_generation=MAX_CELLS,
                coordinate_min=-(2**31), coordinate_max=2**31 - 1, binding_atom_utf8_bytes=MAX_ATOM_BYTES,
                manifest_bytes=MAX_MANIFEST_BYTES, index_bytes=MAX_INDEX_BYTES,
                server_body_bytes=MAX_SERVER_BODY_BYTES, client_body_bytes=MAX_CLIENT_BODY_BYTES,
                server_artifact_bytes=MAX_SERVER_ARTIFACT_BYTES, client_artifact_bytes=MAX_CLIENT_ARTIFACT_BYTES,
                pair_bytes=MAX_PAIR_BYTES), observed=dict(server_manifest_bytes=HEADER.unpack_from(server)[4],
                server_artifact_bytes=len(server), client_artifact_bytes=len(client), pair_bytes=len(server) + len(client),
                max_atoms_manifest_bytes=HEADER.unpack_from(worst_server)[4],
                max_atoms_server_artifact_bytes=len(worst_server),
                max_atoms_client_artifact_bytes=len(worst_client)),
                codec=dict(header_bytes=HEADER_BYTES, index_entry_bytes=ENTRY.size,
                           server_record_bytes=33, client_record_bytes=1),
                synthetic_fixture_sha256=dict(server=hashlib.sha256(server).hexdigest(),
                                               client=hashlib.sha256(client).hexdigest()))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", help="write canonical JSON evidence")
    args = parser.parse_args()
    output = json.dumps(measure(), indent=2, sort_keys=True) + "\n"
    if args.output:
        with open(args.output, "w", encoding="utf-8") as target:
            target.write(output)
    else:
        print(output, end="")
