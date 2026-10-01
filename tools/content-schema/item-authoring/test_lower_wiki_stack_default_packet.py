"""Guards for a documented source default, distinct from explicit raw No."""

import copy
import unittest
from collections import defaultdict

import lower_wiki_stack_default_packet as lower


class QualificationTests(unittest.TestCase):
    def setUp(self):
        self.source = {
            "source_item_id": 1,
            "item_key": "item",
            "content": "{{Infobox Object|itemid=1|name=Ring}}",
            "revision_timestamp": "2026-08-01T00:00:00Z",
            "page_id": 10,
            "revision_id": 11,
            "content_sha256": "digest",
            "title": "Ring",
        }
        self.definition = {
            "stack_class": "Unknown",
            "identity": {"family": "Item", "key": "item", "revision": "definition-r1"},
        }
        self.binding = {"external_id": "1", "target": self.definition["identity"]}
        self.obj = {"name": "ring", "flags": {"flags.take": True}}

    def observation(self):
        return {
            k: self.source[k]
            for k in ("page_id", "revision_id", "content_sha256", "revision_timestamp")
        } | {"wiki_title": self.source["title"], "fields": {}}

    def reasons(self, **changes):
        args = {
            "source": self.source,
            "definition": self.definition,
            "binding": self.binding,
            "obj": self.obj,
            "routed": set(),
            "observations": [self.observation()],
            "page_ids": defaultdict(set, {10: {1}}),
            "cutoff": "2026-09-27T23:59:59Z",
        }
        return lower.reasons(**(args | changes))

    def test_absent_differs_from_blank_explicit_and_duplicate(self):
        self.assertEqual(self.reasons(), [])
        for argument in ("|stackable=", "|stackable=no", "|stackable=yes|stackable=no"):
            source = self.source | {
                "content": self.source["content"][:-2] + argument + "}}"
            }
            self.assertIn("STACKABLE_ARGUMENT_PRESENT", self.reasons(source=source))
        self.assertIn(
            "PRESENT_RETAINED_STACKABLE_ARGUMENT",
            self.reasons(
                observations=[{"page_id": 10, "fields": {"stackable": "yes"}}]
            ),
        )

    def test_nested_notes_are_not_arguments_and_comment_is_rejected(self):
        content = "{{Infobox Object|itemid=1|name=Ring|notes={{Other|stackable=yes}}}}"
        self.assertNotIn("stackable", lower.raw_parameters(content))
        for content in (
            "<!--" + self.source["content"] + "-->",
            "{{Infobox Object/Draft|name=Ring}}",
            "{{Infobox Object|itemid=1",
            "{{Infobox Object|itemid=1|stackable<!--x-->=yes}}",
            "{{Infobox Object|itemid=1|{{Dynamic}}=yes}}",
            "{{Infobox Object|itemid=1|stackable&#32;=yes}}",
        ):
            with self.assertRaises(ValueError):
                lower.raw_parameters(content)

    def test_positive_domain_temporal_identity_and_shared_page_guards(self):
        self.assertIn(
            "DOMAIN_HOLD_NO_AFFIRMATIVE_TAKE",
            self.reasons(obj={"name": "ring", "flags": {}}),
        )
        for flag in (
            "flags.clip",
            "flags.corpse",
            "flags.player_corpse",
            "flags.liquidpool",
            "flags.bank",
        ):
            obj = self.obj | {"flags": self.obj["flags"] | {flag: True}}
            self.assertIn("AFFIRMATIVE_WORLD_DOMAIN_HOLD", self.reasons(obj=obj))
        for changes, reason in (
            (
                {
                    "source": self.source
                    | {"revision_timestamp": "2026-09-28T00:00:00Z"}
                },
                "REVISION_AFTER_QUALIFICATION_CUTOFF",
            ),
            (
                {"binding": self.binding | {"external_id": "2"}},
                "NO_EXACT_SOURCE_BINDING",
            ),
            ({"routed": {"item"}}, "EXISTING_WORLD_OWNER"),
            ({"page_ids": {10: {1, 2}}}, "SHARED_OR_MISSING_RETAINED_PAGE"),
            ({"obj": self.obj | {"name": "ring's"}}, "EXACT_OFFICIAL_NAME_REQUIRED"),
        ):
            self.assertIn(reason, self.reasons(**changes))

    def test_known_false_idempotence_and_all_blocked_states(self):
        for state in ("CONFLICT", "NOT_APPLICABLE"):
            group = {"semantics": {"stack": {"state": state}}}
            leaf = {
                "semantics": {
                    "stack": {
                        "state": "KNOWN",
                        "value": {"stackable": {"state": state}},
                    }
                }
            }
            for case in (group, leaf):
                self.assertIn(
                    "BLOCKED_STACK_EVIDENCE",
                    self.reasons(definition=self.definition | case),
                )
        for value in (False, True):
            d = copy.deepcopy(self.definition)
            d["semantics"] = {
                "stack": {
                    "state": "KNOWN",
                    "value": {
                        "stackable": {"state": "KNOWN", "value": value},
                        "stack_max": {"state": "KNOWN", "value": 7},
                    },
                }
            }
            self.assertEqual(
                "KNOWN_STACKABLE_CONFLICT" in self.reasons(definition=d), value
            )

    def test_snapshot_coordinates_and_reverse_binding_are_not_substitutable(self):
        for field in (
            "page_id",
            "revision_id",
            "content_sha256",
            "revision_timestamp",
            "wiki_title",
        ):
            observation = self.observation() | {field: "substituted"}
            self.assertIn(
                "PINNED_RAW_SNAPSHOT_COORDINATE_DRIFT",
                self.reasons(observations=[observation]),
            )
        self.assertIn(
            "PINNED_RAW_SNAPSHOT_COORDINATE_DRIFT",
            self.reasons(observations=[self.observation(), self.observation()]),
        )
        binding = {
            "disposition": "EXACT",
            "external_id": "1",
            "source_key": "oteryn:source.crystalserver",
            "source_revision": "rev",
            "identity_namespace": "ots/item_server_id",
            "target": {"family": "Item", "revision": "definition-r1", "key": "first"},
        }
        self.assertEqual(len(lower.exact_bindings([binding], ["rev"])), 1)
        with self.assertRaises(ValueError):
            lower.exact_bindings(
                [binding, binding | {"target": binding["target"] | {"key": "second"}}],
                ["rev"],
            )
        source = self.source | {
            "content": "{{Infobox Object|itemid=1|name=Ring|actualname=Other}}"
        }
        self.assertIn("EXACT_OFFICIAL_NAME_REQUIRED", self.reasons(source=source))

    def test_native_identity_and_presentation_proof_must_remain_qualified(self):
        definition = self.definition | {
            "identity": self.definition["identity"] | {"revision": "definition-r2"}
        }
        self.assertIn(
            "BINDING_TARGET_NATIVE_IDENTITY_DRIFT", self.reasons(definition=definition)
        )
        for state in ("CONFLICT", "NOT_APPLICABLE"):
            for presentation in (
                {"state": state},
                {"state": "KNOWN", "value": {"name": {"state": state}}},
            ):
                definition = self.definition | {
                    "semantics": {"presentation": presentation}
                }
                self.assertIn(
                    "BLOCKED_PRESENTATION_EVIDENCE", self.reasons(definition=definition)
                )
        for name in ("ring", "other"):
            definition = self.definition | {
                "semantics": {
                    "presentation": {
                        "state": "KNOWN",
                        "value": {"name": {"state": "KNOWN", "value": name}},
                    }
                }
            }
            self.assertEqual(
                "KNOWN_PRESENTATION_NAME_CONFLICT"
                in self.reasons(definition=definition),
                name != "ring",
            )

    def test_closed_source_packet_and_digest_substitution_guard(self):
        packet = lower.build()
        self.assertEqual(packet["counts"], {"promotions": 1487, "holds": 164})
        self.assertEqual(packet["source_policy"], "DERIVED_DOCUMENTED_TEMPLATE_DEFAULT")
        self.assertTrue(all(r["stackable"] is False for r in packet["promotions"]))
        with self.assertRaises(ValueError):
            lower.checked(lower.ROOT, lower.PROOF, "0" * 64)


if __name__ == "__main__":
    unittest.main()
