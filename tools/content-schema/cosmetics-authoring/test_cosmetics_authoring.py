"""Synthetic authored facts against real pinned source bytes; no product source promotion."""
import copy
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest import mock
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

    def test_promoted_booleans_and_integers_are_not_interchangeable(self):
        original = copy.deepcopy(self.authoring)
        for path, document_value, promoted_value in [
            ("/outfits/0/starter_female", True, 1),
            ("/outfits/0/premium", False, 0),
            ("/outfits/0/starter_addons_female", 1, True),
            ("/outfits/0/starter_addons_female", 0, False),
        ]:
            with self.subTest(path=path, document=document_value, promoted=promoted_value):
                self.authoring = copy.deepcopy(original)
                self.replace(path, document_value)
                c.validate(self.authoring)
                promotion = next(p for p in self.authoring["promotions"] if p["path"] == path)
                promotion["value"] = promoted_value
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

    def stage_truncated(self, root, count, mutate):
        """Stage the real source cut to `count` records, re-pinned, then apply `mutate`."""
        authoring = copy.deepcopy(self.authoring)
        staged = (root / c.MANIFEST).parent
        shutil.copytree((c.ROOT / c.MANIFEST).parent, staged)
        manifest = c.parse((root / c.MANIFEST).read_bytes())
        pin = manifest["files"][0]
        manifest["files"] = [pin]
        shard_path = staged / pin["path"]
        shard = c.parse(shard_path.read_bytes())
        shard["records"] = shard["records"][:count]
        mutate(manifest, pin, shard)
        first = shard["records"][0]
        document = authoring["document"]
        refs = [row[sex] for row in document["outfits"] for sex in ["female", "male"]]
        for ref in refs + [row["look"] for row in document["mounts"]]:
            ref["id"], ref["source_record_sha256"] = first["source_id"], first["source_record_sha256"]
        data = json.dumps(shard).encode()
        shard_path.write_bytes(data)
        pin["sha256"] = c.sha(data)
        raw = json.dumps(manifest).encode()
        (root / c.MANIFEST).write_bytes(raw)
        authoring["document"]["source_manifest_sha256"] = c.sha(raw)
        for promotion in authoring["promotions"]:
            if promotion["path"] == "/source_manifest_sha256":
                promotion["value"] = c.sha(raw)
        return authoring

    def build_unvalidated(self, authoring, root):
        # The truncated fixture repeats one look for every reference; only the source loop is under test.
        with mock.patch.object(c, "validate"):
            return c.build(authoring, root / c.MANIFEST)

    def test_source_ordinals_and_counts_reject_json_booleans(self):
        def sound(manifest, pin, shard):
            pin["records"] = len(shard["records"])
            manifest["record_count"] = len(shard["records"])

        def index_bool(index):
            def mutate(manifest, pin, shard):
                sound(manifest, pin, shard)
                shard["records"][index]["source_index"] = bool(index)
            return mutate

        def count_bool(field):
            def mutate(manifest, pin, shard):
                sound(manifest, pin, shard)
                (pin if field == "pin" else manifest)["records" if field == "pin" else "record_count"] = True
            return mutate

        cases = [("source_index false for 0", 1, index_bool(0)),
                 ("source_index true for 1", 2, index_bool(1)),
                 ("pin records true for 1", 1, count_bool("pin")),
                 ("manifest record_count true for 1", 1, count_bool("manifest"))]
        for label, count, mutate in cases:
            with self.subTest(label), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                authoring = self.stage_truncated(root, count, sound)
                self.assertEqual(c.parse(self.build_unvalidated(authoring, root))["availability"], "authored")
            with self.subTest(label), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                authoring = self.stage_truncated(root, count, mutate)
                with self.assertRaises(ValueError):
                    self.build_unvalidated(authoring, root)

    def test_every_staged_record_digest_is_validated_even_if_unreferenced(self):
        def sound(manifest, pin, shard):
            pin["records"] = len(shard["records"])
            manifest["record_count"] = len(shard["records"])

        def digest(value):
            def mutate(manifest, pin, shard):
                sound(manifest, pin, shard)
                shard["records"][1]["source_record_sha256"] = value
            return mutate

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            authoring = self.stage_truncated(root, 2, sound)
            self.assertEqual(c.parse(self.build_unvalidated(authoring, root))["availability"], "authored")
        good = "a" * 64
        for label, value in [("short", good[:63]), ("long", good + "a"), ("uppercase", "A" * 64),
                             ("all zero", "0" * 64), ("non hex", "g" * 64), ("not a string", 7)]:
            with self.subTest(label), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                authoring = self.stage_truncated(root, 2, digest(value))
                with self.assertRaisesRegex(ValueError, "digest"):
                    self.build_unvalidated(authoring, root)

    def test_staged_observations_must_have_the_exact_rust_decoder_shape(self):
        def sound(manifest, pin, shard):
            pin["records"] = len(shard["records"])
            manifest["record_count"] = len(shard["records"])

        def mutated(change):
            def mutate(manifest, pin, shard):
                sound(manifest, pin, shard)
                change(shard)
            return mutate

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            authoring = self.stage_truncated(root, 2, sound)
            self.assertEqual(c.parse(self.build_unvalidated(authoring, root))["availability"], "authored")
        cases = {
            "shard unknown member": lambda s: s.update(extra=1),
            "shard missing schema": lambda s: s.pop("schema"),
            "record unknown member": lambda s: s["records"][1].update(extra=1),
            "record missing flags": lambda s: s["records"][1].pop("flags"),
            "record missing frame_groups": lambda s: s["records"][0].pop("frame_groups"),
            "record not an object": lambda s: s["records"].__setitem__(1, 7),
        }
        for label, change in cases.items():
            with self.subTest(label), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                authoring = self.stage_truncated(root, 2, mutated(change))
                with self.assertRaisesRegex(ValueError, "observation shape"):
                    self.build_unvalidated(authoring, root)

    def test_definition_source_citations_require_their_own_promotion(self):
        for path in ["/outfits/0/source", "/mounts/0/source"]:
            with self.subTest(path=path):
                altered = copy.deepcopy(self.authoring)
                altered["promotions"] = [p for p in altered["promotions"] if p["path"] != path]
                with self.assertRaisesRegex(ValueError, "incomplete"):
                    c.validate(altered)

    def test_staged_manifest_must_have_the_full_rust_decoder_shape(self):
        def staged_build(mutate):
            with tempfile.TemporaryDirectory() as temporary:
                staged = Path(temporary)
                shutil.copytree((c.ROOT / c.MANIFEST).parent, staged, dirs_exist_ok=True)
                manifest = c.parse((staged / "manifest.json").read_bytes())
                mutate(manifest)
                raw = json.dumps(manifest, sort_keys=True).encode()
                (staged / "manifest.json").write_bytes(raw)
                authoring = copy.deepcopy(self.authoring)
                authoring["document"]["source_manifest_sha256"] = c.sha(raw)
                for promotion in authoring["promotions"]:
                    if promotion["path"] == "/source_manifest_sha256":
                        promotion["value"] = c.sha(raw)
                return c.build(authoring, staged / "manifest.json")
        staged_build(lambda manifest: None)
        for label, mutate in [
            ("empty file", lambda m: m["inputs"]["appearances"].update(file="")),
            ("control file", lambda m: m["inputs"]["appearances"].update(file="a\nb")),
            ("overlong file", lambda m: m["inputs"]["appearances"].update(file="a" * 1025)),
            ("missing file", lambda m: m["inputs"]["appearances"].pop("file")),
            ("missing excluded_top_level", lambda m: m.pop("excluded_top_level")),
            ("unknown member", lambda m: m.update(extra=1)),
            ("unknown shard member", lambda m: m["files"][0].update(extra=1)),
        ]:
            with self.subTest(label), self.assertRaisesRegex(ValueError, "manifest shape"):
                staged_build(mutate)

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
