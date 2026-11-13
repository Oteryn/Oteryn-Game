#!/usr/bin/env python3
"""Bind nginx's verified runtime certificate to its one deployment DNS SAN."""
import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import quote_from_bytes


def runtime_map(certificate, identity):
    labels = identity.split('.')
    if len(identity) > 253 or len(labels) < 2 or any(
        not re.fullmatch(r'[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?', label)
        for label in labels
    ):
        raise ValueError('invalid_runtime_identity')
    # This lane issues a SAN extension containing exactly this one DNS entry.
    # Match the entire extension display, never a substring of another SAN type.
    extension = subprocess.check_output(
        ['openssl', 'x509', '-in', str(certificate), '-noout', '-ext', 'subjectAltName'],
        stderr=subprocess.PIPE,
    ).decode('ascii').strip().splitlines()
    if len(extension) != 2 or not extension[0].startswith('X509v3 Subject Alternative Name:') or extension[1].strip() != f'DNS:{identity}':
        raise ValueError('runtime_certificate_san_mismatch')
    # Both openssl x509 and nginx's PEM_write_bio_X509 produce canonical PEM.
    pem = subprocess.check_output(
        ['openssl', 'x509', '-in', str(certificate), '-outform', 'PEM'], stderr=subprocess.PIPE,
    )
    escaped = quote_from_bytes(pem, safe='-_.~')
    # nginx string-map keys are case insensitive. A case-sensitive anchored
    # regex preserves the exact certificate bytes (including base64 case).
    return ('# Generated per run from the runtime-status certificate; no HTTP header is trusted.\n'
            'map "$ssl_client_verify:$ssl_client_escaped_cert" $login_local_runtime_identity {\n'
            '    default "";\n'
            f'    ~^SUCCESS:{escaped}$ "{identity}";\n'
            '}\n')


if __name__ == '__main__':
    try:
        Path(sys.argv[3]).write_text(runtime_map(Path(sys.argv[1]), sys.argv[2]))
    except (OSError, ValueError, IndexError, subprocess.SubprocessError):
        print('runtime_identity_configuration_failed', file=sys.stderr)
        sys.exit(1)
