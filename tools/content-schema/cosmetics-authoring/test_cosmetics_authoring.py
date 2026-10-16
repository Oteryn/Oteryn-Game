"""Synthetic authored facts against real pinned source bytes; no product source promotion."""
import copy
from pathlib import Path
import shutil
import tempfile
import unittest
from jsonschema import ValidationError

import cosmetics_authoring as c


class CosmeticsAuthoring(unittest.TestCase):
    def setUp(self):
        self.authoring = c.parse((c.HERE / "synthetic-authoring.json").read_bytes())

    def replace(self, path, value):
        parts = path.lstrip("/").split("/")
        parent = (c.pointer(self.authoring["document"], "/" + "/".join(parts[:-1]))
                  if len(parts) > 1 else self.authoring["document"])
        parent[int(parts[-1]) if isinstance(parent, list) else parts[-1]] = value
        for promotion in self.authoring["promotions"]:
            if promotion["path"] == path:
                promotion["value"] = copy.deepcopy(value)

    def test_real_source_materializes_exact_bytes_deterministically(self):
        first = c.build(self.authoring, c.ROOT / c.MANIFEST)
        self.assertEqual(first, c.build(self.authoring, c.ROOT / c.MANIFEST))
        packet = c.parse(first)
        self.assertEqual(packet["availability"], "authored")
        self.assertEqual(packet["source_manifest"].encode(), (c.ROOT / c.MANIFEST).read_bytes())
        for shard in packet["source_shards"]:
            self.assertEqual(shard["json"].encode(),
                             (c.ROOT / c.MANIFEST).parent.joinpath(shard["path"]).read_bytes())

    def test_missing_promotion_is_not_inferred(self):
        self.authoring["promotions"].pop()
        with self.assertRaisesRegex(ValueError, "incomplete"):
            c.validate(self.authoring)

    def test_duplicate_promotion_is_rejected(self):
        self.authoring["promotions"].append(self.authoring["promotions"][0])
        with self.assertRaisesRegex(ValueError, "duplicated"):
            c.validate(self.authoring)

    def test_changed_value_cannot_reuse_old_promotion(self):
        self.authoring["document"]["outfits"][0]["starter_female"] = False
        with self.assertRaisesRegex(ValueError, "mismatched"):
            c.validate(self.authoring)

    def test_unknown_and_observation_only_never_materialize(self):
        for field, value in [("state", "UNKNOWN"), ("source_kind", "OtsHypothesisOnly"),
                             ("source_kind", "SOURCE_OBSERVATIONS_ONLY")]:
            altered = copy.deepcopy(self.authoring)
            altered["promotions"][0][field] = value
            with self.assertRaises(ValidationError):
                c.validate(altered)

    def test_fallback_requires_an_explicit_starter_for_each_sex(self):
        self.replace("/outfits/0/starter_female", False)
        with self.assertRaisesRegex(ValueError, "explicit starter"):
            c.validate(self.authoring)

    def test_addon_mask_cannot_exceed_count(self):
        self.replace("/outfits/0/addon_count", 0)
        with self.assertRaisesRegex(ValueError, "mask"):
            c.validate(self.authoring)

    def test_wrong_source_binding_is_not_repaired_from_source(self):
        self.replace("/source_manifest_sha256", "a" * 64)
        with self.assertRaisesRegex(ValueError, "identity/digest"):
            c.build(self.authoring, c.ROOT / c.MANIFEST)

    def test_wrong_look_record_cannot_be_inferred_by_membership(self):
        look = copy.deepcopy(self.authoring["document"]["outfits"][0]["female"])
        look["source_record_sha256"] = "a" * 64
        self.replace("/outfits/0/female", look)
        with self.assertRaisesRegex(ValueError, "exact source record"):
            c.build(self.authoring, c.ROOT / c.MANIFEST)

    def test_duplicate_json_unknown_field_and_wrong_profile_fail(self):
        with self.assertRaises(ValueError):
            c.parse(b'{"nested":{"a":1,"a":2}}')
        for field, value in [("undecided", True), ("profile", {"kind": "other"})]:
            altered = copy.deepcopy(self.authoring)
            altered[field] = value
            with self.assertRaises(ValidationError):
                c.validate(altered)

    def test_canonical_bounds_ascii_and_zero_citation_are_rejected(self):
        for field, value in [("key", "oteryn:" + "a" * 154)]:
            altered = copy.deepcopy(self.authoring)
            altered["document"]["outfits"][0][field] = value
            with self.assertRaises(ValidationError):
                c.validate(altered)
        for field, value in [("reference", "non-ASCII-é"), ("sha256", "0" * 64)]:
            altered = copy.deepcopy(self.authoring)
            altered["document"]["outfits"][0]["source"][field] = value
            with self.assertRaises(ValidationError):
                c.validate(altered)
        for field in ["outfits", "mounts"]:
            altered = copy.deepcopy(self.authoring)
            altered["document"][field] *= 4097
            with self.assertRaises(ValidationError):
                c.validate(altered)

    def test_unknown_decision_and_nonproduction_identity_are_rejected(self):
        altered = copy.deepcopy(self.authoring)
        altered["promotions"][0]["decision"] = "UNKNOWN"
        with self.assertRaisesRegex(ValueError, "known printable"):
            c.validate(altered)
        altered = copy.deepcopy(self.authoring)
        promotion = next(p for p in altered["promotions"] if p["path"] == "/outfits/0/addon_count")
        promotion["value"] = 2.0
        with self.assertRaisesRegex(ValueError, "integer JSON"):
            c.validate(altered)
        self.authoring["document"]["outfits"][0]["key"] = "oteryn:outfit.synthetic"
        with self.assertRaisesRegex(ValueError, "nonproduction"):
            c.validate(self.authoring)

    def test_missing_authoring_produces_no_population_and_stale_output_refuses(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.assertEqual(c.materialized_outputs(root), {})
            output = root / c.OUTPUT
            output.parent.mkdir(parents=True)
            output.write_bytes(b"stale")
            with self.assertRaisesRegex(ValueError, "missing authoring"):
                c.materialized_outputs(root)

    def test_real_materializer_uses_authored_source_and_refuses_corrupted_shard(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / c.AUTHORING
            source.parent.mkdir(parents=True)
            source.write_bytes(c.canonical(self.authoring))
            staged = (root / c.MANIFEST).parent
            shutil.copytree((c.ROOT / c.MANIFEST).parent, staged)
            expected = c.build(self.authoring, root / c.MANIFEST)
            self.assertEqual(c.materialized_outputs(root), {c.OUTPUT: expected})
            shard = staged / c.parse((root / c.MANIFEST).read_bytes())["files"][0]["path"]
            shard.write_bytes(shard.read_bytes() + b" ")
            with self.assertRaisesRegex(ValueError, "shard digest"):
                c.materialized_outputs(root)


if __name__ == "__main__":
    unittest.main()
