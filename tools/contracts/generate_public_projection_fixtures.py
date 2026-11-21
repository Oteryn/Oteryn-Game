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


def character_view_fixtures():
    # Rev 3 examples. Files end with LF; LF is not part of the transmitted body.
    def write(name, wire):
        (OUT / f'{name}_v2.json').write_bytes(
            json.dumps(wire, ensure_ascii=False, separators=(',', ':')).encode() + b'\n')

    def snapshot(name, family, revision, record, observed='1790000000'):
        wire = {'contract_version': 2, 'operation': f'PublishCharacter{family.title()}V2',
                'source_authority': 'oteryn:game:primary', 'character_id': CHARACTER,
                'projection_epoch': '1', 'projection_revision': str(revision),
                'source_observed_at': observed, family: record}
        write(name, wire)
        return wire

    # Preserve the other rev 3 examples without changing main's existing G-4 wire.
    def example(name, operation, subject, revision, body):
        write(name, {'contract_version': 2, 'operation': operation,
              'source_authority': 'oteryn:game:primary', **subject,
              'projection_epoch': '1', 'projection_revision': revision,
              'source_observed_at': '1790000000', **body})

    example('guild_rev3_example', 'PublishGuildV2', {'guild_id': GUILD}, '5', {'guild': [{
        'world_id': WORLD, 'name': 'Red Rose', 'state': 'ACTIVE', 'founded_at': '1789000000',
        'ranks': [{'level': 1, 'name': 'Leader'}, {'level': 2, 'name': 'Vice Leader'},
                  {'level': 3, 'name': 'Member'}],
        'members': [{'character_id': CHARACTER, 'name': 'Aldric', 'rank_level': 1,
                     'joined_at': '1789000000'}]}]})
    example('house_shop_rev3_example', 'PublishHouseV2',
            {'world_id': WORLD, 'house_key': 'oteryn:content.house.example-shop'}, '12',
            {'house': [{'name': 'Example Shop', 'kind': 'shop',
                        'town': 'oteryn:content.area.city.example', 'size_sqm': '24',
                        'beds': '0', 'rent_gold': '5000', 'status': 'VACANT'}]})

    base = {'world_id': WORLD, 'character_revision': '19'}
    skills = snapshot('character_skills', 'skills', 7, [{**base,
        'level': '412', 'total_experience': '11400000000', 'magic_level': 12,
        'fist_level': 10, 'club_level': 10, 'sword_level': 10, 'axe_level': 10,
        'distance_level': 90, 'shielding_level': 50, 'fishing_level': 10}])
    achievements = snapshot('character_achievements', 'achievements', 8, [{**base,
        'catalogue_revision': 'fixture-v1', 'earned': [
            {'key': 'oteryn:achievement/fixture_earned', 'earned_at': '1789990000'}]}])
    snapshot('character_equipment', 'equipment', 9, [{**base, 'slots': [
        {'slot': 'Head', 'item_key': 'oteryn:item.fixture-helmet', 'item_revision': 'fixture-v1'}]}])
    snapshot('character_outfit', 'outfit', 10, [{**base,
        'content_revision': 'fixture-v1', 'outfit_key': 'outfit.citizen',
        'colours': [1, 2, 3, 4], 'addons': 0, 'mount': []}])
    snapshot('character_skills_suppression', 'skills', 8, [], '1790000001')
    watermark = {'contract_version': 2, 'operation': 'PublishPublicProjectionWatermarkV2',
                 'source_authority': 'oteryn:game:primary', 'family': 'character_skills',
                 'projection_epoch': '1', 'complete_through': '1790000000',
                 'observed_at': '1790000003'}
    write('character_skills_watermark', watermark)
    write('character_skills_invalid_types', {**skills, 'projection_epoch': '0',
                                           'projection_revision': 7, 'skills': None})
    write('character_achievements_invalid_account', {**achievements, 'achievements': [],
          'account_id': '01934f10-7c01-7001-805b-3b1122334401'})
    write('character_skills_invalid_watermark', {**watermark, 'complete_through': '1790000004'})
    # Schema-valid, but conflicts with the occupied revision 9 in sequence.
    snapshot('character_equipment_invalid_conflict', 'equipment', 9, [{**base, 'slots': []}])


def character_view_limits():
    entries = ROOT / 'docs/contracts/resource-limits/entries'
    for ordinal, family, maximum in [
        (10094, 'skills', 4096), (10095, 'achievements', 262144),
        (10096, 'equipment', 8192), (10097, 'outfit', 2048),
    ]:
        limit_id = f'PUBPROJ-V2-{family.upper()}-REQUEST-BYTES'
        entry = {
            'id': limit_id, 'owner_contract': 'OTERYN_GAME_PUBLIC_PROJECTIONS_V2.md',
            'resource': f'V2 character {family} snapshot request bytes', 'unit': 'bytes',
            'hard_maximum': maximum,
            'configurable_range': {'minimum': maximum, 'maximum': maximum},
            'failure_category': 'UNAVAILABLE',
            'allocation_impact': 'Overflow refuses the whole snapshot; never truncate source facts.',
            'client_visible': False,
            'boundary_tests': [f'crates/gameplay/native-admission-source/src/public_projections/v2/tests.rs:public_projections_v2_{family}_golden_bytes'],
            'consumers': ['crates/gameplay/native-admission-source/src/public_projections/v2.rs',
                          'crates/gameplay/native-admission-source/src/descriptor.rs'],
            'notes': 'Generated by tools/contracts/generate_public_projection_fixtures.py. '
                     'Accepted Platform G-4 revision 3 at f5f91a6e81bc8b89b51cbb77517253cf549dc4fa. '
                     'Owner-authorized Part A; activation remains off.',
        }
        (entries / f'{ordinal}-{limit_id}.json').write_text(json.dumps(entry, indent=2) + '\n')


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    character_view_fixtures()
    character_view_limits()
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
