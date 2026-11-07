"""Refresh the existing embedded Bestiary source roster from generated Creature shards.

Run before compiling consumers: adding Cake Golem changes the last shard's name.
This changes source-population pins, never Bestiary rules or source records.
"""
import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONSUMER = 'crates/gameplay/content/src/content/project/bestiary.rs'


def refreshed(root=ROOT):
    index = json.loads((root / 'content/creatures/definitions/index.json').read_bytes())
    if index['schema'] != 'OTERYN_FAMILY_INDEX/v1' or index['family'] != 'Creature':
        raise ValueError('BESTIARY_ROSTER_SOURCE_INDEX')
    count = 0
    bestiary = 0
    rows = []
    for path in index['shards']:
        if not re.fullmatch(r'content/creatures/definitions/creatures-\d{5}-\d{5}\.json', path):
            raise ValueError('BESTIARY_ROSTER_SOURCE_PATH')
        shard = json.loads((root / path).read_bytes())
        if shard['family'] != 'Creature' or len(shard['records']) != shard['shard']['count']:
            raise ValueError('BESTIARY_ROSTER_SOURCE_SHARD')
        count += len(shard['records'])
        bestiary += sum(row['authoring']['profile'].get('bestiary') is not None for row in shard['records'])
        rows.append(f'    ("{path}", include_bytes!("../../../../../../{path}")),')
    if count != index['record_count'] or not rows:
        raise ValueError('BESTIARY_ROSTER_SOURCE_COUNT')
    source = (root / CONSUMER).read_text()
    old_creatures = re.search(r'const CREATURE_COUNT: usize = ([\d_]+);', source)
    old_bestiary = re.search(r'const BESTIARY_COUNT: usize = ([\d_]+);', source)
    if old_creatures is None or old_bestiary is None:
        raise ValueError('BESTIARY_ROSTER_CONSUMER_PINS')
    source, replacements = re.subn(
        r'const SHARDS: \[\(&str, &\[u8\]\); \d+\] = \[.*?\n\];',
        f'const SHARDS: [(&str, &[u8]); {len(rows)}] = [\n' + '\n'.join(rows) + '\n];',
        source, count=1, flags=re.S)
    if replacements != 1:
        raise ValueError('BESTIARY_ROSTER_CONSUMER_SHARDS')
    source = source.replace(f'const CREATURE_COUNT: usize = {old_creatures[1]};', f'const CREATURE_COUNT: usize = {count:_};')
    source = source.replace(f'const BESTIARY_COUNT: usize = {old_bestiary[1]};', f'const BESTIARY_COUNT: usize = {bestiary:_};')
    source = source.replace(f'assert_eq!(catalogue.bestiary.len(), {old_bestiary[1]});', f'assert_eq!(catalogue.bestiary.len(), {bestiary:_});')
    source = source.replace(f'canonical_projection_retains_all_{old_bestiary[1]}_admitted_bestiary_blocks', f'canonical_projection_retains_all_{bestiary}_admitted_bestiary_blocks')
    source = source.replace(f'assert_eq!(limits().max_reference_records, {old_creatures[1]});', f'assert_eq!(limits().max_reference_records, {count:_});')
    return source


def regenerate(root=ROOT, check=False):
    import subprocess
    source = subprocess.run(['rustfmt', '--edition', '2024'], input=refreshed(root), text=True, capture_output=True, check=True).stdout
    path = root / CONSUMER
    if path.read_text() != source:
        if check:
            raise ValueError('BESTIARY_ROSTER_STALE')
        path.write_text(source)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    regenerate(check=args.check)
