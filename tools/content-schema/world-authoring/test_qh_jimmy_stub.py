"""Top of the City: Jimmy's passage stub is generated, exact and refuses drift."""

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(HERE))


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


stub = load("qh_jimmy_stub_test", HERE / "qh_jimmy_stub.py")
codec = stub.codec
IDS = [408, 440, 441, 442, 449, 450]


class CommittedWorld(unittest.TestCase):
    def test_committed_world_carries_the_exact_stub(self):
        index = json.loads((ROOT / stub.INDEX).read_bytes())
        self.assertEqual(index["source"].get(stub.SOURCE), stub.proof(16))
        stub.validate_overlay({}, ROOT, index)

    def test_jimmy_cell_is_passage_floor(self):
        self.assertEqual(stub.layout()[32857, 31383], [408])
        self.assertEqual(stub.layout()[32857, 31385], [440])

    def test_overlay_is_current(self):
        outputs = {}
        stub.overlay_outputs(outputs, ROOT)
        self.assertEqual(outputs, {})


class Synthetic(unittest.TestCase):
    def build(self, closing=stub.CLOSING):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        base = Path(temporary.name)
        palette = [
            {
                "key": f"oteryn:item.tibia.i{i}",
                "provisional": False,
                "source_item_id": i,
            }
            for i in IDS
        ]
        at = {i: n for n, i in enumerate(IDS)}
        tiles = []
        for y, row in [(31380, stub.PASSAGE), (31381, closing)]:
            tiles += [
                (x, y, 0, 0, (), [(at[i], 0, None)]) for x, i in zip(stub.XS, row)
            ]
        sx, sy = stub.XS[0] // 32, 31381 // 32
        raw = codec.encode_region(
            6,
            128,
            122,
            {(sy % 8) * 8 + sx % 8: codec.encode_sector_tiles(sx, sy, tiles)},
        )
        totals = {"regions": 1, "sectors": 1, "tiles": 8, "items": 8}
        index = {
            "source": {},
            "regions": [
                {
                    "path": stub.REGION,
                    "sha256": stub.digest(raw),
                    "tiles": 8,
                    "items": 8,
                }
            ],
            "palette": palette,
            "totals": totals,
        }
        summary = {
            "source": {},
            "totals": totals.copy(),
            "bytes_on_disk": len(raw),
            "tiles_by_floor": {"6": 8},
            "palette": {"families": {"item": {"entries": 6, "occurrences": 8}}},
        }
        for path, data in [
            (stub.INDEX, stub.canonical(index)),
            (stub.SUMMARY, stub.canonical(summary)),
            (stub.REGION, raw),
        ]:
            target = base / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
        return base

    def test_opens_the_wall_and_adds_sixteen_tiles(self):
        base = self.build()
        outputs = {}
        stub.overlay_outputs(outputs, base)
        index = json.loads(outputs[stub.INDEX])
        summary = json.loads(outputs[stub.SUMMARY])
        self.assertEqual(index["totals"]["tiles"], 24)
        self.assertEqual(index["regions"][0]["items"], 24)
        self.assertEqual(summary["tiles_by_floor"]["6"], 24)
        self.assertEqual(summary["palette"]["families"]["item"]["occurrences"], 24)
        self.assertEqual(stub.donor_delta(summary), {"tiles": 16, "items": 16})
        for path, data in outputs.items():
            (base / path).write_bytes(data)
        again = {}
        stub.overlay_outputs(again, base)
        self.assertEqual(again, {})

    def test_donor_delta_refuses_incorrect_items_added(self):
        summary = {"source": {stub.SOURCE: stub.proof(17)}}
        with self.assertRaisesRegex(ValueError, "QH_JIMMY_STUB_EVIDENCE_DRIFT"):
            stub.donor_delta(summary)

    def test_refuses_a_drifted_source_wall(self):
        base = self.build(closing=(450, 408, 440, 449))
        with self.assertRaisesRegex(ValueError, "QH_JIMMY_STUB_SOURCE_WALL_DRIFT"):
            stub.overlay_outputs({}, base)


if __name__ == "__main__":
    unittest.main()
