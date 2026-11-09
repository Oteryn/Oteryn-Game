"""M38 contact admission from the retained page bound to the existing NPC identity."""
import copy
import json
import tempfile
import unittest
from unittest.mock import patch
from pathlib import Path

import convert_placements as cp
import native_contacts
import retained_contact_metadata


class YseldaContactTest(unittest.TestCase):
    @staticmethod
    def record():
        return next(r for r in json.loads((cp.NPCS / 'npcs-01000-01281.json').read_text())['records']
                    if r['declaration']['identity']['key'] == 'oteryn:npc.yselda')

    def test_existing_page_binding_resolves_its_retained_contact_without_embedded_metadata(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'npcs-fixture.json').write_text(json.dumps({'records': [self.record()]}))
            self.assertEqual(native_contacts.candidates(root, []), {})
            rows = native_contacts.candidates(root, ['oteryn:npc.yselda'])
        contact = rows['oteryn:npc.yselda']
        self.assertEqual(contact['placements'][0]['position'], {'x':32619,'y':32488,'z':12})
        proof = contact['native_provenance']
        self.assertEqual(proof['pageid'], 52101)
        self.assertEqual(proof['revid'], 353837)
        self.assertEqual(proof['source_sha256'], 'da53912b608e44dcbfc6839c2f2fd12b5dbda6b3ea5ce70e32712bb4fb7e5eb3')
        self.assertIn('not retail placement equivalence', proof['coordinate_note'])

    def test_wrong_identity_or_unbound_page_cannot_borrow_a_retained_contact(self):
        record = self.record()
        for change in ['foreign_target', 'wrong_page', 'missing_binding', 'different_identity']:
            with self.subTest(change=change), tempfile.TemporaryDirectory() as temporary:
                altered = copy.deepcopy(record)
                binding = next(b for b in altered['source_bindings'] if b['identity_namespace']=='mediawiki/page_id')
                if change == 'foreign_target': binding['target']['key']='oteryn:npc.minzy'
                if change == 'wrong_page': binding['external_id']='12328'
                if change == 'missing_binding': altered['source_bindings'].remove(binding)
                if change == 'different_identity': altered['declaration']['identity']['key']='oteryn:npc.not_yselda'
                root=Path(temporary)
                (root/'npcs-fixture.json').write_text(json.dumps({'records':[altered]}))
                with self.assertRaises(ValueError):
                    native_contacts.candidates(root,[altered['declaration']['identity']['key']])

    def test_yselda_is_published_only_at_the_retained_cell(self):
        rows=[r['declaration'] for name,text in cp.convert().items() if name.startswith('placements-')
              for r in json.loads(text)['records'] if r['declaration']['npc']=='oteryn:npc.yselda']
        self.assertEqual([r['cell'] for r in rows],[{'x':32619,'y':32488,'floor':12}])
        self.assertTrue(rows[0]['non_walkable_contact'])
        self.assertFalse(rows[0].get('block_solid_contact', False))

    def test_retained_capture_keeps_revision_identity_and_coordinate_checks(self):
        page = next(p for p in json.loads(retained_contact_metadata.CAPTURE.read_text())['pages']
                    if p.get('pageid') == 52101)
        changes = [('revid', True), ('revid', 0), ('sha256', 'invalid'),
                   ('role', 'creature'), ('removed', '13.0'), ('name', 'Sharon'), ('title', 'Sharon'),
                   ('positions', [[True, 32488, 12]]), ('positions', [[32619, 32488, -12]]),
                   ('positions', [[65536, 32488, 12]]), ('positions', [])]
        with tempfile.TemporaryDirectory() as temporary:
            capture = Path(temporary) / 'capture.json'
            for field, value in changes:
                with self.subTest(field=field, value=value):
                    altered = dict(page, **{field: value})
                    capture.write_text(json.dumps({'pages': [altered]}))
                    with self.assertRaises(ValueError):
                        retained_contact_metadata.resolve(self.record(), capture)
            capture.write_text(json.dumps({'pages': [page, page]}))
            with self.assertRaisesRegex(ValueError, 'ambiguous'):
                retained_contact_metadata.resolve(self.record(), capture)

    def test_non_walkable_policy_cannot_select_an_unscoped_or_donor_contact(self):
        scope = json.loads(cp.SCOPE.read_text())
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'scope.json'
            for selected in [['oteryn:npc.anna'], ['oteryn:npc.not_present'],
                             ['oteryn:npc.yselda', 'oteryn:npc.yselda']]:
                with self.subTest(selected=selected):
                    path.write_text(json.dumps(dict(scope, non_walkable_contacts=selected)))
                    with patch.object(cp, 'SCOPE', path), self.assertRaises(ValueError):
                        cp.convert()

    def test_malformed_embedded_metadata_does_not_fall_back_to_the_capture(self):
        record = self.record()
        record['declaration']['fields'].append({
            'field_path': 'oteryn:source.npc.bulk.source_metadata',
            'value': {'type': 'Text', 'value': '{malformed'},
        })
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'npcs-fixture.json').write_text(json.dumps({'records': [record]}))
            with self.assertRaises(ValueError):
                native_contacts.candidates(root, ['oteryn:npc.yselda'])
