"""Reject plausible numeric bindings when independent named identity fails."""
from copy import deepcopy
import unittest

from binding_evidence import bind


class BindingEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.client = {"id": 53763, "name": "basic punch scroll", "record_sha256": "a" * 64}
        self.key = "oteryn:item.tibia.i53763"
        self.definition = {
            "definition": {"identity": {"family": "Item", "key": self.key, "revision": "definition-r1"}},
            "path": "content/items/definitions/example.json", "definition_sha256": "b" * 64,
        }
        self.facts = {self.key: {"item_id": 53763, "observations": [{
            "wiki_title": "Basic Punch Scroll", "url": "https://tibia.fandom.com/wiki/Basic_Punch_Scroll",
            "page_id": 110798, "revision_id": 1195588, "revision_sha1": "c" * 40,
            "revision_timestamp": "2026-07-13T14:38:44Z", "content_sha256": "d" * 64,
        }]}}

    def checked(self, client=None, definition=None, facts=None):
        return bind("Basic Punch Scroll", client or self.client,
                    {self.key: definition or self.definition}, self.facts if facts is None else facts)

    def test_unknown_canonical_name_uses_revisioned_wiki_identity(self):
        result = self.checked()
        self.assertIsNone(result["evidence"]["canonical"]["name"])
        self.assertEqual(result["evidence"]["identity_basis"], "CLIENT_AND_WIKI")

    def test_numeric_id_and_client_name_alone_do_not_qualify_unknown_definition(self):
        with self.assertRaisesRegex(ValueError, "numeric coincidence"):
            self.checked(facts={})

    def test_conflicting_client_name_rejected_despite_correct_numeric_id(self):
        with self.assertRaisesRegex(ValueError, "conflicting client name"):
            self.checked(client={**self.client, "name": "basic slash scroll"})

    def test_conflicting_canonical_name_rejected_despite_matching_wiki(self):
        definition = deepcopy(self.definition)
        definition["definition"]["semantics"] = {
            "presentation": {"value": {"name": {"state": "KNOWN", "value": "basic slash scroll"}}}
        }
        with self.assertRaisesRegex(ValueError, "conflicting canonical name"):
            self.checked(definition=definition)

    def test_wiki_key_and_numeric_item_disagreement_rejected(self):
        facts = deepcopy(self.facts)
        facts[self.key]["item_id"] = 53767
        with self.assertRaisesRegex(ValueError, "different item"):
            self.checked(facts=facts)

    def test_missing_canonical_reference_rejected(self):
        with self.assertRaisesRegex(ValueError, "no canonical Item"):
            bind("Basic Punch Scroll", self.client, {}, self.facts)


if __name__ == "__main__":
    unittest.main()
