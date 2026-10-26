#!/usr/bin/env python3
"""Project admitted canonical Creature/core identities and Bestiary facts for Soulpit.

No Reference assets are copied. Regenerate after changing the core or Creature catalogue.
"""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
# Canary 47dfd51 SoulPit.SoulCoresConfiguration.monsterVariationsSoulCore.
# Event links use exact names from both pinned Item XMLs and existing canonical Creatures.
# Their source hashes and remaining gaps are in soulpit-metadata-source-proof.json.
VARIANT_CORES = {
    'blue_butterfly': 47503,
    'bride_of_night': 47662,
    'brown_horse': 48962,
    'butterfly': 47503,
    'doomsday_cultist': 47658,
    'grey_horse': 48963,
    'horse': 48961,
    'midnight_warrior': 47660,
    'nomad': 47558,
    'nomad_blue': 48965,
    'nomad_female': 48966,
    'purple_butterfly': 47490,
    'red_butterfly': 47504,
}

OUTPUT = ROOT / 'crates/gameplay/items/src/soulpit_species.json'


def generate(check=False):
    ids = set(json.loads((ROOT / 'crates/gameplay/items/src/soul_core_ids.json').read_text()))
    rows = []
    for path in sorted((ROOT / 'content/creatures/definitions').glob('creatures-*.json')):
        for record in json.loads(path.read_text())['records']:
            profile = record['authoring']['profile']
            details = profile.get('details', {})
            core = details.get('soul_core_item')
            bestiary = profile.get('bestiary')
            taxonomy = details.get('bestiary')
            if not bestiary or not taxonomy:
                continue
            creature = record['definition']['identity']['key']
            item_id = (int(core['key'].removeprefix('oteryn:item.tibia.i')) if core
                       else VARIANT_CORES.get(creature.removeprefix('oteryn:creature.')))
            if item_id is None:
                continue
            if item_id not in ids:
                continue
            rows.append({'creature': record['definition']['identity']['key'],
                         'revision': record['definition']['identity']['revision'],
                         'core': item_id, 'class': taxonomy['class'],
                         'difficulty': bestiary['difficulty'],
                         'occurrence': bestiary.get('occurrence', 'unknown')})
    rows.sort(key=lambda row: row['creature'])
    if len({row['creature'] for row in rows}) != len(rows):
        raise ValueError('duplicate creature')
    encoded = json.dumps(rows, indent=2, ensure_ascii=False) + '\n'
    if check:
        if OUTPUT.read_text() != encoded:
            raise ValueError('stale Soulpit species projection')
    else:
        OUTPUT.write_text(encoded)
    print(f'Soulpit: {len(rows)} species, {len(set(row["core"] for row in rows))} bound cores')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    generate(parser.parse_args().check)
