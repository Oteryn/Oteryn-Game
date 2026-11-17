"""L2 startup selectors stay pinned to their donor and canonical claim."""

import hashlib
import json
import sys
import tarfile
import unittest

import quest_access_bindings as access
import questl2_chest_bindings as lane


class L2ChestBindings(unittest.TestCase):
    def test_brooch_selector_matches_donor_and_canonical_claim(self):
        sys.path.insert(0, str(access.ROOT / "tools/content-schema/quest-authoring"))
        import lua_tables

        with tarfile.open(
            access.ROOT
            / "tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz"
        ) as corpus:
            manifest = json.load(corpus.extractfile("corpus-manifest.json"))
            row = next(
                r
                for r in manifest["files"]
                if r["source"] == "crystalserver"
                and r["revision"] == "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
                and r["path"] == "data-global/startup/tables/chest.lua"
            )
            raw = corpus.extractfile(row.get("blob_path", row.get("cache_path"))).read()
        self.assertTrue(lane.SOURCE.endswith(hashlib.sha256(raw).hexdigest()))
        entry = next(
            r
            for r in lua_tables.assignments(raw.decode(), {"ChestUnique"})[
                "ChestUnique"
            ]["fields"]
            if r["key"] == 6055
        )
        source = lua_tables.as_python(entry["value"])
        self.assertEqual(entry["line"], 992)
        self.assertEqual(source["reward"], [[3205, 1]])
        self.assertEqual(
            lane.BINDINGS,
            [
                {
                    "position": [source["itemPos"][k] for k in "xyz"],
                    "item": source["itemId"],
                    "attribute": "unique",
                    "value": entry["key"],
                }
            ],
        )
        claims = json.loads(
            (access.ROOT / "content/interactions/reward_claims/index.json").read_bytes()
        )
        definitions = [
            r["definition"]
            for shard in claims["shards"]
            for r in json.loads((access.ROOT / shard).read_bytes())["records"]
        ]
        claim = next(
            d
            for d in definitions
            if d["identity"]["key"]
            == "oteryn:reward-claim.quest.u7_24.family_brooch.brooch"
        )
        self.assertEqual(
            claim["placements"][0]["appearance_tibia_id"], source["itemId"]
        )
        self.assertEqual(
            claim["placements"][0]["source_binding"]["project_position"],
            source["itemPos"],
        )

    def test_actual_overlay_is_idempotent(self):
        paths = [access.INDEX, access.SUMMARY, lane.REGION]
        before = {p: (access.ROOT / p).read_bytes() for p in paths}
        after = dict(before)
        lane.overlay_outputs(after)
        self.assertEqual(after, before)
        index = json.loads(after[access.INDEX])
        _, imported = access.overlay_region(
            after[lane.REGION], index["palette"], lane.BINDINGS
        )
        self.assertEqual(imported, {(32248, 31866, 8): "unique"})

    def test_unrelated_synthetic_world_is_untouched(self):
        outputs = {"fixture": b"unchanged"}
        lane.overlay_outputs(outputs)
        self.assertEqual(outputs, {"fixture": b"unchanged"})
