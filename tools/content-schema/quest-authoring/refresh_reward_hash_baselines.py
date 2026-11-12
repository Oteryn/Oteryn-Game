"""Refresh the existing exact quest hash assertions from the canonical Rust loader."""
import argparse
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
TARGET = ROOT / 'crates/gameplay/quests/src/quest/reward_callbacks.rs'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    source = TARGET.read_text()
    start = source.index('    fn published_ungated_quest_hash_baseline()')
    end = source.index('    #[test]', start)
    baseline = source[start:end]
    pattern = re.compile(r'("(oteryn:quest\.[^"]+)",\s*")[0-9a-f]{64}("\s*,)')
    keys = [match[2] for match in pattern.finditer(baseline)]
    if not keys or len(set(keys)) != len(keys):
        raise ValueError('missing or ambiguous reward hash baseline declarations')
    hashes = json.loads(subprocess.check_output([
        'cargo', 'run', '--locked', '--quiet', '-p', 'oteryn-gameplay-quests',
        '--example', 'quest_definition_hashes', '--', *keys,
    ], cwd=ROOT, text=True))
    if set(hashes) != set(keys) or any(not re.fullmatch('[0-9a-f]{64}', h) for h in hashes.values()):
        raise ValueError('canonical loader returned invalid quest hashes')
    refreshed = pattern.sub(lambda m: m[1] + hashes[m[2]] + m[3], baseline)
    if args.check and refreshed != baseline:
        raise ValueError('stale reward hash baselines')
    if not args.check:
        TARGET.write_text(source[:start] + refreshed + source[end:])
    print(f'reward quest hash baselines: {len(keys)} exact assertions')


if __name__ == '__main__':
    main()
