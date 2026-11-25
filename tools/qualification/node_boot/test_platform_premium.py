import base64
import unittest

from platform_premium import totp


class PlatformPremiumTests(unittest.TestCase):
    def test_totp_matches_rfc6238_sha1_truncated_to_six_digits(self):
        key = base64.b32encode(b"12345678901234567890").rstrip(b"=")
        self.assertEqual(totp(key, 59), "287082")
        self.assertEqual(totp(key, 1111111109), "081804")
