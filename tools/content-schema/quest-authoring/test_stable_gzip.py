import gzip
import unittest

from donor_sources.stable_gzip import stable_gzip

PAYLOAD = b'{"status":"PARSED"}\n' * 64


class StableGzipTest(unittest.TestCase):
    def test_header_is_fixed_on_every_python_version(self):
        # magic, deflate, no flags, mtime 0, XFL (2 = level 9, 0 = level 6), OS 3 (Unix)
        self.assertEqual(stable_gzip(PAYLOAD)[:10], bytes.fromhex('1f8b0800000000000203'))
        self.assertEqual(stable_gzip(PAYLOAD, level=6)[:10], bytes.fromhex('1f8b0800000000000003'))

    def test_only_the_os_byte_differs_from_gzip_compress(self):
        for level in (6, 9):
            plain = gzip.compress(PAYLOAD, compresslevel=level, mtime=0)
            fixed = stable_gzip(PAYLOAD, level=level)
            self.assertEqual(fixed[:9] + fixed[10:], plain[:9] + plain[10:])
            self.assertEqual(gzip.decompress(fixed), PAYLOAD)


if __name__ == '__main__':
    unittest.main()
