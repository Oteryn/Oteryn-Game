"""Exact retained source selectors; no guessed quest object positions."""
import unittest
import quest_startup_bindings


class SerpentineStartupBindings(unittest.TestCase):
    def test_five_retained_source_objects_are_restored(self):
        rows = quest_startup_bindings.document()['bindings']
        for position, item, attribute, value in [
            ([33151, 32861, 7], 2928, 'action', 5632),
            ([33152, 32866, 8], 2772, 'action', 5633),
            ([33148, 32864, 7], 1949, 'action', 5630),
            ([33150, 32864, 7], 1949, 'action', 5631),
            ([33150, 32862, 7], 2472, 'unique', 6184),
        ]:
            matches = [row for row in rows if row['position'] == position
                       and row['item'] == item and row['attribute'] == attribute]
            self.assertEqual(len(matches), 1, (position, item, attribute))
            self.assertEqual(matches[0]['value'], value)
            self.assertGreater(matches[0]['line'], 0)


if __name__ == '__main__':
    unittest.main()
