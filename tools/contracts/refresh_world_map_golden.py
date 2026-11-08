#!/usr/bin/env python3
"""Regenerate the seeded map-wire golden only after v6 wire compatibility passes."""
import argparse
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
TEST = ROOT / 'crates/gameplay/dispatch/src/gameplay_transport/world_map_tests.rs'

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', action='append', default=[],
                        help='Cargo configuration override used for this verification build')
    args = parser.parse_args()
    configs = [part for value in args.config for part in ('--config', value)]
    env = dict(os.environ, OTERYN_RECORD_MAP_GOLDEN='1')
    result = subprocess.run([
        'cargo', 'test', '--locked', '-j', '1', '-p', 'oteryn-gameplay-dispatch',
        '--lib', '--all-features', *configs,
        'seeded_walks_send_the_bytes_recorded_before_the_viewport_optimisation',
        '--', '--nocapture', '--test-threads=1',
    ], cwd=ROOT, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    print(result.stdout, end='')
    result.check_returncode()
    assert 'LEGACY_WIRE_VERIFIED updates=324 bytes=5431963' in result.stdout
    matches = re.findall(r'^GOLDEN updates=324 bytes=5431963 digest=([0-9a-f]{64})$', result.stdout, re.M)
    assert len(matches) == 1, 'expected exactly one verified recording'
    text = TEST.read_text()
    pattern = r'(const GOLDEN: \(usize, usize, &str\) = \(\s*324,\s*5_431_963,\s*")[0-9a-f]{64}(",\s*\);)'
    updated, count = re.subn(pattern, lambda match: match[1] + matches[0] + match[2], text)
    assert count == 1, 'golden declaration changed; refusing an ambiguous edit'
    if updated != text:
        TEST.write_text(updated)
    print('Updated verified seeded map-wire recording:', matches[0])

if __name__ == '__main__':
    main()
