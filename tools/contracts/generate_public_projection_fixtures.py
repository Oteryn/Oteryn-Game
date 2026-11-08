#!/usr/bin/env python3
"""Regenerate the shared owner-accepted public-projection V2 byte fixtures."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'docs/contracts/fixtures/public-projections'
CHARACTER = '01934f10-7c04-7001-805b-3b1122334401'
KILLER = '01934f10-7c04-7001-805b-3b1122334402'
WORLD = '01934f10-7c02-7001-805b-3b1122334401'
GUILD = '01934f10-7c06-7001-805b-3b1122334401'
OCCURRENCE = '01934f10-7c08-7001-805b-3b1122334401'


def fixture(name, operation, subject, body):
    wire = {'contract_version': 2, 'operation': operation,
            'source_authority': 'oteryn:game:primary', **subject,
            'projection_epoch': '1', 'projection_revision': '3',
            'source_observed_at': '1790000000', **body}
    (OUT / f'{name}_v2.json').write_bytes(
        json.dumps(wire, ensure_ascii=False, separators=(',', ':')).encode())


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for kind, killers in [
        ('player', [{'kind': 'player', 'character_id': KILLER, 'name': 'Beren'}]),
        ('creature', [{'kind': 'creature', 'creature_key': 'oteryn:monster.dragon', 'name': 'Dragon'}]),
        ('pvp_hidden', []),
    ]:
        fixture(f'character_deaths_{kind}', 'PublishCharacterDeathsV2',
                {'character_id': CHARACTER}, {'deaths': [{
                    'death_occurrence_id': OCCURRENCE, 'occurred_at': '1789990000',
                    'level': '413', 'pvp': kind != 'creature', 'killers': killers}]})
    fixture('guild', 'PublishGuildV2', {'guild_id': GUILD}, {'guild': [{
        'world_id': WORLD, 'name': 'Red Rose', 'state': 'ACTIVE',
        'founded_at': '1789000000', 'ranks': [
            {'level': 1, 'name': 'Leader'}, {'level': 2, 'name': 'Vice Leader'},
            {'level': 3, 'name': 'Member'}], 'members': [{
                'character_id': CHARACTER, 'name': 'Aldric', 'rank_level': 1,
                'joined_at': '1789000000'}]}]})
    for status, extras in [
        ('VACANT', {}), ('UNAVAILABLE', {}),
        ('AUCTION', {'current_bid': '500', 'auction_ends_at': '1792000000'}),
        ('OWNED', {'owner_kind': 'CHARACTER', 'owner_name': 'Aldric', 'paid_until': '1792000000'}),
        ('MOVE_OUT_PENDING', {'owner_kind': 'GUILD', 'owner_name': 'Red Rose',
                              'paid_until': '1792000000', 'move_out_at': '1791000000'}),
    ]:
        fixture(f'house_{status.lower()}', 'PublishHouseV2',
                {'world_id': WORLD, 'house_key': 'oteryn:content.house.example-street-1'},
                {'house': [{'name': 'Example Street 1', 'kind': 'private_house',
                           'town': 'oteryn:content.area.city.example', 'size_sqm': '24',
                           'beds': '2', 'rent_gold': '5000', 'status': status, **extras}]})


if __name__ == '__main__':
    main()
