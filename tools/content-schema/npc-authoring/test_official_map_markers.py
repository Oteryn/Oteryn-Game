"""Binary provenance and authoring authority boundary checks for official map observations."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import official_map_markers as m


class OfficialMapMarkerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = m.SOURCE.read_bytes()
        cls.manifest = m.MANIFEST.read_bytes()
        cls.targets = m.canonical([{'key': 'oteryn:npc.a_bloodshade', 'name': 'A Bloodshade'},
                                  {'key': 'oteryn:npc.s_zallar_m_andar', 'name': "S'Zallar M'Andar"},
                                  {'key': 'oteryn:npc.a_skull', 'name': 'A Skull'},
                                  {'key': 'oteryn:npc.death_npc', 'name': 'Death (NPC)'}])

    def compile(self, **changes):
        args = {'data': self.data, 'target_data': self.targets, 'manifest_bytes': self.manifest}
        args.update(changes)
        return m.compile_packet(**args)

    def test_deterministic_compilation(self):
        self.assertEqual(m.canonical(self.compile()), m.canonical(self.compile()))

    def test_full_field_and_child_byte_proofs(self):
        packet = self.compile()
        for observation in packet['observations']:
            for p in [observation['proof'], *observation['field_proofs'].values(),
                      observation['field3_same_integer_area_observation']['proof']]:
                self.assertLess(p['byte_start'], p['byte_end'])
                fragment = self.data[p['byte_start']:p['byte_end']]
                self.assertEqual(hashlib.sha256(fragment).hexdigest(), p['fragment_sha256'])

    def test_preserves_literal_names_and_case_only_match(self):
        skulls = [o for o in self.compile()['observations'] if o['target_proposal_key'] == 'oteryn:npc.a_skull']
        self.assertEqual(2, len(skulls))
        self.assertTrue(all(o['source_name'] == 'a Skull' and o['name_match'] == 'LITERAL_CASE_ONLY' for o in skulls))

    def test_never_removes_parenthetical_title_without_source(self):
        keys = {o['target_proposal_key'] for o in self.compile()['observations']}
        self.assertNotIn('oteryn:npc.death_npc', keys)

    def test_qualified_name_requires_exact_npc_image_bytes(self):
        with tempfile.TemporaryDirectory() as td:
            p = Path(td) / 'capture.html'
            fragment = b'<img class="npc_name" alt="Death" title="Death" />'
            p.write_bytes(b'prefix' + fragment + b'suffix')
            qualification = {'schema': 'OTERYN_NPC_LITERAL_SOURCE_NAME_QUALIFICATIONS/v1', 'records': [
                {'target_proposal_key': 'oteryn:npc.death_npc', 'source_name': 'Death',
                 'proof': {'path': str(p), 'sha256': m.digest(p.read_bytes()), 'byte_start': 6,
                           'byte_end': 6 + len(fragment), 'fragment_sha256': m.digest(fragment)}}]}
            packet = self.compile(qualified_data=m.canonical(qualification))
            death = [o for o in packet['observations'] if o['target_proposal_key'] == 'oteryn:npc.death_npc']
            self.assertEqual(1, len(death))
            self.assertEqual('QUALIFIED_LITERAL_SOURCE_NAME', death[0]['name_match'])
            qualification['records'][0]['source_name'] = 'Skullfrost'
            with self.assertRaises(m.MarkerError):
                self.compile(qualified_data=m.canonical(qualification))

    def test_manifest_custody_rejects_changed_binary(self):
        with self.assertRaises(m.MarkerError):
            self.compile(data=self.data[:-1])

    def test_self_consistent_replacement_manifest_is_not_the_official_pin(self):
        manifest = json.loads(self.manifest)
        manifest['archive_sha256'] = '0' * 64
        with self.assertRaises(m.MarkerError):
            self.compile(manifest_bytes=m.canonical(manifest))

    def test_truncation_and_wrong_marker_shape_fail_closed(self):
        with self.assertRaises(m.MarkerError):
            list(m.wire_fields(b'\x12\x09\x01'))
        with self.assertRaises(m.MarkerError):
            m.exact_message(b'\x08\x01', [(1, 2), (2, 2), (3, 0)])

    def test_duplicate_proposals_fail_closed(self):
        targets = json.loads(self.targets)
        targets.append(copy.deepcopy(targets[0]))
        with self.assertRaises(m.MarkerError):
            self.compile(target_data=m.canonical(targets))

    def test_validator_rejects_position_and_authority_tampering(self):
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / 'targets.json'
            path.write_bytes(self.targets)
            packet = m.build(target_path=path)
            self.assertTrue(m.validate(packet, target_path=path))
            changed = copy.deepcopy(packet)
            changed['observations'][0]['position']['x'] += 1
            with self.assertRaises(m.MarkerError):
                m.validate(changed, target_path=path)
            changed = copy.deepcopy(packet)
            changed['authority']['placement_admission'] = True
            with self.assertRaises(m.MarkerError):
                m.validate(changed, target_path=path)

    def test_variant_and_authority_boundaries_preserved(self):
        packet = self.compile()
        bloodshades = [o for o in packet['observations'] if o['source_name'] == 'A Bloodshade']
        self.assertEqual(2, len(bloodshades))
        self.assertTrue(all(o['identity_disposition'] == 'EXISTING_NATIVE_VARIANTS_ONLY' for o in bloodshades))
        self.assertTrue(packet['authority']['evidence_only'])
        self.assertFalse(packet['authority']['canonical_identity_allocation'])
        self.assertFalse(packet['authority']['placement_admission'])
        self.assertFalse(packet['authority']['runtime_qualified'])
        self.assertIn('movement_can_walk', packet['unknown_facts'])
        self.assertTrue(packet['summary']['all_marker_field3_codes_have_same_integer_area'])
        self.assertEqual('opaque_varint_with_same_integer_area_join', packet['source_schema']['marker_field3'])
        self.assertFalse(packet['source_schema']['appearance_or_movement_fields'])


if __name__ == '__main__':
    unittest.main()
