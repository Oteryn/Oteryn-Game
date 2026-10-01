"""No-network qualification and reproducibility checks for bounded capacity repair."""

import unittest
from unittest.mock import patch

import lower_wiki_capacity_packet as lower


def observation(volume):
    return {
        "fields": {"volume": volume},
        "page_id": 1,
        "revision_id": 2,
        "content_sha256": "retained",
        "url": "https://example.test/item",
    }


class Qualification(unittest.TestCase):
    def qualify(self, second=None, old=None, *, bound=True, routed=False, flag=True):
        definition = {"semantics": {"container": {"state": "UNKNOWN"}}}
        if old is not None:
            definition["semantics"]["container"] = {
                "state": "KNOWN",
                "value": {"capacity": {"state": "KNOWN", "value": old}},
            }
        records = {"item": {"item_id": 1, "observations": [observation("8")]}}
        definitions = {"oteryn:item.tibia.i1": definition}
        if second is not None:
            records["second"] = {"item_id": 2, "observations": second}
            definitions["oteryn:item.tibia.i2"] = {
                "semantics": {
                    "container": {
                        "state": "KNOWN",
                        "value": {"capacity": {"state": "KNOWN", "value": 6}},
                    }
                }
            }
        with (
            patch.object(lower, "SCOPE", frozenset({1})),
            patch.object(lower, "CORRECTIONS", {}),
        ):
            return lower.qualify(
                {"records": records},
                definitions,
                set(definitions) if bound else set(),
                {"oteryn:item.tibia.i1"} if routed else set(),
                {1: {"flags": {"flags.container": flag}}},
            )

    def test_unknown_repair_stays_identical_after_materialization(self):
        self.assertEqual(self.qualify(), self.qualify(old=8))
        rows, _ = self.qualify()
        self.assertEqual(rows[0]["capacity"], 8)
        self.assertEqual(rows[0]["sources"][0]["revision_id"], 2)

    def test_conflicting_pages_and_known_values_are_held(self):
        _, holds = self.qualify([observation("8"), observation("6")])
        self.assertEqual(holds[0]["reason"], "WIKI_PAGE_DISAGREEMENT")
        _, holds = self.qualify([observation("8")])
        self.assertEqual(holds[0]["reason"], "KNOWN_CAPACITY_CONFLICT")
        _, holds = self.qualify([observation("65536")])
        self.assertEqual(holds[0]["reason"], "MALFORMED_WIKI_VOLUME")

    def test_binding_map_owner_client_flag_and_known_drift_fail_closed(self):
        for args in [{"bound": False}, {"routed": True}, {"flag": False}, {"old": 6}]:
            with self.subTest(args=args), self.assertRaises(ValueError):
                self.qualify(**args)

    def test_pinned_packet_reproduces(self):
        import json

        packet = lower.build()
        data = (
            json.dumps(
                packet, sort_keys=True, ensure_ascii=False, separators=(",", ":")
            )
            + "\n"
        ).encode()
        self.assertEqual(data, lower.OUTPUT.read_bytes())
        self.assertEqual(packet["counts"], {"promotions": 18, "holds": 33})


if __name__ == "__main__":
    unittest.main()
