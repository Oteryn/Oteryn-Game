"""Runtime identity must be certificate-bound and agree with Game assignments."""
from pathlib import Path
import unittest
import re
import subprocess
import tempfile
from urllib.parse import quote_from_bytes

from runtime_identity import runtime_map

ROOT = Path(__file__).parent


class RuntimeIdentityTests(unittest.TestCase):
    def test_runtime_forwarding_and_allowlist_use_the_dns_identity(self):
        config = (ROOT / "nginx.conf").read_text()
        self.assertIn('include /run/pki/runtime-status-map.conf;', config)
        start = config.index('location = /internal/v1/game-auth/native-runtime-status {')
        block = config[start:config.index('\n        }', start)]
        self.assertIn('if ($login_local_runtime_identity = "") { return 401; }', block)
        self.assertIn('fastcgi_param SSL_CLIENT_S_DN $login_local_runtime_identity;', block)
        self.assertNotIn('$ssl_client_s_dn', block)
        self.assertIn('fastcgi_param HTTP_SSL_CLIENT_S_DN "";', block)
        script = (ROOT / "run.sh").read_text()
        self.assertIn('LL_RUNTIME_STATUS_IDENTITIES="{\\\"$RUNTIME_IDENTITY\\\":', script)
        self.assertNotIn('LL_RUNTIME_STATUS_IDENTITIES="{\\\"CN=', script)


class CertificateBindingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.directory.cleanup)
        cls.root = Path(cls.directory.name)
        cls.identity = 'node.runtime-status.login-local.test'
        cls.run_openssl('req', '-x509', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:P-256',
                        '-nodes', '-subj', '/CN=Review12 CA', '-days', '1',
                        '-keyout', str(cls.root / 'ca.key'), '-out', str(cls.root / 'ca.pem'))
        cls.expected = cls.leaf('expected', 'DNS:' + cls.identity)
        cls.other = cls.leaf('other', 'DNS:' + cls.identity)

    @staticmethod
    def run_openssl(*arguments):
        return subprocess.check_output(['openssl', *arguments], stderr=subprocess.PIPE)

    @classmethod
    def leaf(cls, name, san):
        key, csr, cert, ext = [cls.root / (name + suffix) for suffix in ['.key', '.csr', '.pem', '.ext']]
        cls.run_openssl('req', '-new', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:P-256',
                        '-nodes', '-subj', '/CN=' + cls.identity, '-keyout', str(key), '-out', str(csr))
        text = 'basicConstraints=critical,CA:FALSE\nextendedKeyUsage=clientAuth\n'
        if san:
            text += 'subjectAltName=' + san + '\n'
        ext.write_text(text)
        cls.run_openssl('x509', '-req', '-in', str(csr), '-CA', str(cls.root / 'ca.pem'),
                        '-CAkey', str(cls.root / 'ca.key'), '-CAcreateserial', '-days', '1',
                        '-extfile', str(ext), '-out', str(cert))
        return cert

    def test_only_the_exact_verified_certificate_maps_to_the_san(self):
        program = runtime_map(self.expected, self.identity)
        entry = next(line.strip() for line in program.splitlines() if line.strip().startswith('~'))
        expression, identity = entry.split(' ', 1)
        self.assertEqual(identity, '"' + self.identity + '";')
        self.assertFalse(expression.startswith('~*'))
        matcher = re.compile(expression[1:])
        pem = self.run_openssl('x509', '-in', str(self.expected), '-outform', 'PEM')
        escaped = quote_from_bytes(pem, safe='-_.~')
        self.assertIsNotNone(matcher.fullmatch('SUCCESS:' + escaped))
        for verification in ['NONE', 'FAILED:certificate verify failed', 'success']:
            self.assertIsNone(matcher.fullmatch(verification + ':' + escaped))
        self.assertIsNone(matcher.fullmatch('SUCCESS:' + escaped.swapcase()))
        # Same CA, subject AND DNS SAN, but a different leaf/key is not this deployment's node.
        other = quote_from_bytes(self.run_openssl('x509', '-in', str(self.other), '-outform', 'PEM'), safe='-_.~')
        self.assertIsNone(matcher.fullmatch('SUCCESS:' + other))
        self.assertIn('default "";', program)
        self.assertNotIn('$http_', program)
        self.assertNotIn('$ssl_client_s_dn', program)

    def test_missing_multiple_wrong_and_non_dns_sans_are_refused(self):
        for index, san in enumerate([None, 'DNS:' + self.identity + ',DNS:other.test',
                                     'IP:127.0.0.1', 'DNS:other.test',
                                     'URI:urn:DNS:' + self.identity]):
            with self.subTest(san=san):
                certificate = self.leaf('invalid' + str(index), san)
                with self.assertRaises(ValueError):
                    runtime_map(certificate, self.identity)

    def test_identity_configuration_cannot_inject_nginx_directives(self):
        for identity in ['CN=' + self.identity, 'Node.test', 'node_test.test', 'node..test',
                         'node.test"; default attacker;', 'node.test\n']:
            with self.subTest(identity=identity), self.assertRaises(ValueError):
                runtime_map(self.expected, identity)


if __name__ == '__main__':
    unittest.main()
