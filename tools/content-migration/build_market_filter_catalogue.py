"""T124: regenerate numeric navigation metadata for the existing admitted Market cohort.

No asset bytes, item names, new identities, or marketability decisions are emitted.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/item-authoring'))
from client_appearance_census import load_client_appearances, DEFAULT_APPEARANCES


def generate():
    source = ROOT / 'docs/agents/evidence/OTV2-20261031-item-market-crystal-parity-v1.json'
    appearances = load_client_appearances(DEFAULT_APPEARANCES)
    ids = json.loads(source.read_text())['crystal_market_ids']
    rows = []
    for item_id in ids:
        flags = appearances[item_id]['flags']
        category = flags['market.category']
        if not 1 <= category <= 31:
            raise ValueError('Unrecognized admitted market category')
        rows.append([item_id, category, flags.get('upgradeclassification.upgrade_classification')])
    return (json.dumps({'schema': 'OTERYN_MARKET_FILTERS/v1',
        'admission_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
        'appearances_sha256': hashlib.sha256(DEFAULT_APPEARANCES.read_bytes()).hexdigest(),
        'rows': rows}, separators=(',', ':')) + '\n').encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    target = ROOT / 'crates/gameplay/content/src/content/native_gameplay/market_filter_catalogue.json'
    data = generate()
    if args.check:
        if target.read_bytes() != data:
            raise SystemExit('Market filter catalogue is stale')
    else:
        target.write_bytes(data)


if __name__ == '__main__':
    main()
