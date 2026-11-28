"""A Crystal spawn has one final contact after placement conversion."""

import json
import unittest
from collections import defaultdict

import convert_placements as cp


class CrystalSpawnIdentityTest(unittest.TestCase):
    def test_crystal_spawn_has_one_contact(self):
        contacts = defaultdict(set)
        for name, text in cp.convert().items():
            if not name.startswith("placements-"):
                continue
            for record in json.loads(text)["records"]:
                row = record["declaration"]
                cell = tuple(row["cell"][axis] for axis in ("x", "y", "floor"))
                source_cell = next(
                    (
                        tuple(
                            source["source_cell"][axis] for axis in ("x", "y", "floor")
                        )
                        for source in row["provenance"]
                        if "source_cell" in source
                    ),
                    cell,
                )
                for source in row["provenance"]:
                    if (
                        source.get("source_row", "").split(":")[0]
                        == "data-global/world/world-npc.xml"
                    ):
                        contacts[(row["npc"], source["revision"], source_cell)].add(
                            cell
                        )
        self.assertEqual(
            {key: cells for key, cells in contacts.items() if len(cells) > 1}, {}
        )
