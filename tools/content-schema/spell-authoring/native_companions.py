"""Exact-source companion behaviour recipes, never Lua execution or runtime admission.

The cast script and every helper needed by the recipe are hash qualified. Canary
wins a two-source executable conflict (S21); a Crystal-only historical alias
retains its explicit source and shared reference ID. No unknown source body,
pattern list, item/0, inferred swimming rule or invented encounter gate is emitted.
"""
import copy
import hashlib
import math
from pathlib import Path
import struct

PINS = {'canary': '99902524e052f37574194466c2949c576e4ab269',
        'crystal': 'ff7ede593c69d4c658b382c97443e8155926924a'}
REPOSITORIES = {'canary': 'opentibiabr/canary', 'crystal': 'zimbadev/crystalserver'}
SPECS = {'charge': {'carrier': 'instant',
            'sources': {'canary': {'path': 'data/scripts/spells/support/charge.lua',
                                   'blob': 'bea6e2ee880d9da285e95b161e6ea59957261e26',
                                   'sha256': '27697754086cdcc9627bdf67d4855f8627a38e8af7dc083e3a7283597d3a0765'},
                        'crystal': {'path': 'data/scripts/spells/support/charge.lua',
                                    'blob': 'bea6e2ee880d9da285e95b161e6ea59957261e26',
                                    'sha256': '27697754086cdcc9627bdf67d4855f8627a38e8af7dc083e3a7283597d3a0765'}}},
 'druid familiar': {'carrier': 'instant',
                    'sources': {'canary': {'path': 'data/scripts/spells/familiar/druid_familiar.lua',
                                           'blob': '9f70dd045304ec483a26c81ce1db35b126a6e446',
                                           'sha256': '74ad5c9e86abc4201ec70d2ce254acb2f6133b928ef64853e755ea02444ba40c'}}},
 'haste': {'carrier': 'instant',
           'sources': {'canary': {'path': 'data/scripts/spells/support/haste.lua',
                                  'blob': '7c899e8c784036213e63b6216cb23efb3f4ee2e5',
                                  'sha256': '83f584501356155fde482b135496aa1e800e664edb79c12b1a767d5ffab68c4a'},
                       'crystal': {'path': 'data/scripts/spells/support/haste.lua',
                                   'blob': '7c899e8c784036213e63b6216cb23efb3f4ee2e5',
                                   'sha256': '83f584501356155fde482b135496aa1e800e664edb79c12b1a767d5ffab68c4a'}}},
 'knight familiar': {'carrier': 'instant',
                     'sources': {'canary': {'path': 'data/scripts/spells/familiar/knight_familiar.lua',
                                            'blob': '8e5ae2c6ec111a59e10e325d6c61151751bd65de',
                                            'sha256': '525a7820f8d78c4562b4e54e31aad982aed6c24302b891fb37c29ef570ee982d'}}},
 'monk familiar': {'carrier': 'instant',
                   'sources': {'canary': {'path': 'data/scripts/spells/familiar/monk_familiar.lua',
                                          'blob': '781b6a855314f99ed651e68243a601f97bbebd82',
                                          'sha256': '3ecf699a021029d263d675366b426738d27044b248813efa54fcbb0af9ca18db'},
                               'crystal': {'path': 'data/scripts/spells/familiar/monk_familiar.lua',
                                           'blob': 'f1b719bdbaf3ff3fd3b5c62c2b4920e8b34023dc',
                                           'sha256': '15c1707b450ea3513c6fe58ce033005573c65574991b3c0f7d6b335c30d0e5a3'}}},
 'paladin familiar': {'carrier': 'instant',
                      'sources': {'canary': {'path': 'data/scripts/spells/familiar/paladin_familiar.lua',
                                             'blob': '030bbb77330c0eae6eba0f6bd9803a703db5e3a4',
                                             'sha256': '4ae12fec754563e50a0c463bc66bb0229770dd4f640e5936bc81eea6f18f0888'}}},
 'sorcerer familiar': {'carrier': 'instant',
                       'sources': {'canary': {'path': 'data/scripts/spells/familiar/sorcerer_familiar.lua',
                                              'blob': '48619ca0c6d71c72e335b417aef3872118a0a460',
                                              'sha256': 'fd8e84e95cf7d9e4f97e6bcc9154ea112a2e5ce04cc014411deb94622625af28'}}},
 'strong haste': {'carrier': 'instant',
                  'sources': {'canary': {'path': 'data/scripts/spells/support/strong_haste.lua',
                                         'blob': 'e20b7a891cca707846e41d2eafe182dd0b898b52',
                                         'sha256': '2d6317d9b16858c7faf7ab2636d1bb09c5aaee7556fec556b6515b7e95e5bbf5'},
                              'crystal': {'path': 'data/scripts/spells/support/strong_haste.lua',
                                          'blob': 'e20b7a891cca707846e41d2eafe182dd0b898b52',
                                          'sha256': '2d6317d9b16858c7faf7ab2636d1bb09c5aaee7556fec556b6515b7e95e5bbf5'}}},
 'summon creature': {'carrier': 'instant',
                     'sources': {'canary': {'path': 'data/scripts/spells/support/summon_creature.lua',
                                            'blob': '991e4df21103e7d488957aaf2786356fd8791702',
                                            'sha256': '8fc4dfc69741b95277ac212dbb9ae005a879be0e6005cbb50c192b7505cbbad2'},
                                 'crystal': {'path': 'data/scripts/spells/support/summon_creature.lua',
                                             'blob': '991e4df21103e7d488957aaf2786356fd8791702',
                                             'sha256': '8fc4dfc69741b95277ac212dbb9ae005a879be0e6005cbb50c192b7505cbbad2'}}},
 'summon druid familiar': {'carrier': 'instant',
                           'sources': {'crystal': {'path': 'data/scripts/spells/familiar/druid_familiar.lua',
                                                   'blob': '5a642be688c0d717c6722dc2eda5f6bdf4def826',
                                                   'sha256': '44cb86bb3ed5f53c2b7dd40ef0c6df5dd51309f4a1418b9b33363efa3894f16e'}}},
 'summon knight familiar': {'carrier': 'instant',
                            'sources': {'crystal': {'path': 'data/scripts/spells/familiar/knight_familiar.lua',
                                                    'blob': 'f3116eb60bba09e0e77c00197a87107f3b43753d',
                                                    'sha256': '6df8b1e444ebd75d8b598002c977d310dcb25437245a8132ea024c90284c3559'}}},
 'summon paladin familiar': {'carrier': 'instant',
                             'sources': {'crystal': {'path': 'data/scripts/spells/familiar/paladin_familiar.lua',
                                                     'blob': 'b0ea97b122989f3e5702db67f9c637a48a762307',
                                                     'sha256': '16e21ac293e7bb915eb6acea0f9449c8973a5f4b68abad8097bda5e399214870'}}},
 'summon sorcerer familiar': {'carrier': 'instant',
                              'sources': {'crystal': {'path': 'data/scripts/spells/familiar/sorcerer_familiar.lua',
                                                      'blob': 'cceb7d770f9ed45b7923aed10421631a98f879c3',
                                                      'sha256': 'c4c85a995d177bb73056ab1743e2c9d7f563965570dffcd72c1908a4bfbbf4a1'}}},
 'swift foot': {'carrier': 'instant',
                'sources': {'canary': {'path': 'data/scripts/spells/support/swift_foot.lua',
                                       'blob': 'fc010237ae83cb9922ee5d51898752b0850ef1bd',
                                       'sha256': '3adce27fc87b5c84b4ea4e4c46fb321dd3bf6d6c6dd601fd422cff0cc60f655b'},
                            'crystal': {'path': 'data/scripts/spells/support/swift_foot.lua',
                                        'blob': 'aff9e06abc31aa6a51640c21c75c60ba01b3a577',
                                        'sha256': '68e01dbb55adb0aaddac079825b9496a948a5a7e164a20eb57b6245db7a44ee6'}}},
 'animate dead rune': {'carrier': 'rune',
                       'sources': {'canary': {'path': 'data/scripts/runes/animate_dead_rune.lua',
                                              'blob': '28f5893c21b30ce78507131287d0ef412fef7b11',
                                              'sha256': '6990f6ebcf6bace0dfa076c1966627ed97be04dbdc462adeeeefe0d8826d3cb4'},
                                   'crystal': {'path': 'data/scripts/runes/animate_dead_rune.lua',
                                               'blob': '28f5893c21b30ce78507131287d0ef412fef7b11',
                                               'sha256': '6990f6ebcf6bace0dfa076c1966627ed97be04dbdc462adeeeefe0d8826d3cb4'}}},
 'convince creature rune': {'carrier': 'rune',
                            'sources': {'canary': {'path': 'data/scripts/runes/convince_creature.lua',
                                                   'blob': 'f591c6f96147376b67d7ef2eb867bb7a7cea2a19',
                                                   'sha256': '20a9a6308e0a69e0474f8e7b65bf1fb5649327739a2e707e83201fc5b3c40a52'},
                                        'crystal': {'path': 'data/scripts/runes/convince_creature.lua',
                                                    'blob': 'f591c6f96147376b67d7ef2eb867bb7a7cea2a19',
                                                    'sha256': '20a9a6308e0a69e0474f8e7b65bf1fb5649327739a2e707e83201fc5b3c40a52'}}}}
HELPER_HASHES = {'canary': {'data/libs/functions/player.lua': 'fdbca67f13f64dc47449b393825ee2caa96076951c02130643be925ee73ee8cd',
            'data/libs/systems/familiar.lua': 'd12558c52f581a9149ebb738f68aa4ad74734b3fe79eb9863252fb5a90018dcc',
            'data/scripts/creaturescripts/familiar/on_login.lua': 'c059061691f2ef6143d804512bfa64c717b5b9c650bc151294d85e04c9eb571a',
            'data/scripts/creaturescripts/familiar/on_death.lua': '55beeda23657585574b3db8f3cdd3b3524e8111724ef27a71dd40a61b641b389',
            'data/scripts/creaturescripts/familiar/on_advance.lua': '68c6568c328d487ac6f2297043f1e090c1b2569192264023bf573e4f20f7e5c9',
            'src/creatures/combat/condition.cpp': '3601f95a59a637d9b9722bb99eb4858de4ae119ba263dde658ccf0e277c3993f',
            'src/creatures/creature.cpp': 'b25809a5bcb30595400f48d80a8b13bf3d50f74e206e33db1602d4510801beac',
            'src/creatures/players/player.cpp': 'be92f3b797a5692843b1cce09250287ce52f91afbcfd5edaf9c6fddccfbb9232',
            'src/io/functions/iologindata_load_player.cpp': '5bda3cc0cb6dec5cf49c30c5abe35ee734225bc1016db75aee8f4ed7776ca732',
            'src/io/functions/iologindata_save_player.cpp': '0debc968db2351c3eb82f8c9ef6c4acf77d3fdc79d26a36c9508c51c6e9042c4',
            'src/game/game.cpp': 'b337fb7d7ce61d9ccde4f315cb696df0add9a0bf933c9b5c634b9ed79d7f01dd',
            'config.lua.dist': '099939f439c755171c18d8dadd23d82f50bd3ebd219099248e847481fd84cd15',
            'src/lua/functions/creatures/combat/condition_functions.cpp': '5a1d6efacfc2d072f95a832bbd96bbe0d12b7152abef3d24449153c0fb2831a3'},
 'crystal': {'data/libs/functions/player.lua': '897edd1b4509622480dfc713d226ae54dbef1ce06c7a4a7af556fbc6744f6a0e',
             'data/libs/systems/familiar.lua': 'd12558c52f581a9149ebb738f68aa4ad74734b3fe79eb9863252fb5a90018dcc',
             'data/scripts/creaturescripts/familiar/on_login.lua': '2f65f3b4c556f36a9eefec9f72296198609fec5a525b96ad61797eecbbca59e7',
             'data/scripts/creaturescripts/familiar/on_death.lua': '55beeda23657585574b3db8f3cdd3b3524e8111724ef27a71dd40a61b641b389',
             'data/scripts/creaturescripts/familiar/on_advance.lua': '68c6568c328d487ac6f2297043f1e090c1b2569192264023bf573e4f20f7e5c9',
             'src/creatures/combat/condition.cpp': 'f5816ed3f546d9551facfcf655cf727f20de6ced808027d29da8b287514bf2e4',
             'src/creatures/creature.cpp': 'c38b9b326b8b6943e33c5b06ea53a8d3d081784453b4a600acfec9185e75e39a',
             'src/creatures/players/player.cpp': '5f03562f860a19ebe623553d77f78b64e145ecc97c87788a55375d994be8aa1d',
             'src/io/functions/iologindata_load_player.cpp': '90f6131d2c50e1171f75f9ddbea67976e36302ab6503418aabea365d56c5c50e',
             'src/io/functions/iologindata_save_player.cpp': 'd8287b0c8dece0301f867b07f4e838c94203305cdc208063888b33449b3174eb',
             'src/game/game.cpp': '4b26d0c5168e0907ae9044b2587a37ba647f6b6651a94e02f1c12a2e3af58d45',
             'config.lua.dist': 'a7947e370f3be151597d82bbaf90852ee3ab530b788510cffe5843b1a8305c7b',
             'src/lua/functions/creatures/combat/condition_functions.cpp': 'e97e074ca2fb3dc6a9ea595b37be6ae0a3b404349a52907aa5451152f57169c0'}}

HELPER_HASHES['canary']['src/creatures/combat/condition.hpp'] = 'be1130673ba7b98dd2ae62093bbcbe065fc1463e30be6771bde8b616126729ec'
HELPER_HASHES['crystal']['src/creatures/combat/condition.hpp'] = 'f033d32a7ca88fb50d749f71f4ade9211566ddd3abba790f7632649a2bfef3d7'

HELPER_HASHES['canary']['src/lua/functions/core/game/global_functions.cpp'] = '29016a1c1edbcd597e9bd6d51f34edce2f616b9d4abb762e76a6e99537e30c02'
HELPER_HASHES['canary']['src/lua/functions/lua_functions_loader.hpp'] = '6fadaba7fe81ac542e3f8801fbc29b09c3018125131489b659d1a17613eb657c'
HELPER_HASHES['crystal']['src/lua/functions/core/game/global_functions.cpp'] = 'f87c9d7c8122a95b4361ffbc332a804b0dd6ec072afdf8f563226cd146110e15'
HELPER_HASHES['crystal']['src/lua/functions/lua_functions_loader.hpp'] = '53daa6dd4c3659f33d857f73f1ff18ab59612e2a3f409ec94eed102d970c0aa9'

FAMILIARS = {'knight': (194, 991), 'paladin': (195, 992), 'sorcerer': (196, 994),
             'druid': (197, 993), 'monk': (282, 1818)}
SPEEDS = {'haste': (30000, '1.3', '40', 33000, '0.3', '-24'),
          'strong haste': (22000, '1.7', '40', 22000, '0.7', '-56'),
          'charge': (5000, '1.9', '40', 5000, '0.9', '-72'),
          'swift foot': (10000, '1.8', '72', 10000, '0.8', '-72')}


def _read(source, path, records, texts):
    value = texts.get((source, path))
    if value is None and records[source].get('file') == path:
        value = texts.get(source)
    if value is not None:
        return value if isinstance(value, bytes) else value.encode('utf-8')
    root = records[source].get('source_root')
    if root is None:
        raise ValueError(f'{source}: missing pinned helper source {path}')
    try:
        return (Path(root) / path).read_bytes()
    except OSError as exc:
        raise ValueError(f'{source}: missing pinned source {path}') from exc


def _helper_paths(name):
    if name in SPEEDS:
        return ('src/creatures/combat/condition.cpp', 'src/creatures/combat/condition.hpp',
                'src/creatures/creature.cpp',
                'src/game/game.cpp', 'src/lua/functions/creatures/combat/condition_functions.cpp')
    if 'familiar' in name:
        return tuple(p for p in HELPER_HASHES['canary']
                     if p != 'src/lua/functions/creatures/combat/condition_functions.cpp')
    # The script declares these operations; the world/creature owner supplies
    # their real facts and executes the named create/setSummon calls.
    return ('src/creatures/creature.cpp', 'src/game/game.cpp')


def _qualify(name, spell_type, records, texts):
    spec = SPECS.get(name)
    if spec is None or spell_type != spec['carrier']:
        return None
    if not records or set(records) - set(spec['sources']):
        raise ValueError(f'{name}: source/carrier identity is not qualified')
    qualified = []
    for source in sorted(records):
        expected = spec['sources'][source]
        record = records[source]
        if (record.get('source') != source or record.get('file') != expected['path']
                or record.get('blob') != expected['blob']
                or record.get('spell_type') != spell_type
                or str(record.get('name', '')).casefold() != name):
            raise ValueError(f'{name}: {source} census identity changed')
        if record.get('revision', PINS[source]) != PINS[source]:
            raise ValueError(f'{name}: {source} is not the qualified revision')
        raw = _read(source, expected['path'], records, texts)
        blob = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
        if hashlib.sha256(raw).hexdigest() != expected['sha256'] or blob != expected['blob']:
            raise ValueError(f'{name}: {source} complete cast source changed')
        proofs = [{'path': expected['path'], 'sha256': expected['sha256'], 'blob': blob}]
        for path in _helper_paths(name):
            digest = hashlib.sha256(_read(source, path, records, texts)).hexdigest()
            if digest != HELPER_HASHES[source][path]:
                raise ValueError(f'{name}: {source} helper changed: {path}')
            proofs.append({'path': path, 'sha256': digest})
        qualified.append({'source': source, 'repository': REPOSITORIES[source],
                          'revision': PINS[source], 'files': proofs})
    return ('canary' if 'canary' in records else 'crystal'), qualified


def _recipe(name, source):
    if 'familiar' in name:
        vocation = name.removeprefix('summon ').split()[0]
        spell_id, look = FAMILIARS[vocation]
        return {'key': 'familiar_summon', 'parameters': {
            'vocation': vocation, 'reference_spell_id': spell_id,
            'creature_name': vocation.title() + ' familiar', 'default_look_type': look,
            'shared_cooldown_identity': spell_id,
            'admission_order': ['premium', 'owned_summon_count', 'vocation_familiar', 'spawn_room'],
            'summon_limit': 1, 'summon_limit_exemption': 'account_type_at_least_god',
            'spawn': {'position': 'caster', 'extended': True, 'force': False, 'master': 'caster'},
            'creation_speed': {'basis': 'owner_current_minus_familiar_base', 'minimum_delta': 0},
            'duration': {'config': 'familiar_time_minutes', 'seconds_multiplier': 30,
                         'default_config_value': 30, 'expiry_storage': 'unix_expiry'},
            'cooldown': {'identity': 'reference_spell_id', 'duration_multiplier': 2,
                         'vip_reduction_minutes_config': 'vip_familiar_time_cooldown_reduction',
                         'vip_reduction_cap': 'duration_seconds', 'vip_seconds_multiplier': 60,
                         'rate_divisor_config': 'rate_spell_cooldown',
                         'default_vip_reduction': 0, 'default_rate_divisor': 1,
                         'start': 'after_creation_success', 'offline': 'remaining_condition_ticks',
                         'condition_id': 'default', 'sub_id': 'reference_spell_id',
                         'permanent_ticks_are_persistent': False, 'preserve_on_owner_death': True,
                         'replacement': 'preserve_infinite_or_longer_remaining'},
            'warnings': [{'remaining_ms': 10000, 'message': '10 seconds'},
                         {'remaining_ms': 60000, 'message': 'one minute'}],
            'warning_dispatch': {'prefix': 'Your summon will disappear in less than ',
                                 'requires_owner_present': True, 'cancel_on_familiar_death': True,
                                 'short_lifetime_delay': 'remaining_minus_warning',
                                 'negative_delay_conversion': 'unsigned_zero', 'scheduler_minimum_ms': 100},
            'expiry': {'requires_owner_and_creature_present': True, 'remove_creature': True,
                       'reset_warning_handles': True},
            'login': {'lifetime_basis': 'current_unix_time' if source == 'canary' else 'last_logout_unix_time',
                      'clamp_remaining_to_zero': source == 'canary', 'requires_premium': True,
                      'minimum_level': 200, 'remove_vocation_look_if_ineligible': True,
                      'restore_default_look_if_zero': True,
                      'grant_vocation_look_if_missing': True, 'recreate_if_remaining_positive': True},
            'advance_look': {'skip_if': 'level_below_200_and_not_premium',
                             'restore_default_if_zero': True, 'grant_if_missing': True},
            'familiar_death': {'match': 'owner_vocation_familiar_name', 'expiry': 'current_unix_time',
                              'cancel_warning_handles': True, 'reset_spell_cooldown': False},
            'owner_removal': {'remove_summons': True, 'invoke_familiar_death': False},
            'manual_dispel': {'remove_matching_name': True, 'clear_saved_expiry': False,
                              'cancel_warning_handles': False, 'stop_after_first_match': True,
                              'caster_effect': 'magic_blue', 'creature_effect': 'poff'},
            'return_to_owner': {'metric': 'chebyshev', 'strict_distance_tiles': 15,
                                'on_any_floor_change': True, 'block_master_teleport_tile': True,
                                'protection_zone_exclusion': False},
            'protection_zone': {'can_follow': True, 'can_set_attack_target': False},
            'register_party_protection': source == 'crystal',
            'party_protection_registration': {'event': 'cast_success' if source == 'crystal' else 'never',
                                              'target': 'all_owned_summons',
                                              'order': 'after_creation_helpers_before_cooldown'},
            'visuals': {'refusal': 'poff', 'caster_success': 'magic_blue', 'creature_success': 'teleport'},
            'failure_messages': {'premium': 'You need a premium account.',
                                 'summon_limit': "You can't have other summons.",
                                 'vocation': 'not_possible', 'spawn_room': 'not_enough_room'},
        }}
    if name in SPEEDS:
        duration, a, b, familiar_duration, multiplier, offset = SPEEDS[name]
        swift = name == 'swift foot'
        return {'key': 'companion_haste', 'parameters': {
            'caster': {'condition': 'haste', 'duration_ms': duration,
                       'formula': {'mina': a, 'minb': b, 'maxa': a, 'maxb': b},
                       'basis': 'base_speed_minus_40', 'coefficient_arithmetic': 'float32',
                       'bound_conversion': 'truncate_toward_zero', 'delta': 'formula_total_minus_base'},
            'familiar': {'filter': 'monster_type_is_familiar', 'duration_ms': familiar_duration,
                         'basis': 'max_owner_base_familiar_base', 'multiplier': multiplier, 'offset': offset,
                         'arithmetic': 'lua_double_then_int32',
                         'condition': 'haste_if_positive_else_paralyze', 'zero_delta_paralyze_total': 40},
            'execution_order': ['familiar_conditions', 'caster_combat', 'swift_damage_modifier']
                               if swift else ['familiar_conditions', 'caster_combat'],
            'replacement': {'slot': 'type_id_subid', 'replace_duration': True,
                            'replace_delta': True, 'stronger_wins': False,
                            'preserve_infinite_from_timed': True},
            'opposite_condition_removal': True,
            'expiry_speed_change': 'negate_owned_delta',
            'caster_effect': 'magic_green',
            'ordinary_summons_receive_condition': False,
            'damage_dealt_percent': {'none': 70 if swift else 100,
                                     'regular': (70 if source == 'canary' else 50) if swift else 100,
                                     'greater': (70 if source == 'canary' else 100) if swift else 100},
            'damage_modifier_duration_ms': duration if swift else 0,
            'damage_modifier_after_combat_success': swift,
            'damage_modifier_condition': {'type': 'attributes', 'id': 'combat', 'sub_id': 0,
                                          'buff': False, 'preserve_infinite_from_timed': True,
                                          'preserve_longer_remaining': True,
                                          'refresh': 'end_old_effects_then_replace_entire_slot',
                                          'other_parameters': 'condition_attribute_defaults'},
            'attacks_and_casts_allowed': True,
        }}
    source_kind = {'summon creature': 'named_creature', 'animate dead rune': 'corpse_tile',
                   'convince creature rune': 'target_creature'}[name]
    corpse = source_kind == 'corpse_tile'
    named = source_kind == 'named_creature'
    return {'key': 'acquire_summon', 'parameters': {
        'source': source_kind, 'summon_cap': 2,
        'admission_order': (['monster_type', 'summonable', 'summon_cap', 'mana', 'spawn_room'] if named else
                            ['tile', 'top_down_item', 'movable_corpse', 'summon_cap_and_black_skull', 'spawn_room']
                            if corpse else ['monster_target', 'convinceable_and_master', 'summon_cap', 'mana']),
        'cap_and_flag_exemption': 'none' if corpse else 'can_summon_all' if named else 'can_convince_all',
        'required_flag': 'none' if corpse else 'summonable' if named else 'convinceable',
        'target_master': 'not_applicable' if named or corpse else 'none_or_a_carved_stone_tile',
        'refuse_black_skull': corpse,
        'corpse': {'selection': 'top_down_item' if corpse else 'none', 'requires_movable': corpse,
                   'requires_is_corpse': corpse, 'additional_age_check': False},
        'created_creature': 'Skeleton' if corpse else 'named_type' if named else 'existing_target',
        'spawn': {'position': 'target_tile' if corpse else 'caster' if named else 'existing_target',
                  'extended': named or corpse, 'force': corpse,
                  'assign_master_in_create': named, 'assign_master_after_create': corpse},
        'mana': {'source': 'none' if corpse else 'creature_type_mana_cost',
                 'insufficient_exemption': 'has_infinite_mana' if not corpse else 'none',
                 'subtract_actual_cost': not corpse, 'add_mana_spent': not corpse},
        'commit_order': ['create_skeleton', 'remove_corpse', 'assign_master', 'target_magic_blue'] if corpse else
                        ['create_owned_creature', 'subtract_mana', 'add_mana_spent', 'caster_magic_blue',
                         'creature_teleport'] if named else
                        ['subtract_mana', 'add_mana_spent', 'assign_master', 'caster_magic_blue'],
        'rune_charge_on_success': not named,
        'visuals': {'refusal': 'poff', 'success': 'magic_blue', 'created_teleport': named},
        'failure_messages': {'invalid_source': 'not_possible',
                             'summon_cap': 'You cannot summon more creatures.' if named else
                                           'You cannot control more creatures.',
                             'mana': 'not_enough_mana',
                             'spawn_room': 'not_enough_room' if named else 'not_possible'},
    }}


def build(name, spell_type, records, texts):
    """Native execution (key/parameters), or None outside the sixteen qualified records.

    records is source -> census record with optional source_root; texts is
    (source, path) -> exact UTF-8 text/bytes. Missing/changed known sources fail.
    """
    name = ' '.join(str(name).casefold().split())
    qualified = _qualify(name, spell_type, records, texts)
    if qualified is None:
        return None
    source, _ = qualified
    return copy.deepcopy(_recipe(name, source))


def evidence(name, spell_type, records, texts):
    """Qualification/selection packet separate from the executable parameters."""
    name = ' '.join(str(name).casefold().split())
    result = _qualify(name, spell_type, records, texts)
    if result is None:
        return None
    source, proofs = result
    conflicts = []
    if len(records) == 2 and name == 'swift foot':
        conflicts.append('Canary applies 70% damage for every wheel grade; Crystal applies 70/50/100%. '
                         'S21 selects the pinned Canary callback; no removed Wheel mechanic is invented.')
    if 'familiar' in name:
        conflicts.append('Pinned Canary login uses current time (offline lifetime runs); Crystal uses last logout '
                         '(offline lifetime pauses). This record retains its selected source helper; aliases share '
                         'reference ID, not independent familiar or cooldown slots.')
        conflicts.append('Source copies speed at creation and separate haste casts, not continuous owner equality. '
                         'No source swimming pause or generic lever-boss gate is invented by this recipe.')
    return {'selection': source, 'rule': 'S21' if len(records) == 2 else 'S4',
            'sources': proofs, 'conflicts': conflicts,
            'qualification': 'exact OTS execution authoring; world/character/scheduler owners and admission required'}


def _closed(value):
    if isinstance(value, dict):
        return {'type': 'object', 'additionalProperties': False,
                'properties': {key: _closed(item) for key, item in value.items()},
                'required': list(value)}
    if isinstance(value, list):
        return {'type': 'array', 'prefixItems': [_closed(item) for item in value],
                'items': False, 'minItems': len(value), 'maxItems': len(value)}
    return {'const': value}


def schemas():
    """Strict closed recipes, including the actual source conflict alternatives."""
    groups = {}
    for name, spec in SPECS.items():
        for source in spec['sources']:
            recipe = _recipe(name, source)
            variants = groups.setdefault(recipe['key'], [])
            schema = _closed(recipe['parameters'])
            if schema not in variants:
                variants.append(schema)
    # anyOf deliberately permits identical source models; oneOf could reject a
    # source-neutral acquisition recipe present in two qualified repositories.
    return {key: {'anyOf': variants} for key, variants in groups.items()}


def _f32(value):
    return struct.unpack('f', struct.pack('f', value))[0]


def caster_speed_delta(parameters, base_speed):
    """Pure source formula math; the real speed owner applies clamping/conditions."""
    model = parameters['caster']
    formula = model['formula']
    product = _f32(_f32(float(formula['mina'])) * (base_speed - 40))
    total = _f32(product + _f32(float(formula['minb'])))
    return math.trunc(total) - base_speed


def familiar_speed_delta(parameters, owner_base, familiar_base):
    model = parameters['familiar']
    return math.trunc(max(owner_base, familiar_base) * float(model['multiplier']) + float(model['offset']))


def familiar_times(parameters, config_minutes=30, vip=False, vip_reduction_minutes=0, rate_divisor=1):
    """Source unit/order fidelity, including its seconds-vs-minutes VIP cap."""
    if rate_divisor <= 0 or not math.isfinite(rate_divisor):
        raise ValueError('cooldown rate divisor must be positive and finite')
    duration_seconds = config_minutes * parameters['duration']['seconds_multiplier']
    cooldown_seconds = duration_seconds * parameters['cooldown']['duration_multiplier']
    if vip:
        cooldown_seconds -= min(vip_reduction_minutes, duration_seconds) * 60
    return duration_seconds * 1000, cooldown_seconds * 1000 / rate_divisor


def login_remaining(parameters, expiry_unix, now_unix, last_logout_unix):
    model = parameters['login']
    basis = now_unix if model['lifetime_basis'] == 'current_unix_time' else last_logout_unix
    remaining = expiry_unix - basis
    return max(remaining, 0) if model['clamp_remaining_to_zero'] else remaining


def warning_schedule(parameters, remaining_ms):
    model = parameters['warning_dispatch']
    return [{'index': i, 'raw_delay_ms': remaining_ms - warning['remaining_ms'],
             'delay_ms': max(model['scheduler_minimum_ms'], remaining_ms - warning['remaining_ms'], 0),
             'message': model['prefix'] + warning['message']}
            for i, warning in enumerate(parameters['warnings'])]


def must_return(parameters, dx, dy, dz, master_tile_is_teleport=False):
    rule = parameters['return_to_owner']
    return (not master_tile_is_teleport and
            (abs(dz) > 0 or max(abs(dx), abs(dy)) > rule['strict_distance_tiles']))


def acquisition_admission(parameters, facts):
    """Pure ordered refusal/cost plan. Mutations remain with the real world owner.

    Required facts are read only when their source check is reached, preserving
    flag exemptions. Nothing is reserved, spent, removed, spawned or owned here.
    """
    kind = parameters['source']
    exemption = parameters['cap_and_flag_exemption']
    bypass = exemption != 'none' and bool(facts.get(exemption, False))
    messages = parameters['failure_messages']
    if kind == 'named_creature':
        if not facts['type_found']:
            return {'accepted': False, 'reason': messages['invalid_source']}
        if not bypass and not facts['summonable']:
            return {'accepted': False, 'reason': messages['invalid_source']}
    elif kind == 'corpse_tile':
        if not (facts['tile_present'] and facts['top_down_item_present'] and facts['is_corpse'] and facts['movable']):
            return {'accepted': False, 'reason': messages['invalid_source']}
    else:
        if not facts['target_is_monster']:
            return {'accepted': False, 'reason': messages['invalid_source']}
        if not bypass and (not facts['convinceable'] or
                           (facts['master_name'] is not None and
                            str(facts['master_name']).lower() != 'a carved stone tile')):
            return {'accepted': False, 'reason': messages['invalid_source']}
    if (not bypass and facts['owned_summons'] >= parameters['summon_cap']) or (
            parameters['refuse_black_skull'] and facts['black_skull']):
        return {'accepted': False, 'reason': messages['summon_cap']}
    cost = 0 if kind == 'corpse_tile' else facts['creature_mana_cost']
    if kind != 'corpse_tile' and facts['mana'] < cost and not facts['has_infinite_mana']:
        return {'accepted': False, 'reason': messages['mana']}
    if kind != 'target_creature' and not facts['spawn_room']:
        return {'accepted': False, 'reason': messages['spawn_room']}
    return {'accepted': True, 'mana_cost': cost, 'add_mana_spent': parameters['mana']['add_mana_spent'],
            'consume_rune_charge': parameters['rune_charge_on_success'],
            'commit_order': copy.deepcopy(parameters['commit_order'])}
