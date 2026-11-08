import json
import unittest

from compact_record_json import encode


class CompactRecordJsonTests(unittest.TestCase):
    def test_nested_packets_preserve_values_and_keep_each_record_on_one_line(self):
        records = [{'z': [1, 2], 'name': 'café\nsecond line'}, {'a': {'flag': False}}]
        document = {'bundle': {'records': records}, 'defaults': {'3068': 0}, 'empty': []}
        raw = encode(document)
        self.assertEqual(json.loads(raw), document)
        for record in records:
            self.assertIn(json.dumps(record, ensure_ascii=False, sort_keys=True,
                                     separators=(',', ':')),
                          [line.removesuffix(',') for line in raw.decode().splitlines()])
        self.assertEqual(encode(json.loads(raw)), raw)
        self.assertTrue(raw.endswith(b'\n'))


if __name__ == '__main__':
    unittest.main()
