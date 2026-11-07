import json
import hashlib
import tarfile
import unittest
from pathlib import Path

import validate_encounter as validator


EXPECTED_HARD_HOLDS = {
    'claustrophobic_inferno_raid_1': [
        (1, list(range(43, 52))),
        (1, list(range(58, 82))),
    ],
    'claustrophobic_inferno_raid_2': [
        (1, list(range(43, 52))),
        (1, list(range(58, 82))),
        (0, list(range(208, 209))),
    ],
    'claustrophobic_inferno_raid_3': [
        (1, list(range(43, 52))),
        (1, list(range(58, 82))),
    ],
    'goshnars_greed': [
        (4, list(range(98, 107))),
        (5, list(range(94, 114))),
        (6, list(range(90, 125))),
        (4, list(range(92, 109))),
        (1, list(range(156, 176))),
        (2, list(range(92, 105))),
    ],
    'goshnars_malice': [
        (3, list(range(979, 1005))),
        (3, list(range(984, 992))),
        (1, list(range(139, 149))),
        (3, list(range(1071, 1109))),
        (3, [1006, 1007, 1008, 1009, 1010, 1011, 1012, 1013, 1014, 1015, 1016, 1017, 1018, 1019, 1020, 1021, 1022, 1023, 1024, 1025, 1026, 1027, 1028, 1029, 1030, 1031, 1032, 1033, 1034, 1035, 1036, 1037, 1038, 1039, 1040, 1041, 1042, 1043, 1044, 1045, 1046, 1047, 1048, 1049, 1050, 1051, 1052, 1053, 1054, 1055, 1056, 1057, 1058, 1110, 1111, 1112, 1113, 1114, 1115, 1116, 1117, 1118, 1119, 1120, 1121, 1122, 1123, 1124, 1125, 1126, 1127, 1128, 1129, 1130, 1131, 1132, 1133, 1134, 1135, 1136, 1137, 1138, 1139, 1140, 1141, 1142, 1143, 1144, 1145, 1146, 1147, 1148]),
        (2, list(range(48, 53))),
    ],
    'goshnars_spite': [
        (1, list(range(415, 433))),
        (1, list(range(400, 414))),
        (1, list(range(434, 481))),
        (1, list(range(364, 399))),
        (0, list(range(602, 618))),
    ],
    'goshnars_cruelty': [
        (2, list(range(138, 155))),
        (1, list(range(860, 912))),
        (1, list(range(1089, 1104))),
        (2, list(range(107, 108))),
        (0, list(range(1477, 1495))),
        (4, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 37]),
    ],
    'goshnars_hatred': [
        (1, list(range(600, 640))),
        (1, list(range(672, 696))),
        (1, list(range(579, 596))),
        (1, list(range(655, 663))),
        (2, list(range(136, 146))),
    ],
    'goshnars_megalomania': [
        (1, list(range(913, 941))),
        (1, list(range(942, 1030))),
        (2, list(range(117, 127))),
        (3, list(range(141, 151))),
        (4, list(range(130, 140))),
        (2, list(range(71, 72))),
        (3, list(range(112, 113))),
        (1, list(range(1031, 1048))),
        (0, list(range(1370, 1384))),
        (0, list(range(37, 38))),
    ],
}


class SoulWarCrystalReconstructionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parent
        cls.sample_root = cls.root / "samples" / "soul-war-crystal-reconstruction"
        cls.index = json.loads((cls.sample_root / "index.json").read_text(encoding="utf-8"))

    def test_index_is_closed_and_non_runtime(self):
        self.assertFalse(self.index["runtime_qualified"])
        self.assertEqual(
            self.index["encounters"],
            [
                "claustrophobic_inferno_raid_1",
                "claustrophobic_inferno_raid_2",
                "claustrophobic_inferno_raid_3",
                "goshnars_greed",
                "goshnars_malice",
                "goshnars_spite",
                "goshnars_cruelty",
                "goshnars_hatred",
                "goshnars_megalomania",
            ],
        )
        self.assertEqual(
            self.index["source"]["revision"],
            "00ce02a57ca5a12e48f32a3476e37471167e4c3f",
        )

    def test_every_bundle_validates_and_keeps_hard_holds(self):
        for name in self.index["encounters"]:
            with self.subTest(name=name):
                base = self.sample_root / name
                encounter = json.loads((base / "encounter.json").read_text(encoding="utf-8"))
                catalog = json.loads((base / "catalog.json").read_text(encoding="utf-8"))
                manifest = json.loads((base / "manifest.json").read_text(encoding="utf-8"))
                self.assertEqual(validator.validate(encounter, catalog, manifest), [])
                self.assertEqual(manifest["classification"], "SOURCE_BACKED_OTERYN_RECONSTRUCTION")
                self.assertEqual(
                    {src["revision"] for src in manifest["sources"]},
                    {"00ce02a57ca5a12e48f32a3476e37471167e4c3f"},
                )
                self.assertTrue(manifest["entries"])

    def test_all_existing_unresolved_source_ranges_are_preserved(self):
        expected = EXPECTED_HARD_HOLDS
        for name, ranges in expected.items():
            with self.subTest(name=name):
                manifest = json.loads((self.sample_root / name / "manifest.json").read_bytes())
                encounter = json.loads((self.sample_root / name / "encounter.json").read_bytes())
                self.assertEqual(manifest["encounter"], encounter["identity"]["key"])
                actual = [(row["source_index"], row["source_lines"]) for row in manifest["entries"]
                          if row["status"] == "unresolved_semantics"]
                self.assertEqual(actual, ranges)
        greed = json.loads((self.sample_root / "goshnars_greed" / "manifest.json").read_bytes())
        sphere = [row for row in greed["entries"] if row["source_index"] == 6
                  and row["status"] == "unresolved_semantics"]
        self.assertEqual(len(sphere), 1)
        self.assertIn("every 3000 ms", sphere[0]["resolution"])

    def test_corrected_manifest_destinations_reference_the_intended_rules(self):
        cases = [
            ("goshnars_megalomania", 0, list(range(588, 601)), "blue_phase_returns_purple"),
            ("goshnars_hatred", 1, list(range(647, 654)), "hatred_hardens_on_player_hit"),
            ("goshnars_hatred", 1, list(range(655, 663)), "blaze_returns_to_ashes_and_empowers_boss"),
        ]
        for name, source_index, source_lines, rule_key in cases:
            with self.subTest(name=name, rule=rule_key):
                manifest = json.loads((self.sample_root / name / "manifest.json").read_bytes())
                encounter = json.loads((self.sample_root / name / "encounter.json").read_bytes())
                entry = next(row for row in manifest["entries"]
                             if row["source_index"] == source_index and row["source_lines"] == source_lines)
                self.assertEqual(entry["status"], "mapped")
                self.assertTrue(entry["destination"].startswith("/encounter/rules/"))
                self.assertEqual(encounter["rules"][int(entry["destination"].rsplit("/", 1)[1])]["key"], rule_key)
        for number in (1, 2, 3):
            manifest = json.loads((self.sample_root / f"claustrophobic_inferno_raid_{number}" / "manifest.json").read_bytes())
            mapped = [row for row in manifest["entries"] if row.get("destination") == "/encounter/state/timers"]
            self.assertEqual([row["source_lines"] for row in mapped], [[259, 260]])

    def test_every_manifest_source_and_line_range_matches_the_pinned_corpus(self):
        root = self.sample_root.parents[4]
        corpus_dir = root / "tools/content-schema/quest-authoring/samples/donor-source"
        declaration = json.loads((corpus_dir / "corpus-input.json").read_bytes())
        archive = corpus_dir / declaration["path"]
        self.assertEqual(hashlib.sha256(archive.read_bytes()).hexdigest(), declaration["sha256"])
        with tarfile.open(archive, "r:gz") as stream:
            raw_manifest = stream.extractfile("corpus-manifest.json").read()
            self.assertEqual(hashlib.sha256(raw_manifest).hexdigest(), declaration["manifest_sha256"])
            corpus = json.loads(raw_manifest)
            rows = {(row["revision"], row["path"]): row for row in corpus["files"]
                    if row["source"] == "crystal-summer"}
            source_cache = {}
            for name in self.index["encounters"]:
                manifest = json.loads((self.sample_root / name / "manifest.json").read_bytes())
                sources = []
                for source in manifest["sources"]:
                    key = (source["revision"], source["path"])
                    with self.subTest(encounter=name, source=source["path"]):
                        row = rows[key]
                        self.assertEqual(source["repository"], row["repository"])
                        self.assertEqual(source["blob_sha1"], row["git_blob_sha1"])
                        if key not in source_cache:
                            raw = stream.extractfile(row["cache_path"]).read()
                            self.assertEqual(hashlib.sha256(raw).hexdigest(), row["sha256"])
                            self.assertEqual(hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest(), source["blob_sha1"])
                            source_cache[key] = raw.decode("utf-8").splitlines()
                        sources.append(source_cache[key])
                for entry in manifest["entries"]:
                    with self.subTest(encounter=name, entry=entry["resolution"]):
                        self.assertTrue(entry["source_lines"])
                        self.assertGreaterEqual(min(entry["source_lines"]), 1)
                        self.assertLessEqual(max(entry["source_lines"]), len(sources[entry["source_index"]]))

    def test_survival_and_final_phase_guards_use_existing_vocabulary(self):
        greed = json.loads((self.sample_root / "goshnars_greed" / "encounter.json").read_bytes())
        rules = {row["key"]: row for row in greed["rules"]}
        survival = {"kind": "in_anchor", "subject": {"triggering": True}, "anchor": "arena"}
        for name in ("weak_soul_becomes_strong", "strong_soul_becomes_powerful", "powerful_soul_buffs_boss"):
            self.assertIn(survival, rules[name]["conditions"])
            self.assertIn({"kind": "remove", "triggering": True}, rules[name]["actions"])
            self.assertFalse(any(action["kind"] == "transform" for action in rules[name]["actions"]))
        powerful = rules["powerful_soul_buffs_boss"]
        self.assertFalse(any(action["kind"] == "spawn" for action in powerful["actions"]))
        manifest = json.loads((self.sample_root / "goshnars_greed" / "manifest.json").read_bytes())
        self.assertTrue(any(row["status"] == "unresolved_semantics" and "delayed per-original-entity replacement" in row["resolution"] for row in manifest["entries"]))
        mega = json.loads((self.sample_root / "goshnars_megalomania" / "encounter.json").read_bytes())
        rules = {row["key"]: row for row in mega["rules"]}
        master = {"kind": "has_master", "role": "aspect_of_power", "value": False}
        self.assertIn(master, rules["aspect_death_counts"]["conditions"])
        self.assertIn(master, rules["aspect_returns_after_five_seconds"]["conditions"])
        self.assertEqual(rules["green_phase_becomes_blue"]["actions"][0],
                         {"kind": "teleport", "who": {"role": "goshnars_megalomania"}, "to": "boss"})
        self.assertIn({"kind": "flag", "flag": "purple_phase", "value": False}, rules["boss_death"]["conditions"])
        manifest = json.loads((self.sample_root / "goshnars_megalomania" / "manifest.json").read_bytes())
        self.assertNotIn("SoulWarMegalomaniaDeath", manifest["covers"])
        self.assertEqual(set(manifest["covers"]["MegalomaniaDeath"]),
                         {"canary:creature/goshnar_s_megalomania_green", "canary:creature/goshnar_s_megalomania_blue"})

    def test_known_unrepresentable_mechanics_stay_explicit(self):
        expected = {
            "claustrophobic_inferno_raid_1": ("players", "cooldown"),
            "claustrophobic_inferno_raid_2": ("players", "bare 'Pos'"),
            "claustrophobic_inferno_raid_3": ("players", "cooldown"),
            "goshnars_malice": ("white", "reflect"),
            "goshnars_greed": ("Soul Sphere", "Soulsnatcher"),
            "goshnars_spite": ("per-player", "Weeping Soul"),
            "goshnars_cruelty": ("per-player", "transform elemental"),
            "goshnars_hatred": ("per-player", "Condensed Remorse"),
            "goshnars_megalomania": ("player", "white"),
        }
        for name, needles in expected.items():
            with self.subTest(name=name):
                manifest = json.loads(
                    (self.sample_root / name / "manifest.json").read_text(encoding="utf-8")
                )
                unresolved = "\n".join(
                    row["resolution"]
                    for row in manifest["entries"]
                    if row["status"] == "unresolved_semantics"
                )
                self.assertTrue(unresolved, f"{name} unexpectedly lost all hard holds")
                for needle in needles:
                    self.assertIn(needle.lower(), unresolved.lower())


if __name__ == "__main__":
    unittest.main()
