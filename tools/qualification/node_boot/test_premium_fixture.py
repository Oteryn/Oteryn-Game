"""The disposable packet accounts use the same Premium producer as live Game casts."""
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parent


class PremiumFixtureTests(unittest.TestCase):
    def test_packet_caster_uses_the_platform_premium_ledger_and_snapshot(self):
        run = (ROOT / "run.sh").read_text()
        self.assertIn("PremiumTimeLedger::class", run)
        self.assertIn('OTERYN_PREMIUM_SNAPSHOT_URL=https://127.0.0.1:', run)
        self.assertIn("oteryn-game-premium-fixture", run)
        overlay = (ROOT / "compose.override.yml").read_text()
        self.assertIn("PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_ENABLED", overlay)
        route = (ROOT / "premium.conf").read_text()
        self.assertIn("/internal/v1/products-entitlements/premium-snapshots/read", route)
        self.assertIn("fastcgi_param SSL_CLIENT_VERIFY $ssl_client_verify;", route)
        self.assertIn('fastcgi_param HTTP_SSL_CLIENT_VERIFY "";', route)
