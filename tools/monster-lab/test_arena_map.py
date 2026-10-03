import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from arena_map import SOURCE, SOURCE_SHA256, build_arena_map, export_map, prepare_arena

REPOSITORY = Path(__file__).resolve().parents[2]


class ExistingArenaTests(unittest.TestCase):
    def setUp(self):
        self.source = REPOSITORY / SOURCE
        self.document = json.loads(self.source.read_text())

    def check_mutation(self, mutate, message):
        document = copy.deepcopy(self.document)
        mutate(document)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "source.json"
            raw = json.dumps(document).encode()
            path.write_bytes(raw)
            with self.assertRaisesRegex(ValueError, message):
                export_map(path, hashlib.sha256(raw).hexdigest())

    def test_actual_committed_room_preserved_and_two_open_slots(self):
        before = self.source.read_bytes()
        result = export_map(self.source, SOURCE_SHA256)["map"]
        self.assertEqual(len(result["cells"]), 4)
        self.assertEqual([(s["x"], s["y"], s["floor"]) for s in result["spawn_slots"]],
                         [(0, 0, 0), (1, 0, 0)])
        self.assertEqual(self.source.read_bytes(), before)
        self.assertEqual(build_arena_map(self.source), result)
        cells = {cell["key"]: cell for cell in result["cells"]}
        self.assertEqual(cells["oteryn:cell/entry-north"]["walkability"], "Blocked")
        self.assertEqual(cells["oteryn:cell/entry-door"]["walkability"], "DoorStateDependent")
        self.assertFalse(cells["oteryn:cell/entry-door"]["spawn_eligible"])

    def test_changed_source_hash_rejected(self):
        with self.assertRaisesRegex(ValueError, "SHA256"):
            export_map(self.source, "0" * 64)

    def test_out_of_bounds_rejected(self):
        self.check_mutation(lambda d: d["placements"][0].update(x=2), "outside")

    def test_blocked_monster_spawn_rejected(self):
        self.check_mutation(lambda d: d["native_first_entry"]["cells"][1].update(
            collision="Blocked"), "walkable")

    def test_unknown_monster_spawn_rejected(self):
        self.check_mutation(lambda d: d["native_first_entry"]["cells"].pop(1), "walkable")

    def test_bounded_cell_export(self):
        with self.assertRaisesRegex(ValueError, "cell limit"):
            export_map(self.source, SOURCE_SHA256, maximum_cells=3)

    def test_config_export_readback(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "arena-map.json"
            result = prepare_arena({"repository": str(REPOSITORY)}, output)
            self.assertEqual(json.loads(output.read_text()), result)


if __name__ == "__main__":
    unittest.main()
