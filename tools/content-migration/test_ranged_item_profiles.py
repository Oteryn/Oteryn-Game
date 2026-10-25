"""Offline ranged source lowering, strict failure and committed-output checks."""
import copy
import hashlib
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import ranged_item_profiles as ranged


def item(attributes, nested="", item_id='id="1"', name="test"):
    def nodes(values):
        return "".join(f'<attribute key="{key}" value="{value}"/>' for key, value in values.items())
    script = f'<attribute key="script" value="weapon">{nodes(nested)}</attribute>' if nested else ""
    return f'<items><item {item_id} name="{name}">{nodes(attributes)}{script}</item></items>'


class RangedItemProfilesTest(unittest.TestCase):
    def test_pinned_categories_and_samples(self):
        profiles, held, counts = ranged.source_profiles(ranged.ROOT)
        self.assertEqual(counts, {"DISTANCE": 58, "THROWING": 15, "AMMUNITION": 25, "WAND": 34, "ROD": 32})
        self.assertEqual(len(profiles), 163)
        self.assertEqual([(row['item_id'], row['profile']['range_cells']) for row in held], [(49522, 8)])
        for item_id, kind, attack, distance in [(3350, 'DISTANCE', 0, 6), (3349, 'DISTANCE', 0, 5), (3277, 'THROWING', 25, 3)]:
            with self.subTest(item=item_id):
                self.assertEqual((profiles[item_id]['kind'], profiles[item_id]['attack'], profiles[item_id]['range_cells']), (kind, attack, distance))
        for item_id, ammo, attack in [(3447, 'ARROW', 25), (3446, 'BOLT', 30)]:
            self.assertEqual((profiles[item_id]['ammunition'], profiles[item_id]['attack'], profiles[item_id]['element']), (ammo, attack, 'PHYSICAL'))
        for item_id, kind, element, mana in [(3074, 'WAND', 'ENERGY', 1), (3066, 'ROD', 'EARTH', 2)]:
            self.assertEqual(profiles[item_id], dict(kind=kind, element=element, damage_min=8, damage_max=18, mana_per_shot=mana, range_cells=3))

    def test_defaults_are_compiled_once_and_ranges_expand(self):
        raw = item({'weaponType': 'distance', 'ammoType': 'arrow'}, item_id='fromid="1" toid="2"')
        profiles, held, counts = ranged.build(raw)
        self.assertEqual(profiles[1], dict(kind='DISTANCE', attack=0, ammunition='ARROW', range_cells=1, hit_chance=0, max_hit_chance=90))
        self.assertEqual(profiles[1], profiles[2])
        self.assertIsNot(profiles[1], profiles[2])
        self.assertEqual((held, counts), ([], {'DISTANCE': 2}))

    def test_magic_requires_all_fields_and_valid_damage(self):
        top = {'weaponType': 'wand', 'range': '3'}
        nested = {'weaponType': 'wand', 'mana': '2', 'fromDamage': '8', 'toDamage': '18', 'wandType': 'earth'}
        for field in ['mana', 'fromDamage', 'toDamage', 'wandType']:
            incomplete = dict(nested)
            del incomplete[field]
            with self.subTest(field=field), self.assertRaises(ValueError):
                ranged.build(item(top, incomplete))
        for replacement in [{'mana': '0'}, {'fromDamage': '19'}, {'wandType': 'unknown'}]:
            with self.assertRaises(ValueError):
                ranged.build(item(top, {**nested, **replacement}))

    def test_elemental_ammunition_and_signed_launcher_bonus(self):
        profiles, _, _ = ranged.source_profiles(ranged.ROOT)
        self.assertEqual((profiles[761]['element'], profiles[761]['elemental_attack']), ('ENERGY', 14))
        self.assertEqual(profiles[8024]['hit_chance'], -20)
        self.assertEqual((profiles[25735]['element'], profiles[25735]['elemental_attack']), ('EARTH', 2))
        with self.assertRaises(ValueError):
            ranged.build(item({'weaponType': 'ammunition', 'ammoType': 'unknown'}))
        with self.assertRaises(ValueError):
            ranged.build(item({'weaponType': 'distance', 'hitChance': '-1'}))

    def test_source_digest_and_duplicate_identity_fail_closed(self):
        with patch.object(Path, 'read_bytes', return_value=b'<items/>'):
            with self.assertRaisesRegex(ValueError, 'RANGED_SOURCE_DIGEST'):
                ranged.source_profiles(ranged.ROOT)
        raw = item({'weaponType': 'distance'})
        with self.assertRaisesRegex(ValueError, 'RANGED_DUPLICATE_ITEM'):
            ranged.build(raw.replace('</items>', raw[7:]))
        raw = item({'weaponType': 'distance', 'range': '3'})
        with self.assertRaisesRegex(ValueError, 'RANGED_DUPLICATE_ATTRIBUTE'):
            ranged.build(raw.replace('</item>', '<attribute key="range" value="4"/></item>'))

    def test_committed_tree_regenerates_byte_for_byte(self):
        for path, data in ranged.outputs().items():
            with self.subTest(path=path):
                self.assertEqual((ranged.ROOT / path).read_bytes(), data)
        report = json.loads((ranged.ROOT / ranged.REPORT).read_bytes())
        self.assertEqual(report['undefined_items'], [15792])

    def test_separate_profiles_preserve_sealed_owners_and_resolve_existing_values(self):
        rows = {row['definition']['identity']['key']: row for row in ranged.item_rows(ranged.ROOT)}
        records = json.loads((ranged.ROOT / ranged.RANGED_PROFILES).read_bytes())['records']
        self.assertEqual(len(records), 162)
        for row in rows.values():
            self.assertNotIn('ranged', row.get('authoring', {}))
        seen = set()
        for record in records:
            key = record['item']['key']
            self.assertNotIn(key, seen)
            seen.add(key)
            row = rows[key]
            self.assertEqual(record['item'], row['definition']['identity'])
            owner = row.get('authoring', {})
            profile = record['profile']
            resolved = ranged.resolved_profile(profile, owner)
            for field, canonical in [('attack', 'weapon_attack_modifier_points'),
                                     ('hit_chance', 'weapon_absolute_hit_chance_percent')]:
                if canonical in owner:
                    self.assertNotIn(field, profile)
                    duplicate = {**profile, field: resolved[field]}
                    with self.assertRaisesRegex(ValueError, 'RANGED_DUPLICATE_OWNER'):
                        ranged.resolved_profile(duplicate, owner)
            if key == 'oteryn:item.tibia.i3446':
                self.assertEqual(resolved['hit_chance'], 86)
            if key == 'oteryn:item.tibia.i16163':
                self.assertEqual(resolved['attack'], 4)
        for ratio in [dict(numerator=1, denominator=0), dict(numerator=1, denominator=2)]:
            with self.assertRaisesRegex(ValueError, 'RANGED_HIT_CHANCE_NOT_INTEGER'):
                ranged.resolved_profile({}, {'weapon_absolute_hit_chance_percent': ratio})

    def test_native_overlay_preserves_admission_and_rebuilds_digest(self):
        original = (ranged.ROOT / ranged.PROFILES).read_bytes()
        document = json.loads(original)
        clean = copy.deepcopy(document)
        for record in clean['records']:
            record['authoring'].pop('ranged', None)
        generated = {ranged.PROFILES: json.dumps(clean).encode(), ranged.MANIFEST: (ranged.ROOT / ranged.MANIFEST).read_bytes()}
        ranged.apply_native(ranged.ROOT, generated)
        self.assertEqual(generated[ranged.PROFILES], original)
        self.assertEqual(json.loads(generated[ranged.MANIFEST])['item_profiles']['sha256'], hashlib.sha256(original).hexdigest())
        self.assertEqual(len(document['records']), len(clean['records']))

    def test_native_overlay_rejects_separate_source_drift_atomically(self):
        original_read = Path.read_bytes
        path = ranged.ROOT / ranged.RANGED_PROFILES
        source = json.loads(path.read_bytes())
        generated = {ranged.PROFILES: (ranged.ROOT / ranged.PROFILES).read_bytes(),
                     ranged.MANIFEST: (ranged.ROOT / ranged.MANIFEST).read_bytes()}
        for mutation in ['missing', 'duplicate', 'changed']:
            changed = copy.deepcopy(source)
            if mutation == 'missing':
                changed['records'].pop()
            elif mutation == 'duplicate':
                changed['records'].append(changed['records'][0])
            else:
                changed['records'][0]['profile']['range_cells'] = 7
            def read(current):
                return ranged.encode(changed) if current == path else original_read(current)
            with self.subTest(mutation=mutation), patch.object(Path, 'read_bytes', read):
                candidate = dict(generated)
                with self.assertRaisesRegex(ValueError, 'RANGED_PROFILE_SOURCE_DRIFT'):
                    ranged.apply_native(ranged.ROOT, candidate)
                self.assertEqual(candidate, generated)


if __name__ == '__main__':
    unittest.main()
