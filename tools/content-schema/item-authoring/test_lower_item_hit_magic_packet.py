"""Literal units, zero ML, all-page agreement and normal current-name guards."""

import copy
import importlib.util
import json
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

import lower_item_hit_magic_packet as fields

# Main's migration validator imports its sibling modules, as its own scripts do.
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "content-migration"))


class HitMagic(unittest.TestCase):
    def args(self, value="6", member="hit_chance"):
        raw = (
            "{{Infobox Object|itemid=7|name=Bow|"
            + ("hit_mod=" if member == "hit_chance" else "mlrequired=")
            + value
            + "}}"
        )
        page = {
            "revision_id": 1,
            "revision_timestamp": "2026-09-01T00:00:00Z",
            "content_sha256": "declared article digest",
            "source_part": "part",
            "source_part_sha256": "part sha",
            "source_page_ordinal": 0,
            "own_objects": [{"box_index": 0, "raw_itemid_values": ["7"]}],
        }
        witness = {k: v for k, v in page.items() if k != "own_objects"}
        witness.update(
            page_id=1,
            box_index=0,
            raw_infobox=raw,
            raw_infobox_sha256=fields.base.sha(raw.encode()),
            balanced=True,
            inside_comment=False,
            positive_exact_infobox_object_match=True,
        )
        return [
            {
                "source_item_id": 7,
                "official_name": "bow",
                "sources": [witness],
                "facts": {
                    member: {"numerator": 3, "denominator": 50}
                    if member == "hit_chance"
                    else 0
                },
            },
            {7: {1}},
            {1: page},
            "2026-09-27T23:59:59Z",
        ]

    def test_exact_signed_relative_units_and_explicit_zero(self):
        for raw, n, d in [("6", 3, 50), ("-7", -7, 100), ("0", 0, 1), ("200", 2, 1)]:
            args = self.args(raw)
            args[0]["facts"]["hit_chance"] = {"numerator": n, "denominator": d}
            self.assertEqual(fields.parse_sources(*args), args[0]["facts"])
        args = self.args("0", "required_magic_level")
        self.assertEqual(fields.parse_sources(*args), {"required_magic_level": 0})
        for raw, member in [
            ("", "hit_chance"),
            ("6%", "hit_chance"),
            ("0.06", "hit_chance"),
            ("6|hit_mod=6", "hit_chance"),
            (str(2**70), "hit_chance"),
            (str((2**63 - 1) * 100), "hit_chance"),
            ("-1", "required_magic_level"),
            ("65536", "required_magic_level"),
        ]:
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                fields.parse_sources(*self.args(raw, member))

    def test_all_present_pages_must_agree(self):
        args = self.args()
        second = copy.deepcopy(args[0]["sources"][0])
        second["page_id"] = 2
        args[0]["sources"].append(second)
        args[1][7].add(2)
        args[2][2] = copy.deepcopy(args[2][1])
        self.assertEqual(fields.parse_sources(*args), args[0]["facts"])
        second["raw_infobox"] = second["raw_infobox"].replace("hit_mod=6", "hit_mod=7")
        second["raw_infobox_sha256"] = fields.base.sha(second["raw_infobox"].encode())
        with self.assertRaisesRegex(ValueError, "disagreement"):
            fields.parse_sources(*args)

    def test_migration_allows_only_sealed_magic_owners(self):
        spec = importlib.util.spec_from_file_location(
            "migration",
            fields.ROOT
            / "tools/content-migration/validate_world_project_v2_to_tree.py",
        )
        migration = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(migration)
        weapon_rows, receipt = migration.closed_weapon_metadata()
        owners = copy.deepcopy(receipt["parent_authoring"])
        legacy = {migration.target_id(o["item"]): o for o in owners}
        rows = json.loads(fields.OUTPUT.read_text())["promotions"]
        magic = [r for r in rows if "required_magic_level" in r["facts"]]
        for row in magic:
            legacy[migration.target_id(row["target"])] = {
                "item": row["target"],
                "required_magic_level": row["facts"]["required_magic_level"],
            }
        use = json.loads(
            (
                fields.ROOT
                / "docs/agents/evidence/OTV2-20261002-item-use-observation-promotion-v1.json"
            ).read_text()
        )
        for row in use["promotions"]:
            owner = legacy.setdefault(
                migration.target_id(row["target"]), {"item": row["target"]}
            )
            owner["use_observation"] = row["facts"]
        aliases = migration.item_alias_targets()
        staged = {
            migration.staged_item_target(row["target"], aliases): row
            for row in json.loads(
                (
                    fields.ROOT
                    / "docs/agents/evidence/OTV2-20260925-item-enrichment-wave1-staged.json"
                ).read_text()
            )["items"]
        }
        keys = {migration.target_id(r["target"]) for r in magic}
        forge = migration.closed_forge_owner()
        legacy[migration.target_id(forge["item"])] = forge
        for weapon_key, row in weapon_rows.items():
            legacy.setdefault(weapon_key, {"item": row["target"]}).update(
                copy.deepcopy(row["facts"])
            )
        migration.extend_forge289_owners(legacy)
        migration.validate_item_authoring_targets(legacy, staged)
        key = next(iter(keys))
        for bad in (
            legacy | {("Item", "unqualified", "definition-r1"): {}},
            {k: v for k, v in legacy.items() if k != key},
            legacy | {key: legacy[key] | {"required_magic_level": 65535}},
            legacy | {key: legacy[key] | {"use_ability": "invented"}},
        ):
            with self.assertRaises(migration.ValidationError):
                migration.validate_item_authoring_targets(bad, staged)
        with (
            patch("pathlib.Path.read_bytes", return_value=b"forged packet"),
            self.assertRaisesRegex(migration.ValidationError, "PACKET_DIGEST"),
        ):
            migration.validate_item_authoring_targets(legacy, staged)

    def test_no_name_exception_blocked_hit_and_owned_ml_conflict(self):
        source = self.args()[0]
        source["target"] = {
            "family": "Item",
            "key": "item",
            "revision": "definition-r1",
        }
        facts = source["facts"]
        definition = {"semantics": {}}
        self.assertEqual(
            fields.qualify(source, definition, [], facts)["hit_percentage_points"], 6
        )
        for state in [
            {"state": "CONFLICT"},
            {"state": "NOT_APPLICABLE"},
            {"state": "KNOWN", "value": "weapon of carving"},
        ]:
            definition["semantics"] = {
                "presentation": {"state": "KNOWN", "value": {"name": state}}
            }
            with self.assertRaisesRegex(ValueError, "normal native name"):
                fields.qualify(source, definition, [], facts)
        definition["semantics"] = {"weapon": {"state": "CONFLICT"}}
        with self.assertRaisesRegex(ValueError, "blocked"):
            fields.qualify(source, definition, [], facts)
        definition["semantics"] = {}
        owner = {"item": source["target"], "required_magic_level": 1}
        with self.assertRaisesRegex(ValueError, "magic level"):
            fields.qualify(source, definition, [owner], {"required_magic_level": 0})
        for changed in [
            [owner, owner],
            [{"item": source["target"] | {"revision": "other"}}],
        ]:
            with self.assertRaisesRegex(ValueError, "target/duplicate"):
                fields.qualify(source, definition, changed, facts)


if __name__ == "__main__":
    unittest.main()
