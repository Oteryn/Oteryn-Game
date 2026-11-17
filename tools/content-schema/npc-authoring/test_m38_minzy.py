import json
import unittest
import copy
import tempfile
from pathlib import Path
import native_contacts
import convert_placements as cp


class MinzyContactTest(unittest.TestCase):
    def test_retained_br_contact_is_published_with_its_actual_provenance(self):
        files = cp.convert()
        contacts = [r['declaration'] for name, text in files.items()
                    if name.startswith('placements-')
                    for r in json.loads(text)['records']
                    if r['declaration']['npc'] == 'oteryn:npc.minzy']
        self.assertEqual(len(contacts), 1)
        self.assertEqual(contacts[0]['cell'], {'x': 32743, 'y': 31971, 'floor': 12})
        proof = contacts[0]['provenance'][0]
        self.assertEqual(proof['source_sha256'],
                         '792ec5bbc93991f54d94d97ca9149f6ae8df781f2bb93eb7ccc77949bba6df62')
        self.assertEqual(proof['revid'], 408078)
        self.assertEqual(proof['repository'], 'www.tibiawiki.com.br')
        self.assertIn('not retail placement equivalence', proof['coordinate_note'])

    def test_native_import_requires_an_explicit_key_and_source_proof(self):
        record = next(r for r in json.loads((cp.NPCS / 'npcs-00500-00999.json').read_text())['records']
                      if r['declaration']['identity']['key'] == 'oteryn:npc.minzy')
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            target = root / 'npcs-fixture.json'
            target.write_text(json.dumps({'records': [record]}))
            self.assertEqual(native_contacts.candidates(root, []), {})
            self.assertEqual(set(native_contacts.candidates(root, ['oteryn:npc.minzy'])),
                             {'oteryn:npc.minzy'})
            for group, field, value in [('facts', 'br_coordinate_frame', 'unknown'),
                                        ('facts', 'br_coordinates', [[True, 31971, 12]]),
                                        ('source', 'source_sha256', 'not-a-sha'),
                                        ('source', 'revision', True)]:
                changed = copy.deepcopy(record)
                entry = next(f for f in changed['declaration']['fields']
                             if f['field_path'].endswith('.bulk.source_metadata'))
                metadata = json.loads(entry['value']['value'])
                selected = (metadata['wiki_facts'] if group == 'facts' else
                            next(s for s in metadata['sources'] if s['source'] == 'TibiaWiki BR'))
                selected[field] = value
                entry['value']['value'] = json.dumps(metadata)
                target.write_text(json.dumps({'records': [changed]}))
                with self.assertRaises(ValueError):
                    native_contacts.candidates(root, ['oteryn:npc.minzy'])
            target.write_text(json.dumps({'records': [record]}))
            with self.assertRaisesRegex(ValueError, 'missing native contacts'):
                native_contacts.candidates(root, ['oteryn:npc.not_present'])
