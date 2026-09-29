"""Adversarial boundary checks for synthetic spatial measurement candidate."""
import hashlib
import json
import struct
import unittest
import spatial_resource_profile as p


B = dict(world_id="synthetic:world", coordinate_frame_ref="synthetic:frame",
         map_revision_ref="synthetic:revision", content_generation="11" * 32)
C = dict(x=0, y=0, z=0, collision=False, evidence_binding_sha256="22" * 32)


def reject(action):
    try:
        action()
    except p.Rejected:
        return
    raise AssertionError("expected fail closed")


def reseal(data):
    out = bytearray(data)
    out[24:56] = hashlib.sha256(out[p.HEADER_BYTES:]).digest()
    return bytes(out)


class CandidateTests(unittest.TestCase):
    def test_roundtrip_and_exact_bindings(self):
        server, client = p.encode(B, [C], 1), p.encode(B, [C], 2)
        self.assertEqual(p.decode(server, B, (0, 0, 0), 1, C["evidence_binding_sha256"]), [False])
        self.assertEqual(p.decode(client, B, (0, 0, 0), 2), [None])
        self.assertEqual(server, p.encode(B, [C], 1))
        self.assertNotIn(bytes.fromhex(C["evidence_binding_sha256"]), client)
        for k in p.KEYS:
            changed = dict(B)
            changed[k] = ("33" * 32) if k == "content_generation" else "different"
            reject(lambda changed=changed: p.decode(server, changed, (0, 0, 0), 1, C["evidence_binding_sha256"]))
        reject(lambda: p.decode(server, B, (1, 0, 0), 1, C["evidence_binding_sha256"]))
        reject(lambda: p.decode(server, B, (0, 0, 0), 1, "33" * 32))
        reject(lambda: p.decode(server, B, (0, 0, 0), 2))
        reject(lambda: p.decode(client, B, (0, 0, 0), 1, C["evidence_binding_sha256"]))

    def test_dimensions_and_unsupported_semantics(self):
        p.encode(B, [dict(C, x=2**31 - 1, y=-(2**31), z=2**31 - 1)], 1)
        for changed in (dict(C, x=2**31), dict(C, y=-(2**31)-1),
                        dict(C, collision=None), dict(C, evidence_binding_sha256="00" * 32),
                        dict(C, placements=[]), dict(C, presentation_footprint=[]),
                        dict(C, collision_footprint=[])):
            reject(lambda changed=changed: p.encode(B, [changed], 1))
        reject(lambda: p.encode(B, [], 1))
        reject(lambda: p.encode(B, [C, C], 1))
        reject(lambda: p.encode(B, [C, dict(C, x=1)], 1))
        reject(lambda: p.encode(dict(B, world_id="w" * 513), [C], 1))
        p.encode(dict(B, world_id="w" * 512), [C], 1)
        reject(lambda: p.encode(dict(B, world_id="é" * 257), [C], 1))
        p.encode(dict(B, world_id="é" * 256), [C], 1)
        reject(lambda: p.encode(dict(B, map_revision_ref="x\n"), [C], 1))

    def test_sections_record_and_duplicate_decode(self):
        server = p.encode(B, [C], 1)
        ml = p.HEADER.unpack_from(server)[4]
        index_at = p.HEADER_BYTES + ml
        body_at = index_at + p.MAX_INDEX_BYTES
        for mutation in (
            lambda d: d.__setitem__(slice(16, 20), struct.pack("<I", p.MAX_INDEX_BYTES + 1)),
            lambda d: d.__setitem__(slice(20, 24), struct.pack("<I", p.MAX_SERVER_BODY_BYTES + 1)),
            lambda d: d.__setitem__(slice(index_at, index_at+4), struct.pack("<I", 2)),
            lambda d: d.__setitem__(slice(index_at+16, index_at+20), struct.pack("<I", 1)),
            lambda d: d.__setitem__(body_at, 2),
        ):
            damaged = bytearray(server)
            mutation(damaged)
            reject(lambda damaged=damaged: p.decode(reseal(bytes(damaged)), B, (0, 0, 0), 1, C["evidence_binding_sha256"]))
        reject(lambda: p.decode(server + b"x", B, (0, 0, 0), 1, C["evidence_binding_sha256"]))
        reject(lambda: p.decode(server[:-1], B, (0, 0, 0), 1, C["evidence_binding_sha256"]))
        reject(lambda: p.decode(bytes(p.MAX_SERVER_ARTIFACT_BYTES + 1), B, (0, 0, 0), 1, C["evidence_binding_sha256"]))
        client = bytearray(p.encode(B, [C], 2))
        client[-1] = 1
        reject(lambda: p.decode(reseal(bytes(client)), B, (0, 0, 0), 2))

    def test_limits_and_overflow(self):
        m = p.measure()
        d = m["dimensions"]
        self.assertEqual(d["server_artifact_bytes"], 64 + 7500 + 56 + 33)
        self.assertEqual(d["client_artifact_bytes"], 64 + 7500 + 56 + 1)
        self.assertEqual(d["pair_bytes"], d["server_artifact_bytes"] + d["client_artifact_bytes"])
        self.assertEqual(p.HEADER.size, 64)
        self.assertEqual(p.ENTRY.size, 52)
        # Three length headers are checked before slicing, even though this
        # manifest grammar cannot physically populate the entire 7,500 budget.
        artifact = p.encode(B, [C], 1)
        for field, over in ((12, p.MAX_MANIFEST_BYTES + 1),
                            (16, p.MAX_INDEX_BYTES + 1),
                            (20, p.MAX_SERVER_BODY_BYTES + 1)):
            bad = bytearray(artifact)
            bad[field:field+4] = struct.pack("<I", over)
            reject(lambda bad=bad: p.decode(bytes(bad), B, (0, 0, 0), 1, C["evidence_binding_sha256"]))
        client = p.encode(B, [C], 2)
        bad_client = bytearray(client)
        bad_client[20:24] = struct.pack("<I", p.MAX_CLIENT_BODY_BYTES + 1)
        reject(lambda: p.decode(bytes(bad_client), B, (0, 0, 0), 2))
        reject(lambda: p.decode(bytes(p.MAX_CLIENT_ARTIFACT_BYTES + 1), B, (0, 0, 0), 2))
        self.assertEqual(p.enforce_pair_length(p.MAX_SERVER_ARTIFACT_BYTES,
                                               p.MAX_CLIENT_ARTIFACT_BYTES), p.MAX_PAIR_BYTES)
        reject(lambda: p.enforce_pair_length(p.MAX_SERVER_ARTIFACT_BYTES + 1,
                                             p.MAX_CLIENT_ARTIFACT_BYTES))
        reject(lambda: p.enforce_pair_length(p.MAX_SERVER_ARTIFACT_BYTES,
                                             p.MAX_CLIENT_ARTIFACT_BYTES + 1))
        self.assertEqual(p.checked_add(p.U64, 0), p.U64)
        reject(lambda: p.checked_add(p.U64, 1))
        self.assertEqual(p.checked_mul(p.U64, 1), p.U64)
        reject(lambda: p.checked_mul(p.U64, 2))
        self.assertEqual(p.checked_mul(0, p.U64), 0)
        reject(lambda: p.checked_add(-1, 1))
        reject(lambda: p.checked_mul(1, -1))


if __name__ == "__main__":
    unittest.main(verbosity=2)
