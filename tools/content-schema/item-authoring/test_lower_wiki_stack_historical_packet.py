"""Current imported observations and genuine selected historical observations stay separate."""

import copy
import json
import unittest
from collections import defaultdict

import lower_wiki_stack_historical_packet as lower
from check_stack_default_historical_context import qualification_context


class HistoricalFrameTests(unittest.TestCase):
    def setUp(self):
        frame = json.loads((lower.ROOT / lower.PROOF).read_text())["frames"][0]
        current = frame["current"]
        identity = {
            "family": "Item",
            "key": current["item_key"],
            "revision": "definition-r1",
        }
        records = json.loads((lower.ROOT / lower.base.WIKI).read_text())["records"]
        observations = next(
            r["observations"] for r in records.values() if r["item_id"] == 3337
        )
        self.args = {
            "frame": frame,
            "definition": {"identity": identity, "stack_class": "Unknown"},
            "binding": {"external_id": "3337", "target": identity},
            "obj": {"id": 3337, "name": "bone club", "flags": {"flags.take": True}},
            "routed": set(),
            "current_observations": observations,
            "pages": defaultdict(set, {current["page_id"]: {3337}}),
            "cutoff": "2026-09-27T23:59:59Z",
        }

    def qualify(self, **updates):
        return lower.qualify(**(self.args | updates))

    def test_real_frame_and_packet_do_not_replace_current_imported_observations(self):
        before = copy.deepcopy(self.args["current_observations"])
        row = self.qualify()
        self.assertNotEqual(
            row["historical_observation"]["revision_id"], before[0]["revision_id"]
        )
        self.assertEqual(self.args["current_observations"], before)
        data = (lower.ROOT / lower.base.WIKI).read_bytes()
        with qualification_context() as historical_root:
            self.assertEqual(
                lower.build(historical_root)["counts"], {"promotions": 7, "holds": 0}
            )
        self.assertEqual((lower.ROOT / lower.base.WIKI).read_bytes(), data)

    def test_actual_current_snapshot_coordinates_cannot_be_substituted(self):
        for field in (*lower.COORDS, "wiki_title"):
            observations = copy.deepcopy(self.args["current_observations"])
            observations[0][field] = "substitution"
            with self.assertRaises(ValueError):
                self.qualify(current_observations=observations)

    def test_current_binding_and_known_name_guards_remain_mandatory(self):
        for binding in (
            None,
            self.args["binding"] | {"external_id": "2"},
            self.args["binding"]
            | {"target": self.args["definition"]["identity"] | {"revision": "other"}},
        ):
            with self.assertRaises(ValueError):
                self.qualify(binding=binding)
        presentation = {
            "state": "KNOWN",
            "value": {"name": {"state": "KNOWN", "value": "different name"}},
        }
        for definition in (
            self.args["definition"] | {"stack_class": "StackCapable"},
            self.args["definition"] | {"semantics": {"presentation": presentation}},
        ):
            with self.assertRaises(ValueError):
                self.qualify(definition=definition)

    def test_historical_cutoff_raw_digest_identity_and_argument_guards(self):
        for mutation in (
            {"revision_timestamp": "2026-09-28T00:00:00Z"},
            {"content_sha256": "0" * 64},
            {"page_id": 2},
        ):
            frame = copy.deepcopy(self.args["frame"])
            frame["historical"].update(mutation)
            with self.assertRaises(ValueError):
                self.qualify(frame=frame)
        for argument in ("|stackable=", "|stackable=yes", "|itemid=2"):
            frame = copy.deepcopy(self.args["frame"])
            raw = (
                "{{Infobox Object|name=Bone Club|actualname=bone club|itemid=3337"
                + argument
                + "}}"
            )
            frame["historical"].update(
                content=raw, content_sha256=lower.base.sha(raw.encode())
            )
            with self.assertRaises(ValueError):
                self.qualify(frame=frame)

    def test_native_blocks_world_owner_and_idempotence(self):
        for state in ("CONFLICT", "NOT_APPLICABLE"):
            definition = self.args["definition"] | {
                "semantics": {"stack": {"state": state}}
            }
            with self.assertRaises(ValueError):
                self.qualify(definition=definition)
        with self.assertRaises(ValueError):
            self.qualify(routed={self.args["frame"]["current"]["item_key"]})
        semantics = {
            "stack": {
                "state": "KNOWN",
                "value": {"stackable": {"state": "KNOWN", "value": False}},
            }
        }
        self.assertFalse(
            self.qualify(definition=self.args["definition"] | {"semantics": semantics})[
                "stackable"
            ]
        )


if __name__ == "__main__":
    unittest.main()
