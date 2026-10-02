"""Regression checks for complete compiler input and carrier-specific removals."""
import hashlib
import json
import pathlib
import unittest
from unittest.mock import patch

from build_executable_catalog import build, build_selection

SOURCE = pathlib.Path("/workspace/spells-r20-implemented")


@unittest.skipUnless((SOURCE / "readiness.json").is_file(), "pinned r20 bundle source unavailable")
class ExecutableCatalogTest(unittest.TestCase):
    def altered_readiness(self, change):
        original = pathlib.Path.read_text
        readiness = json.loads((SOURCE / "readiness.json").read_text())
        change(readiness)

        def read(path, *args, **kwargs):
            if path == SOURCE / "readiness.json":
                return json.dumps(readiness)
            return original(path, *args, **kwargs)

        with patch.object(pathlib.Path, "read_text", read):
            return build(SOURCE)

    def test_full_coverage_preserves_active_rune_items(self):
        catalog = build(SOURCE)
        self.assertEqual(len(catalog["bundles"]), 246)
        self.assertEqual(len(catalog["removed"]), 6)
        for name in ("light stone shower rune", "lightest missile rune"):
            active = [x for x in catalog["bundles"] if x["bundle"]["spell"]["name"].lower() == name]
            self.assertEqual([x["bundle"]["spell"]["carrier"] for x in active], ["rune"])
            self.assertEqual([x["carrier"] for x in catalog["removed"] if x["name"] == name], ["instant"])

    def test_every_manifest_hash_is_the_source_byte_hash(self):
        for entry in build(SOURCE)["bundles"]:
            identity = entry["source_identities"][0]
            source = SOURCE / "bundles" / identity["bundle_id"] / "manifest.json"
            self.assertEqual(identity["manifest_sha256"], hashlib.sha256(source.read_bytes()).hexdigest())
            self.assertEqual(identity["identity"], entry["bundle"]["spell"]["identity"])
            self.assertEqual(identity["sources"], entry["manifest"]["sources"])

    def test_removed_instant_cannot_be_relabelled_ready(self):
        def change(value):
            row = next(x for x in value["spells"] if x["name"] == "lightest missile rune" and x["spell_type"] == "instant")
            row.update(status="ready", errors=[], blockers=[])
        with self.assertRaisesRegex(ValueError, "inconsistent ready status"):
            self.altered_readiness(change)

    def test_exclusion_requires_concrete_policy_and_url(self):
        def change(value):
            row = next(x for x in value["spells"] if x["status"] == "blocked")
            row["blockers"] = ["S24: removed, no source"]
        with self.assertRaisesRegex(ValueError, "missing removal evidence"):
            self.altered_readiness(change)

    def test_unknown_status_cannot_masquerade_as_policy_exclusion(self):
        def change(value):
            row = next(x for x in value["spells"] if x["status"] == "blocked")
            row["status"] = "corrupt"
        with self.assertRaisesRegex(ValueError, "unexpected census carrier or status"):
            self.altered_readiness(change)

    def test_source_identity_name_carrier_and_revision_are_bound_to_census(self):
        original = pathlib.Path.read_text
        target = SOURCE / "bundles" / "instant-light_healing" / "spell.json"
        for field, value in (("name", "Energy Strike"), ("carrier", "rune"),
                             ("identity", {"key": "candidate:spell/energy_strike", "revision": "spell-p2-r20"}),
                             ("identity", {"key": "candidate:spell/light_healing", "revision": "unqualified"})):
            with self.subTest(field=field, value=value):
                def read(path, *args, **kwargs):
                    text = original(path, *args, **kwargs)
                    if path == target:
                        document = json.loads(text)
                        document["spell"][field] = value
                        return json.dumps(document)
                    return text
                with patch.object(pathlib.Path, "read_text", read):
                    with self.assertRaisesRegex(ValueError, "census/bundle identity mismatch"):
                        build(SOURCE)

    @unittest.skipUnless(pathlib.Path("/workspace/spell-sources/canary/.git").exists(), "pinned sources unavailable")
    def test_familiar_selection_keeps_conflicting_source_evidence(self):
        catalog = build(SOURCE)
        selection = build_selection(catalog, "a" * 64, pathlib.Path("/workspace/spell-sources"))
        self.assertEqual(len(selection["selections"]), 4)
        for row in selection["selections"]:
            self.assertEqual(row["policy"], "S21")
            self.assertTrue(row["selected"]["key"].startswith("candidate:spell/"))
            self.assertNotIn("summon_", row["selected"]["key"])
            self.assertEqual(len(row["source_proofs"]), 6)
            self.assertEqual(row["differences"]["login"]["selected"]["lifetime_basis"], "current_unix_time")
            self.assertEqual(row["differences"]["login"]["alternative"]["lifetime_basis"], "last_logout_unix_time")

    def test_familiar_selection_rejects_unqualified_source_head(self):
        class WrongHead:
            stdout = "0" * 40
        with patch("build_executable_catalog.subprocess.run", return_value=WrongHead()):
            with self.assertRaisesRegex(ValueError, "unexpected canary source HEAD"):
                build_selection(build(SOURCE), "a" * 64, pathlib.Path("/workspace/spell-sources"))


if __name__ == "__main__":
    unittest.main()
