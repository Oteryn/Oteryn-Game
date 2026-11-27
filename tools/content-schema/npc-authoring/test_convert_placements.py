"""Tests for convert_placements.py: the Npc.Placement family is the scoped promotion candidates."""

import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import convert_placements as cp  # noqa: E402


class ConvertPlacementsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.files = cp.convert()
        cls.index = json.loads(cls.files["index.json"])
        cls.held = json.loads(cls.files["held.json"])["held"]
        cls.records = [
            r
            for name, text in sorted(cls.files.items())
            if name.startswith("placements-")
            for r in json.loads(text)["records"]
        ]

    def test_committed_family_is_the_conversion(self):
        for name, text in self.files.items():
            self.assertEqual((cp.OUT / name).read_text(), text, name)
        self.assertEqual({p.name for p in cp.OUT.glob("*.json")}, set(self.files))

    def test_shadows_errand_contacts_use_pinned_crystal_cells(self):
        expected = {
            ("boveas", 32431, 32090, 15): (3300, 2),
            ("cerdras", 32301, 31815, 6): (634, 1),
            ("dreadeye", 32800, 31572, 14): (3174, 2),
            ("milos", 33258, 31823, 3): (118, 1),
            ("scutty", 32574, 31915, 12): (3012, 1),
            ("telas", 33273, 31788, 11): (2913, 2),
            ("telas", 33276, 31800, 6): (619, 2),
        }
        contacts = {name for name, *_ in expected}
        placements = [r["declaration"] for r in self.records
                      if cp.local_name(r["declaration"]["npc"]) in contacts]
        def anchor(row):
            cell = row["cell"]
            return cp.local_name(row["npc"]), cell["x"], cell["y"], cell["floor"]

        self.assertCountEqual(map(anchor, placements), expected.keys())
        xml = "data-global/world/world-npc.xml"
        for row in placements:
            line, regional_citations = expected[anchor(row)]
            # The merged placement retains the quest proof and the regional line citations.
            self.assertCountEqual(
                [source["source_row"] for source in row["provenance"]],
                [xml] + [f"{xml}:{line}"] * regional_citations,
            )
            for source in row["provenance"]:
                self.assertEqual(source["origin"], "crystal")
                self.assertEqual(source["revision"], "201189f871d23597b82caf88fd611454a2ddefb8")

    def test_l1_reward_npcs_use_only_the_global_source_cells(self):
        expected = {
            'oteryn:npc.cael': {'x':33028,'y':31513,'floor':10},
            'oteryn:npc.cranky_lizard_crone': {'x':33192,'y':31044,'floor':6},
            'oteryn:npc.palimuth': {'x':32801,'y':31223,'floor':7},
            'oteryn:npc.yalahari': {'x':32801,'y':31188,'floor':7},
        }
        actual=[r['declaration'] for r in self.records if r['declaration']['npc'] in expected]
        self.assertEqual(len(actual),4)
        self.assertEqual({r['npc']:r['cell'] for r in actual},expected)
        self.assertTrue(all(r['provenance'] for r in actual))

    def test_marid_journey_npcs_use_crystal_global_anchors(self):
        expected = {
            "bo_ques": (33102, 32521, 5),
            "fa_hradin": (33102, 32540, 5),
            "gabel": (33101, 32520, 1),
            "haroun": (33109, 32525, 4),
            "maryza": (32634, 31889, 9),
            "melchior": (33142, 32817, 7),
            "nah_bob": (33103, 32520, 2),
            "rata_mari": (33037, 32626, 7),
            "the_orc_king": (32983, 31732, 9),
            "umar": (33102, 32531, 6),
        }
        actual = []
        for record in self.records:
            row = record["declaration"]
            key = row["npc"].removeprefix("oteryn:npc.")
            if key in expected:
                actual.append((key, tuple(row["cell"][axis] for axis in ("x", "y", "floor"))))
            if key == "umar":
                adjustment, = [source for source in row["provenance"]
                               if source.get("basis") == "CHOSEN_OTERYN_APPROXIMATION"]
                self.assertEqual(adjustment["source_cell"], {"x": 33102, "y": 32532, "floor": 6})
                self.assertEqual(adjustment["cell"], row["cell"])
        self.assertEqual(sorted(actual), sorted(expected.items()))

    def test_classic_quest_contacts_use_the_pinned_global_npc_anchors(self):
        expected = {
            "alesar": (33048, 32621, 5),
            "baa_leal": (33044, 32623, 4),
            "budrik": (32525, 31905, 8),
            "eroth": (32662, 31682, 6),
            "gabel": (33101, 32520, 1),
            "malor": (33044, 32622, 1),
            "one_eyed_joe": (32025, 32884, 6),
            "partos": (32323, 32280, 8),
            "the_orc_king": (32983, 31732, 9),
        }
        for name, (x, y, floor) in expected.items():
            with self.subTest(npc=name):
                rows = [r["declaration"] for r in self.records
                        if r["declaration"]["npc"] == "oteryn:npc." + name]
                self.assertEqual([row["cell"] for row in rows],
                                 [{"x": x, "y": y, "floor": floor}])
                self.assertEqual(rows[0]["provenance"][0]["revision"],
                                 "201189f871d23597b82caf88fd611454a2ddefb8")

    def test_storkus_uses_the_pinned_vampire_hunter_cell(self):
        rows = [r["declaration"] for r in self.records
                if r["declaration"]["npc"] == "oteryn:npc.storkus"]
        self.assertEqual([r["cell"] for r in rows], [{"x": 32589, "y": 31962, "floor": 7}])
        self.assertEqual(rows[0]["provenance"][0]["revision"],
                         "201189f871d23597b82caf88fd611454a2ddefb8")

    def test_gustavo_is_placed_at_the_newhaven_guard_post(self):
        rows = [r['declaration']['cell'] for r in self.records
                if r['declaration']['npc'] == 'oteryn:npc.gustavo_the_guard']
        self.assertEqual(rows, [{'x': 32560, 'y': 32488, 'floor': 7}])

    def test_jack_springer_uses_the_pinned_isle_of_the_kings_cell(self):
        rows = [r['declaration'] for r in self.records
                if r['declaration']['npc'] == 'oteryn:npc.jack_springer']
        self.assertEqual([r['cell'] for r in rows], [{'x': 32326, 'y': 32261, 'floor': 9}])
        source, = rows[0]['provenance']
        self.assertEqual(source['revision'], '201189f871d23597b82caf88fd611454a2ddefb8')
        self.assertEqual(source['source_row'], 'data-global/world/world-npc.xml')

    def test_captain_max_leaves_the_ferry_return_clear(self):
        rows = [r["declaration"] for r in self.records
                if r["declaration"]["npc"] == "oteryn:npc.captain_max"]
        self.assertEqual([row["cell"] for row in rows], [{"x": 32298, "y": 32895, "floor": 6}])
        self.assertEqual(rows[0]["direction"], "north")
        routes = cp.ROOT / "crates/gameplay/content/src/content/npc_catalogue/travel_routes.json"
        route, = [row for row in json.loads(routes.read_text())
                  if row["key"] == "oteryn:service.travel.captain_max_calassa/liberty_bay"]
        self.assertEqual((route["x"], route["y"], route["floor"]), (32298, 32896, 6))

    def test_m38_swan_keeps_the_exact_stationary_source_anchor(self):
        actual = [r['declaration'] for r in self.records
                  if r['declaration']['npc'] == 'oteryn:npc.a_swan']
        self.assertEqual(len(actual), 1)
        self.assertEqual(actual[0]['cell'], {'x': 33185, 'y': 31732, 'floor': 7})
        self.assertIn('placements-00000-00055.json', self.files)
        candidate = next(c for c in json.loads(cp.CANDIDATES.read_text())['candidates']
                         if c['identity']['key'] == 'oteryn:npc.a_swan')
        self.assertEqual(candidate['movement']['walk_interval_ms'], 0)
        self.assertTrue(all(p['definition_sha256'] in
                           {v['sha256'] for v in candidate['provenance'].values()}
                           for p in actual[0]['provenance']))

    def test_m31_king_keeps_the_reviewed_royal_source_cell(self):
        rows = [r['declaration'] for r in self.records
                if r['declaration']['npc'] == 'oteryn:npc.king_tibianus']
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['cell'], {'x': 32311, 'y': 32172, 'floor': 6})
        self.assertEqual(rows[0]['direction'], 'north')
        self.assertEqual({p['origin'] for p in rows[0]['provenance']}, {'canary', 'crystal'})

    def test_m31_lumberjack_contacts_keep_distinct_source_provenance(self):
        expected = {
            'oteryn:npc.chip': {'x': 32418, 'y': 31702, 'floor': 7},
            'oteryn:npc.queen_eloise': {'x': 32314, 'y': 31754, 'floor': 7},
        }
        actual = {r['declaration']['npc']: r['declaration'] for r in self.records
                  if r['declaration']['npc'] in expected}
        self.assertEqual(set(actual), set(expected))
        for key, cell in expected.items():
            self.assertEqual(actual[key]['cell'], cell)
            self.assertEqual(actual[key]['direction'], 'north')
        chip = actual['oteryn:npc.chip']['provenance']
        self.assertEqual(len(chip), 1)
        self.assertEqual(chip[0]['origin'], 'native_wiki_br')
        self.assertEqual(chip[0]['url'], 'https://www.tibiawiki.com.br/wiki/Chip')
        self.assertEqual(chip[0]['revid'], 416548)
        self.assertEqual(chip[0]['source_sha256'],
                         '8f569e3cda079c69cd1d118410c86fceef07cb1bcc77f488d2829f3d601a3402')
        self.assertEqual({p['origin'] for p in actual['oteryn:npc.queen_eloise']['provenance']},
                         {'canary', 'crystal'})

    def test_m38_mistrock_keeps_the_native_beregar_contact_anchor(self):
        actual = [r['declaration'] for r in self.records
                  if r['declaration']['npc'] == 'oteryn:npc.maris_mistrock']
        self.assertEqual(len(actual), 1)
        self.assertEqual(actual[0]['cell'], {'x': 32640, 'y': 31437, 'floor': 7})
        candidate = next(c for c in json.loads(cp.CANDIDATES.read_text())['candidates']
                         if c['identity']['key'] == 'oteryn:npc.maris_mistrock')
        self.assertEqual(candidate['placements'][0]['position'],
                         {'x': 32640, 'y': 31437, 'z': 7})
        self.assertTrue(all(p['definition_sha256'] in
                           {v['sha256'] for v in candidate['provenance'].values()}
                           for p in actual[0]['provenance']))

    def test_only_scoped_admitted_npcs(self):
        scope = set(json.loads(cp.SCOPE.read_text())["npcs"])
        contacts = json.loads(cp.CONTACTS.read_text())
        scope.update(f'oteryn:npc.{row["npc"]}' for group in contacts.values() for row in group)
        regional = json.loads(cp.regional_placements.SELECTION.read_text())["placements"]
        scope.update(f"oteryn:npc.{row['npc']}" for row in regional)
        self.assertEqual({r["declaration"]["npc"] for r in self.records}, scope - {h["npc"] for h in self.held})
        self.assertLessEqual(scope, cp.admitted_keys())

    def test_monk_pilgrimage_npcs_use_the_reviewed_crystal_cells(self):
        expected = {
            'oteryn:npc.ambassador_manop': {'x': 32057, 'y': 31898, 'floor': 4},
            'oteryn:npc.enpa_deia_pema': {'x': 33603, 'y': 31468, 'floor': 7},
        }
        actual = {r['declaration']['npc']: r['declaration'] for r in self.records
                  if r['declaration']['npc'] in expected}
        self.assertEqual(set(actual), set(expected))
        for key, cell in expected.items():
            self.assertEqual(actual[key]['cell'], cell)
            self.assertTrue(any(p['origin'] == 'crystal' for p in actual[key]['provenance']))

    def test_travora_captains_keep_their_source_cells(self):
        expected = {
            "oteryn:npc.captain_breezelda": (32341, 31109, 6),
            "oteryn:npc.captain_greyhound": (32387, 31822, 6),
            "oteryn:npc.captain_seahorse": (33175, 31763, 6),
            "oteryn:npc.captain_sinbeard": (33091, 32883, 6),
            "oteryn:npc.captain_tiberius": (32056, 32368, 6),
            "oteryn:npc.karith": (32818, 31278, 5),
            "oteryn:npc.petros": (33290, 32481, 6),
        }
        rows = [r["declaration"] for r in self.records if r["declaration"]["npc"] in expected]
        self.assertEqual(len(rows), len(expected))
        self.assertEqual(
            {r["npc"]: tuple(r["cell"][axis] for axis in ("x", "y", "floor")) for r in rows},
            expected,
        )
        self.assertTrue(all(r["provenance"] for r in rows))

    def test_viola_uses_the_newhaven_magic_shop_source_cell(self):
        viola = next(r['declaration'] for r in self.records
                     if r['declaration']['npc'] == 'oteryn:npc.viola')
        self.assertEqual(viola['cell'], {'x': 32571, 'y': 32507, 'floor': 7})
        self.assertEqual(viola['direction'], 'south')
        self.assertTrue(any(row['origin'] == 'crystal' for row in viola['provenance']))

    def test_records_sorted_unique_and_keyed_by_cell(self):
        keys = [r["declaration"]["identity"]["key"] for r in self.records]
        self.assertEqual(len(keys), len(set(keys)))
        shards = [json.loads(self.files[Path(name).name])["records"] for name in self.index["shards"]]
        original, *mainland = shards
        for group in (original, [row for shard in mainland for row in shard]):
            group_keys = [row["declaration"]["identity"]["key"] for row in group]
            self.assertEqual(group_keys, sorted(group_keys))
        for r in self.records:
            d = r["declaration"]
            c = d["cell"]
            self.assertEqual(
                d["identity"]["key"],
                f"oteryn:npc_placement.{d['npc'].removeprefix('oteryn:npc.')}.x{c['x']}_y{c['y']}_z{c['floor']}",
            )
            self.assertTrue(d["provenance"])

    def test_wiki_placement_is_north_and_says_so(self):
        zirella = next(r for r in self.records if r["declaration"]["npc"] == "oteryn:npc.zirella")["declaration"]
        self.assertEqual(zirella["direction"], "north")
        self.assertEqual(zirella["provenance"][0]["origin"], "wiki")
        self.assertIn("north", zirella["provenance"][0]["direction_note"])

    def test_no_cell_is_shared(self):
        cells = [(r["declaration"]["cell"]["x"], r["declaration"]["cell"]["y"], r["declaration"]["cell"]["floor"]) for r in self.records]
        self.assertEqual(len(cells), len(set(cells)))

    def test_summer_mayor_uses_his_admitted_cell_and_actual_source_revision(self):
        mayor = next(r["declaration"] for r in self.records
                     if r["declaration"]["npc"] == "oteryn:npc.mayor_pocaro")
        self.assertEqual(mayor["cell"], {"x": 34073, "y": 32504, "floor": 6})
        self.assertEqual(mayor["direction"], "north")
        self.assertEqual(mayor["provenance"][0]["revision"],
                         "00ce02a57ca5a12e48f32a3476e37471167e4c3f")
        candidate = next(c for c in json.loads(cp.CANDIDATES.read_text())["candidates"]
                         if c["identity"]["key"] == mayor["npc"])
        self.assertEqual(mayor["provenance"][0]["definition_sha256"],
                         candidate["provenance"]["crystal"]["sha256"])

    def test_index_counts_and_held_digest(self):
        self.assertEqual(self.index["record_count"], len(self.records))
        self.assertEqual(self.index["held"]["count"], len(self.held))
        import hashlib

        self.assertEqual(self.index["held"]["sha256"], hashlib.sha256(self.files["held.json"].encode()).hexdigest())
        self.assertTrue(all(h["reason"] in cp.REASONS for h in self.held))


if __name__ == "__main__":
    unittest.main()
