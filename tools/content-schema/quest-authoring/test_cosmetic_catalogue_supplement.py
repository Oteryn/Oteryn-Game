"""Catalogue integration preserves admitted identities and old grant receipts."""

import copy
import json
from pathlib import Path
import sys
import unittest

import cosmetic_catalogue_supplement as supplement

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "cosmetics-authoring"))
import cosmetics_authoring as tooling


class CatalogueSupplementTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.author = json.loads((Path(__file__).resolve().parents[3]
            / "content/cosmetics/outfits/authoring.json").read_bytes())
        # Main now contains both canonical and quest identity layers. Exercise the
        # canonical pre-supplement input this helper extends, retaining all mounts.
        document = cls.author["document"]
        original_outfits = {row["key"]: row for row in document["outfits"]}
        document["outfits"] = [row for row in document["outfits"]
                               if row["key"].startswith("oteryn:content.outfit.")]
        keys = {row["key"] for row in document["outfits"]}
        for fallback in document["fallbacks"].values():
            original = original_outfits[fallback["outfit_key"]]
            fallback["outfit_key"] = next(row["key"] for row in document["outfits"]
                if all(row[sex] == original[sex] for sex in ("female", "male")))
        document["grant_mappings"] = [row for row in document["grant_mappings"]
            if "outfit_key" not in row["target"] or row["target"]["outfit_key"] in keys]
        cls.author["promotions"] = [row for row in cls.author["promotions"]
            if not row["path"].startswith("/outfits/")]
        source = cls.author["promotions"][0]["source"]
        for path in tooling.promotion_paths(document):
            if path.startswith("/outfits/"):
                cls.author["promotions"].append({"path": path,
                    "value": tooling.pointer(document, path), "state": "PROMOTED",
                    "decision": "owner:canonical-supplement-fixture", "source_kind": "STRUCTURED_REFERENCE_DATA",
                    "source": copy.deepcopy(source)})
        for row in cls.author["promotions"]:
            row["value"] = tooling.pointer(document, row["path"])


    def test_main_identities_mounts_palette_and_grants_survive_idempotent_extension(self):
        result = supplement.merge(self.author, self.author, tooling)
        self.assertEqual(result["document"], self.author["document"])
        self.assertEqual(result["profile"], self.author["profile"])
        self.assertEqual(supplement.merge(result, self.author, tooling), result)
        tooling.validate(result)

    def test_old_quest_grant_resolves_to_the_existing_canonical_outfit(self):
        incoming = copy.deepcopy(self.author)
        citizen = next(row for row in incoming["document"]["outfits"]
                       if row["key"] == "oteryn:content.outfit.citizen_outfits")
        citizen["key"] = "oteryn:outfit.citizen"
        incoming["document"]["outfits"] = [citizen]
        incoming["document"]["grant_mappings"] = [{
            "grant_key": "oteryn:outfit.citizen/quest-grant/0",
            "target": {"kind": "Outfit", "outfit_key": "oteryn:outfit.citizen"},
            "compatible_provenance": ["crystal-9f5a72c-quest-rewards-v1"]}]
        before = copy.deepcopy(self.author)
        result = supplement.merge(self.author, incoming, tooling)
        self.assertEqual(self.author, before)
        self.assertEqual(result["document"]["outfits"], before["document"]["outfits"])
        grant = next(row for row in result["document"]["grant_mappings"]
                     if row["grant_key"] == "oteryn:outfit.citizen/quest-grant/0")
        self.assertEqual(grant["target"], {"kind": "Outfit", "outfit_key": "oteryn:content.outfit.citizen_outfits"})

    def test_source_palette_and_appearance_conflicts_cannot_replace_admitted_facts(self):
        for mutation in ["source", "palette", "appearance"]:
            incoming = copy.deepcopy(self.author)
            if mutation == "source":
                incoming["document"]["source_appearances_sha256"] = "0" * 64
            elif mutation == "palette":
                incoming["document"]["palette"]["colours"][0][0] ^= 1
            else:
                incoming["document"]["outfits"][0]["female"]["source_record_sha256"] = "0" * 64
            with self.assertRaises(ValueError, msg=mutation):
                supplement.merge(self.author, incoming, tooling)

    def test_existing_receipt_cannot_be_retargeted_to_another_outfit(self):
        incoming = copy.deepcopy(self.author)
        grant = next(row for row in incoming["document"]["grant_mappings"] if "outfit_key" in row["target"])
        grant["target"]["outfit_key"] = next(row["key"] for row in incoming["document"]["outfits"]
                                                 if row["key"] != grant["target"]["outfit_key"])
        with self.assertRaisesRegex(ValueError, "retargets an admitted grant"):
            supplement.merge(self.author, incoming, tooling)


if __name__ == "__main__":
    unittest.main()
