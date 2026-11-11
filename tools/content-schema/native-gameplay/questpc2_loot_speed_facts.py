"""Retain the existing qualified book speed when closing its native loot custody.

Canary items.xml:5813-5817 (SHA1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2)
already produces zero through the pinned Item importer. Accepted statistics win.
"""
KEY = 'oteryn:item.tibia.i2824'


def retain(record, observed):
    if record['production_definition']['production_key'] != KEY:
        return
    if observed['attributes'].get('speed_bonus') != 0:
        raise ValueError('NATIVE_GOAL_LOOT_SPEED_SOURCE_REQUIRED')
    if record['attributes'].get('speed_bonus') is None:
        record['attributes']['speed_bonus'] = 0
