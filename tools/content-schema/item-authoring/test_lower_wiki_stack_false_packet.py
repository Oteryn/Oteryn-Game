"""Source disagreement/unknown handling and pinned stack-false packet reproducibility."""

import json
import unittest

import lower_wiki_stack_false_packet as lower


def observation(value):
    return {
        "fields": {} if value is None else {"stackable": value},
        "page_id": 1,
        "revision_id": 2,
        "content_sha256": "retained",
        "url": "https://example.test/item",
    }


class StackFalseQualification(unittest.TestCase):
    def qualify(
        self,
        values=("no",),
        *,
        flag=False,
        bound=True,
        routed=False,
        state="UNKNOWN",
        old=None,
    ):
        key = "oteryn:item.tibia.i1"
        stack = {"state": state}
        if old is not None:
            stack = {
                "state": "KNOWN",
                "value": {"stackable": {"state": "KNOWN", "value": old}},
            }
        return lower.qualify(
            {
                "records": {
                    key: {
                        "item_id": 1,
                        "observations": [observation(v) for v in values],
                    }
                }
            },
            {key: {"stack_class": "Unknown", "semantics": {"stack": stack}}},
            {key} if bound else set(),
            {key} if routed else set(),
            {1: {"flags": {"flags.cumulative": flag}}},
        )

    def test_explicit_negative_and_absence_are_distinct_and_idempotent(self):
        self.assertEqual(self.qualify(), self.qualify(old=False))
        rows, holds = self.qualify(("No", None))
        self.assertEqual((rows[0]["stackable"], holds), (False, []))
        self.assertEqual(self.qualify((None,)), ([], []))
        self.assertEqual(self.qualify(("yes",)), ([], []))

    def test_no_majority_conflict_binding_map_owner_and_blocked_states(self):
        for args, reason in [
            ({"values": ("no", "no", "yes")}, "WIKI_PAGE_DISAGREEMENT"),
            ({"values": ("no", "?")}, "MALFORMED_WIKI_VALUE"),
            ({"flag": True}, "CLIENT_CUMULATIVE_TRUE"),
            ({"bound": False}, "NO_EXACT_SOURCE_BINDING"),
            ({"routed": True}, "EXISTING_MAP_OWNER"),
            ({"state": "CONFLICT"}, "BLOCKED_EVIDENCE_STATE"),
            ({"old": True}, "KNOWN_STACK_CONFLICT"),
        ]:
            rows, holds = self.qualify(**args)
            self.assertEqual((rows, holds[0]["reason"]), ([], reason))

    def test_missing_semantics_scope_and_full_snapshot_variant_guard(self):
        key = "oteryn:item.tibia.i49217"
        snapshot = {
            "records": {key: {"item_id": 49217, "observations": [observation("no")]}}
        }
        definitions = {key: {"stack_class": "Unknown"}}
        args = (snapshot, definitions, {key}, set(), {})
        rows, holds = lower.qualify(*args)
        self.assertEqual((len(rows), holds), (1, []))
        definitions[key] = {"stack_class": "StackCapable"}
        self.assertEqual(lower.qualify(*args)[1][0]["reason"], "KNOWN_STACK_CONFLICT")
        definitions[key] = {"stack_class": "Unknown"}
        snapshot["records"]["unbound"] = {
            "item_id": 9,
            "observations": [observation(None)],
        }
        self.assertEqual(
            lower.qualify(*args)[1][0]["reason"], "SHARED_PAGE_VARIANT_UNQUALIFIED"
        )
        snapshot["records"] = {
            "outside": {"item_id": 2, "observations": [observation("no")]}
        }
        self.assertEqual(
            lower.qualify(
                snapshot,
                {"oteryn:item.tibia.i2": {"stack_class": "Unknown"}},
                {"oteryn:item.tibia.i2"},
                set(),
                {},
            ),
            ([], []),
        )

    def test_packet_reproduces(self):
        packet = lower.build()
        self.assertEqual(packet["counts"], {"promotions": 2392, "holds": 13})
        self.assertEqual(
            (
                json.dumps(
                    packet, sort_keys=True, ensure_ascii=False, separators=(",", ":")
                )
                + "\n"
            ).encode(),
            lower.OUTPUT.read_bytes(),
        )


if __name__ == "__main__":
    unittest.main()
