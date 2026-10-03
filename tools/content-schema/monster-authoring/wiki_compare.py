"""Compare converted Canary monster bundles with TibiaWiki (Fandom) at the 2026-09-27 reference cut.

Evidence tooling only. For each monster page it records the last revision at or before the cut and
the current revision (to expose post-cut edits), the SHA-256 of the cut revision's wikitext and
only the compared infobox facts. No wiki prose is stored. Wiki values are player observations and
never become Game truth by this comparison.

Usage: python wiki_compare.py --canary <Canary checkout> --batch canary-47dfd51f [--cache DIR]
"""
import argparse
import hashlib
import json
import math
import re
import subprocess
import time
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from decimal import Decimal
from fractions import Fraction
from pathlib import Path

import canary_batch as cb

ROOT = Path(__file__).resolve().parent
API = 'https://tibia.fandom.com/api.php'
PAGE_URL = 'https://tibia.fandom.com/wiki/'
USER_AGENT = 'OterynEvidenceCollector/0.1 (+https://github.com/Oteryn/Oteryn-Game)'
TARGET_CUT = '2026-09-27'
LOW_CONFIDENCE_DROPS = 10
CUT_TIMESTAMP = '2026-09-28T00:00:00Z'
ELEMENTS = {'physical': 'physicalDmgMod', 'earth': 'earthDmgMod', 'fire': 'fireDmgMod', 'death': 'deathDmgMod',
            'energy': 'energyDmgMod', 'holy': 'holyDmgMod', 'ice': 'iceDmgMod', 'life_drain': 'hpDrainDmgMod',
            'drowning': 'drownDmgMod'}
# Nomads disambiguation (page 23774, cut revision 692522) lists these distinct creatures.
# Registration names and exact source race IDs identify variants; a shared display name does not.
CREATURE_PAGE_ROUTES = {
    'humans/nomad': ('Nomad', 310, 'Nomad (Basic)', 12143),
    'humans/nomad_blue': ('Nomad Blue', 776, 'Nomad (Blue)', 60961),
    'humans/nomad_female': ('Nomad Female', 777, 'Nomad (Female)', 60960),
    # Public Fandom source-pointer and canonical Creature revisions captured at the same cut.
    # None means the pinned Lua source has no raceId; it does not assert a wiki race ID.
    'aquatics/fish': ('Fish', 784, 'Fish (Creature)', 56541),
    'aquatics/northern_pike': ('Northern Pike', 783, 'Northern Pike (Creature)', 56542),
    'bosses/pythius_the_rotten': ('Pythius The Rotten', None, 'Pythius the Rotten (Creature)', 31198),
    'event_creatures/pinata_dragon': ('Pinata Dragon', None, 'Piñata Dragon', 80171),
    'humans/monk': ('Monk', 57, 'Monk (Creature)', 1244),
    'mammals/sabretooth': ('Sabretooth', 2267, 'Sabretooth (Creature)', 98812),
    'mammals/horse': ('Horse', 750, 'Horse (Grey)', 85688),
    'vermins/butterfly': ('Butterfly', 227, 'Butterfly (Blue)', 38911),
    'quests/the_explorer_society/pink_butterfly': ('Pink Butterfly', 213, 'Butterfly (Purple)', 3235),
    'raids/chayenne': ('Chayenne', None, 'Chayenne (Creature)', 57948),
    'quests/forgotten_knowledge/baby_dragon': ('Baby Dragon', None, 'Baby Dragon (Creature)', 79884),
    'quests/forgotten_knowledge/icicle': ('Icicle', None, 'Icicle (Creature)', 79755),
    'quests/in_service_of_yalahar/yalahari': ('Yalahari', None, 'Yalahari (Creature)', 31119),
    'quests/svargrond_arena/scrapper/avalanche': ('Avalanche', None, 'Avalanche (Creature)', 16559),
}
CREATURE_PAGE_POINTERS = {
    'humans/nomad': ('Nomads', 23774),
    'humans/nomad_blue': ('Nomads', 23774),
    'humans/nomad_female': ('Nomads', 23774),
    'aquatics/fish': ('Fish', 1192),
    'aquatics/northern_pike': ('Northern Pike', 16985),
    'bosses/pythius_the_rotten': ('Pythius the Rotten', 31558),
    'event_creatures/pinata_dragon': ('Pinata Dragon', 84428),
    'humans/monk': ('Monk', 107879),
    'mammals/sabretooth': ('Sabretooth', 42021),
    'mammals/horse': ('Horse', 85686),
    'vermins/butterfly': ('Butterfly', 38909),
    'quests/the_explorer_society/pink_butterfly': ('Butterfly', 38909),
    'raids/chayenne': ('Chayenne', 58211),
    'quests/forgotten_knowledge/baby_dragon': ('Baby Dragon', 77957),
    'quests/forgotten_knowledge/icicle': ('Icicle', 20003),
    'quests/in_service_of_yalahar/yalahari': ('Yalahari', 30058),
    'quests/svargrond_arena/scrapper/avalanche': ('Avalanche', 19931),
}

# Captured MediaWiki redirects Horse (Gray) to Horse (Grey); exact race 750 selects this variant.
CREATURE_PAGE_POINTER_LINKS = {'mammals/horse': 'Horse (Gray)'}

# Exact pinned registrations whose public Creature pages differ only in case.
# Every entry has dated Creature name/actualname proof; no generic case lookup is enabled.
CREATURE_CASE_ROUTES = {
    'familiars/druid_familiar': ('Druid familiar', 'Druid Familiar', 80843),
    'quests/rotten_blood/echo_of_chagorz': ('Echo Of Chagorz', 'Echo of Chagorz', 103144),
    'quests/rotten_blood/echo_of_ichgahal': ('Echo Of Ichgahal', 'Echo of Ichgahal', 103146),
    'quests/rotten_blood/echo_of_murcion': ('Echo Of Murcion', 'Echo of Murcion', 103142),
    'quests/rotten_blood/echo_of_vemiath': ('Echo Of Vemiath', 'Echo of Vemiath', 103149),
    'familiars/knight_familiar': ('Knight familiar', 'Knight Familiar', 80840),
    'familiars/monk_familiar': ('Monk familiar', 'Monk Familiar', 108515),
    'quests/grave_danger/nargol_the_impaler': ('Nargol The Impaler', 'Nargol the Impaler', 90249),
    'familiars/paladin_familiar': ('Paladin familiar', 'Paladin Familiar', 80842),
    'bosses/raging_mage': ('Raging mage', 'Raging Mage', 53983),
    'quests/grave_danger/rewar_the_bloody': ('Rewar The Bloody', 'Rewar the Bloody', 90186),
    'familiars/sorcerer_familiar': ('Sorcerer familiar', 'Sorcerer Familiar', 80844),
}


# Exact pinned render carrier supplements the shared Butterfly disambiguation/race proof.
CREATURE_SOURCE_LOOKS = {'quests/the_explorer_society/pink_butterfly': 213}
# Source encounter scripts and dated Creature pages independently identify these actor forms.
CREATURE_CONTEXT_ROUTES = {
    'raids/egg_the_welter': {
        'registration': 'Egg', 'title': 'Egg (Creature)', 'page_id': 65534,
        'look_type_ex': 4839, 'health': 800, 'event': 'TheWelterEgg',
        'wiki_context_field': 'notes', 'wiki_context_link': 'The Welter',
    },
    'quests/dangerous_depth/fiery_heart': {
        'registration': 'Fiery Heart', 'title': 'Fiery Heart (Creature)', 'page_id': 83582,
        'look_type_ex': 391, 'health': 7500, 'event': None,
        'wiki_context_field': 'location', 'wiki_context_link': 'Warzone 6',
    },
}


# Explicit encounter forms proven by pinned donor files and dated canonical Creature context.
# Wiki external race IDs are not substituted for absent source raceId.
CREATURE_ENCOUNTER_FORM_ROUTES = {
    'quests/pits_of_inferno/demon_goblin': {
        'registration': 'Demon Goblin', 'display': 'Demon Goblin',
        'title': 'Demon (Goblin)', 'page_id': 14110, 'actualname': 'demon',
        'outfit': {'lookType': 35, 'lookHead': 0, 'lookBody': 0, 'lookLegs': 0,
                   'lookFeet': 0, 'lookAddons': 0, 'lookMount': 0},
        'source_fields': {'health': 50, 'maxHealth': 50, 'experience': 25, 'speed': 75, 'corpse': 5995},
        'physical_percent': 0,
        'context_links': {'location': ['Pits of Inferno'], 'notes': ['Goblin']},
        'context_phrases': {'notes': ['demon outfit and name']},
    },
    'quests/cults_of_tibia/bosses/the_sinister_hermit_clean': {
        'registration': 'The Sinister Hermit', 'display': 'The Sinister Hermit',
        'title': 'The Sinister Hermit (Blue)', 'page_id': 82265, 'actualname': 'The Sinister Hermit',
        'outfit': {'lookType': 153, 'lookHead': 0, 'lookBody': 85, 'lookLegs': 79,
                   'lookFeet': 9, 'lookAddons': 3, 'lookMount': 0},
        'source_fields': {'health': 30000, 'maxHealth': 30000, 'experience': 0, 'speed': 125, 'corpse': 0},
        'physical_percent': 0,
        'context_links': {'notes': ['Cults of Tibia Quest'], 'strategy': ['The Sinister Hermit (Yellow)']},
        'context_phrases': {'strategy': ['boss is vulnerable', 'not standing on the geyser']},
    },
    'quests/cults_of_tibia/bosses/the_sinister_hermit_dirty': {
        'registration': 'The Sinister Hermit Dirty', 'display': 'The Sinister Hermit',
        'title': 'The Sinister Hermit (Yellow)', 'page_id': 82264, 'actualname': 'The Sinister Hermit',
        'outfit': {'lookType': 153, 'lookHead': 0, 'lookBody': 97, 'lookLegs': 79,
                   'lookFeet': 9, 'lookAddons': 3, 'lookMount': 0},
        'source_fields': {'health': 30000, 'maxHealth': 30000, 'experience': 0, 'speed': 125, 'corpse': 0},
        'physical_percent': 100,
        'context_links': {'notes': ['Cults of Tibia Quest'], 'strategy': ['The Sinister Hermit (Blue)']},
        'context_phrases': {'strategy': ['invulnerable to physical damage', 'standing on a geyser']},
    },
}


# These dated Creature pages explicitly describe the raid, summon or encounter form.
# The complete pinned donor table identifies the source variant; it does not inherit mitigation.
# Only mitigation is projected: source armor, resistances, loot and mechanics retain their evidence.
CREATURE_MITIGATION_VARIANTS = {'quests/forgotten_knowledge/cosmic_energy_prism_a_invu': {'registration': 'Cosmic Energy Prism A Invu',
                                                           'display': 'Cosmic Energy Prism A',
                                                           'source_table_sha256': 'f77096f1cb3010c839388bfb5a30c786b2c1c96cf6903d2d8dffdf400ad26892',
                                                           'title': 'Cosmic Energy Prism A',
                                                           'page_id': 79847,
                                                           'wiki_race': None,
                                                           'context_links': {'notes': ['Lloyd',
                                                                                       'Forgotten Knowledge '
                                                                                       'Quest'],
                                                                             'strategy': ['Lloyd']},
                                                           'context_phrases': ['they can only be killed '
                                                                               'one-by-one']},
 'quests/forgotten_knowledge/cosmic_energy_prism_b_invu': {'registration': 'Cosmic Energy Prism B Invu',
                                                           'display': 'Cosmic Energy Prism B',
                                                           'source_table_sha256': '04405142f16b22bdd90909c6dcc657894df476890655adfb2a7836ac476d99e6',
                                                           'title': 'Cosmic Energy Prism B',
                                                           'page_id': 79848,
                                                           'wiki_race': None,
                                                           'context_links': {'notes': ['Lloyd',
                                                                                       'Forgotten Knowledge '
                                                                                       'Quest'],
                                                                             'strategy': ['Lloyd']},
                                                           'context_phrases': ['they can only be killed '
                                                                               'one-by-one']},
 'quests/forgotten_knowledge/cosmic_energy_prism_c_invu': {'registration': 'Cosmic Energy Prism C Invu',
                                                           'display': 'Cosmic Energy Prism C',
                                                           'source_table_sha256': '69f864b39fb5d8afca7d3bd26820981cacafd15bb6344a68976396ee82c59875',
                                                           'title': 'Cosmic Energy Prism C',
                                                           'page_id': 79849,
                                                           'wiki_race': None,
                                                           'context_links': {'notes': ['Lloyd',
                                                                                       'Forgotten Knowledge '
                                                                                       'Quest'],
                                                                             'strategy': ['Lloyd']},
                                                           'context_phrases': ['they can only be killed '
                                                                               'one-by-one']},
 'quests/forgotten_knowledge/cosmic_energy_prism_d_invu': {'registration': 'Cosmic Energy Prism D Invu',
                                                           'display': 'Cosmic Energy Prism D',
                                                           'source_table_sha256': 'dae001fec9f06506b4ff0dee7f968d5bb77b499767b076fbb5cdb44765f5db78',
                                                           'title': 'Cosmic Energy Prism D',
                                                           'page_id': 79850,
                                                           'wiki_race': None,
                                                           'context_links': {'notes': ['Lloyd',
                                                                                       'Forgotten Knowledge '
                                                                                       'Quest'],
                                                                             'strategy': ['Lloyd']},
                                                           'context_phrases': ['they can only be killed '
                                                                               'one-by-one']},
 'raids/orc_armor': {'registration': 'Orc Armor',
                     'display': 'Orc Warlord',
                     'source_table_sha256': '61c7d13fac503f9ce5d20ba286c834663918693de9960bf707a10e484c800fb8',
                     'title': 'Orc Warlord',
                     'page_id': 1500,
                     'wiki_race': '2',
                     'context_links': {'strategy': ['Amazon Armor', 'Femor Hills']},
                     'context_phrases': ['from orc warlord in raids']},
 'raids/orc_helmet': {'registration': 'Orc Helmet',
                      'display': 'Orc Warlord',
                      'source_table_sha256': '38a1776e57129a3e6a1336cdd4b8248aa5e1b17b41701b92e2fc0730ff7fd21c',
                      'title': 'Orc Warlord',
                      'page_id': 1500,
                      'wiki_race': '2',
                      'context_links': {'strategy': ['Amazon Helmet', 'Femor Hills']},
                      'context_phrases': ['from orc warlord in raids']},
 'raids/orc_shield': {'registration': 'Orc Shield',
                      'display': 'Orc Warlord',
                      'source_table_sha256': '0caf95f99278963befcb25d8f427bb8baf29f0dd9d788beadbcfc48346ab6dc1',
                      'title': 'Orc Warlord',
                      'page_id': 1500,
                      'wiki_race': '2',
                      'context_links': {'strategy': ['Amazon Shield', 'Femor Hills']},
                      'context_phrases': ['from orc warlord in raids']},
 'raids/orc_sambackpack': {'registration': 'Orc Sambackpack',
                           'display': 'Orc',
                           'source_table_sha256': '2bb77feec177a249efb559b955cc280895fda2093be09235374bb8e05ebe0c98',
                           'title': 'Orc',
                           'page_id': 1178,
                           'wiki_race': '5',
                           'context_links': {'notes': ['Old and Used Backpack']},
                           'context_phrases': ['rarely spawns']},
 'dawnport/spidris_elitesumom': {'registration': 'Spidris Elite Summon',
                                 'display': 'Spidris Elite',
                                 'source_table_sha256': '9121861e26098e4d9ba3ef5060ecc5c804a8abbcae9cd36aa51872531c3beca1',
                                 'title': 'Spidris Elite',
                                 'page_id': 57285,
                                 'wiki_race': '797',
                                 'context_links': {'notes': ['Hive Overseer'], 'location': ['Hive Overseer']},
                                 'context_phrases': ['also summoned by']}}


def mitigation_number(raw):
    """A certain complete decimal percent token; ranges, prose and approximation never become facts."""
    if not isinstance(raw, str) or not re.fullmatch(r'(?:\d+(?:\.\d+)?|\.\d+)\s*%?', raw.strip()):
        return None
    value = Fraction(raw.strip().rstrip('%').strip())
    return value if 0 <= value <= 100 else None


# Canary's horse registration colors differ from the canonical Bestiary colors.
# Exact race IDs, not the registration names, bind these two mitigation-only routes.
CREATURE_MITIGATION_RACE_ROUTES = {
    'mammals/grey_horse': {
        'registration': 'Grey Horse', 'race_id': 751, 'look_type': 434,
        'title': 'Horse (Brown)', 'page_id': 85687,
        'source_table_sha256': '23f719da7b8d5f7fc6af5b5f2c270deec64b479de8aab075da1924a292473b7f',
    },
    'mammals/brown_horse': {
        'registration': 'Brown Horse', 'race_id': 752, 'look_type': 436,
        'title': 'Horse (Taupe)', 'page_id': 53979,
        'source_table_sha256': 'a8c5969958d57ac4ca4850761a2f0dd06fabf9b40daeab00ae11874af68da5e8',
    },
}


def mitigation_race_page(relative, name, source, cache):
    route = CREATURE_MITIGATION_RACE_ROUTES[relative]
    binding = {'method': 'PinnedSourceRaceAndDatedCreatureMitigation', 'source_file': relative,
               'source_registration_name': name, 'field_scope': ['mitigation_percent'],
               'source_mitigation_inherited': False}
    try:
        fingerprint = hashlib.sha256(json.dumps(source, sort_keys=True, separators=(',', ':'),
                                               ensure_ascii=False).encode('utf-8')).hexdigest()
    except (TypeError, ValueError):
        fingerprint = None
    outfit = source.get('outfit')
    if (name != route['registration'] or source.get('name') != 'Horse'
            or type(source.get('raceId')) is not int or source['raceId'] != route['race_id']
            or not isinstance(outfit, dict) or type(outfit.get('lookType')) is not int
            or outfit['lookType'] != route['look_type']
            or fingerprint != route['source_table_sha256']):
        return None, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                      'reason': 'qualified mitigation race source fingerprint mismatch'}
    record = fetch(route['title'], cache)
    cut = record.get('cut')
    if not eligible_cut_revision(cut):
        return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                        'reason': 'qualified mitigation race Creature revision missing or after reference cut'}
    fields = infobox(cut['content'])
    if (not fields or cut['page_id'] != route['page_id'] or cut['title'] != route['title']
            or fields.get('name') != route['title'] or fields.get('actualname') != 'horse'
            or fields.get('race_id', '').strip() != str(route['race_id'])
            or mitigation_number(fields.get('mitigation')) is None):
        return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                        'reason': 'canonical mitigation race Creature identity or numeric value mismatch'}
    return record, {**binding, 'status': 'VERIFIED', 'source_race_id': route['race_id'],
                    'source_look_type': route['look_type'], 'source_table_sha256': fingerprint,
                    'source_display_name': 'Horse',
                    'canonical_color_name_differs_from_registration': True}


def mitigation_variant_page(relative, name, source, cache):
    route = CREATURE_MITIGATION_VARIANTS[relative]
    binding = {'method': 'PinnedVariantAndDatedCreatureMitigation', 'source_file': relative,
               'source_registration_name': name, 'field_scope': ['mitigation_percent'],
               'source_mitigation_inherited': False}
    try:
        fingerprint = hashlib.sha256(json.dumps(source, sort_keys=True, separators=(',', ':'),
                                               ensure_ascii=False).encode('utf-8')).hexdigest()
    except (TypeError, ValueError):
        fingerprint = None
    if (name != route['registration'] or source.get('name') != route['display']
            or 'raceId' in source or fingerprint != route['source_table_sha256']):
        return None, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                      'reason': 'qualified mitigation variant source fingerprint mismatch'}
    record = fetch(route['title'], cache)
    cut = record.get('cut')
    if not cut or not eligible_cut_revision(cut):
        return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                        'reason': 'qualified mitigation Creature revision missing, malformed or after reference cut'}
    fields = infobox(cut.get('content', ''))
    if (not fields or cut.get('page_id') != route['page_id'] or cut.get('title') != route['title']
            or fields.get('name', '').casefold() != route['title'].casefold()
            or fields.get('actualname', '').casefold() != route['display'].casefold()
            or (route['wiki_race'] is not None and fields.get('race_id', '').strip() != route['wiki_race'])):
        return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                        'reason': 'canonical mitigation Creature page or declared identity mismatch'}
    for field, links in route['context_links'].items():
        observed = set(re.findall(r'\[\[([^\]|#]+)(?:[^\]]*)\]\]', fields.get(field, '')))
        if not set(links) <= observed:
            return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                            'reason': 'Creature page does not explicitly describe the qualified variant context'}
    context = ' '.join(fields.get(field, '') for field in route['context_links']).casefold()
    if (any(phrase not in context for phrase in route['context_phrases'])
            or mitigation_number(fields.get('mitigation')) is None):
        return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                        'reason': 'qualified variant context or certain numeric mitigation missing'}
    return record, {**binding, 'status': 'VERIFIED', 'source_display_name': route['display'],
                    'source_table_sha256': fingerprint, 'wiki_context_links': route['context_links']}


# One canonical Creature omits its race ID. The accepted client import independently
# binds its ID and rendered look; this is not a generic name-based race fallback.
CREATURE_STATICDATA_ROUTES = {
    'winter_update_2025/imperial': {
        'registration': 'Imperial', 'actualname': 'imperial', 'race_id': 2775, 'look_type': 1914,
        'page_id': 109642, 'title': 'Imperial',
        'source_table_sha256': '013a070ff774f9eb07efcc490146f9f27b6fd13a2ced9da9b9d28a83c40ad359',
        'staticdata_path': 'imports/cipsoft-staticdata/creatures/creatures-00750-00832.json',
        'staticdata_file_sha256': 'eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6',
        'staticdata_record': {
            'f4': 1, 'f5': 3, 'f6': 1, 'f7': 0,
            'look': {'hex': '08fa0e120808001000180020001800'},
            'name': 'imperial', 'source_id': 2775, 'source_index': 797,
            'staticdata_sha256': '3bd84c94e1a00e88911b2a7f03f8af636f5dad6e2c942f60f277543ace0d583f',
        },
        'cut_content_sha256': 'ac83a4be2fcc81b837eb51ea33cb4d99ec16a864f10909be8f5e2c08edf59bfd',
    },
}


# Generated exact source/client/canonical-cut identity routes from the reviewed bulk packet.
# No numeric Wiki race IDs are fabricated; callbacks require the full pinned file guard.
CREATURE_STATICDATA_ROUTES.update(json.loads(r'''
{
  "quests/rotten_blood/meandering_mushroom": {
    "registration": "Meandering Mushroom",
    "actualname": "meandering mushroom",
    "race_id": 2376,
    "look_type": 1621,
    "page_id": 102960,
    "title": "Meandering Mushroom",
    "source_table_sha256": "153c5daae0fa911eeaad22b090bc4068a3585618193c5535434bd3ac6931978c",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00500-00749.json",
    "staticdata_file_sha256": "2c81949301abe20b4867220decc7195182c43470e0d976d6b46e13c0b062b04b",
    "staticdata_record": {
      "f4": 5,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08d50c120808001000180020001800"
      },
      "name": "meandering mushroom",
      "source_id": 2376,
      "source_index": 705,
      "staticdata_sha256": "c1ca3d8e6507b2cfe225b68a1002831d1289c3a0d200a0702f60546f65da1655"
    },
    "cut_content_sha256": "786f1dc42aea6698b8bfd01209d2ff271d9f98412d9c23f310fe98b27b975659",
    "reference_cut_only": true
  },
  "inkborn/cinder_wyrmling": {
    "registration": "Cinder Wyrmling",
    "actualname": "cinder wyrmling",
    "race_id": 2670,
    "look_type": 1850,
    "page_id": 108817,
    "title": "Cinder Wyrmling",
    "source_table_sha256": "88c4a7baafd73f94986cdf332b7a4f8c7e0af110339243539d38879688f683cd",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08ba0e1208084d105e185e204f1801"
      },
      "name": "cinder wyrmling",
      "source_id": 2670,
      "source_index": 774,
      "staticdata_sha256": "9dedd67322ef4e06e57a0ce077c92e7ea1118edddabe3408ec8637a5f58f2bbb"
    },
    "cut_content_sha256": "b2010a4f3bc3dfc7d402d1d30db9418b89088dc5d9678c18d6d6797639af750c",
    "reference_cut_only": true
  },
  "winter_update_2025/creepy_crawler": {
    "registration": "Creepy Crawler",
    "actualname": "creepy crawler",
    "race_id": 2763,
    "look_type": 1890,
    "page_id": 109636,
    "title": "Creepy Crawler",
    "source_table_sha256": "e6816f4997e66c5b85ff27f8c1d51a32aaff7261fe7459297594e5038dc9222e",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08e20e120808001000180020001800"
      },
      "name": "creepy crawler",
      "source_id": 2763,
      "source_index": 792,
      "staticdata_sha256": "659fdb405bc92fb3004bb31d416e813f5d1f96fe4cd11b73b04a7d9ec1ccf774"
    },
    "cut_content_sha256": "6cf1771087a93ac1cb70608ce2c690ce1d36e3a7e0e2489ce60b7018d44ffcaf",
    "reference_cut_only": true
  },
  "winter_update_2025/crypt_construct": {
    "registration": "Crypt Construct",
    "actualname": "crypt construct",
    "race_id": 2760,
    "look_type": 1887,
    "page_id": 109637,
    "title": "Crypt Construct",
    "source_table_sha256": "9e8e8b2b923b626dcdd78d3a6b6e895599ac5df7edab094cc61eda82f2dbd538",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08df0e120808001000180020001800"
      },
      "name": "crypt construct",
      "source_id": 2760,
      "source_index": 790,
      "staticdata_sha256": "d2ceadc7505138f815265bbd76d50aa0e2290a83058064e8162f8b3d96b57555"
    },
    "cut_content_sha256": "d0891817f5cbb8ab7331ea345f38d9851d2da2826132c24eaf6744ab460f0777",
    "reference_cut_only": true
  },
  "winter_update_2025/crypt_fiend": {
    "registration": "Crypt Fiend",
    "actualname": "crypt fiend",
    "race_id": 2758,
    "look_type": 1885,
    "page_id": 109638,
    "title": "Crypt Fiend",
    "source_table_sha256": "a708dc25e2d1a4b745b47bc57bbf14c8aa18e841470c4d5a8a65e8c33c4dbb65",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08dd0e120808001000180020001800"
      },
      "name": "crypt fiend",
      "source_id": 2758,
      "source_index": 788,
      "staticdata_sha256": "47827983c84eb0052a01d71bcddb171408261dfa59211bc9c47c7fc893ea797a"
    },
    "cut_content_sha256": "86a7c4d2067d13d8634264c71b950eb39adc1b2de745c82a2a87f0cd2661f96e",
    "reference_cut_only": true
  },
  "winter_update_2025/crypt_mage": {
    "registration": "Crypt Mage",
    "actualname": "Crypt Mage",
    "race_id": 2766,
    "look_type": 1905,
    "page_id": 109639,
    "title": "Crypt Mage",
    "source_table_sha256": "f2b9281f00e4efd8db518b935df911d04a7c6612df6daa577523e0f7b8263892",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08f10e120808001000180020001800"
      },
      "name": "crypt mage",
      "source_id": 2766,
      "source_index": 795,
      "staticdata_sha256": "f9b45495740a21cc09782543b4f373d9f29fb5a3d04de5d65c548dc7553c0d25"
    },
    "cut_content_sha256": "d35a1c5fa4f7d217cb3f1cf7cb6126ef67d8b963785ee92b0ac44055fd005c38",
    "reference_cut_only": true
  },
  "winter_update_2025/cyclursus": {
    "registration": "Cyclursus",
    "actualname": "",
    "race_id": 2757,
    "look_type": 1884,
    "page_id": 109640,
    "title": "Cyclursus",
    "source_table_sha256": "56a7f577a32a5a86ebb927e9c0f85fe1c1889a68809487d162e2513c272a3361",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08dc0e120808001000180020001800"
      },
      "name": "cyclursus",
      "source_id": 2757,
      "source_index": 787,
      "staticdata_sha256": "1b4a8b3b0c001989fd04b9b7f9150ff954456019e9d4b3987e6265ac9c0dabb2"
    },
    "cut_content_sha256": "12ec09c4f0b01114b54b47819b3ee15b0ca6fb1fa8ea83100400498912091d5f",
    "reference_cut_only": true
  },
  "summer_update_2026/devoted_radiant_acolyte": {
    "registration": "Devoted Radiant Acolyte",
    "actualname": "devoted radiant acolyte",
    "race_id": 2844,
    "look_type": 1969,
    "page_id": 110684,
    "title": "Devoted Radiant Acolyte",
    "source_table_sha256": "18574f9acd65f3e0e19e8aa1ac6a3d863de1439a9109d684d8f58ea6b5f7e4d7",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 1,
      "f6": 1,
      "f7": 0,
      "look": {
        "hex": "08b10f120808001000180020001803"
      },
      "name": "devoted radiant acolyte",
      "source_id": 2844,
      "source_index": 821,
      "staticdata_sha256": "6ab213f5096a6fd7e06bd40d1042d7160c2555bf04eb8ff4228cb967fecc2e9e"
    },
    "cut_content_sha256": "fcaeec5dffe282796251ffcef5ab5b7d1a41a1c03ff72a047675fbcb7513e79d",
    "reference_cut_only": true
  },
  "summer_update_2026/devoted_radiant_inquisitor": {
    "registration": "Devoted Radiant Inquisitor",
    "actualname": "devoted radiant inquisitor",
    "race_id": 2848,
    "look_type": 1967,
    "page_id": 110686,
    "title": "Devoted Radiant Inquisitor",
    "source_table_sha256": "048f5b0daa4b10a95f846c9181d416711f20db787b2dcc385145e3d950dcbcf7",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 1,
      "f6": 1,
      "f7": 0,
      "look": {
        "hex": "08af0f120808001000180020001803"
      },
      "name": "devoted radiant inquisitor",
      "source_id": 2848,
      "source_index": 825,
      "staticdata_sha256": "8f24e053b6e553659912e58e0b5c0652db7c5f3b168019993c9f394e3fa1cc83"
    },
    "cut_content_sha256": "4aac329ae89fba549e80cc1c51ee595acbca5583d2e1e838aef322376e65a1b6",
    "reference_cut_only": true
  },
  "summer_update_2026/devoted_radiant_paragon": {
    "registration": "Devoted Radiant Paragon",
    "actualname": null,
    "race_id": 2842,
    "look_type": 1966,
    "page_id": 110688,
    "title": "Devoted Radiant Paragon",
    "source_table_sha256": "d9dcc4a276b7dca9be80a67f01b49441778c63bcbf7a9b88fd36c3550ae634d1",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 1,
      "f6": 1,
      "f7": 0,
      "look": {
        "hex": "08ae0f120808001000180020001801"
      },
      "name": "devoted radiant paragon",
      "source_id": 2842,
      "source_index": 819,
      "staticdata_sha256": "652f7fbaf1375eb069b8f9a26f05f22cb5daa6fcf54a242df0841d7bcf4303ae"
    },
    "cut_content_sha256": "43e6f002b95a47e579ec56d921aea8bd75b652e7386f060a793b9e2e4a75b18d",
    "reference_cut_only": true
  },
  "summer_update_2026/devoted_radiant_templar": {
    "registration": "Devoted Radiant Templar",
    "actualname": "devoted radiant templar",
    "race_id": 2846,
    "look_type": 1965,
    "page_id": 110690,
    "title": "Devoted Radiant Templar",
    "source_table_sha256": "785febed38edfa4a7e1f23233e4b83609f26558e16357314c048b1a0afa11edf",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 1,
      "f6": 1,
      "f7": 0,
      "look": {
        "hex": "08ad0f120808001000180020001803"
      },
      "name": "devoted radiant templar",
      "source_id": 2846,
      "source_index": 823,
      "staticdata_sha256": "8ab642ecb256b900bc744888795fd3d9fbb581f30ddf250d5e3ff96fb82d98cc"
    },
    "cut_content_sha256": "d59058e7bf1fcab70e751cde8fbef3bfce927c9e7d2a9978d299d064e01ee6b4",
    "reference_cut_only": true
  },
  "summer_update_2026/devoted_radiant_warden": {
    "registration": "Devoted Radiant Warden",
    "actualname": "devoted radiant warden",
    "race_id": 2840,
    "look_type": 1964,
    "page_id": 110692,
    "title": "Devoted Radiant Warden",
    "source_table_sha256": "407b8945accd8f9ab87b172c86a06e75b1b28ffb0093a1bb6bae5bc56b4978df",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 1,
      "f6": 1,
      "f7": 0,
      "look": {
        "hex": "08ac0f120808001000180020001801"
      },
      "name": "devoted radiant warden",
      "source_id": 2840,
      "source_index": 817,
      "staticdata_sha256": "555d854bc4bbfdb73f13bd739cb2bc973b573448dc488e73b3e94dab3b99a36e"
    },
    "cut_content_sha256": "703769e6bd887870df6fb1742a27a8d03f04231deec2f04b84bf72f85f755db0",
    "reference_cut_only": true
  },
  "summer_update_2026/devoted_radiant_zealot": {
    "registration": "Devoted Radiant Zealot",
    "actualname": "devoted radiant zealot",
    "race_id": 2850,
    "look_type": 1968,
    "page_id": 110694,
    "title": "Devoted Radiant Zealot",
    "source_table_sha256": "094f2c078225d13cf885b32d88ccfcecde2d9eb8ea06e25a175030518bd0a984",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 1,
      "f6": 1,
      "f7": 0,
      "look": {
        "hex": "08b00f120808001000180020001803"
      },
      "name": "devoted radiant zealot",
      "source_id": 2850,
      "source_index": 827,
      "staticdata_sha256": "f7002ae727ea1f1bb21e313d3eb2b517c5ec19178a41fb0926dc922a3c3055f0"
    },
    "cut_content_sha256": "42dfe3f09a980e535b564de12d509836a3b1a9a5957777728fc89ad3e3ea9549",
    "reference_cut_only": true
  },
  "winter_update_2025/haunted_hunter": {
    "registration": "Haunted Hunter",
    "actualname": "haunted hunter",
    "race_id": 2762,
    "look_type": 1889,
    "page_id": 109641,
    "title": "Haunted Hunter",
    "source_table_sha256": "a473e018dcc862fe4f12c018306ab3e3fb99de0158b71a513e27e66915cb5a0d",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08e10e120808001000180020001800"
      },
      "name": "haunted hunter",
      "source_id": 2762,
      "source_index": 791,
      "staticdata_sha256": "a3523ddf0c2528c12049cffaa204eb8e3f1f929821a2d3df7a4116b01c48b5d3"
    },
    "cut_content_sha256": "b3e4a49e72775f71c7fc8bf92c8e0145deb59218cf480c8ac5eab8fb90c45fb7",
    "reference_cut_only": true
  },
  "summer_update_2026/iceplume_strider": {
    "registration": "Iceplume Strider",
    "actualname": "iceplume strider",
    "race_id": 2803,
    "look_type": 1950,
    "page_id": 110696,
    "title": "Iceplume Strider",
    "source_table_sha256": "586dd38877e4091f69a7db75e792dff9865c982cf30f03aaada550b2ea25eef9",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "089e0f120808001033180020001800"
      },
      "name": "iceplume strider",
      "source_id": 2803,
      "source_index": 813,
      "staticdata_sha256": "c49ebc31a7b182bcae6e788c6367255587f54c563b1530d6cacc211480e5c7d7"
    },
    "cut_content_sha256": "98da2e06517ecd35439145bb0690036e4b26f2cf671a4414c67b0c4072261861",
    "reference_cut_only": true
  },
  "inkborn/ink_splash": {
    "registration": "Ink Splash",
    "actualname": "ink splash",
    "race_id": 2639,
    "look_type": 1064,
    "page_id": 108944,
    "title": "Ink Splash",
    "source_table_sha256": "bd24027160239b1096f284e6e235c10572aca28f43f56be6d42f8ac28ef1e7cf",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 3,
      "f5": 2,
      "f6": 1,
      "f7": 0,
      "look": {
        "hex": "08a808120808001000180020001800"
      },
      "name": "ink splash",
      "source_id": 2639,
      "source_index": 767,
      "staticdata_sha256": "0e1338071c4233893d15a87c03939b662bcb541c24b121ca72f3bc28f725eb7c"
    },
    "cut_content_sha256": "dac45373daca6383a1da6c4d70d38e67a405b11f73f94e630553cbd4d60f294f",
    "reference_cut_only": true
  },
  "summer_update_2026/jaracal": {
    "registration": "Jaracal",
    "actualname": "jaracal",
    "race_id": 2856,
    "look_type": 1961,
    "page_id": 110698,
    "title": "Jaracal",
    "source_table_sha256": "7690da407a73e3b9151254c0438e0a7499f54397267b3f6af11c97cba9570016",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08a90f120808521061184c204c1800"
      },
      "name": "jaracal",
      "source_id": 2856,
      "source_index": 830,
      "staticdata_sha256": "959fc4232d616c6fc2b81879ca3f18786470fd689ff07b0ca52cafd38c9792f0"
    },
    "cut_content_sha256": "1ec8d3d9737ceb7ec3db8f11c9c6822a540c1d02931235d57357afdbacd0eb6d",
    "reference_cut_only": true
  },
  "summer_update_2026/moonspawn_blightspitter": {
    "registration": "Moonspawn Blightspitter",
    "actualname": "moonspawn blightspitter",
    "race_id": 2851,
    "look_type": 1970,
    "page_id": 110700,
    "title": "Moonspawn Blightspitter",
    "source_table_sha256": "f26fef3c9baf5e10a2154b7c5938abaa5bdc1fde350dcaf0e9c2b5d8f1e9ee2b",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08b20f120808001000180020001800"
      },
      "name": "moonspawn blightspitter",
      "source_id": 2851,
      "source_index": 828,
      "staticdata_sha256": "99bfbdab4b190cdec6d467ac753d4131df5313fabb0f6b156ae6d11f22488939"
    },
    "cut_content_sha256": "c9969a04f8229ec4f329c0744c255e38000183131fecc7fe1186163b52cc6c52",
    "reference_cut_only": true
  },
  "summer_update_2026/moonspawn_oozecrown": {
    "registration": "Moonspawn Oozecrown",
    "actualname": "moonspawn oozecrown",
    "race_id": 2852,
    "look_type": 1971,
    "page_id": 110702,
    "title": "Moonspawn Oozecrown",
    "source_table_sha256": "257c82763603ee00662ca1a0540860eeeb02f3e32bc7f8a05bbf7f9ee35ca039",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08b30f120808001000180020001800"
      },
      "name": "moonspawn oozecrown",
      "source_id": 2852,
      "source_index": 829,
      "staticdata_sha256": "94a6c88dda2ce019cc79e90aecd12055f28dcca2eaf9cc73d0ad27e03a06bcb4"
    },
    "cut_content_sha256": "2f3a5c835edb41a51ca28b97c19575665fc8fb6e656b6b9aae36064ff9184b3c",
    "reference_cut_only": true
  },
  "summer_update_2026/moonstone_overseer": {
    "registration": "Moonstone Overseer",
    "actualname": "moonstone overseer",
    "race_id": 2858,
    "look_type": 1956,
    "page_id": 110706,
    "title": "Moonstone Overseer",
    "source_table_sha256": "c3602495b35837a338bc82b3c0457132e9eacfe0eca4116b10543fb2246ed5d3",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08a40f120808001000180020001800"
      },
      "name": "moonstone overseer",
      "source_id": 2858,
      "source_index": 832,
      "staticdata_sha256": "a382e7b7a1968ea443ef4227b104d8b0f6244db2c03089a309e96a9a32ffd35b"
    },
    "cut_content_sha256": "225ca11406548d8cb126148506b3f2384d1139659a2f1585a4c6127a3da71cd6",
    "reference_cut_only": true
  },
  "winter_update_2025/night_harpy": {
    "registration": "Night Harpy",
    "actualname": "night harpy",
    "race_id": 2764,
    "look_type": 1899,
    "page_id": 109643,
    "title": "Night Harpy",
    "source_table_sha256": "b1490bede6f722fe4390d3bc168e0484b3f74d36d7cab67cc3a7806e326f2183",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 1,
      "f6": 1,
      "f7": 0,
      "look": {
        "hex": "08eb0e120808001000180020001800"
      },
      "name": "night harpy",
      "source_id": 2764,
      "source_index": 793,
      "staticdata_sha256": "a58f02b64f849e6a3e80f30b137d5e24db94fb0015d0d89fd391c066be3603fc"
    },
    "cut_content_sha256": "276a9142de39a1a000cdd9ea91acb9ee347f0c3dbba7995ef87943c92eb25b5f",
    "reference_cut_only": true
  },
  "summer_update_2026/radiant_acolyte": {
    "registration": "Radiant Acolyte",
    "actualname": "radiant acolyte",
    "race_id": 2843,
    "look_type": 1969,
    "page_id": 110708,
    "title": "Radiant Acolyte",
    "source_table_sha256": "a8a207702710b9f55e2dcd64e69b9a9a23ece37d4cbfa50b21a8d95419cf6be2",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08b10f120808001000180020001800"
      },
      "name": "radiant acolyte",
      "source_id": 2843,
      "source_index": 820,
      "staticdata_sha256": "86bf7c574b211d8bff14b5686bef5d1493af445532bcb98d2cf331629b6218aa"
    },
    "cut_content_sha256": "7efe854782a5936640777ddca92832580dce6bea45e00d655983c4b16581a5ef",
    "reference_cut_only": true
  },
  "summer_update_2026/radiant_inquisitor": {
    "registration": "Radiant Inquisitor",
    "actualname": "radiant inquisitor",
    "race_id": 2847,
    "look_type": 1967,
    "page_id": 110710,
    "title": "Radiant Inquisitor",
    "source_table_sha256": "d2464d302aa52cbbd0676d94542539ee3d3de47ce1fdf733ca148b7fb8477678",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08af0f120808001000180020001800"
      },
      "name": "radiant inquisitor",
      "source_id": 2847,
      "source_index": 824,
      "staticdata_sha256": "5ffab723e63b0b71b5f169739633227ccb703c9bda01bde33a2aadd1cff927c5"
    },
    "cut_content_sha256": "49a94eed0803adf738481c12d176b735ee65f0e23dee6fcce2ea8f390d569652",
    "reference_cut_only": true
  },
  "summer_update_2026/radiant_paragon": {
    "registration": "Radiant Paragon",
    "actualname": "radiant paragon",
    "race_id": 2841,
    "look_type": 1966,
    "page_id": 110712,
    "title": "Radiant Paragon",
    "source_table_sha256": "347a3717004abd8a0155a78ea6b64ba814839c129859b8ab622f697bc46426af",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08ae0f120808001000180020001802"
      },
      "name": "radiant paragon",
      "source_id": 2841,
      "source_index": 818,
      "staticdata_sha256": "d74a48e8b26895bbf61cbf04ac192761de3e8c1147c076ae682561adab65a85b"
    },
    "cut_content_sha256": "6c4825924fa5eda93bed26f68f3e66d02d339b4baf02a8da4be84861d250cd07",
    "reference_cut_only": true
  },
  "summer_update_2026/radiant_templar": {
    "registration": "Radiant Templar",
    "actualname": "radiant templar",
    "race_id": 2845,
    "look_type": 1965,
    "page_id": 110714,
    "title": "Radiant Templar",
    "source_table_sha256": "1d5c47d7e7d6111f13a0163777bc66fb6dbda4868e0879b63211f438a571540b",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08ad0f120808001000180020001800"
      },
      "name": "radiant templar",
      "source_id": 2845,
      "source_index": 822,
      "staticdata_sha256": "b80323ea3787817c2e1af2c54282497fd968ee673baa38223e210321f26b7043"
    },
    "cut_content_sha256": "2de0459250420fd4265ba713d24799418ca60a0d3026087b70b8a6890c7a7acf",
    "reference_cut_only": true
  },
  "summer_update_2026/radiant_warden": {
    "registration": "Radiant Warden",
    "actualname": "radiant warden",
    "race_id": 2839,
    "look_type": 1964,
    "page_id": 110716,
    "title": "Radiant Warden",
    "source_table_sha256": "d7a3a7d3369afa1b79e3fcc50df036692f53f20be22c148bec253ca246336e4f",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08ac0f120808001000180020001800"
      },
      "name": "radiant warden",
      "source_id": 2839,
      "source_index": 816,
      "staticdata_sha256": "584875df8dca13cbb8e17ccaa0e72d5c14f561f1d6f6ace95c877e2e37a33c5b"
    },
    "cut_content_sha256": "2e2958c90324ca3be34599c7ce6b82453c329570a1754de81ca9498892baa17c",
    "reference_cut_only": true
  },
  "summer_update_2026/radiant_zealot": {
    "registration": "Radiant Zealot",
    "actualname": "radiant zealot",
    "race_id": 2849,
    "look_type": 1968,
    "page_id": 110718,
    "title": "Radiant Zealot",
    "source_table_sha256": "563105548dc5f69161e67655b90d347e506aef027aa9f4a5ed5a90dcc6fad62e",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08b00f120808001000180020001800"
      },
      "name": "radiant zealot",
      "source_id": 2849,
      "source_index": 826,
      "staticdata_sha256": "4e3c739ed6d893a74aff2a4806409e9bb89488650c48b1bd76d1adbf8731682e"
    },
    "cut_content_sha256": "9a2736b948ea08ed144b5a1edb251717464b4e1b7b2a0d449b335b618aab46ae",
    "reference_cut_only": true
  },
  "winter_update_2025/raubritter_chastener": {
    "registration": "Raubritter Chastener",
    "actualname": "raubritter chastener",
    "race_id": 2752,
    "look_type": 1902,
    "page_id": 109644,
    "title": "Raubritter Chastener",
    "source_table_sha256": "fbbde0ddeb44039989317b399aa2ed0582d76c36f0d952657493c8c541b9d84e",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08ee0e1208085e10131815204e1800"
      },
      "name": "raubritter chastener",
      "source_id": 2752,
      "source_index": 786,
      "staticdata_sha256": "14e2be1e3627b9c44c96b4c358d2b7b64c3e083052ad82b6b421d4cbdc244d8e"
    },
    "cut_content_sha256": "9af83e64c252e46b0d2fdf73015dfbc8cc08ea74e48c5188d6039689aaa84c62",
    "reference_cut_only": true
  },
  "winter_update_2025/raubritter_marksman": {
    "registration": "Raubritter Marksman",
    "actualname": "raubritter marksman",
    "race_id": 2751,
    "look_type": 1901,
    "page_id": 109645,
    "title": "Raubritter Marksman",
    "source_table_sha256": "3759e91b09b83b0202d3d25900ab3ad3b67475b0acd9fe59a98c19d7bdf138e1",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08ed0e1208085e1013187620021802"
      },
      "name": "raubritter marksman",
      "source_id": 2751,
      "source_index": 785,
      "staticdata_sha256": "d11ab7154fc3aac25b9999edc6ba8614c5f1e1b7ae15735b76b40d0e1160bfbe"
    },
    "cut_content_sha256": "528364ac5e8850a098840338a67c7fa13a76ffcef5c6b415ba9406972f55179d",
    "reference_cut_only": true
  },
  "winter_update_2025/raubritter_skirmisher": {
    "registration": "Raubritter Skirmisher",
    "actualname": "raubritter skirmisher",
    "race_id": 2750,
    "look_type": 1900,
    "page_id": 109646,
    "title": "Raubritter Skirmisher",
    "source_table_sha256": "0d759243d60d17e26efb40e9e2a877cd897251c620aedc47b7b953a034e2cd7e",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08ec0e1208085e1013180320131800"
      },
      "name": "raubritter skirmisher",
      "source_id": 2750,
      "source_index": 784,
      "staticdata_sha256": "9214c2565044d850ef65dfbf07356d76430024cac743f229608b922307c7a9f1"
    },
    "cut_content_sha256": "65ab6353ec42d600fc5d19dc8a205432632553b8713cc4db8ae27e9125d6c2de",
    "reference_cut_only": true
  },
  "winter_update_2025/roaming_dread": {
    "registration": "Roaming Dread",
    "actualname": null,
    "race_id": 2765,
    "look_type": 1904,
    "page_id": 109647,
    "title": "Roaming Dread",
    "source_table_sha256": "841604b1838ec0bc750637c7514d72c1d3e06e14ce5bf89c920e0a6cdefd832d",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08f00e120808001000180020001800"
      },
      "name": "roaming dread",
      "source_id": 2765,
      "source_index": 794,
      "staticdata_sha256": "a4a901f94927f33a168d34db9fee59249637f17637f0aa8503c8c86cd48ef8e8"
    },
    "cut_content_sha256": "3e2c8a44cc3b05ac241d3d14749119ac5b047900ee3179c00b34dd3bd2d501b8",
    "reference_cut_only": true
  },
  "inkborn/shell_drake": {
    "registration": "Shell Drake",
    "actualname": "shell drake",
    "race_id": 2675,
    "look_type": 1857,
    "page_id": 108818,
    "title": "Shell Drake",
    "source_table_sha256": "87c7aeebfc61654ebdc331727531677f0a8a53913c119e950f0b9abf012256df",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08c10e1208080010501856204e1803"
      },
      "name": "shell drake",
      "source_id": 2675,
      "source_index": 779,
      "staticdata_sha256": "9ddd8747ec327c44c9649a031f113a979afc17148f71bd26696542710333f3f1"
    },
    "cut_content_sha256": "f98ba71f0b7e49a3c0e25f6dae0d3392e5242ce1767a60682d9f536462c1ebf0",
    "reference_cut_only": true
  },
  "summer_update_2026/silverfrost_sentinel": {
    "registration": "Silverfrost Sentinel",
    "actualname": "silverfrost sentinel",
    "race_id": 2804,
    "look_type": 1951,
    "page_id": 110720,
    "title": "Silverfrost Sentinel",
    "source_table_sha256": "2a6139fd217717b4a2cfbb521507d4e920de4b1b32eb3171442da353c8c4561f",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "089f0f120808511071180020001800"
      },
      "name": "silverfrost sentinel",
      "source_id": 2804,
      "source_index": 814,
      "staticdata_sha256": "6aaa4689ffaa0d4e9dc9cfb819bc79ea17b1cb1e8e2018591459af6387480aba"
    },
    "cut_content_sha256": "8ce7debb9593e97eb23ea4a67b9fa7700e257113a545b47316a92637c0491ac0",
    "reference_cut_only": true
  },
  "winter_update_2025/stag": {
    "registration": "Stag",
    "actualname": "stag",
    "race_id": 2774,
    "look_type": 1913,
    "page_id": 109648,
    "title": "Stag",
    "source_table_sha256": "42ba9117a423d99f91feeab849e06c0b454284a0c623491f04ecf4fcd13452cc",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 1,
      "f5": 0,
      "f6": 1,
      "f7": 0,
      "look": {
        "hex": "08f90e120808001000180020001800"
      },
      "name": "stag",
      "source_id": 2774,
      "source_index": 796,
      "staticdata_sha256": "c2495910cfb8531dd5b093a81ef474d8d4bd6e3a6f6588784e9108a4c5beebc4"
    },
    "cut_content_sha256": "d832fd5cef59be436691c23a58ae4bc0e4370f893fdcc619e5c0e66c5e69c5c9",
    "reference_cut_only": true,
    "pinned_callback_keys": [
      "onSpawn"
    ],
    "source_repository": {
      "repository": "zimbadev/crystalserver",
      "revision": "00ce02a57ca5a12e48f32a3476e37471167e4c3f"
    },
    "source_path": "data-global/monster/winter_update_2025/stag.lua",
    "source_file_sha256": "e3ec6434cabac687b502bb69d0a5abd3f40a11fc24ffa92c04d3cef45e777aeb"
  },
  "summer_update_2026/true_feverbloom_asura": {
    "registration": "True Feverbloom Asura",
    "actualname": "true feverbloom asura",
    "race_id": 2805,
    "look_type": 1068,
    "page_id": 110722,
    "title": "True Feverbloom Asura",
    "source_table_sha256": "b2a694d8b2b047a5dc5630f9c7d5544eb1c08833e1e2d2f8897659b8eabaf566",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 1,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08ac081208083e103d1872200f1801"
      },
      "name": "true feverbloom asura",
      "source_id": 2805,
      "source_index": 815,
      "staticdata_sha256": "7125bce34e4a149b5cce17c4e4799793b637af0a50eb0ff2df01dd9b59c501a8"
    },
    "cut_content_sha256": "f925f5e7f2ff696f61ae4f6197e60d2c593db34de159668479da301e8651e02f",
    "reference_cut_only": true
  },
  "winter_update_2025/walking_dread": {
    "registration": "Walking Dread",
    "actualname": "walking dread",
    "race_id": 2759,
    "look_type": 1886,
    "page_id": 109649,
    "title": "Walking Dread",
    "source_table_sha256": "350188ccab42949ef7fbd05ed82539ff5d99fa824de80aaeb203e26e387da426",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 5,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08de0e120808001000180020001800"
      },
      "name": "walking dread",
      "source_id": 2759,
      "source_index": 789,
      "staticdata_sha256": "1582e0c93776902427ec862366f883b01342f26c257d0f74ffd2fd6a1dda989c"
    },
    "cut_content_sha256": "088576c5852e7d5780bf9e4ccb79f4c3277030dc26c884bf6e9ec4f6fc249276",
    "reference_cut_only": true
  },
  "summer_update_2026/winged_jaracal": {
    "registration": "Winged Jaracal",
    "actualname": "winged jaracal",
    "race_id": 2802,
    "look_type": 1961,
    "page_id": 110724,
    "title": "Winged Jaracal",
    "source_table_sha256": "46d682071cc3c6ceb5de06d71d10f50b9df690771c7aa9c04117d983729d5e0a",
    "staticdata_path": "imports/cipsoft-staticdata/creatures/creatures-00750-00832.json",
    "staticdata_file_sha256": "eed18ff5ae82fcefdf6893ff2078ac893451a7fee849c9416b87db594b0044f6",
    "staticdata_record": {
      "f4": 4,
      "f5": 0,
      "f6": 1,
      "f7": 1,
      "look": {
        "hex": "08a90f120808711028185f205f1801"
      },
      "name": "winged jaracal",
      "source_id": 2802,
      "source_index": 812,
      "staticdata_sha256": "051ada1f4b7ab882640c45aea11f0f4aba8fc8d50d895c047fac619bdbe2829f"
    },
    "cut_content_sha256": "cdedb81e808c97074e2b2f76c50d934a06595356212c2bf1bac78e36a933a31b",
    "reference_cut_only": true
  }
}
'''))


def staticdata_creature_page(relative, name, source, cache):
    route = CREATURE_STATICDATA_ROUTES[relative]
    binding = {'method': 'ExactAcceptedStaticdataAndCanonicalCreature', 'source_file': relative,
               'source_registration_name': name, 'wiki_race_id_status': 'UNSPECIFIED'}
    try:
        serialized_source = source
        if route.get('pinned_callback_keys'):
            from lupa.luajit21 import lua_type
            converter = cb.CONVERTER
            if converter.source != route['source_repository']:
                raise ValueError('callback source repository or revision differs')
            source_path = converter.monster_root / route['source_path']
            if str(Path(converter.monster_dir) / (relative + '.lua')) != route['source_path']:
                raise ValueError('callback source path differs')
            source_bytes = source_path.read_bytes()
            pinned = subprocess.check_output(['git', '-C', str(converter.monster_root), 'show',
                                              route['source_repository']['revision'] + ':' + route['source_path']])
            if (source_bytes != pinned
                    or hashlib.sha256(source_bytes).hexdigest() != route['source_file_sha256']):
                raise ValueError('callback source file differs from its pin')
            if any(lua_type(source.get(key)) != 'function' for key in route['pinned_callback_keys']):
                raise ValueError('pinned callback is absent or not a Lua function')
            serialized_source = {**source, **{key: {'pinned_source_callback': key}
                                             for key in route['pinned_callback_keys']}}
        fingerprint = hashlib.sha256(json.dumps(serialized_source, sort_keys=True, separators=(',', ':'),
                                               ensure_ascii=False).encode('utf-8')).hexdigest()
        path = ROOT.parents[2] / route['staticdata_path']
        raw = path.read_bytes()
        if hashlib.sha256(raw).hexdigest() != route['staticdata_file_sha256']:
            raise ValueError('accepted staticdata bytes changed')
        payload = json.loads(raw)
        selected = [r for r in payload['records'] if r.get('source_id') == route['race_id']
                    or r.get('source_index') == route['staticdata_record']['source_index']]
        if payload.get('family') != 'Creature' or selected != [route['staticdata_record']]:
            raise ValueError('accepted staticdata row identity changed')
        look_fields = list(cb.fields(bytes.fromhex(selected[0]['look']['hex'])))
        looks = [value for number, value in look_fields if number == 1 and type(value) is int]
        if looks != [route['look_type']]:
            raise ValueError('accepted staticdata look does not match the pinned source')
    except (OSError, ValueError, KeyError, TypeError, AttributeError, subprocess.CalledProcessError):
        return None, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                      'reason': 'qualified staticdata identity proof missing or mismatched'}
    outfit = source.get('outfit')
    if (name != route['registration'] or source.get('name', name) != route['registration']
            or type(source.get('raceId')) is not int or source['raceId'] != route['race_id']
            or not isinstance(outfit, dict) or type(outfit.get('lookType')) is not int
            or outfit['lookType'] != route['look_type']
            or fingerprint != route['source_table_sha256']):
        return None, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                      'reason': 'qualified staticdata source fingerprint mismatch'}
    record = fetch(route['title'], cache)
    cut = record.get('cut')
    if not eligible_cut_revision(cut):
        return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                        'reason': 'qualified staticdata Creature revision missing or after reference cut'}
    fields = infobox(cut['content'])
    wiki_race = fields.get('race_id', fields.get('raceid', fields.get('raceId', ''))).strip()
    if (not fields or cut['page_id'] != route['page_id'] or cut['title'] != route['title']
            or fields.get('name') != route['title']
            or fields.get('actualname') != route['actualname']
            or hashlib.sha256(cut['content'].encode('utf-8')).hexdigest() != route['cut_content_sha256']
            or (wiki_race and wiki_race != str(route['race_id']))):
        return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                        'reason': 'qualified staticdata canonical Creature identity mismatch'}
    current = record.get('current') or {}
    current_fields = infobox(current.get('content', ''))
    current_race = current_fields.get('race_id', current_fields.get('raceid',
                                       current_fields.get('raceId', ''))).strip()
    if (not current and route.get('reference_cut_only')):
        binding['current_identity_observation'] = 'UNSPECIFIED_NOT_INVENTED'
    elif (not current_fields or current.get('page_id') != route['page_id']
            or current.get('title') != route['title']
            or current_fields.get('name') != route['title']
            or (current_fields.get('actualname', '').strip()
                and current_fields['actualname'].casefold() != route['registration'].casefold())
            or (current_race and current_race != str(route['race_id']))):
        return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                        'reason': 'current canonical Creature content is missing or contradicts staticdata identity'}
    if wiki_race:
        binding.update(wiki_race_id_status='PRESENT_MATCHES', wiki_race_id=int(wiki_race))
    if not route['actualname']:
        binding['wiki_actualname_status'] = 'UNSPECIFIED_NOT_INVENTED'
    return record, {**binding, 'status': 'VERIFIED', 'source_race_id': route['race_id'],
                    'source_outfit_look_type': route['look_type'], 'source_table_sha256': fingerprint,
                    'staticdata_path': route['staticdata_path'],
                    'staticdata_file_sha256': route['staticdata_file_sha256'],
                    'staticdata_source_index': route['staticdata_record']['source_index'],
                    'staticdata_record_sha256': route['staticdata_record']['staticdata_sha256']}


def api(params):
    query = urllib.parse.urlencode({**params, 'format': 'json', 'formatversion': '2'})
    request = urllib.request.Request(API + '?' + query, headers={'User-Agent': USER_AGENT})
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def revision(title, start=None):
    params = {'action': 'query', 'titles': title, 'prop': 'revisions', 'rvprop': 'ids|timestamp|content',
              'rvslots': 'main', 'rvlimit': 1, 'redirects': 1}
    if start:
        params.update(rvstart=start, rvdir='older')
    page = api(params)['query']['pages'][0]
    if page.get('missing') or not page.get('revisions'):
        return None
    rev = page['revisions'][0]
    return {'page_id': page['pageid'], 'title': page['title'], 'revision_id': rev['revid'],
            'revision_timestamp': rev['timestamp'], 'content': rev['slots']['main']['content']}


def cut_cache(cache):
    """The cache directory of the current target date: a record is valid only for the cut it was fetched at."""
    directory = cache / TARGET_CUT
    directory.mkdir(parents=True, exist_ok=True)
    return directory


def fetch(title, cache):
    path = cut_cache(cache) / (cb.slug(title) + '.json')
    if path.exists():
        return json.loads(path.read_text(encoding='utf-8'))
    record = {'cut': revision(title, CUT_TIMESTAMP), 'current': revision(title),
              'retrieved_at': datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')}
    path.write_text(json.dumps(record, ensure_ascii=False), encoding='utf-8')
    time.sleep(0.5)
    return record


def infobox(text):
    """Top-level `| key = value` fields of the first Infobox Creature, honouring nested {{ }} and [[ ]]."""
    start = text.find('{{Infobox Creature')
    if start < 0:
        return {}
    depth, i, fields, current = 0, start, {}, None
    buffer = []
    while i < len(text):
        pair = text[i:i + 2]
        if pair in ('{{', '[['):
            depth += 1
            if depth > 1:
                buffer.append(pair)
            i += 2
            continue
        if pair in ('}}', ']]'):
            depth -= 1
            if depth == 0:
                break
            buffer.append(pair)
            i += 2
            continue
        if text[i] == '|' and depth == 1:
            if current is not None:
                fields[current] = ''.join(buffer).strip()
            buffer, current = [], None
            j = text.find('=', i)
            nxt = min(k for k in (text.find('|', i + 1), text.find('}}', i + 1), len(text)) if k >= 0)
            if 0 <= j < nxt:
                current = text[i + 1:j].strip()
                i = j + 1
                continue
        else:
            buffer.append(text[i])
        i += 1
    if current is not None:
        fields[current] = ''.join(buffer).strip()
    return fields


def eligible_cut_revision(cut):
    """Cached evidence must carry an aware timestamp at or before the actual reference cut."""
    if (not isinstance(cut, dict) or not isinstance(cut.get('revision_timestamp'), str)
            or not isinstance(cut.get('content'), str) or not isinstance(cut.get('title'), str)
            or not cut['title'] or type(cut.get('page_id')) is not int or cut['page_id'] <= 0
            or type(cut.get('revision_id')) is not int or cut['revision_id'] <= 0):
        return False
    try:
        timestamp = datetime.fromisoformat(cut['revision_timestamp'].replace('Z', '+00:00'))
        boundary = datetime.fromisoformat(CUT_TIMESTAMP.replace('Z', '+00:00'))
        return (timestamp.tzinfo is not None and timestamp.utcoffset() is not None
                and timestamp.astimezone(timezone.utc) <= boundary.astimezone(timezone.utc))
    except (ValueError, OverflowError):
        return False


def creature_page(relative, name, source, cache):
    """Resolve a creature page with explicit variant proof; NPC/name similarity never binds it."""
    if relative in CREATURE_STATICDATA_ROUTES:
        return staticdata_creature_page(relative, name, source, cache)
    if relative in CREATURE_MITIGATION_RACE_ROUTES:
        return mitigation_race_page(relative, name, source, cache)
    if relative in CREATURE_MITIGATION_VARIANTS:
        return mitigation_variant_page(relative, name, source, cache)
    title = name
    route = CREATURE_PAGE_ROUTES.get(relative)
    case_route = CREATURE_CASE_ROUTES.get(relative)
    context_route = CREATURE_CONTEXT_ROUTES.get(relative)
    form_route = CREATURE_ENCOUNTER_FORM_ROUTES.get(relative)
    binding = {'method': 'CreatureInfobox', 'source_registration_name': name}
    if form_route:
        expected_name, title, expected_page = (form_route['registration'], form_route['title'], form_route['page_id'])
        outfit = source.get('outfit')
        elements = source.get('elements')
        physical = ([e for e in elements
                     if isinstance(e, dict) and e.get('type') == '@COMBAT_PHYSICALDAMAGE']
                    if isinstance(elements, list) else [])
        if (name != expected_name or source.get('name', name) != form_route['display']
                or 'raceId' in source or 'events' in source or not isinstance(outfit, dict)
                or set(outfit) != set(form_route['outfit'])
                or any(type(outfit[k]) is not int or outfit[k] != value
                       for k, value in form_route['outfit'].items())
                or any(type(source.get(k)) is not int or source[k] != value
                       for k, value in form_route['source_fields'].items())
                or len(physical) != 1 or type(physical[0].get('percent')) is not int
                or physical[0]['percent'] != form_route['physical_percent']):
            return None, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                          'reason': 'qualified encounter form source fingerprint mismatch'}
        binding.update(method='EncounterFormAndSourceFile', source_file=relative,
                       source_display_name=form_route['display'], source_outfit=form_route['outfit'])
        if 'the_sinister_hermit_' in relative:
            binding['mechanic_context_conflict'] = (
                'Pinned source changes dirty to clean on geyser step-in; Wiki describes elemental damage on geyser. '
                'Identity phase context only; transformation trigger is not adopted or runtime-qualified.')
    elif context_route:
        expected_name, title, expected_page = (context_route['registration'], context_route['title'],
                                                context_route['page_id'])
        outfit = source.get('outfit')
        expected_event = context_route['event']
        event_matches = ('events' not in source if expected_event is None else
                         source.get('events') == [expected_event])
        if (name != expected_name or 'raceId' in source or not isinstance(outfit, dict)
                or set(outfit) != {'lookTypeEx'} or type(outfit.get('lookTypeEx')) is not int
                or outfit['lookTypeEx'] != context_route['look_type_ex'] or not event_matches
                or type(source.get('health')) is not int or source['health'] != context_route['health']
                or type(source.get('maxHealth')) is not int or source['maxHealth'] != context_route['health']
                or type(source.get('speed')) is not int or source['speed'] != 0):
            return None, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                          'reason': 'encounter actor source identity mismatch'}
        binding.update(method='EncounterActorAndSourceFile', source_file=relative,
                       source_outfit_look_type_ex=context_route['look_type_ex'],
                       source_event=expected_event)
    elif case_route:
        expected_name, title, expected_page = case_route
        if (name != expected_name or 'raceId' in source
                or name.casefold() != title.casefold()):
            return None, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                          'reason': 'explicit case route source identity mismatch'}
        binding.update(method='ExplicitCaseVariantAndSourceFile', source_file=relative)
    elif route:
        expected_name, expected_race, title, expected_page = route
        source_race = source.get('raceId')
        race_matches = (source_race is None if expected_race is None else
                        type(source_race) is int and source_race == expected_race)
        if name != expected_name or not race_matches:
            return None, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN', 'reason': 'source variant identity mismatch'}
        expected_look = CREATURE_SOURCE_LOOKS.get(relative)
        if expected_look is not None:
            outfit = source.get('outfit')
            if (not isinstance(outfit, dict) or 'lookTypeEx' in outfit
                    or type(outfit.get('lookType')) is not int
                    or outfit['lookType'] != expected_look):
                return None, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                              'reason': 'source render carrier does not match the qualified variant'}
            binding['source_outfit_look_type'] = expected_look
        pointer = CREATURE_PAGE_POINTERS.get(relative)
        if pointer is None:
            return None, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN', 'reason': 'explicit source pointer missing'}
        pointer_title, pointer_page = pointer
        disambiguation = fetch(pointer_title, cache).get('cut')
        if not eligible_cut_revision(disambiguation):
            return None, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                          'reason': 'disambiguation timestamp missing, malformed or after reference cut'}
        links = set(re.findall(r'\[\[([^\]|#]+)(?:[^\]]*)\]\]',
                               disambiguation.get('content', '') if disambiguation else ''))
        # The Monk vocation page names its creature counterpart through this exact template field.
        if '{{Disambiguation' in disambiguation.get('content', ''):
            links.update(value.strip() for value in re.findall(
                r'\|\s*disambig_title\s*=\s*([^|}]+)', disambiguation['content']))
        if (not disambiguation or disambiguation.get('page_id') != pointer_page
                or disambiguation.get('title') != pointer_title
                or CREATURE_PAGE_POINTER_LINKS.get(relative, title) not in links):
            return None, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN', 'reason': 'disambiguation proof missing or mismatched'}
        binding.update(method=('DisambiguationAndExactRaceId' if pointer_title == 'Nomads' else
                               'SourcePointerAndExactRaceId' if expected_race is not None else
                               'SourcePointerAndSourceFile'), source_file=relative,
                       pointer_link_title=CREATURE_PAGE_POINTER_LINKS.get(relative, title),
                       disambiguation_page_id=pointer_page,
                       disambiguation_revision_id=disambiguation['revision_id'],
                       disambiguation_sha256=hashlib.sha256(disambiguation['content'].encode('utf-8')).hexdigest())
    record = fetch(title, cache)
    cut = record.get('cut')
    if not cut:
        return record, {**binding, 'status': 'WIKI_PAGE_MISSING'}
    if not eligible_cut_revision(cut):
        return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                        'reason': 'Creature timestamp missing, malformed or after reference cut'}
    fields = infobox(cut.get('content', ''))
    if not fields:
        return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN', 'reason': 'page has no Creature infobox'}
    if (route or case_route or context_route or form_route) and (cut.get('page_id') != expected_page or cut.get('title') != title):
        return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN', 'reason': 'canonical variant page mismatch'}
    if case_route or context_route:
        declared_name, actual_name = fields.get('name'), fields.get('actualname')
        if (not isinstance(declared_name, str) or declared_name.casefold() != title.casefold()
                or not isinstance(actual_name, str) or actual_name.casefold() != name.casefold()):
            return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                            'reason': 'Creature declared name or actualname does not prove case-only identity'}
    if form_route:
        if (fields.get('name', '').casefold() != title.casefold()
                or fields.get('actualname', '').casefold() != form_route['actualname'].casefold()):
            return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                            'reason': 'canonical declared name or encounter actualname mismatch'}
        for field, expected_links in form_route['context_links'].items():
            text = fields.get(field, '')
            links = set(re.findall(r'\[\[([^\]|#]+)(?:[^\]]*)\]\]', text))
            if not set(expected_links) <= links:
                return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                                'reason': 'canonical encounter links do not prove the qualified form'}
        for field, expected_phrases in form_route['context_phrases'].items():
            text = fields.get(field, '').casefold()
            if any(phrase not in text for phrase in expected_phrases):
                return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                                'reason': 'canonical encounter phase description mismatch'}
        binding['wiki_context_links'] = form_route['context_links']
    if context_route:
        context_field = context_route['wiki_context_field']
        context_text = fields.get(context_field, '')
        context_links = set(re.findall(r'\[\[([^\]|#]+)(?:[^\]]*)\]\]', context_text))
        if context_route['wiki_context_link'] not in context_links:
            return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN',
                            'reason': 'Creature encounter context does not match the qualified actor'}
        binding.update(wiki_context_field=context_field,
                       wiki_context_link=context_route['wiki_context_link'])
    source_race = source.get('raceId')
    if source_race is not None:
        raw = fields.get('race_id', fields.get('raceid', fields.get('raceId')))
        if type(source_race) is not int or not re.fullmatch(r'[1-9]\d*', (raw or '').strip()) or int(raw) != source_race:
            return record, {**binding, 'status': 'WIKI_IDENTITY_UNKNOWN', 'reason': 'Creature race ID missing or mismatched'}
        binding.update(source_race_id=source_race, wiki_race_id=int(raw))
        if not route:
            binding['method'] = 'CreatureInfoboxAndExactRaceId'
    return record, {**binding, 'status': 'VERIFIED'}


def field_line(content, key):
    """1-based wikitext line of the top-level `| key =` field, or None."""
    for number, line in enumerate(content.splitlines(), 1):
        if re.match(r'^\s*\|\s*' + re.escape(key) + r'\s*=', line):
            return number
    return None


def version_key(version):
    """Tibia client versions are decimal numbers plus build parts: 8.54 < 8.6 < 11.02 < 13.21.14040."""
    parts = re.findall(r'\d+', version.split('e')[0])
    return (Decimal(parts[0] + '.' + (parts[1] if len(parts) > 1 else '0')),) + tuple(int(p) for p in parts[2:])


def subpages(title, cache):
    """Titles `<title> (...)` for a disambiguation page, from the MediaWiki prefix index (cached)."""
    path = cut_cache(cache) / (cb.slug('subpages ' + title) + '.json')
    if path.exists():
        return json.loads(path.read_text(encoding='utf-8'))
    pages = api({'action': 'query', 'list': 'allpages', 'apprefix': title + ' (', 'aplimit': 50})['query']['allpages']
    titles = sorted(p['title'] for p in pages)
    path.write_text(json.dumps(titles, ensure_ascii=False), encoding='utf-8')
    time.sleep(0.5)
    return titles


def item_page(title, cache, variants=True):
    """Item ids declared by the item's own wiki page at the cut (`| itemid =`). A disambiguation page lists
    its `<title> (...)` item pages as variants, each with its ids and `droppedby` creatures."""
    record = fetch(title, cache)
    cut = record['cut']
    if not cut:
        return {'page_title': title, 'status': 'WIKI_PAGE_MISSING'}
    line = field_line(cut['content'], 'itemid')
    raw = cut['content'].splitlines()[line - 1].split('=', 1)[1] if line else ''
    page = {'page_title': cut['title'], 'page_id': cut['page_id'], 'cut_revision_id': cut['revision_id'],
            'cut_content_sha256': hashlib.sha256(cut['content'].encode('utf-8')).hexdigest(),
            'current_revision_id': record['current']['revision_id'], 'retrieved_at': record['retrieved_at'],
            'status': 'COMPARED', 'itemid_line': line, 'item_ids': [int(v) for v in re.findall(r'\d+', raw)]}
    dropped = field_line(cut['content'], 'droppedby')
    if dropped:
        text = cut['content'].splitlines()[dropped - 1]
        page['droppedby_line'] = dropped
        page['dropped_by'] = [n.strip().lower() for n in re.sub(r'.*\{\{Dropped By\|', '', text).rstrip('}').split('|') if n.strip()]
    if variants and not line and '{{disambig}}' in cut['content'].lower():
        page['variants'] = [v for v in (item_page(sub, cache, False) for sub in subpages(cut['title'], cache))
                            if v['status'] == 'COMPARED' and v['item_ids']]
    return page


def loot_statistics(title, wanted, cache):
    """Kill and drop counts of the highest-version block of `Loot Statistics:<title>` at the cut.

    Every item of that block is returned; items in `wanted` (wiki loot missing in Canary) also get
    their item page ids so an ambiguous name can be resolved.
    """
    record = fetch('Loot Statistics:' + title, cache)
    cut, current = record['cut'], record['current']
    if not cut:
        return {'page_title': 'Loot Statistics:' + title, 'status': 'WIKI_PAGE_MISSING'}
    blocks, block = [], None
    for number, line in enumerate(cut['content'].splitlines(), 1):
        if line.lstrip().startswith('{{Loot'):
            # Only {{Loot2}} blocks record kills-with-drop ("times"); older {{Loot}} blocks count items.
            block = {'items': {}, 'loot2': line.lstrip().startswith('{{Loot2')}
            blocks.append(block)
            continue
        if block is None or line.strip() == '}}':
            block = None if line.strip() == '}}' else block
            continue
        match = re.match(r'^\s*\|\s*(version|kills)\s*=\s*(\S+)', line)
        if match:
            block[match.group(1)] = (match.group(2), number)
            continue
        match = re.match(r'^\s*\|\s*([^,|=]+?),\s*times:\s*(\d+),\s*amount:\s*([\d-]+)', line)
        if match:
            block['items'][match.group(1).strip().lower()] = (int(match.group(2)), match.group(3), number, match.group(1).strip())
    latest = max((b for b in blocks if b['loot2'] and 'version' in b and 'kills' in b), key=lambda b: version_key(b['version'][0]))
    items = []
    for name, (times, amount, line, wiki_name) in sorted(latest['items'].items(), key=lambda kv: kv[1][2]):
        item = {'name': name, 'wiki_name': wiki_name, 'times': times, 'amount': amount, 'line': line}
        if name in wanted:
            item['item_page'] = item_page(wiki_name, cache)
        items.append(item)
    return {'page_title': cut['title'], 'page_id': cut['page_id'], 'cut_revision_id': cut['revision_id'],
            'cut_revision_timestamp': cut['revision_timestamp'],
            'cut_content_sha256': hashlib.sha256(cut['content'].encode('utf-8')).hexdigest(),
            'current_revision_id': current['revision_id'], 'edited_after_cut': current['revision_id'] != cut['revision_id'],
            'retrieved_at': record['retrieved_at'], 'status': 'COMPARED',
            'version': latest['version'][0], 'version_line': latest['version'][1],
            'kills': int(latest['kills'][0]), 'kills_line': latest['kills'][1], 'items': items,
            'rule': 'block with the highest game version; probability estimate = times / kills'}


def wilson(times, kills, z=1.959963984540054):
    """95% Wilson score interval of times/kills, in percent."""
    p, n = times / kills, kills
    denominator = 1 + z * z / n
    centre = (p + z * z / (2 * n)) / denominator
    half = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / denominator
    return [round(max(0.0, centre - half) * 100, 4), round(min(1.0, centre + half) * 100, 4)]


def loot_chances(source, stats):
    """Compare each Canary loot entry's chance with the wiki estimate of the same item (D15 loot-rate rule)."""
    by_name = {item['name']: item for item in stats['items']}
    entries = source.get('loot', [])
    names = [(e.get('name') or cb.CONVERTER.names.get(int(e['id']), f'item {e["id"]}')).lower() for e in entries]
    rows = []
    for position, (entry, name) in enumerate(zip(entries, names)):
        low, high = entry.get('minCount', 1), entry.get('maxCount', 1)
        row = {'position': position, 'item': name, 'canary_percent': float(cb.percent_from_chance(entry.get('chance', 0))),
               'canary_amount': str(low) if low == high else f'{low}-{high}'}
        wiki = by_name.get(name)
        if wiki is None:
            variants = [i for i in stats['items'] if re.sub(r'\s*\(.*\)$', '', i['name']) == name]
            wiki = variants[0] if len(variants) == 1 else None
        if names.count(name) > 1:
            row['status'] = 'MULTI_ENTRY'
            row['note'] = 'Canary splits this item over several entries; the wiki counts kills with any drop, so they are not compared.'
        elif wiki is not None and not 0 <= wiki['times'] <= stats['kills']:
            row.update(status='INVALID_STATISTICS', times=wiki['times'], kills=stats['kills'],
                       note='Wiki drop count exceeds the kill count of the block.')
        elif wiki is None:
            row.update(status='NOT_OBSERVED', times=0, kills=stats['kills'], interval_95_wilson=wilson(0, stats['kills']),
                       note='Not listed in the highest-version block.')
        else:
            estimate = wiki['times'] * 100 / stats['kills']
            interval = wilson(wiki['times'], stats['kills'])
            row.update(wiki_item=wiki['wiki_name'], wiki_line=wiki['line'], wiki_amount=wiki['amount'], times=wiki['times'],
                       kills=stats['kills'], wiki_percent=round(estimate, 6), interval_95_wilson=interval,
                       confidence='estimate' if wiki['times'] >= LOW_CONFIDENCE_DROPS else 'low_confidence',
                       status='CONSISTENT' if interval[0] <= row['canary_percent'] <= interval[1] else 'DIFF')
        rows.append(row)
    return rows


def number(value):
    """Exact number of a wiki field value, with thousands separators; None when absent, approximate or uncertain."""
    if value is None or re.search(r'[?~]', value):
        return None
    match = re.match(r'^\s*(-?\d{1,3}(?:,\d{3})+|-?\d+)(\.\d+)?', value)
    return Fraction(match.group(1).replace(',', '') + (match.group(2) or '')) if match else None


def wiki_loot_items(value):
    """Presence names only: quantities/rarity never imply a probability or count.

    Loot Item accepts quantity/name in either of its first two positional slots.
    Retain empty slots and explicit numeric parameters; named metadata is not a name.
    Malformed/ambiguous entries stay visible rather than becoming a missing Item.
    """
    names, unresolved = set(), []
    quantity = re.compile(r'(?:[~≈]\s*)?(?:\d+(?:,\d{3})*|\?)(?:\s*[-–—]\s*(?:\d+(?:,\d{3})*|\?))?\s*[?+]?|[~?]')
    rarity = re.compile(r'(?:always|common|uncommon|semi[- ]rare|rare|very[- ]rare)\??', re.I)
    text = value or ''
    consumed_until = 0
    for match in re.finditer(r'\{\{\s*Loot[ _]Item\s*(?=[|}])', text, re.I):
        if match.start() < consumed_until:
            continue  # Nested markup belongs to the enclosing Item argument.
        depth, links, cursor, parts, buffer = 1, 0, match.end(), [], []
        while cursor < len(text):
            pair = text[cursor:cursor+2]
            if pair == '{{':
                depth += 1
            elif pair == '}}':
                depth -= 1
                if depth == 0:
                    parts.append(''.join(buffer).strip())
                    break
            elif pair == '[[':
                links += 1
            elif pair == ']]':
                links -= 1
            elif text[cursor] == '|' and depth == 1 and links == 0:
                parts.append(''.join(buffer).strip()); buffer = []; cursor += 1
                continue
            else:
                buffer.append(text[cursor]); cursor += 1
                continue
            buffer.append(pair); cursor += 2
        consumed_until = cursor + 2 if depth == 0 else len(text)
        if depth != 0 or links != 0:
            unresolved.append({'arguments': parts, 'reason': 'Unclosed template or Item link.'})
            continue
        # The first delimiter separates the template name, not parameter 1.
        if parts and parts[0] == '':
            parts = parts[1:]
        positions, next_position, invalid = {}, 1, False
        for part in parts:
            named = re.match(r'^([^=]+)=(.*)$', part, re.S)
            if named:
                key, argument = named[1].strip(), named[2].strip()
                if not key.isdigit():
                    continue
                position = int(key)
            else:
                position, argument = next_position, part
                next_position += 1
            if position in positions or position < 1:
                invalid = True
            positions[position] = argument
        first, second = positions.get(1, ''), positions.get(2, '')
        def modifier(argument):
            return not argument or quantity.fullmatch(argument) or rarity.fullmatch(argument)
        candidates = [argument for argument in (first, second) if not modifier(argument)]
        if invalid or len(candidates) != 1:
            unresolved.append({'arguments': parts, 'reason': 'Missing or ambiguous positional Item name.'})
            continue
        name = candidates[0]
        link = re.fullmatch(r'\[\[([^|\]#]+)(?:\|[^\]]+)?\]\]', name)
        if link:
            name = link[1].strip()
        if not name or re.search(r'[?~{}\[\]<>]', name):
            unresolved.append({'arguments': parts, 'reason': 'Uncertain or unsupported Item name markup.'})
            continue
        names.add(name.lower())
    return {'names': names, 'unresolved': unresolved}


def wiki_loot(value):
    """Backward-compatible definite Item names; diagnostics use wiki_loot_items."""
    return wiki_loot_items(value)['names']


def compare(relative, canary, _batch_dir, cache):
    name, source, _ = cb.load_monster(cb.CONVERTER.monster_root / cb.CONVERTER.monster_dir / (relative + '.lua'), [])
    slug = cb.slug(name)
    # Compare the plain Canary conversion, never a bundle that already carries adopted wiki values.
    cb.CONVERTER.wiki, cb.CONVERTER.pending_definitions = {}, set()
    _, monster, _, _, manifest, _ = cb.CONVERTER.convert(relative)
    record, binding = creature_page(relative, name, source, cache)
    result = {'monster': slug, 'wiki_title': name, 'page_url': PAGE_URL + urllib.parse.quote(name.replace(' ', '_'))}
    result['creature_identity'] = binding
    if binding['status'] != 'VERIFIED':
        return {**result, 'status': binding['status'], 'rows': []}
    cut, current = record['cut'], record['current']
    result.update(wiki_title=cut['title'], page_url=PAGE_URL + urllib.parse.quote(cut['title'].replace(' ', '_')))
    fields = infobox(cut['content'])
    result.update({'page_id': cut['page_id'], 'cut_revision_id': cut['revision_id'], 'cut_revision_timestamp': cut['revision_timestamp'],
                   'cut_content_sha256': hashlib.sha256(cut['content'].encode('utf-8')).hexdigest(),
                   'current_revision_id': current['revision_id'], 'current_revision_timestamp': current['revision_timestamp'],
                   'edited_after_cut': current['revision_id'] != cut['revision_id'], 'retrieved_at': record['retrieved_at']})
    c, b = monster['creature'], monster['behavior']
    rows = []

    def row(field, canary_value, wiki_raw, wiki_value, note=None, key=None):
        if wiki_raw in (None, '', '?') or str(wiki_raw).strip().lower() == 'unknown':
            status = 'WIKI_UNKNOWN'
        elif re.search(r'[?~]', str(wiki_raw)):
            status = 'WIKI_UNCERTAIN'
        elif wiki_value is None:
            status = 'WIKI_UNPARSED'
        elif canary_value == wiki_value:
            status = 'MATCH'
        else:
            status = 'DIFF'
        entry = {'field': field, 'canary': canary_value, 'wiki': wiki_value if wiki_value is not None else wiki_raw, 'wiki_raw': wiki_raw, 'status': status}
        if key and field_line(cut['content'], key):
            entry['wiki_line'] = field_line(cut['content'], key)
        if note:
            entry['note'] = note
        rows.append(entry)

    def as_number(value):
        return None if value is None else (int(value) if value.denominator == 1 else float(value))

    stats = c['stats']
    if binding.get('field_scope') == ['mitigation_percent']:
        mitigation = stats.get('mitigation_percent')
        value = float(Fraction(mitigation['numerator'], mitigation['denominator'])) if mitigation else None
        row('mitigation_percent', value, fields.get('mitigation'),
            as_number(mitigation_number(fields.get('mitigation'))),
            'Dated Creature page explicitly covers this source-qualified variant; only mitigation is projected.',
            'mitigation')
        result.update(status='COMPARED', rows=rows, qualified_fields=['mitigation_percent'])
        return result
    for field, key, value in (('max_health', 'hp', stats['max_health']), ('experience', 'exp', stats['experience']),
                              ('armor', 'armor', stats['armor']), ('speed', 'speed', stats['speed'])):
        row(field, value, fields.get(key), as_number(number(fields.get(key))),
            'Canary monster.speed is the raw engine value; the wiki lists observed speed.' if field == 'speed' else None, key)
    mitigation = stats.get('mitigation_percent')
    row('mitigation_percent', float(Fraction(mitigation['numerator'], mitigation['denominator'])) if mitigation else None,
        fields.get('mitigation'), as_number(mitigation_number(fields.get('mitigation'))), key='mitigation')
    resist = {r['damage_type']: Fraction(r['reduction_percent']['numerator'], r['reduction_percent']['denominator']) for r in c['resistances']}
    resist.update({damage: Fraction(100) for damage in c['immunities']['damage_types']})
    for element, key in ELEMENTS.items():
        taken = number(fields.get(key))
        row(f'resistance.{element}', as_number(resist.get(element, Fraction(0))), fields.get(key),
            as_number(100 - taken) if taken is not None else None, 'Wiki lists damage taken; resistance = 100 - taken; a Canary damage immunity counts as 100.', key)
    summoning = c['summoning']
    for field, key, flag in (('summon_mana_cost', 'summon', 'summonable'), ('convince_mana_cost', 'convince', 'convinceable')):
        raw = fields.get(key)
        wiki_value = as_number(number(raw)) if number(raw) is not None else ('--' if raw and raw.strip().lower() in ('--', '-', 'no') else None)
        row(field, summoning.get('mana_cost') if summoning[flag] else '--', raw, wiki_value, key=key)
    for field, key, value in (('illusionable', 'illusionable', c['flags']['illusionable']),
                              ('pushable', 'pushable', b['movement']['pushable']),
                              ('push_items', 'pushobjects', b['movement']['push_items']),
                              ('sense_invisible', 'senseinvis', b['targeting']['sense_invisible']),
                              ('paralyze_immune', 'paraimmune', 'paralyze' in c['immunities']['conditions'])):
        raw = fields.get(key)
        row(field, value, raw, {'yes': True, 'no': False}.get((raw or '').strip().lower()), key=key)
    row('flee_health', b['targeting']['flee_health'], fields.get('runsat'), as_number(number(fields.get('runsat'))), key='runsat')
    bestiary = c.get('bestiary', {})
    for field, key in (('bestiary.class', 'bestiaryclass'), ('bestiary.difficulty', 'bestiarylevel'), ('bestiary.occurrence', 'occurrence')):
        canary_value = bestiary.get(field.split('.')[1])
        raw = fields.get(key)
        wiki_value = raw.strip().lower().replace(' ', '_') if raw else None
        row(field, canary_value.lower() if isinstance(canary_value, str) else canary_value, raw, wiki_value, key=key)
    canary_loot = set()
    for entry in source.get('loot', []):
        item_name = entry.get('name') or cb.CONVERTER.names.get(int(entry['id']), f'item {entry["id"]}')
        canary_loot.add(item_name.lower())
    loot_presence = wiki_loot_items(fields.get('loot'))
    wiki_items = loot_presence['names']
    if wiki_items or canary_loot or loot_presence['unresolved']:
        only_canary, only_wiki = canary_loot - wiki_items, wiki_items - canary_loot
        variants = sorted((c_name, w_name) for c_name in only_canary for w_name in only_wiki
                          if re.sub(r'\s*\(.*\)$', '', w_name) == c_name)
        only_canary -= {c_name for c_name, _ in variants}
        only_wiki -= {w_name for _, w_name in variants}
        entry = {'field': 'loot.items', 'status': 'WIKI_UNPARSED' if loot_presence['unresolved'] else
                 ('DIFF' if only_canary or only_wiki else 'MATCH'),
                 'only_canary': sorted(only_canary), 'only_wiki': sorted(only_wiki),
                 'note': 'Item names only; chances are compared in loot_chances from the Loot Statistics page.'}
        if loot_presence['unresolved']:
            entry['parse_diagnostics'] = loot_presence['unresolved']
            entry['unconfirmed_canary'] = entry['only_canary']
            entry['only_canary'] = []  # Unparsed entries cannot prove absence.
            entry['note'] += ' Ambiguous entries are unknown presence, not missing Items.'
        if field_line(cut['content'], 'loot'):
            entry['wiki_line'] = field_line(cut['content'], 'loot')
        stats = loot_statistics(cut['title'], sorted(only_wiki), cache)
        result['loot_statistics'] = stats
        if stats['status'] == 'COMPARED' and canary_loot:
            result['loot_chances'] = loot_chances(source, stats)
        if variants:
            entry['name_variants'] = [{'canary': c_name, 'wiki': w_name} for c_name, w_name in variants]
            entry['note'] += ' Wiki disambiguated names (e.g. "book (grey)") are matched to the Canary base name.'
        rows.append(entry)
    result.update({'status': 'COMPARED', 'rows': rows})
    if any('COMBAT_UNDEFINEDDAMAGE' in e.get('resolution', '') for e in manifest['entries']):
        # D25: the converter decides an undefined combat element from these wiki abilities.
        import wiki_scenes
        _, shapes = wiki_scenes.scene_shapes(cache)
        result['abilities'] = [{**{k: v for k, v in a.items() if k != 'tiles'}, 'tiles': sorted(map(list, a.get('tiles', ())))}
                               for a in wiki_scenes.wiki_abilities(cut['content'], shapes, cache, {})]
    return result


def totals(results):
    """Totals over full results (before compact())."""
    summary, chances, confidence, fields = {}, {}, {}, {}
    for result in results:
        for entry in result.get('rows', []):
            summary[entry['status']] = summary.get(entry['status'], 0) + 1
            if entry['status'] == 'DIFF':
                fields[entry['field']] = fields.get(entry['field'], 0) + 1
        for entry in result.get('loot_chances', []):
            chances[entry['status']] = chances.get(entry['status'], 0) + 1
            if 'confidence' in entry:
                confidence[entry['confidence']] = confidence.get(entry['confidence'], 0) + 1
    return (dict(sorted(summary.items())), dict(sorted(chances.items())), dict(sorted(confidence.items())),
            dict(sorted(fields.items(), key=lambda kv: (-kv[1], kv[0]))))


def compact(result):
    """Population form: revisions, per-status counts and only the rows that are not MATCH/CONSISTENT."""
    out = {k: v for k, v in result.items() if k not in ('rows', 'loot_chances', 'loot_statistics')}
    counts = {}
    for entry in result.get('rows', []):
        counts[entry['status']] = counts.get(entry['status'], 0) + 1
    out['row_counts'] = dict(sorted(counts.items()))
    out['rows'] = [r for r in result.get('rows', []) if r['status'] == 'DIFF'
                   or (r['field'] == 'loot.items' and r['status'] == 'WIKI_UNPARSED')]
    stats = result.get('loot_statistics')
    if stats:
        # Keep the statistics of wiki-only loot items: canary_batch.py adopts them (D15).
        out['loot_statistics'] = {**{k: v for k, v in stats.items() if k != 'items'},
                                  'items': [i for i in stats.get('items', []) if 'item_page' in i]}
    chance_counts = {}
    for entry in result.get('loot_chances', []):
        chance_counts[entry['status']] = chance_counts.get(entry['status'], 0) + 1
    if chance_counts:
        out['loot_chance_counts'] = dict(sorted(chance_counts.items()))
    # Every loot chance is kept (trimmed): canary_batch.py applies the D15 loot rate rule from it.
    keep = ('position', 'item', 'status', 'confidence', 'canary_percent', 'wiki_percent', 'interval_95_wilson', 'times', 'kills',
            'wiki_line', 'canary_amount', 'wiki_amount', 'note')
    out['loot_chances'] = [{k: c[k] for k in keep if k in c} for c in result.get('loot_chances', [])]
    return out



def write_compact_population(path, report):
    """Write one fact record per line, preserving headers independently of dictionary order."""
    header = {key: value for key, value in report.items() if key != 'monsters'}
    header_text = json.dumps(header, ensure_ascii=False, indent=2)
    # Remove only the serialized header object's closing brace; metadata remains complete.
    prefix = header_text[:-1].rstrip() + (',' if header else '')
    records = ',\n'.join('    ' + json.dumps(record, ensure_ascii=False, separators=(',', ':'))
                         for record in report['monsters'])
    path.write_text(prefix + '\n  "monsters": [\n' + records + '\n  ]\n}\n',
                    encoding='utf-8', newline='\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--batch', default=cb.REV, choices=sorted(cb.BATCHES))
    parser.add_argument('--population', action='store_true', help='compare every Canary monster file (compact output)')
    parser.add_argument('--cache', type=Path, default=Path('/tmp/oteryn-wiki-cache'))
    args = parser.parse_args()
    args.cache.mkdir(parents=True, exist_ok=True)
    objects = cb.load_appearance_objects(args.canary / 'data/items/appearances.dat')
    items = cb.load_items_xml(args.canary / 'data/items/items.xml')
    names, index = cb.name_index(objects, items)
    cb.CONVERTER = cb.Converter(args.canary, objects, items, names, index)
    if args.population:
        results, skipped = [], []
        for path in sorted((args.canary / cb.MONSTER_DIR).rglob('*.lua')):
            relative = str(path.relative_to(args.canary / cb.MONSTER_DIR))[:-4]
            try:
                results.append(compare(relative, args.canary, None, args.cache))
            except Exception as exc:  # files the converter cannot convert (see population_census.py)
                skipped.append({'file': relative, 'error': f'{type(exc).__name__}: {str(exc).splitlines()[0][:100]}'})
        out = ROOT / 'samples' / 'wiki-population-2026-09-27.json'
    else:
        results, skipped = [compare(relative, args.canary, None, args.cache) for relative in cb.BATCHES[args.batch]], []
        out = ROOT / 'samples' / args.batch / 'wiki-2026-09-27.json'
    rows, chances, confidence, fields = totals(results)
    statuses = {}
    for result in results:
        statuses[result['status']] = statuses.get(result['status'], 0) + 1
    if args.population:
        results = [compact(result) for result in results]
    report = {'source': 'TibiaWiki (Fandom), CC BY-SA; only compared facts are recorded', 'api': API,
              'target_cut': TARGET_CUT, 'cut_rule': f'last revision at or before {CUT_TIMESTAMP}',
              'classification': 'Wiki = player-observed reference evidence; Canary = OTS_HYPOTHESIS_ONLY',
              'row_status_totals': rows,
              'loot_chance_rule': f'highest-version Loot Statistics block at the cut; estimate = times / kills with a 95% Wilson interval; '
                                  f'fewer than {LOW_CONFIDENCE_DROPS} drops is low_confidence. CONSISTENT = Canary inside the interval',
              'loot_chance_totals': chances, 'loot_chance_confidence': confidence}
    if args.population:
        report.update({'scope': 'Every convertible Canary monster file. Per monster the DIFF rows (the only ones D15 '
                                'adopts) and all loot chances (trimmed) are listed; other statuses are counted.',
                       'monster_status_totals': dict(sorted(statuses.items())), 'diff_fields': fields, 'not_converted': skipped})
    report['monsters'] = results
    if args.population:
        write_compact_population(out, report)
    else:
        out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps({'out': str(out), 'totals': rows, 'loot_chances': chances, 'confidence': confidence,
                      **({'monsters': statuses, 'top_diff_fields': dict(list(fields.items())[:12])} if args.population else {})}))


if __name__ == '__main__':
    main()
