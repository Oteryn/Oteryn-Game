import unittest
import build_activity_source_world as producer


class SourceTopOrder(unittest.TestCase):
    def test_source_cpp_precedence_and_default(self):
        self.assertEqual(producer.source_top_order({}), 0)
        self.assertEqual(producer.source_top_order({2: [1]}), 1)
        self.assertEqual(producer.source_top_order({3: [1]}), 2)
        self.assertEqual(producer.source_top_order({4: [1]}), 3)
        self.assertEqual(producer.source_top_order({2: [1], 3: [1], 4: [1]}), 1)
        self.assertEqual(producer.source_top_order({3: [1], 4: [1]}), 3)
        self.assertEqual(producer.source_top_order({2: [0], 4: [1]}), 3)


if __name__ == '__main__':
    unittest.main()
