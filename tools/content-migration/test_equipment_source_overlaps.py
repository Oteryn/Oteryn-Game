"""Source identities prove every subtraction; the runtime union is never consulted."""
import copy
import re
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import equipment_source_overlaps as aliases
import refresh_equipment_profile_pin as pins


class SourceOverlapProofs(unittest.TestCase):
    SOURCE = '''const PROFILE_ADDITIONS: &[(&str, usize)] = &[
    ("UNCHANGED", 123),
    ("QMECH-M49", 19),
    ("QMECH-M50", 9),
    ("M26-ENDGAME", 6),
    ("QREADY1-CRAFT", 6),
    ("ENGNPC-M05", 1),
    ("M22-TAINTED-TALISMAN", 1),
    ("NPCFIX2-M22", 1),
];'''

    @classmethod
    def setUpClass(cls):
        cls.bosses = pins.keys(next(document for name, document, _
            in pins.source_cohorts(pins.ROOT) if name == 'SRX3-BOSS-CORPSES'))

    def test_exact_source_overlaps_remove_34_duplicate_contributions(self):
        result = aliases.refresh(self.SOURCE, pins.ROOT, self.bosses)
        counts = {name: int(value) for name, value in re.findall(r'\("([^"]+)", (\d+)\)', result)}
        self.assertEqual(counts, {'UNCHANGED': 123, 'QMECH-M49': 5, 'QMECH-M50': 0,
            'M26-ENDGAME': 1, 'QREADY1-CRAFT': 3, 'ENGNPC-M05': 0,
            'M22-TAINTED-TALISMAN': 0, 'NPCFIX2-M22': 0})
        self.assertEqual(sum(int(v) for v in re.findall(r', (\d+)\)', self.SOURCE)) - sum(counts.values()), 34)
        self.assertEqual(aliases.refresh(result, pins.ROOT, self.bosses), result)

    def test_craft_overlap_requires_complete_registered_source_identities(self):
        paths = (aliases.engine.PROOF, aliases.floor.PATH, aliases.spheres.PATH,
                 aliases.elevator.PATH, aliases.rafzane.PATH, aliases.craft.PACKET,
                 aliases.npc.PATH, aliases.endgame.PATH)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for path in paths:
                target = root / path
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes((pins.ROOT / path).read_bytes())
            shared = aliases.source_keys(root, aliases.engine.PROOF,
                                         aliases.names(aliases.engine.IDS))
            craft = aliases.source_keys(root, aliases.craft.PACKET,
                                        aliases.names(aliases.craft.IDS))
            self.assertEqual(craft & shared, aliases.names({21506, 21507, 21573}))
            self.assertEqual(aliases.refresh(self.SOURCE, root, self.bosses),
                             aliases.refresh(self.SOURCE, pins.ROOT, self.bosses))
            for path in (aliases.engine.PROOF, aliases.craft.PACKET):
                target = root / path
                original = target.read_bytes()
                for change, replacement in (("missing", None),
                                            ("foreign", "oteryn:item.tibia.i999999"),
                                            ("duplicate", "oteryn:item.tibia.i21506")):
                    with self.subTest(path=path, change=change):
                        packet = json.loads(original)
                        row = next(row for row in packet['records']
                                   if row['production_definition']['production_key']
                                   == 'oteryn:item.tibia.i21573')
                        if replacement is None:
                            packet['records'].remove(row)
                        else:
                            row['production_definition']['production_key'] = replacement
                        changed = json.dumps(packet).encode()
                        target.write_bytes(changed)
                        try:
                            with self.assertRaisesRegex(ValueError, 'SOURCE_IDENTITIES:' + re.escape(path)):
                                aliases.refresh(self.SOURCE, root, self.bosses)
                            self.assertEqual(target.read_bytes(), changed)
                        finally:
                            target.write_bytes(original)

    def test_craft_declaration_requires_full_or_qualified_unique_count(self):
        old = '("QREADY1-CRAFT", 6),'
        for replacement in ('("QREADY1-CRAFT", 4),', '("QREADY1-CRAFT", 5),',
                            old + '\n    ' + old):
            with self.subTest(replacement=replacement):
                with self.assertRaisesRegex(ValueError, 'COHORT_DECLARATION:QREADY1-CRAFT'):
                    aliases.refresh(self.SOURCE.replace(old, replacement), pins.ROOT, self.bosses)
        packet = json.loads((pins.ROOT / aliases.craft.PACKET).read_bytes())
        normalized = pins.qready1_craft_equipment_pin.refresh(
            self.SOURCE.replace(old, '("QREADY1-CRAFT", 4),'), packet)
        self.assertEqual(normalized, self.SOURCE)
        result = aliases.refresh(normalized, pins.ROOT, self.bosses)
        self.assertIn('("QREADY1-CRAFT", 3),', result)


    def test_both_elevator_sources_include_the_same_nine_fixed_identities(self):
        expected = {6218, 17938, 17939, 17940, 17942, 17943, 17944, 17947, 17970}
        self.assertEqual(aliases.elevator.IDS, expected)
        self.assertEqual(aliases.rafzane.IDS, expected)

    def test_merged_subtraction_does_not_repeat_cohort_owned_corrections(self):
        source = self.SOURCE + '''
    ("M07-FIRE-BUG", 1),
    ("M07-LION-TRIALS", 2),
    ("ENGNPC2-SLUG-CORPSES", 4),
'''
        expected = aliases.names({5467, 21389, 21466, 12526, 12533, 12534, 12535})
        self.assertEqual(pins.merged_source_duplicates(pins.ROOT, source), expected)
        with self.assertRaisesRegex(ValueError, 'cohort declaration differs: M07-LION-TRIALS'):
            pins.merged_source_duplicates(pins.ROOT, source.replace('"M07-LION-TRIALS", 2', '"M07-LION-TRIALS", 1'))

    def test_unproven_boss_overlap_cannot_lower_the_cohort(self):
        with self.assertRaisesRegex(ValueError, 'SOURCE_INTERSECTION:M26-ENDGAME'):
            aliases.refresh(self.SOURCE, pins.ROOT, self.bosses - {'oteryn:item.tibia.i33864'})

    def test_arbitrary_prior_count_cannot_be_silently_repaired(self):
        with self.assertRaisesRegex(ValueError, 'COHORT_DECLARATION:QMECH-M49'):
            aliases.refresh(self.SOURCE.replace('"QMECH-M49", 19', '"QMECH-M49", 18'), pins.ROOT, self.bosses)

    def test_duplicate_or_foreign_packet_identity_refuses_overlap_proof(self):
        expected = aliases.names({21506, 21507})
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for ids in ({21506, 21508}, [21506, 21506]):
                with self.subTest(ids=ids):
                    (root / 'packet.json').write_text(json.dumps({'records': [
                        {'production_definition': {'production_key':
                            f'oteryn:item.tibia.i{item_id}'}} for item_id in ids]}))
                    with self.assertRaisesRegex(ValueError, 'SOURCE_IDENTITIES:packet.json'):
                        aliases.source_keys(root, 'packet.json', expected)


class CraftSourceOverlapProofs(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.craft = json.loads((pins.ROOT / aliases.craft.PACKET).read_bytes())
        cls.engine = json.loads((pins.ROOT / aliases.engine.PROOF).read_bytes())
        # The count-only seam receives already qualified boss identities from its
        # caller. Use the complete committed source packet for that unrelated input;
        # SourceOverlapProofs above separately replays all source qualifications.
        import boss_corpse_profiles as corpses
        cls.bosses = pins.keys(json.loads((pins.ROOT / corpses.PATH).read_bytes()))

    def test_complete_source_packets_count_the_same_ground_grinder_once(self):
        expected = aliases.names({21506, 21507, 21573})
        self.assertEqual(pins.keys(self.craft) & pins.keys(self.engine), expected)
        self.assertIn(21573, aliases.engine.STATIC_IDS)
        result = aliases.refresh(SourceOverlapProofs.SOURCE, pins.ROOT, self.bosses)
        self.assertEqual(
            result,
            SourceOverlapProofs.SOURCE
                .replace('"QMECH-M49", 19', '"QMECH-M49", 5')
                .replace('"QMECH-M50", 9', '"QMECH-M50", 0')
                .replace('"M26-ENDGAME", 6', '"M26-ENDGAME", 1')
                .replace('"QREADY1-CRAFT", 6', '"QREADY1-CRAFT", 3')
                .replace('"ENGNPC-M05", 1', '"ENGNPC-M05", 0')
                .replace('"M22-TAINTED-TALISMAN", 1', '"M22-TAINTED-TALISMAN", 0')
                .replace('"NPCFIX2-M22", 1', '"NPCFIX2-M22", 0'),
        )
        self.assertEqual(aliases.refresh(result, pins.ROOT, self.bosses), result)

    def test_shared_craft_rows_preserve_the_complete_prior_profiles(self):
        engine = {row['production_definition']['production_key']: row
                  for row in self.engine['records']}
        for row in self.craft['records']:
            key = row['production_definition']['production_key']
            if key in engine:
                with self.subTest(key=key):
                    self.assertEqual(aliases.craft.merge_existing(engine[key], row), engine[key])
        grinder = engine['oteryn:item.tibia.i21573']
        self.assertEqual(grinder['admission'], {
            'materializable': True, 'stack_class': 'NonStackable',
            'legal_destinations': ['Ground'],
        })
        self.assertEqual(grinder['semantics']['physical']['value'], {
            'weight': {'state': 'KNOWN', 'value': 0},
            'movable': {'state': 'KNOWN', 'value': False},
            'pickupable': {'state': 'KNOWN', 'value': False},
        })
        self.assertTrue(grinder['attributes']['always_on_top'])
        self.assertEqual(grinder['attributes']['always_on_top_order'], 2)

    def test_grinder_custody_and_known_body_conflicts_still_refuse(self):
        key = 'oteryn:item.tibia.i21573'
        prior = next(row for row in self.engine['records']
                     if row['production_definition']['production_key'] == key)
        observed = next(row for row in self.craft['records']
                        if row['production_definition']['production_key'] == key)
        mutations = [
            (('admission', 'legal_destinations'), ['CharacterInventory', 'Ground']),
            (('production_binding', 'external_id'), '21574'),
            (('production_definition', 'revision_ref'), 'invented-revision'),
            (('semantics', 'physical', 'value', 'weight', 'value'), 1),
            (('semantics', 'physical', 'value', 'pickupable', 'value'), True),
            (('attributes', 'blocks_movement'), False),
        ]
        for path, value in mutations:
            with self.subTest(path=path):
                changed = copy.deepcopy(observed)
                field = changed
                for part in path[:-1]:
                    field = field[part]
                field[path[-1]] = value
                with self.assertRaisesRegex(ValueError, 'QENGINE_REWARD_ITEM_NATIVE_CONFLICT'):
                    aliases.craft.merge_existing(prior, changed)

    def test_missing_duplicate_or_foreign_grinder_cannot_prove_an_overlap(self):
        key = 'oteryn:item.tibia.i21573'
        for path, packet in ((aliases.craft.PACKET, self.craft),
                             (aliases.engine.PROOF, self.engine)):
            for change in ('missing', 'duplicate', 'foreign'):
                with self.subTest(path=path, change=change):
                    changed = copy.deepcopy(packet)
                    row = next(row for row in changed['records']
                               if row['production_definition']['production_key'] == key)
                    if change == 'missing':
                        changed['records'].remove(row)
                    elif change == 'duplicate':
                        changed['records'].append(copy.deepcopy(row))
                    else:
                        row['production_definition']['production_key'] = 'oteryn:item.tibia.i21574'
                    replacement = json.dumps(changed).encode()
                    original_read = Path.read_bytes
                    def read_packet(candidate):
                        if candidate == pins.ROOT / path:
                            return replacement
                        return original_read(candidate)
                    with mock.patch.object(Path, 'read_bytes', read_packet):
                        with self.assertRaisesRegex(ValueError, 'EQUIPMENT_OVERLAP_SOURCE_IDENTITIES:'):
                            aliases.refresh(SourceOverlapProofs.SOURCE, pins.ROOT, self.bosses)

    def test_previous_four_item_contribution_is_not_an_accepted_current_count(self):
        source = SourceOverlapProofs.SOURCE.replace('"QREADY1-CRAFT", 6', '"QREADY1-CRAFT", 4')
        with self.assertRaisesRegex(ValueError, 'EQUIPMENT_OVERLAP_COHORT_DECLARATION:QREADY1-CRAFT'):
            aliases.refresh(source, pins.ROOT, self.bosses)


if __name__ == '__main__':
    unittest.main()
