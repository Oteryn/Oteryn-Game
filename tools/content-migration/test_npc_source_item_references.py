"""Protected alias resolution and immutable raw NPC source custody regressions."""

import copy
import importlib.util
import json
import unittest
from pathlib import Path

import npc_corrective_overlay as overlay
import npc_source_item_references as source


class SourceItemReferencesTests(unittest.TestCase):
    def setUp(self):
        self.old_key = "oteryn:item.registry.i00002771"
        self.active_key = "oteryn:item.tibia.i2874"
        self.service_key = "oteryn:service.trade.ahmet"
        self.alias = {
            "key": self.old_key,
            "version": 1,
            "state": "ALIAS",
            "target": self.active_key,
            "evidence": {"source_item_id": 2874},
        }
        self.source_field = overlay.field(
            "oteryn:source.npc.held_offer_057",
            {
                "old_native_offer": {"item": {"key": self.active_key}},
                "source_proof": {
                    "old_exact_candidate_offer": {
                        "item": {
                            "family": "Item",
                            "key": self.old_key,
                            "revision": "definition-r1",
                        }
                    }
                },
                "runtime_qualified": False,
            },
        )
        self.r4 = {
            "records": [
                {
                    "kind": "Service",
                    "identity": {"key": self.service_key, "revision": "definition-r1"},
                    "fields": [self.source_field],
                }
            ]
        }
        self.r4_raw = overlay.encode(self.r4)
        self.custody = {
            "locator": "evidence/r4/service-corrections.json",
            "sha256": source.digest(self.r4_raw),
        }

    def normalizer(self, aliases=None, active=None, r4=None):
        entries = aliases if aliases is not None else [self.alias]
        table = {
            "schema": "OTERYN_ITEM_KEY_ALIAS_TABLE/v1",
            "key_rule": "OTERYN_TIBIA_ID_KEY_RULE_V1",
            "entries": entries,
            "entries_sha256": source.digest(source.aliases_bytes(entries)),
        }
        return source.SourceItemNormalizer(
            source.aliases_bytes(table),
            {self.active_key} if active is None else active,
            self.r4 if r4 is None else r4,
            self.custody,
        )

    def test_exact_alias_rewrites_only_reference_and_keeps_lossless_r4_locator(self):
        original = copy.deepcopy(self.r4)
        normalizer = self.normalizer()
        result = normalizer.normalize(self.service_key, self.source_field)
        data = json.loads(result["value"]["value"])
        self.assertEqual(
            data["source_proof"]["old_exact_candidate_offer"]["item"]["key"],
            self.active_key,
        )
        self.assertEqual(data["old_native_offer"], {"item": {"key": self.active_key}})
        proof = data["item_reference_canonicalization"]
        self.assertEqual(proof["original_field"]["pointer"], "/records/0/fields/0")
        self.assertEqual(proof["original_field"]["sha256"], source.digest(self.r4_raw))
        self.assertEqual(
            proof["original_field"]["field_sha256"],
            source.digest(source.encode(self.source_field)),
        )
        self.assertEqual(
            proof["original_field"]["text_sha256"],
            source.digest(self.source_field["value"]["value"].encode()),
        )
        reference = proof["references"][0]
        historical = reference["historical_identity"]
        self.assertEqual(
            "oteryn:item." + historical["namespace"] + "." + historical["local_id"],
            self.old_key,
        )
        self.assertEqual(historical["family"], "Item")
        scanner_path = (
            Path(__file__).resolve().parents[1]
            / "content-census/item_key_references.py"
        )
        spec = importlib.util.spec_from_file_location(
            "npc_item_reference_scanner", scanner_path
        )
        scanner = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(scanner)
        self.assertEqual(
            set(scanner.KEY.findall(source.encode(result).decode())), {self.active_key}
        )
        self.assertEqual(reference["alias_version"], 1)
        self.assertEqual(
            reference["alias_entry_sha256"],
            source.digest(source.aliases_bytes(self.alias)),
        )
        self.assertNotIn(self.old_key, source.encode(result).decode())
        self.assertEqual(self.r4, original)
        self.assertEqual(source.encode(self.r4), self.r4_raw)
        self.assertEqual(normalizer.normalize(self.service_key, result), result)

    def test_unknown_retired_key_is_not_derived_from_numeric_suffix(self):
        unknown = copy.deepcopy(self.source_field)
        unknown["value"]["value"] = unknown["value"]["value"].replace(
            self.old_key, "oteryn:item.registry.i00009999"
        )
        with self.assertRaisesRegex(
            source.SourceItemReferenceError, "unknown or embedded"
        ):
            self.normalizer().normalize(self.service_key, unknown)

    def test_missing_active_target_and_alias_chain_fail_closed(self):
        with self.assertRaisesRegex(
            source.SourceItemReferenceError, "no active target"
        ):
            self.normalizer(active=set()).normalize(self.service_key, self.source_field)
        alias = {**self.alias, "target": "oteryn:item.registry.i00002772"}
        with self.assertRaisesRegex(source.SourceItemReferenceError, "not canonical"):
            self.normalizer(aliases=[alias], active={alias["target"]}).normalize(
                self.service_key, self.source_field
            )

    def test_retired_without_successor_is_not_admitted(self):
        alias = {**self.alias, "state": "RETIRED_WITHOUT_SUCCESSOR"}
        with self.assertRaisesRegex(
            source.SourceItemReferenceError, "no active target"
        ):
            self.normalizer(aliases=[alias]).normalize(
                self.service_key, self.source_field
            )

    def test_original_r4_field_and_custody_must_match_exactly(self):
        changed = copy.deepcopy(self.source_field)
        data = json.loads(changed["value"]["value"])
        data["runtime_qualified"] = True
        changed["value"]["value"] = source.encode(data).decode().rstrip("\n")
        with self.assertRaisesRegex(
            source.SourceItemReferenceError, "immutable R4 custody"
        ):
            self.normalizer().normalize(self.service_key, changed)
        with self.assertRaisesRegex(
            source.SourceItemReferenceError, "immutable R4 custody"
        ):
            self.normalizer(r4={"records": []}).normalize(
                self.service_key, self.source_field
            )

    def test_corrupt_alias_entries_digest_is_rejected(self):
        table = {
            "schema": "OTERYN_ITEM_KEY_ALIAS_TABLE/v1",
            "key_rule": "OTERYN_TIBIA_ID_KEY_RULE_V1",
            "entries": [self.alias],
            "entries_sha256": "0" * 64,
        }
        with self.assertRaisesRegex(source.SourceItemReferenceError, "entries digest"):
            source.SourceItemNormalizer(
                source.aliases_bytes(table), {self.active_key}, self.r4, self.custody
            )

    def test_other_source_values_and_executable_offer_are_untouched(self):
        canonical = overlay.field(
            "oteryn:source.npc.facts", {"item": self.active_key, "source_item_id": 2771}
        )
        self.assertEqual(
            self.normalizer().normalize(self.service_key, canonical), canonical
        )
        ordinary = overlay.field("oteryn:native.not_source", {"item": self.old_key})
        self.assertEqual(
            self.normalizer().normalize(self.service_key, ordinary), ordinary
        )

    def test_pipeline_normalizes_before_native_packet_without_loosening_before_fence(
        self,
    ):
        baseline = {
            "records": [
                {
                    "kind": "Service",
                    "identity": {"key": self.service_key, "revision": "definition-r1"},
                    "fields": [],
                }
            ]
        }
        plan = overlay.build_plan(
            baseline,
            self.r4,
            {"services": []},
            {"services": []},
            {"presentation_corrections": []},
            {"records": []},
            custody={"r4": self.custody},
            source_item_normalizer=self.normalizer(),
        )
        result = overlay.apply_plan(baseline, plan)
        self.assertNotIn(self.old_key, overlay.encode(result).decode())
        self.assertEqual(overlay.apply_plan(result, plan), result)
        with self.assertRaisesRegex(overlay.OverlayError, "unexpected source field"):
            overlay.apply_plan(self.r4, plan)
        self.assertEqual(source.encode(self.r4), self.r4_raw)


if __name__ == "__main__":
    unittest.main()
