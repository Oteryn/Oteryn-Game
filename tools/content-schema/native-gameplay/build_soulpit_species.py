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
CORE_OUTPUT = ROOT / 'crates/gameplay/items/src/soul_core_metadata.json'
# Exact core names in both pinned Item XMLs. These wiki observations qualify Item
# transformation tiers; they do not allocate Creature identities or spawn profiles.
WIKI_CORES = {
    47654: 'Nightslayer',
    47691: 'Cake Golem',
    47697: 'Berrypest',
    47773: 'Furious Fire Elemental',
    48089: 'Albino Dragon',
    49007: 'Elder Forest Fury',
    49033: 'Orger',
    49034: 'Roast Pork',
    49035: 'Cow',
    49036: 'Loricate Orger',
    49037: 'Bellicose Orger',
    49040: 'Rabid Wolf',
    49041: 'Ragged Rabid Wolf',
}
WIKI_CLASSES = {
    'Humanos': 'Human', 'Constructos': 'Construct', 'Vermes': 'Vermin',
    'Elementais': 'Elemental', 'Dragões': 'Dragon', 'Criaturas Mágicas': 'Magical',
    'Gigantes': 'Giant', 'Mamíferos': 'Mammal',
}
WIKI_DIFFICULTIES = {
    'Inofensivas': 'harmless', 'Fáceis': 'easy', 'Medianas': 'medium',
}


def core_metadata(rows, ids):
    cores = {}
    profiles = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_text())
    names = {int(row['production_binding']['external_id']):
             row['semantics']['presentation']['value']['name']['value']
             for row in profiles['records']
             if int(row['production_binding']['external_id']) in ids}
    for row in rows:
        metadata = {key: row[key] for key in ('core', 'class', 'difficulty')}
        if cores.setdefault(row['core'], metadata) != metadata:
            # The admitted Item name selects Canary's Feversleep interpretation of
            # 47783; Crystal instead calls it Terrorsleep. Never put one core in two tiers.
            candidates = [candidate for candidate in rows if candidate['core'] == row['core']
                          and candidate['creature'].removeprefix('oteryn:creature.').replace('_', ' ')
                          + ' soul core' == names[row['core']]]
            if len(candidates) != 1:
                raise ValueError(f'conflicting core taxonomy: {row["core"]}')
            cores[row['core']] = {key: candidates[0][key]
                                 for key in ('core', 'class', 'difficulty')}
    observations = {}
    for path in sorted((ROOT / 'imports/tibiawiki/facts/creatures').glob('creatures-*.json')):
        for record in json.loads(path.read_text())['records']:
            title = record['source_title']
            if title in WIKI_CORES.values():
                if title in observations:
                    raise ValueError(f'duplicate wiki observation: {title}')
                observations[title] = record
    # Cross-check only, as in MONSTER-D15-BESTIARY §2: raw client fields
    # do not establish membership, class or canonical Creature identity.
    client_rows = []
    for path in sorted((ROOT / 'imports/cipsoft-staticdata/creatures').glob('creatures-*.json')):
        client_rows.extend(json.loads(path.read_text())['records'])
    tiers = ['harmless', 'trivial', 'easy', 'medium', 'hard', 'challenging']
    for core, title in WIKI_CORES.items():
        record = observations[title]
        categories = set(record['observed_categories'])
        classes = [value for name, value in WIKI_CLASSES.items()
                   if f'Categoria:{name} do Bestiário' in categories]
        difficulties = [value for name, value in WIKI_DIFFICULTIES.items()
                        if f'Categoria:Criaturas {name}' in categories]
        if (core in cores or core not in ids or len(classes) != 1
                or len(difficulties) != 1 or record['observed_fields']['name'] != title
                or names.get(core, '').casefold() != title.casefold() + ' soul core'):
            raise ValueError(f'unqualified core taxonomy: {core} / {title}')
        client = [row for row in client_rows if row['name'].casefold() == title.casefold()]
        if len(client) != 1 or client[0]['f4'] != tiers.index(difficulties[0]):
            raise ValueError(f'client tier conflict: {core} / {title}')
        cores[core] = {'core': core, 'class': classes[0], 'difficulty': difficulties[0]}
    if set(cores) != ids:
        raise ValueError('incomplete admitted core taxonomy')
    return [cores[core] for core in sorted(cores)]


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
    for path, records in [(OUTPUT, rows), (CORE_OUTPUT, core_metadata(rows, ids))]:
        encoded = json.dumps(records, indent=2, ensure_ascii=False) + '\n'
        if check:
            if path.read_text() != encoded:
                raise ValueError(f'stale Soulpit projection: {path.name}')
        else:
            path.write_text(encoded)
    print(f'Soulpit: {len(rows)} species, {len(set(row["core"] for row in rows))} bound cores')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    generate(parser.parse_args().check)
