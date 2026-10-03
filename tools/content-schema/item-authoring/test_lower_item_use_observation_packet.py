"""Positive and opposing complete-source observation guards."""

import copy
import importlib.util
import json
import unittest
from pathlib import Path
from unittest.mock import patch

import lower_item_use_observation_packet as lane
from fandom_use_observation_aliases import build_fandom_use_observation_alias_supplement
import sys

# Main's migration validator imports its sibling modules, as its own scripts do.
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "content-migration"))


class Observations(unittest.TestCase):
    def test_exact_types_no_expression_evaluation_or_cost_suffix_stripping(self):
        self.assertEqual(
            lane.parse_value("damagerange", "13 (8-18)", 1),
            {"kind": "Text", "value": "13 (8-18)"},
        )
        self.assertEqual(
            lane.parse_value("damagerange", "65", 1), {"kind": "Integer", "value": 65}
        )
        self.assertEqual(lane.parse_value("manacost", "+10", 52336), 10)
        self.assertEqual(lane.parse_value("manacost", "0", 1), 0)
        for parameter, value, iid in [
            ("manacost", "+10", 1),
            ("manacost", "+3 mana per attack", 1),
            ("manacost", "4294967296", 1),
            ("damagerange", "4-3", 1),
            ("damagerange", "", 1),
        ]:
            with self.assertRaises(ValueError):
                lane.parse_value(parameter, value, iid)

    def test_explicit_alias_catalog_is_separate_from_legacy_census(self):
        catalog = build_fandom_use_observation_alias_supplement()
        self.assertEqual(json.loads((lane.ROOT / lane.ALIASES).read_text()), catalog)
        self.assertEqual(
            {
                r["source_field"]: r["canonical_source_property"]
                for r in catalog["fields"]
            },
            {"damagerange": "damage", "manacost": "mana_cost"},
        )

    def test_complete_source_scope_uses_strict_actual_names(self):
        packet = lane.build()
        self.assertEqual(packet["counts"]["fields"], 345)
        self.assertEqual(packet["counts"]["items"], 139)
        self.assertEqual(
            packet["counts"]["by_field"],
            {"damage": 111, "damage_type": 138, "mana_cost": 96},
        )

    def test_all_present_opposition_shared_id_and_existing_owner_fail(self):
        proof = json.loads((lane.ROOT / lane.PROOF).read_text())
        source = proof["records"][0]
        record = next(
            r
            for shard in json.loads(
                (lane.ROOT / "content/items/index.json").read_text()
            )["shards"]
            for r in json.loads((lane.ROOT / shard).read_text())["records"]
            if r["definition"]["identity"] == source["target"]
        )
        token = source["own_tokens"][0]
        params = lane.base.raw_parameters(
            proof["competing_own_infobox_witnesses"][token]["raw_own_infobox"]
        )
        box = {
            "balanced": True,
            "inside_comment": False,
            "positive_exact_infobox_object_match": True,
        }
        obj = {"name": source["official_name"], "flags": {"flags.take": True}}
        own = [({}, box, params)]
        self.assertEqual(
            lane.qualify(source, record, source["binding"], obj, own, set())["facts"],
            source["facts"],
        )
        for parameter, value in [
            ("manacost", ["999"]),
            ("manacost", [""]),
            ("itemid", [f"{source['source_item_id']},1"]),
            ("actualname", ["other"]),
            ("damage", ["1"]),
        ]:
            changed = copy.deepcopy(params)
            changed[parameter] = value
            with self.assertRaises(ValueError):
                lane.qualify(
                    source,
                    record,
                    source["binding"],
                    obj,
                    own + [({}, box, changed)],
                    set(),
                )
        changed = copy.deepcopy(record)
        changed.setdefault("authoring", {})["use_observation"] = {"mana_cost": 999}
        with self.assertRaises(ValueError):
            lane.qualify(source, changed, source["binding"], obj, own, set())
        with self.assertRaises(ValueError):
            lane.qualify(
                source, record, source["binding"], obj, own, {source["target"]["key"]}
            )
        changed = copy.deepcopy(record)
        changed["definition"]["semantics"]["presentation"] = {
            "state": "KNOWN",
            "value": {"name": {"state": "KNOWN", "value": "wrong imported name"}},
        }
        for prospective in [False, True]:
            with self.assertRaises(ValueError):
                lane.qualify(
                    source, changed, source["binding"], obj, own, set(), prospective
                )

    def test_migration_closed_observations_and_ml_siblings_reject_forgery(self):
        spec = importlib.util.spec_from_file_location(
            "use_migration",
            lane.ROOT / "tools/content-migration/validate_world_project_v2_to_tree.py",
        )
        migration = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(migration)
        aliases = migration.item_alias_targets()
        staged = {
            migration.staged_item_target(row["target"], aliases): row
            for row in json.loads(
                (
                    lane.ROOT
                    / "docs/agents/evidence/OTV2-20260925-item-enrichment-wave1-staged.json"
                ).read_text()
            )["items"]
        }
        weapon_rows, receipt = migration.closed_weapon_metadata()
        legacy = {
            migration.target_id(row["item"]): copy.deepcopy(row)
            for row in receipt["parent_authoring"]
        }
        packet = json.loads(lane.OUTPUT.read_text())
        for row in packet["promotions"]:
            owner = legacy.setdefault(
                migration.target_id(row["target"]), {"item": row["target"]}
            )
            owner["use_observation"] = copy.deepcopy(row["facts"])
        forge = migration.closed_forge_owner()
        legacy[migration.target_id(forge["item"])] = forge
        self.assertEqual(len(legacy), 316)
        for weapon_key, row in weapon_rows.items():
            legacy.setdefault(weapon_key, {"item": row["target"]}).update(
                copy.deepcopy(row["facts"])
            )
        self.assertEqual(len(legacy), 411)
        migration.extend_forge289_owners(legacy)
        migration.validate_item_authoring_targets(legacy, staged)
        key = next(
            migration.target_id(row["target"])
            for row in packet["promotions"]
            if "required_magic_level" in legacy[migration.target_id(row["target"])]
        )
        missing = copy.deepcopy(legacy)
        missing[key].pop("use_observation")
        wrong = copy.deepcopy(legacy)
        wrong[key]["required_magic_level"] = 65535
        partial = copy.deepcopy(legacy)
        partial[key]["use_observation"].pop("damage_type")
        extra = copy.deepcopy(legacy)
        extra[key]["use_ability"] = "invented"
        mistyped = copy.deepcopy(legacy)
        mistyped[key]["use_observation"]["mana_cost"] = False
        for bad in [
            missing,
            wrong,
            partial,
            extra,
            mistyped,
            legacy | {("Item", "unqualified", "definition-r1"): {}},
            {k: v for k, v in legacy.items() if k != key},
        ]:
            with self.assertRaises(migration.ValidationError):
                migration.validate_item_authoring_targets(bad, staged)
        read_bytes = Path.read_bytes

        def corrupt_use(path):
            return (
                b"forged packet" if path.name == lane.OUTPUT.name else read_bytes(path)
            )

        with (
            patch("pathlib.Path.read_bytes", corrupt_use),
            self.assertRaisesRegex(
                migration.ValidationError, "USE_OBSERVATION_PACKET_DIGEST"
            ),
        ):
            migration.validate_item_authoring_targets(legacy, staged)

    def test_new_relation_requires_sealed_use_and_existing_known_slot(self):
        spec = importlib.util.spec_from_file_location(
            "use_relations",
            lane.ROOT / "tools/content-migration/validate_world_project_v2_to_tree.py",
        )
        migration = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(migration)
        source = json.loads(lane.OUTPUT.read_text())["promotions"][0]["target"]
        row = {
            "source": source,
            "relations": [
                {
                    "relation": "CAPABILITY_GOVERNED_BY",
                    "ruleset": "rulesets/items/imbuements/",
                    "basis": "imbuement.slot_count>=1",
                }
            ],
        }
        definition = {
            "identity": source,
            "semantics": {
                "imbuement": {
                    "state": "KNOWN",
                    "value": {"slot_count": {"state": "KNOWN", "value": 1}},
                }
            },
        }
        migration.validate_use_relation(row, definition)
        badrow = copy.deepcopy(row)
        badrow["relations"][0]["basis"] = "use_observation"
        extra = copy.deepcopy(row)
        extra["relations"].append(extra["relations"][0])
        badtarget = copy.deepcopy(row)
        badtarget["source"]["key"] = "unqualified"
        wrongrule = copy.deepcopy(row)
        wrongrule["relations"][0]["ruleset"] = "rulesets/items/exaltation-forge/"
        partial = copy.deepcopy(row)
        partial["relations"][0].pop("relation")
        extrakey = copy.deepcopy(row)
        extrakey["invented_metadata"] = True
        for candidate in [badrow, extra, badtarget, wrongrule, partial, extrakey]:
            with self.assertRaises(migration.ValidationError):
                migration.validate_use_relation(candidate, definition)
        for slot in [
            {"state": "UNKNOWN"},
            {"state": "KNOWN", "value": 0},
            {"state": "KNOWN", "value": True},
        ]:
            bad = copy.deepcopy(definition)
            bad["semantics"]["imbuement"]["value"]["slot_count"] = slot
            with self.assertRaises(migration.ValidationError):
                migration.validate_use_relation(row, bad)


if __name__ == "__main__":
    unittest.main()
