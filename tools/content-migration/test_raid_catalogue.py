import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import raid_catalogue as raids


def source(path, text):
    raw = text.encode()
    return {"path": path, "text": text, "sha256": hashlib.sha256(raw).hexdigest(),
            "git_blob": hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()}


class RaidCatalogueTest(unittest.TestCase):
    def test_apply_plan_verifies_every_sha_before_any_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            plan = root / raids.APPLY_PLAN
            plan.parent.mkdir(parents=True)
            rows = []
            for i in range(2):
                relative = f"docs/reference/content-prep/raid-content-1/staged-{i}.json"
                raw = (json.dumps({"records": []}) + "\n").encode()
                (root / relative).write_bytes(raw)
                rows.append({"staged_path": relative,
                             "target_path": f"content/encounters/raids/target-{i}.json",
                             "sha256": hashlib.sha256(raw).hexdigest()})
            plan.write_text(json.dumps({"target_files": rows}))
            self.assertEqual(len(raids.verified_apply_plan(root)), 2)
            (root / rows[1]["staged_path"]).write_bytes(b"substituted")
            with self.assertRaisesRegex(ValueError, "RAID_APPLY_SHA256"):
                raids.verified_apply_plan(root)
            self.assertFalse((root / "content").exists())

    def test_source_sha_blob_and_revision_are_checked(self):
        snapshot = {"schema": raids.CAPTURE_SCHEMA, "repository": raids.REPOSITORY,
                    "revision": raids.REVISION,
                    "files": [source(raids.REGISTRY, '<raids/>')]}
        self.assertIn(raids.REGISTRY, raids.verified_sources(snapshot))
        altered = copy.deepcopy(snapshot)
        altered["files"][0]["text"] = '<raids> </raids>'
        with self.assertRaisesRegex(ValueError, "RAID_SOURCE_SHA256"):
            raids.verified_sources(altered)
        altered = copy.deepcopy(snapshot)
        altered["files"][0]["git_blob"] = "0" * 40
        with self.assertRaisesRegex(ValueError, "RAID_SOURCE_GIT_BLOB"):
            raids.verified_sources(altered)
        snapshot["revision"] = "unreviewed"
        with self.assertRaisesRegex(ValueError, "RAID_SOURCE_REVISION"):
            raids.verified_sources(snapshot)

    def test_unbound_area_is_inactive_not_relocated(self):
        area = {"from": {"x": 32000, "y": 32000, "floor": 7},
                "to": {"x": 32020, "y": 32020, "floor": 7}}
        original = copy.deepcopy(area)
        self.assertFalse(raids.bind_area(area, {"coordinate_frame": raids.FRAME,
                                               "region_size": 256, "regions": []}))
        self.assertEqual(area["from"], original["from"])
        self.assertEqual(area["to"], original["to"])
        record = {"key": "raid:unbound", "schedule": {"kind": "legacy_xml"},
                  "variants": [{"waves": [{"entries": [{"creature": "oteryn:creature.badger",
                    "source_name": "Badger", "amount": {"min": 1, "max": 1}, "boss": False,
                    "area": original, "area_anchor": raids.area_anchor(original)}]}]}],
                  "inactive_reasons": []}
        raids.qualify(record, {"oteryn:creature.badger": False},
                      {"coordinate_frame": raids.FRAME, "region_size": 256, "regions": []})
        self.assertFalse(record["active"])
        self.assertEqual(record["inactive_reasons"], ["area_unbound:raid-anchor:32000_32000_7-32020_32020_7"])

    def test_region_binding_preserves_source_coordinates(self):
        area = {"from": {"x": 32000, "y": 32000, "floor": 7},
                "to": {"x": 32020, "y": 32020, "floor": 7}}
        placements = {"coordinate_frame": raids.FRAME, "region_size": 256,
                      "regions": [{"path": "content/world/placements/region-z07-x125-y125.b3",
                                   "sha256": "a" * 64, "tiles": 1}]}
        self.assertTrue(raids.bind_area(area, placements))
        self.assertEqual(area["from"]["x"], 32000)
        self.assertEqual(area["coordinate_frame"], raids.FRAME)

    def test_every_kind_has_a_deterministic_classification(self):
        base = {"variants": [{"waves": [{"entries": [{"boss": False}]}]}],
                "wiki_area": "Thais City", "schedule": {"kind": "legacy_xml"}}
        self.assertEqual(raids.raid_kind(base), "city")
        base["wiki_area"] = "Plains of Havoc"
        self.assertEqual(raids.raid_kind(base), "area")
        base["variants"][0]["waves"][0]["entries"][0]["boss"] = True
        self.assertEqual(raids.raid_kind(base), "boss")
        base["variants"] *= 2
        self.assertEqual(raids.raid_kind(base), "multi_variant")
        base["schedule"] = {"kind": "calendar"}
        self.assertEqual(raids.raid_kind(base), "calendar")

    def test_legacy_delays_are_milliseconds_and_stagger_close_events(self):
        text = ('<raid><announce delay="100" message="First" type="Warning"/>'
                '<areaspawn delay="1300" fromx="1" fromy="2" fromz="7" '
                'tox="3" toy="4" toz="7"><monster name="Badger" minamount="1" '
                'maxamount="3"/></areaspawn><announce delay="60000" '
                'message="Last" type="Event"/></raid>')
        variant = raids.xml_variant(source("data-global/raids/test.xml", text),
                                    {"oteryn:creature.badger": False}, set())
        self.assertEqual([a["delay_ms"] for a in variant["announcements"]], [1000, 60700])
        self.assertEqual(variant["waves"][0]["delay_ms"], 2000)
        self.assertEqual(variant["waves"][0]["entries"][0]["amount"], {"min": 1, "max": 3})
        self.assertEqual(variant["announcements"][0]["kind"], "warning")

    def test_inactive_reasons_are_exact_and_active_never_has_one(self):
        record = {"key": "raid:test", "schedule": {"kind": "unknown"},
                  "variants": [], "inactive_reasons": []}
        raids.qualify(record, {}, {"coordinate_frame": raids.FRAME, "regions": [], "region_size": 256})
        self.assertFalse(record["active"])
        self.assertIn("schedule_unknown", record["inactive_reasons"])
        self.assertIn("waves_unknown", record["inactive_reasons"])

    def test_registered_content_preserves_unrelated_families(self):
        project = {"migrated_families": ["Existing"], "next_population_families": []}
        manifest = {"families": {"Existing": {"records": 1}}, "managed_files": [{"path": "existing"}]}
        lock = {"family_counts": {"Existing": 1}}
        outputs = raids.registered(project, manifest, lock, 4)
        self.assertEqual(outputs[1]["families"]["Existing"], {"records": 1})
        self.assertEqual(outputs[2]["family_counts"][raids.FAMILY], 4)
        self.assertNotIn(raids.FAMILY, manifest["families"])

    def test_calendar_end_is_the_last_active_reset_date(self):
        self.assertEqual(raids.calendar_schedule({"start": "October 31 server save",
                         "end": "November 3 server save", "interval_hours": 4})["window"],
                         {"start_month": 10, "start_day": 31, "end_month": 11, "end_day": 2})
        self.assertEqual(raids.calendar_schedule({"season": "June"})["window"]["end_day"], 30)

    def test_reversed_rectangle_matches_source_uniform_random_without_moving_it(self):
        result = raids.rectangle({"fromx": "32882", "fromy": "32804", "fromz": "7",
                                  "tox": "32818", "toy": "32764", "toz": "7"})
        self.assertEqual(result, {"from": {"x": 32818, "y": 32764, "floor": 7},
                                  "to": {"x": 32882, "y": 32804, "floor": 7}})

    def test_global_timeline_retimes_only_directly_anchored_source_events(self):
        variant = {"announcements": [
            {"raw_delay_ms": 1000, "delay_ms": 1000, "text": "First", "kind": "advance"},
            {"raw_delay_ms": 60000, "delay_ms": 60000, "text": "Last", "kind": "advance"}],
            "waves": [{"raw_delay_ms": 60000, "delay_ms": 61000, "entries": []}]}
        timeline = [{"offset": "00:00:00", "message": "First"},
                    {"offset": "00:06:40", "message": "Last"}]
        self.assertTrue(raids.apply_global_timeline(variant, timeline, [{"source": "wiki:1:2"}]))
        self.assertEqual([a["delay_ms"] for a in variant["announcements"]], [0, 400000])
        self.assertEqual(variant["waves"][0]["delay_ms"], 400000)
        self.assertEqual(variant["waves"][0]["raw_delay_ms"], 60000)
        unanchored = copy.deepcopy(variant)
        unanchored["waves"][0]["raw_delay_ms"] = 55000
        self.assertFalse(raids.apply_global_timeline(unanchored, timeline, []))

    def test_global_count_conflicts_and_custom_identity_do_not_activate(self):
        rows = {r["key"]: r for r in json.loads(raids.outputs(raids.ROOT, register=False)[raids.CATALOGUE])["records"]}
        self.assertFalse(rows["raid:abdendriel.chayenne"]["active"])
        self.assertFalse(rows["raid:carlin.trolls"]["active"])
        self.assertFalse(rows["raid:roshamuul.gaz_haragoth"]["active"])
        orshabaal = rows["raid:edron.orshabaal"]
        self.assertTrue(orshabaal["active"])
        self.assertEqual(orshabaal["variants"][0]["waves"][0]["delay_ms"], 400000)
        self.assertEqual(orshabaal["wiki_records"], ["oteryn:raid.edron_raids.orshabaal_raid_near_edron"])

    def test_inactive_flower_month_retains_all_source_areas_and_shared_budget(self):
        rows = json.loads(raids.outputs(raids.ROOT, register=False)[raids.CATALOGUE])["records"]
        flower = next(r for r in rows if r["key"].endswith("dryad_invasion"))
        self.assertFalse(flower["active"])
        cycle = flower["donor_cycle"]
        self.assertEqual(cycle["interval_ms"], 900000)
        self.assertEqual(len(cycle["areas"]), 4)
        self.assertEqual(cycle["requested_per_wave"], 25)
        self.assertEqual(cycle["shared_attempt_budget"], 250)
        self.assertTrue(all(area["area"]["regions"] for area in cycle["areas"]))

    def test_crystal_bosstiary_marks_horned_fox_case_insensitively(self):
        rows = json.loads(raids.outputs(raids.ROOT, register=False)[raids.CATALOGUE])["records"]
        fox = next(r for r in rows if r["key"] == "raid:kazordoon.thehornedfox")
        self.assertEqual(fox["kind"], "boss")
        entry = next(e for w in fox["variants"][0]["waves"] for e in w["entries"]
                     if e["creature"] == "oteryn:creature.the_horned_fox")
        self.assertEqual(entry["source_name"], "the Horned Fox")
        self.assertTrue(entry["boss"])
        self.assertEqual(entry["boss_source"]["path"], "data-global/monster/raids/the_horned_fox.lua")

    def test_full_generated_catalogue_is_reproducible_and_key_safe(self):
        root = Path(__file__).resolve().parents[2]
        generated = raids.outputs(root, register=False)
        self.assertEqual(generated, raids.outputs(root, register=False))
        for path, raw in generated.items():
            self.assertEqual((root / path).read_bytes(), raw, path)
        rows = json.loads(generated[raids.CATALOGUE])["records"]
        self.assertEqual(len({r["key"] for r in rows}), len(rows))
        for row in rows:
            self.assertRegex(row["key"], r"^[a-zA-Z0-9:._/-]+$")
            self.assertEqual(row["active"], not row["inactive_reasons"])
            for variant in row["variants"]:
                for wave in variant["waves"]:
                    for entry in wave["entries"]:
                        self.assertRegex(entry["area_anchor"], r"^[a-zA-Z0-9:._/-]+$")


if __name__ == "__main__":
    unittest.main()
