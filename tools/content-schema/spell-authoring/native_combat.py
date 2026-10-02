"""Twenty immutable combat/state authoring candidates, qualified as source hypotheses.

This module supplies explicit mechanics, not a runtime executor. S24 preserves
accepted official/wiki overrides; S21 selects the pinned Canary mechanics where
higher sources are silent. Source disagreements and owner boundaries are evidence,
never executable parameters. Source changes fail closed at full-file hashes.
"""
from copy import deepcopy
import hashlib
from pathlib import Path
import subprocess

from formula_corrections_magic import _bound as _magic_reference_bound, _recenter_source_interval
from formula_corrections_skill import _bound as _skill_reference_bound

PINS = {'canary': '99902524e052f37574194466c2949c576e4ab269',
        'crystal': 'ff7ede593c69d4c658b382c97443e8155926924a'}
SPECS = {'avatar of balance': {'canary': {'file': 'data/scripts/spells/support/avatar_of_balance.lua',
                                  'blob': '3b12aa64e59982c1ae7961498d926175af927ff6',
                                  'sha256': '5990e6351b1ecab910ea2c6bd08f62389f98587f4de71033e9292805bf5926e1'},
                       'crystal': {'file': 'data/scripts/spells/support/avatar_of_balance.lua',
                                   'blob': '4dc34dd73f3000394a3fdf216254cc6a033a8975',
                                   'sha256': '2182453457459c7249f2a4f3644f023ff5350afc1d85302decdb9a26e9288212'}},
 'avatar of light': {'canary': {'file': 'data/scripts/spells/support/avatar_of_light.lua',
                                'blob': 'a84fd3b8c8e75b5b9f5d2d75c2c55fde0ce00835',
                                'sha256': 'ae6b1efdfcb9b5301e5b2acc1469367cd1dc7bd33f56c7f8b854d49f3b43f0fc'},
                     'crystal': {'file': 'data/scripts/spells/support/avatar_of_light.lua',
                                 'blob': 'a3d74f6d3a498629007cd7737cb4b99037a570b6',
                                 'sha256': '2805274f94e6647e8a62fe84ff5a3e87b6ef82c74996c9a0c6fba948e344fc30'}},
 'avatar of nature': {'canary': {'file': 'data/scripts/spells/support/avatar_of_nature.lua',
                                 'blob': 'fa32dd6b821b4ab757ac366a131c4f3d4013041a',
                                 'sha256': 'b58c5eb3697bb8e03399f7b1fb643ef3e22021a5231e77f7dcc8a8b3655e36dc'},
                      'crystal': {'file': 'data/scripts/spells/support/avatar_of_nature.lua',
                                  'blob': '985b372c13dea0e79400b97edc4aefaef874185b',
                                  'sha256': '8475c277101a94f7c0fb325dbdb604373bfa98fbda19d4832df399d2dec36d31'}},
 'avatar of steel': {'canary': {'file': 'data/scripts/spells/support/avatar_of_steel.lua',
                                'blob': 'da71ff14f96fcef1bf8247f9d1ee17508edc280c',
                                'sha256': '358e9b902c7cb2b930b4c8c3101906c5847c88ef630ae9428cefbb9871180463'},
                     'crystal': {'file': 'data/scripts/spells/support/avatar_of_steel.lua',
                                 'blob': '2da2439b4a139bd988940de83e4e8321d8c3e089',
                                 'sha256': '4916557d9d679c0d40180119cf9ced2de99fe4246662c389f90950e3ca41a759'}},
 'avatar of storm': {'canary': {'file': 'data/scripts/spells/support/avatar_of_storm.lua',
                                'blob': 'ad644de01ba48e12b7858659e5f06ecc2d2953ee',
                                'sha256': 'f2563bd06d3a589f13796e17a2e1b5f5a02aa0c169985165545c5a76d09169ee'},
                     'crystal': {'file': 'data/scripts/spells/support/avatar_of_storm.lua',
                                 'blob': '848f1e9d5c02fed90faef6800d6e459ea512429a',
                                 'sha256': '29d418f98150f294acfda9ef186a53b7cae4df12630e1a567d75947a8fb94663'}},
 'balanced brawl': {'canary': {'file': 'data/scripts/spells/support/balanced_brawl.lua',
                               'blob': '72c47a8c1e02c59bc579791663545b76a93beb1d',
                               'sha256': '7b7ec08ae1a4a2b7b27a6cb9478859bb6a22347c05363c74678fe6c680aae14e'},
                    'crystal': {'file': 'data/scripts/spells/support/balanced_brawl.lua',
                                'blob': 'f11e6ed84110963ad2bf37ebd147f57e7907b5e7',
                                'sha256': 'fe89e54bb88fabfde5daf85ee233140614e22347a41064574a9b1c2698f0fdc4'}},
 'challenge': {'canary': {'file': 'data/scripts/spells/support/challenge.lua',
                          'blob': '2b29d5ba0ced5be783e6b144f6d85ab6ac43aa43',
                          'sha256': 'ea1558740279f1ff191e9d354ffc6b5b1595a3f144ff7b78f869f2524c484198'},
               'crystal': {'file': 'data/scripts/spells/support/challenge.lua',
                           'blob': '2b29d5ba0ced5be783e6b144f6d85ab6ac43aa43',
                           'sha256': 'ea1558740279f1ff191e9d354ffc6b5b1595a3f144ff7b78f869f2524c484198'}},
 'chivalrous challenge': {'canary': {'file': 'data/scripts/spells/support/chivalrous_challenge.lua',
                                     'blob': 'eb85056b26c785123c6d53b26151390a7751741f',
                                     'sha256': '8f5dd04e0f045cedd0a30b993c154a8e7cc4b8d1d774547de188a92184c8aa41'},
                          'crystal': {'file': 'data/scripts/spells/support/chivalrous_challenge.lua',
                                      'blob': '1ca23f267d075c70d0e1954c204c0b3be1f8c655',
                                      'sha256': '03ddb361d23bbf1d0346fcdf8d206842a2ebb24c76a3da65b26825da2b392573'}},
 'divine dazzle': {'canary': {'file': 'data/scripts/spells/support/divine_dazzle.lua',
                              'blob': '56548b5c4e862d5493bab2e0e79f65ce6d3f0c26',
                              'sha256': '0c9339cded11f2299751a939299b1f40917beffedcb41b46a1ad35ed09ee1c4d'},
                   'crystal': {'file': 'data/scripts/spells/support/divine_dazzle.lua',
                               'blob': 'b2e5854dc956326c2823d3401771c3a4fb123ab8',
                               'sha256': '913f39f790abe9ec6ba77eb32ea025f877b5333817b97af7d060578a03e0a89e'}},
 'energy beam': {'canary': {'file': 'data/scripts/spells/attack/energy_beam.lua',
                            'blob': '8c666b971618d5b13bb49129443d1694aedeabc6',
                            'sha256': '45fbc0e9699cd506a822588795976b6da81a479aded40d074dd3e773875db643'},
                 'crystal': {'file': 'data/scripts/spells/attack/energy_beam.lua',
                             'blob': '8cb15b4f53b360b0e444bf8663947d0ab2578281',
                             'sha256': '2ea17c4b8270395750b5c0e4899fa8d0afdce8fdf547b64dfec091b095306bf3'}},
 'energy wave': {'canary': {'file': 'data/scripts/spells/attack/energy_wave.lua',
                            'blob': '09eae51b8b3534a77f69e114a649a13f444f02e1',
                            'sha256': 'df4e872e78796b08a9bff64deafa078598e37d44e4309ddf2b65a27ff542a3d4'},
                 'crystal': {'file': 'data/scripts/spells/attack/energy_wave.lua',
                             'blob': '3ff932fc08a5f50b613d4737fa9ca3c869dc1ae3',
                             'sha256': '638b2c9738ac996532d84b6a5105b036938cf9312140cf73510bb9b8c041ddc8'}},
 "executioner's throw": {'canary': {'file': 'data/scripts/spells/attack/executioners_throw.lua',
                                    'blob': '08d700bfe54f311ad7920c602bcf54fe3d897fb7',
                                    'sha256': '40a61a0748fcb845ad51268ccea8ad1cff4feb7e6b0fb2d5d5df748649a3e3fb'},
                         'crystal': {'file': 'data/scripts/spells/attack/executioners_throw.lua',
                                     'blob': '98bc6a7bba29cebbea4714470bbc86a1a95ba767',
                                     'sha256': '9d5da87288e84baafcdc059bffcec546cbcf89b9ae4771d0ba18c28d008e3a58'}},
 'front sweep': {'canary': {'file': 'data/scripts/spells/attack/front_sweep.lua',
                            'blob': '245db03aab6e54eda3f1e2a824d6ad8cdc28d564',
                            'sha256': 'c23f1f471628288890b89d4e5d27c0cdf10dc0f1383bfd6a2e15e7f0d51c9fde'},
                 'crystal': {'file': 'data/scripts/spells/attack/front_sweep.lua',
                             'blob': 'a2cdaa65ab4f53776bc8fbfdeca0af18f3731dfd',
                             'sha256': 'cb9580dfca578ce7e53fcfd8bfc652918ef20b11d88445513722703b7d1a74c3'}},
 'great energy beam': {'canary': {'file': 'data/scripts/spells/attack/great_energy_beam.lua',
                                  'blob': 'ff7fdf65c9d10da482715bc7af43e6f11db51415',
                                  'sha256': '57b775ec5c54ef6533092dd32b0ce1e30b6e51f6fef246a0d22000168e6658eb'},
                       'crystal': {'file': 'data/scripts/spells/attack/great_energy_beam.lua',
                                   'blob': '8e09fad625212027f2767c866c25e4a7aef80289',
                                   'sha256': '902edd1b46d4dd2352ae3e4ccc2dee539d89e15f9631af8c5845c80490b6017a'}},
 'ice burst': {'canary': {'file': 'data/scripts/spells/attack/ice_burst.lua',
                          'blob': 'fedcd9d5a0484523fb8473864617b65212d1b846',
                          'sha256': '4da735ea2efb7e8486908fcba38cb6358220cfa936ac8cf9beeaac528175f420'},
               'crystal': {'file': 'data/scripts/spells/attack/ice_burst.lua',
                           'blob': '58dd47f8539724eea61c66def2bcba5792425e8e',
                           'sha256': 'f7b0b7314dea9269a21b1702c380869867ba5f7f7fceada65da7e6010a5612a5'}},
 'magic shield': {'canary': {'file': 'data/scripts/spells/support/magic_shield.lua',
                             'blob': 'ff55e21eafc8973c6a475962213d2832ec1bb1b0',
                             'sha256': 'e07329dc6bdffebea395ccad3022173d68175b36f8d4cb9ef7024752498fcc43'},
                  'crystal': {'file': 'data/scripts/spells/support/magic_shield.lua',
                              'blob': '501f311e130fe3a17c73165fda5de75b80356ee5',
                              'sha256': 'b4b52ec43ca9293b213b347c46df4870cc34e1315d5a6109d66d294c05d72425'}},
 'mass healing': {'canary': {'file': 'data/scripts/spells/healing/mass_healing.lua',
                             'blob': '911249515b2337d1a43f119e7941a28dcf362a9d',
                             'sha256': 'ce0636091f1f2dfe19199cbd293890a91dd40ace78922c1b97386d1739ab8dee'},
                  'crystal': {'file': 'data/scripts/spells/healing/mass_healing.lua',
                              'blob': 'e5fe533d670be0c406d2b66a513cf575dc584a47',
                              'sha256': '8c9c668fb9d2bf5d73af385dd4ee0b8e42a663fd01d98955c9eeeaa93c6a5965'}},
 'mass spirit mend': {'canary': {'file': 'data/scripts/spells/healing/mass_spirit_mend.lua',
                                 'blob': '1211f0cb4e9d6624cd6c000c1b8cbec07e19a5d9',
                                 'sha256': '2a3641a911fe3e1e674fce5fe918c6079c9d9a43c9af41d044f9758b448e8292'},
                      'crystal': {'file': 'data/scripts/spells/healing/mass_spirit_mend.lua',
                                  'blob': '5bda833b123ed3ccb9491f2b6b8ac46bc0befe61',
                                  'sha256': '7f5ebbd86e6f2b18979f6dd26ce2147d758d4e51667c38553553bdb8b0d02a78'}},
 'strong ice wave': {'canary': {'file': 'data/scripts/spells/attack/strong_ice_wave.lua',
                                'blob': 'ac95df810939e1ed39a11ba9dca9e33d0c6aaa11',
                                'sha256': 'eddb1a06133fa58291d84f851aa7f2328da9dea9c59361ae033a04ed6c470750'},
                     'crystal': {'file': 'data/scripts/spells/attack/strong_ice_wave.lua',
                                 'blob': '8f0d23ac9541349092cf12e88512f48fe3333fc1',
                                 'sha256': '0b7f65aeed6265ac222a43eeb9b7da63d4686c6da8d7f56dd1d3e342e4281db0'}},
 'terra burst': {'canary': {'file': 'data/scripts/spells/attack/terra_burst.lua',
                            'blob': '15ce35c4912bc3acbccd96fbd176fd39a5fdac9f',
                            'sha256': '2515f44786b24025c345f05dde4c395b5399959f5c9ad643ab356d949efe13a4'},
                 'crystal': {'file': 'data/scripts/spells/attack/terra_burst.lua',
                             'blob': '01d9b57a8537dafd2b2679e4059f444fb98ab407',
                             'sha256': '50c39f4a2b763eb432096c46a42547a400d4bb339b1cdf676b6a1d0692607014'}}}
HELPERS = {'data/scripts/lib/register_spells.lua': 'a0a6ff2bdfcd82bc4c36978fc57061eab9f1537e8ca966c51300471e3d3313ae',
 'src/creatures/combat/combat.cpp': '3dc306b44b9ea0c04a48cb6dcdca63b3982a4ee74484cb6a9da40265ecc7fc49',
 'src/creatures/monsters/monster.cpp': 'd6d80082007a8852c8f22c2ed1d7a4f345b2000298b5b8468270dfa2a70ad177',
 'src/creatures/players/components/wheel/player_wheel.cpp': 'e1eef362cce79252cae9f9d469b57ccb5a1b1658e848ed8c8d1881f5c8650717',
 'src/io/io_wheel.cpp': '1afc184459f84153749214f804a628d8a1fe9fe6ac9b9f8a8b60077c3c0cbd95',
 'src/lua/functions/core/game/global_functions.cpp': '29016a1c1edbcd597e9bd6d51f34edce2f616b9d4abb762e76a6e99537e30c02',
 'src/map/map_const.hpp': 'a5539ed369bf40ba8d4442b90f5a121895cf8e8a5f786853478628d25be47a1a'}
AREAS = {'AREADIAGONAL_BEAM5': [[1, 0, 0, 0, 0],
                        [0, 1, 0, 0, 0],
                        [0, 0, 1, 0, 0],
                        [0, 0, 0, 1, 0],
                        [0, 0, 0, 0, 3]],
 'AREADIAGONAL_BEAM7': [[1, 0, 0, 0, 0, 0, 0],
                        [0, 1, 0, 0, 0, 0, 0],
                        [0, 0, 1, 0, 0, 0, 0],
                        [0, 0, 0, 1, 0, 0, 0],
                        [0, 0, 0, 0, 1, 0, 0],
                        [0, 0, 0, 0, 0, 1, 0],
                        [0, 0, 0, 0, 0, 0, 3]],
 'AREADIAGONAL_SQUAREWAVE5': [[1, 1, 1, 0, 0],
                              [1, 1, 1, 0, 0],
                              [1, 1, 1, 0, 0],
                              [0, 0, 0, 1, 0],
                              [0, 0, 0, 0, 3]],
 'AREADIAGONAL_WAVE6': [[0, 0, 1], [0, 3, 0], [1, 0, 0]],
 'AREADIAGONAL_WAVE7': [[0, 0, 0, 0, 0, 1, 0],
                        [0, 0, 0, 0, 1, 1, 0],
                        [0, 0, 0, 1, 1, 1, 0],
                        [0, 0, 1, 1, 1, 1, 0],
                        [0, 1, 1, 1, 1, 1, 0],
                        [1, 1, 1, 1, 1, 1, 0],
                        [0, 0, 0, 0, 0, 0, 3]],
 'AREA_BALANCED_BRAWL': [[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                         [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0],
                         [0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0],
                         [0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0],
                         [0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0],
                         [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0],
                         [0, 1, 1, 1, 1, 0, 2, 0, 1, 1, 1, 1, 0],
                         [1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1]],
 'AREA_BEAM10': [[1], [1], [1], [1], [1], [1], [1], [1], [1], [3]],
 'AREA_BEAM5': [[1], [1], [1], [1], [3]],
 'AREA_BEAM7': [[1], [1], [1], [1], [1], [1], [3]],
 'AREA_BEAM8': [[1], [1], [1], [1], [1], [1], [1], [3]],
 'AREA_CIRCLE3X3': [[0, 0, 1, 1, 1, 0, 0],
                    [0, 1, 1, 1, 1, 1, 0],
                    [1, 1, 1, 1, 1, 1, 1],
                    [1, 1, 1, 3, 1, 1, 1],
                    [1, 1, 1, 1, 1, 1, 1],
                    [0, 1, 1, 1, 1, 1, 0],
                    [0, 0, 1, 1, 1, 0, 0]],
 'AREA_CIRCLE3X4': [[0, 0, 0, 1, 1, 1, 0, 0, 0],
                    [0, 0, 1, 1, 1, 1, 1, 0, 0],
                    [0, 1, 1, 1, 1, 1, 1, 1, 0],
                    [1, 1, 1, 1, 1, 1, 1, 1, 1],
                    [1, 1, 1, 1, 3, 1, 1, 1, 1],
                    [1, 1, 1, 1, 1, 1, 1, 1, 1],
                    [0, 1, 1, 1, 1, 1, 1, 1, 0],
                    [0, 0, 1, 1, 1, 1, 1, 0, 0],
                    [0, 0, 0, 1, 1, 1, 0, 0, 0]],
 'AREA_MASS_SPIRIT_MEND': [[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                           [0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0],
                           [0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0],
                           [0, 0, 1, 1, 1, 1, 1, 1, 1, 0, 0],
                           [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0],
                           [0, 1, 1, 1, 1, 3, 1, 1, 1, 1, 0],
                           [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0],
                           [0, 0, 1, 1, 1, 1, 1, 1, 1, 0, 0],
                           [0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0],
                           [0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0],
                           [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]],
 'AREA_RING1_BURST3': [[0, 0, 0, 1, 1, 1, 0, 0, 0],
                       [0, 0, 1, 1, 1, 1, 1, 0, 0],
                       [0, 1, 1, 1, 1, 1, 1, 1, 0],
                       [1, 1, 1, 0, 0, 0, 1, 1, 1],
                       [1, 1, 1, 0, 2, 0, 1, 1, 1],
                       [1, 1, 1, 0, 0, 0, 1, 1, 1],
                       [0, 1, 1, 1, 1, 1, 1, 1, 0],
                       [0, 0, 1, 1, 1, 1, 1, 0, 0],
                       [0, 0, 0, 1, 1, 1, 0, 0, 0]],
 'AREA_SQUARE1X1': [[1, 1, 1], [1, 3, 1], [1, 1, 1]],
 'AREA_SQUAREWAVE5': [[1, 1, 1], [1, 1, 1], [1, 1, 1], [0, 1, 0], [0, 3, 0]],
 'AREA_WAVE6': [[0, 0, 0, 0, 0], [0, 1, 3, 1, 0], [0, 0, 0, 0, 0]],
 'AREA_WAVE7': [[1, 1, 1, 1, 1], [1, 1, 1, 1, 1], [0, 1, 1, 1, 0], [0, 1, 1, 1, 0], [0, 0, 3, 0, 0]]}

CRYSTAL_HEALING_HELPER_SHA256 = '4c33e84b0b6b34f0221eb169a8f7de680ebd1444e8e702472cc1091fd91b99a9'

BOSSES = ['leiden', 'ravennous hunger', 'dorokoll the mystic', 'eshtaba the conjurer',
          'eliz the unyielding', 'mezlon the defiler', 'malkhar deathbringer', 'containment crystal']


def _c(value):
    return {'const': str(value)}


def _v(name):
    return {'var': name}


def _op(name, *args):
    return {'op': name, 'args': list(args)}


def _flat():
    return {'fn': 'level_base_damage_healing', 'args': [_v('level')]}


def _bounds(low, high, healing=False):
    return {'minimum': low, 'maximum': high, 'quantization': 'truncate_each_bound_before_draw',
            'distribution': 'world_healing_roll' if healing else 'world_damage_roll'}


def _magic(lo, hi, addlo=0, addhi=0, healing=False):
    def expr(coefficient, additive):
        return _op('add', _op('add', _flat(), _op('mul', _v('magic_level'), _c(coefficient))), _c(additive))
    return _bounds(expr(lo, addlo), expr(hi, addhi), healing)


def _power(power, healing=False):
    center = _op('add', _flat(), _op('add',
                 _op('mul', _op('div', _c(power), _c(25)), _v('magic_level')),
                 _op('div', _c(power), _c(4))))
    return _bounds(_op('mul', deepcopy(center), _c('0.9')),
                   _op('mul', center, _c('1.1')), healing)


def _inline_power(expression, power):
    if expression.get('var') == 'base_power':
        return _c(power)
    result = deepcopy(expression)
    if 'args' in result:
        result['args'] = [_inline_power(arg, power) for arg in result['args']]
    return result


def _qualified_magic(power, buckets):
    return _bounds(_inline_power(_magic_reference_bound(buckets, -1), power),
                   _inline_power(_magic_reference_bound(buckets, 1), power))


def _nominal_magic_source_interval(power, source_formula):
    result = _recenter_source_interval(source_formula)
    if result is None:
        raise ValueError('source interval cannot be qualified with magic inputs')
    for bound in ("minimum", "maximum"):
        result[bound] = _inline_power(result[bound], power)
    return result


def _qualified_skill(power, buckets):
    model = (power, buckets, '1000', 'weapon', 'round')
    return _bounds(_inline_power(_skill_reference_bound(model, -1), power),
                   _inline_power(_skill_reference_bound(model, 1), power))


def _base_power_healing(power):
    # The pinned Crystal BP healing helper is an explicit source hypothesis,
    # not an independent calculator card or measured live healing formula.
    def expression(coefficient, additive, quantizer):
        contribution = _op('mul', _v('magic_level'), _op('div',
                           _op('mul', _c(power), _c(coefficient)), _c(250)))
        return _op(quantizer, _op('add', _op('add', _flat(), contribution), _c(additive)))
    return _bounds(expression('7.3', 42, 'floor'), expression('12.4', 90, 'ceil'), True)


def _area(orthogonal, diagonal=None, directional=True):
    return {'orthogonal': deepcopy(AREAS[orthogonal]),
            'diagonal': deepcopy(AREAS[diagonal]) if diagonal else None,
            'directional': directional, 'encoding': '0_inactive_1_hit_2_origin_3_origin_hit',
            'same_floor': True, 'clear_sight': True}


def _stages(*values):
    return {str(i): value for i, value in enumerate(values)}


def _gate(perk, revelation=True, required=False):
    return {'owner': 'wheel_of_destiny', 'perk': perk,
            'stage_kind': 'revelation' if revelation else 'augment',
            'stage_min': 0, 'stage_max': 3 if revelation else 2,
            'stage_zero': 'reject_before_costs' if required else 'base_cast',
            'snapshot': 'cast_start'}


def _routing():
    return {'heal': 'players_and_player_summons_and_named_monsters',
            'monster_name_casefold_allowlist': deepcopy(BOSSES),
            'reject_masterless_and_monster_owned_monsters_except_allowlist': True,
            'dispel': 'paralysis', 'dispel_targets': 'legal_combat_area_targets',
            'heal_targets': 'heal_filter_after_area_legality', 'aggressive': False}


def _chain(further, jump, first):
    return {'further_targets': further, 'jump_radius': jump,
            'radius_metric': 'square', 'next_target_metric': 'euclidean',
            'tie_order': 'source_spectator_iteration', 'first_target': first,
            'same_floor': True, 'clear_sight': True, 'combat_legality_required': True,
            'exclude_caster': True, 'repeat_targets': False}


def _templates():
    result = {}
    for name, outfit in [('avatar of balance', 1823), ('avatar of light', 1594),
                         ('avatar of nature', 1596), ('avatar of steel', 1593), ('avatar of storm', 1595)]:
        result[name] = {'key': 'avatar_state', 'parameters': {
            'wheel': _gate(name, required=True), 'duration_ms': 15000, 'outfit_look_type': outfit,
            'incoming_damage_reduction_percent': _stages(0, 5, 10, 15),
            'incoming_reduction_rounding': 'ceil_reduction_per_hit',
            'critical_chance_percent': 100, 'critical_extra_damage_percentage_points': _stages(0, 5, 10, 15),
            'cooldown_ms': _stages(7200000, 7200000, 5400000, 3600000),
            'state_clock': 'world_monotonic_ms', 'state_active_until': 'strictly_greater_than_now',
            'refresh_derived_stats': ['on_apply', 'on_expiry'],
            'expiry_target': 'same_creature_identity', 'on_recast': 'replace_deadline_and_outfit',
            'effect_asset_binding': 'appearance:effect/avatar_appear'}}
    common = {'origin': 'spell', 'block_armor': False, 'use_weapon_charges': False,
              'sign': 'positive_magnitude', 'target_legality': 'world_combat_rules'}
    definitions = [
        ('energy beam', 'energy', 'energyhit', _nominal_magic_source_interval(60, _magic('1.8', 3, 11, 19)),
         _area('AREA_BEAM5', 'AREADIAGONAL_BEAM5'), _area('AREA_BEAM7', 'AREADIAGONAL_BEAM7')),
        ('great energy beam', 'energy', 'energyarea', _qualified_magic(155, 60), _area('AREA_BEAM8'), _area('AREA_BEAM10')),
        ('energy wave', 'energy', 'energyarea', _qualified_magic(150, 80),
         _area('AREA_SQUAREWAVE5', 'AREADIAGONAL_SQUAREWAVE5'), _area('AREA_WAVE7', 'AREADIAGONAL_WAVE7')),
        ('strong ice wave', 'ice', 'icearea', _qualified_magic(140, 80),
         {'orthogonal': [[1,1,1],[1,1,1],[1,1,1],[0,3,0]], 'diagonal': None,
          'directional': True, 'encoding': '0_inactive_1_hit_2_origin_3_origin_hit',
          'same_floor': True, 'clear_sight': True}, _area('AREA_WAVE7')),
        ('front sweep', 'physical', 'hitarea', _qualified_skill(80, 40), _area('AREA_WAVE6','AREADIAGONAL_WAVE6'),
         {'orthogonal': [[1,1,3,1,1]], 'diagonal': [[0,0,0,0,1],[0,0,0,1,0],[0,0,3,0,0],[0,1,0,0,0],[1,0,0,0,0]],
          'directional': True, 'encoding': '0_inactive_1_hit_2_origin_3_origin_hit',
          'same_floor': True, 'clear_sight': True}),
        ('mass healing', 'healing', 'magic_blue', _base_power_healing(200),
         _area('AREA_CIRCLE3X3', directional=False), _area('AREA_CIRCLE3X4', directional=False)),
        ('ice burst', 'ice', 'iceattack', _qualified_magic(115, 60),
         _area('AREA_RING1_BURST3', directional=False), _area('AREA_RING1_BURST3', directional=False)),
        ('terra burst', 'earth', 'smallplants', _qualified_magic(115, 60),
         _area('AREA_RING1_BURST3', directional=False), _area('AREA_RING1_BURST3', directional=False)),
    ]
    for name, element, effect, formula, area, enhanced in definitions:
        beam = name in ('energy beam', 'great energy beam')
        burst = name in ('ice burst', 'terra burst')
        perk = 'Beam Mastery' if beam else 'Twin Burst' if burst else name
        params = {**deepcopy(common), 'element': element,
                  'effect_asset_binding': 'appearance:effect/' + effect,
                  'formula': formula, 'base_area': area, 'enhanced_area': enhanced,
                  'wheel': _gate(perk, beam or burst, burst),
                  'enhanced_area_from_stage': 1 if name == 'energy wave' or beam else 2 if not burst else 1,
                  'target_selection': 'caster_position' if burst or name == 'mass healing' else 'caster_direction',
                  'damage_or_heal_bonus_percent': _stages(0,0,10) if name == 'energy wave' else
                        _stages(0,6,6) if name == 'strong ice wave' else
                        _stages(0,40,40) if name == 'front sweep' else
                        _stages(0,4,4) if name == 'mass healing' else _stages(0,0,0,0)}
        if beam:
            params['beam_mastery'] = {
                'side_damage_percent': _stages(0,25,40,70), 'side_rounding': 'round_nearest_half_away_from_zero',
                'side_offsets_by_direction': {'north':[[-1,0],[1,0]],'south':[[-1,0],[1,0]],
                    'east':[[0,-1],[0,1]],'west':[[0,-1],[0,1]],'northwest':[[1,0],[0,1]],
                    'northeast':[[-1,0],[0,1]],'southwest':[[1,0],[0,-1]],'southeast':[[-1,0],[0,-1]]},
                'side_geometry': 'translate_active_central_tiles_then_exclude_caster_and_central_and_duplicates',
                'reject_side_floor_change_tiles': True, 'side_sight_origin': 'source_adjacent_caster_tile',
                'central_target_bonus_percent': _stages(0,10,12,14), 'central_target_count_cap': 3,
                'counted_targets': 'combat_legal_central_creatures_before_health_resolution',
                'central_bonus_applies_to': 'central_targets_only', 'hit_once_per_creature': True,
                'critical_and_fatal_resolution': 'once_for_complete_beam',
                'reduce_all_spell_cooldowns_ms_per_counted_target': 1000,
                'cooldown_reduction_timing': 'before_health_resolution'}
            if name == 'great energy beam':
                params['shared_spell_cooldown'] = {'peer': 'great death beam', 'duration_ms': 6000}
        if burst:
            params['health_bonus'] = {'health_percent_rounding': 'nearest_half_away_from_zero',
                'comparison': 'greater_than', 'threshold': 60, 'bonus_percent': _stages(0,20,40,60)}
            params['cooldown_ms'] = _stages(22000,22000,18000,14000)
            params['shared_spell_cooldown'] = {'peer': 'terra burst' if name == 'ice burst' else 'ice burst',
                                               'duration_ms': _stages(22000,22000,18000,14000)}
        if name == 'front sweep':
            params.update(block_armor=True, use_weapon_charges=True)
        if name == 'mass healing':
            params['target_routing'] = _routing()
        # Crystal's stance script is newer than the pinned Canary baseline but
        # the candidate's official stance routing applies to these named spells.
        if name in ('energy beam', 'energy wave', 'great energy beam'):
            params['elemental_stance'] = {'owner': 'elemental_stance', 'snapshot': 'cast_start',
                'routes': {'none': 'energy', 'master_of_thunder': 'energy',
                           'master_of_flames': 'fire', 'master_of_decay': 'death'},
                'formula_changes': False, 'presentation_owner': 'elemental_stance'}
        result[name] = {'key': 'wheel_combat', 'parameters': params}
    result["executioner's throw"] = {'key': 'wheel_combat', 'parameters': {
        **deepcopy(common), 'element': 'physical', 'block_armor': True,
        'missile_selection': 'equipped_weapon_type', 'formula': _qualified_skill(60, 10),
        'wheel': _gate("Executioner's Throw", required=True),
        'target_selection': 'explicit_target', 'first_target_range': 5,
        'chain': _chain(_stages(0,2,3,4),3,'explicit_target'),
        'cooldown_ms': _stages(18000,18000,14000,10000),
        'health_bonus': {'health_percent_rounding': 'nearest_half_away_from_zero',
            'comparison': 'less_than_or_equal', 'threshold': 30,
            'bonus_percent': _stages(0,100,125,150)}}}
    result['mass spirit mend'] = {'key': 'mass_spirit_mend', 'parameters': {
        'area': _area('AREA_MASS_SPIRIT_MEND',directional=False), 'target_routing': _routing(),
        'caster_formula': _magic('7.22','12.79',44,79,True), 'others_formula': _power(800,True),
        'formula_route': 'caster_identity_equality', 'effect_asset_binding': 'appearance:effect/magic_blue',
        'wheel': _gate('Mass Spirit Mend',False), 'healing_bonus_percent': _stages(0,8,8),
        'cooldown_ms': _stages(12000,12000,8000), 'harmony_spender': False}}
    for name in ('balanced brawl','challenge','chivalrous challenge','divine dazzle'):
        chain = name in ('chivalrous challenge','divine dazzle')
        params = {'target_filter': 'masterless_ranged_non_reward_monsters' if chain else
                   'masterless_non_reward_monsters' if name == 'balanced brawl' else 'masterless_monsters',
                  'recast': 'replace_deadline', 'same_floor': True, 'combat_legality_required': True,
                  'effect_asset_binding': 'appearance:effect/divine_dazzle' if name=='divine dazzle' else
                     'appearance:effect/chivalrious_challenge' if name=='chivalrous challenge' else
                     'appearance:effect/magic_blue',
                  'on_empty': 'reject_before_costs' if chain else 'commit_legal_area_cast'}
        if chain:
            params['chain'] = _chain(_stages(2,4,4) if name=='divine dazzle' else 3,
                                    7,'attacked_valid_target_else_nearest_valid_target')
            params['reward_boss_cast_refusal'] = {'dx_min':-11,'dx_max':11,'dy_min':-11,'dy_max':11,'same_floor':True}
            params['force_distance'] = 1
            params['forced_melee_ms'] = _stages(8000,8000,12000) if name=='divine dazzle' else 12000
            params['cooldown_ms'] = _stages(16000,16000,8000) if name=='divine dazzle' else 16000
            if name=='divine dazzle':
                params['wheel'] = _gate('Divine Dazzle',False)
            else:
                params['challenge_target'] = 'caster'
                params['challenge_ms'] = 6000
        elif name=='balanced brawl':
            params['area'] = _area('AREA_BALANCED_BRAWL')
            params['force_distance'] = 1
            params['forced_melee_ms'] = 16000
            params['reward_boss_cast_refusal'] = None
        else:
            params['area'] = _area('AREA_SQUARE1X1',directional=False)
            params['challenge_target'] = 'caster'
            params['challenge_ms'] = 6000
        result[name] = {'key':'monster_ai_override','parameters':params}
    capacity = _op('min', _v('maximum_mana'), _op('ceil', _op('add',
        _op('add',_op('mul',_c(7),_v('magic_level')),_op('mul',_c('7.6'),_v('level'))),
        _op('max',_c(300),_op('mul',_c('0.4'),_v('level'))))))
    result['magic shield'] = {'key':'mana_shield_capacity','parameters':{
        'capacity':capacity, 'duration_ms':180000, 'target':'caster',
        'condition':'mana_shield', 'application_timing':'before_presentation_combat',
        'recast':'replace_capacity_and_deadline', 'damage_destination':'mana_before_health_until_capacity_depleted',
        'capacity_snapshot':'cast_start', 'effect_asset_binding':'appearance:effect/magic_blue',
        'wheel_capacity_multiplier':False}}
    return result


TEMPLATES = _templates()


def _head(root):
    try:
        return subprocess.run(['git','-C',str(root),'rev-parse','HEAD'],
                              check=True,capture_output=True,text=True).stdout.strip()
    except (OSError,subprocess.SubprocessError):
        return None


def _qualified(name, spell_type, records, texts):
    if spell_type != 'instant' or name not in SPECS or not isinstance(records,dict) or not isinstance(texts,dict):
        return False
    if 'canary' not in records or set(records)-set(PINS):
        return False
    for source, record in records.items():
        spec = SPECS[name].get(source)
        if not isinstance(record,dict) or spec is None:
            return False
        if record.get('name','').casefold()!=name or record.get('spell_type')!='instant':
            return False
        if record.get('revision',PINS[source])!=PINS[source] or _head(record.get('source_root'))!=PINS[source]:
            return False
        if record.get('file')!=spec['file'] or record.get('blob')!=spec['blob']:
            return False
        text = texts.get((source,spec['file']))
        if not isinstance(text,str):
            return False
        data=text.encode('utf-8')
        if hashlib.sha256(data).hexdigest()!=spec['sha256']:
            return False
        if hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()!=spec['blob']:
            return False
    root=Path(records['canary']['source_root'])
    for path,expected in HELPERS.items():
        supplied=texts.get(('canary',path))
        if supplied is not None and not isinstance(supplied,str):
            return False
        try:
            data=supplied.encode('utf-8') if isinstance(supplied,str) else (root/path).read_bytes()
        except OSError:
            return False
        if hashlib.sha256(data).hexdigest()!=expected:
            return False
    if name == 'mass healing':
        if 'crystal' not in records:
            return False
        path = 'data/scripts/lib/register_spells.lua'
        supplied = texts.get(('crystal', path))
        try:
            data = supplied.encode('utf-8') if isinstance(supplied, str) else (Path(records['crystal']['source_root']) / path).read_bytes()
        except OSError:
            return False
        if hashlib.sha256(data).hexdigest() != CRYSTAL_HEALING_HELPER_SHA256:
            return False
    return True


def build(name, spell_type, records, texts):
    normalized=str(name).casefold()
    return deepcopy(TEMPLATES[normalized]) if _qualified(normalized,spell_type,records,texts) else None


def _closed(value):
    if isinstance(value,dict):
        return {'type':'object','properties':{key:_closed(val) for key,val in value.items()},
                'required':list(value),'additionalProperties':False}
    if isinstance(value,list):
        return {'type':'array','prefixItems':[_closed(item) for item in value],
                'minItems':len(value),'maxItems':len(value),'items':False}
    return {'const':value}


def schemas():
    result={}
    for template in TEMPLATES.values():
        result.setdefault(template['key'],{'oneOf':[]})['oneOf'].append(_closed(template['parameters']))
    return result


REFERENCE_EVIDENCE = [{'authority': 'AcceptedAuthoringReference',
  'path': 'tools/content-schema/spell-authoring/wheel-augments.json',
  'sha256': 'fc1680a45bf0d28a71b7f16b700146a4e804119f39a771cf9cf8af93addf673c',
  'target_date': '2026-09-27'},
 {'authority': 'AcceptedAuthoringReference',
  'path': 'tools/content-schema/spell-authoring/official-changes.json',
  'sha256': '76338a738f8d08aaa809e718b7761415fa436c6f449bacd3c21200a09b5971d1',
  'target_date': '2026-09-27'}]


CONFLICTS = {
 'beam': [
    {'field':'side_damage_percent','selected':'25/40/70','authority':'official:8872; Canary99902524',
     'alternative':'Crystalff7ede5 legacy40/60/80'},
    {'field':'side_geometry_and_count_timing','selected':'pinned Canary combat.cpp topology and pre-health count',
     'authority':'S21 OtsHypothesisOnly','alternative':'Crystal orthogonal flank matrix with approximate diagonals'}],
 'execution': [
    {'field':'further_targets','selected':'2/3/4','authority':'Fandom Revelation Perks1204680',
     'alternative':'Canary callback bounces+1 interpreted as further count by core'},
    {'field':'low_health_boundary','selected':'rounded percent<=30','authority':'S21 Canary hypothesis',
     'alternative':'wiki wording below30percent could imply strict raw boundary'}],
 'shield': [
    {'field':'capacity','selected':'ceil(7ML+7.6L+max(300,0.4L)), capped at cast maximum mana',
     'authority':'Fandom Magic Shield capacity; S21 Canary maximum-mana cap',
     'alternative':'Canary300+7.6L+7ML with pre-8833 Wheel1.25; cap/rounding order requires owner qualification'}],
 'chivalrous': [
    {'field':'further_targets','selected':3,'authority':'Fandom Updates15.25, native chain candidate',
     'alternative':'both pinned OTS callbacks6 =>7total'},
    {'field':'challenge_ms','selected':6000,'authority':'Fandom Chivalrous Challenge',
     'alternative':'both pinned OTS explicit12000'}],
 'mend': [
    {'field':'area','selected':'Canary rounded11x11 matrix','authority':'S21 hypothesis',
     'alternative':'BR describes4sqm; Crystal older expanded-area augment'},
    {'field':'base_power_mana_spender','selected':'800,400,notHarmonyspender',
     'authority':'official:8833; Fandom; Canary99902524',
     'alternative':'older references90,250,Harmonyspender'}],
 'ice': [
    {'field':'base_power','selected':140,'authority':'official:8872 accepted official-changes.json',
     'alternative':'Crystal150; legacy coefficients4.5ML+20..7.6ML+48'},
    {'field':'base_and_diagonal_area','selected':'Canary local4x3 and no diagonal table',
     'authority':'S21 OtsHypothesisOnly','alternative':'Crystal SQUAREWAVE5 with explicit diagonal'}],
}


FORMULA_MODELS = {
    'energy beam': (60, 0, 'floor', 35),
    'energy wave': (150, 80, 'floor', 21),
    'great energy beam': (155, 60, 'floor', 25),
    'strong ice wave': (140, 80, 'floor', 48),
    'ice burst': (115, 60, 'floor', 38),
    'terra burst': (115, 60, 'floor', 42),
    'front sweep': (80, 40, 'round', 5),
    "executioner's throw": (60, 10, 'round', 6),
}


def _formula_evidence(name):
    if name in FORMULA_MODELS:
        power, buckets, rounding, card = FORMULA_MODELS[name]
        return [{'source': 'TibiaTools numerical compatibility reference',
                 'revision': 'eeed345c86a76eb00f5ff7b2f0e0b49927799bf1',
                 'url': 'https://raw.githubusercontent.com/kik-tibia/tibiatools/eeed345c86a76eb00f5ff7b2f0e0b49927799bf1/src/data/spells.json',
                 'sha256': '73d374e29667ed7b1b89765c6eaeb0d56a3adc9474815493166f81be50f9b430',
                 'card_id': card, 'base_power': power, 'buckets': buckets, 'rounding': rounding,
                 'scope': 'base central/no-bonus component only; excludes live cast and downstream modifiers',
                 'endpoint_qualification': 'source half-width only; oracle nominal center, no published bounds' if name == 'energy beam' else 'published compatibility bounds; no RNG-mean/live-runtime proof',
                 'conflict': 'Pinned OTS legacy coefficient/spread and F-inside-variation formulas are superseded by accepted wiki BP and compatibility model. Energy Beam retains source interval half-width under MAGIC-F2 because its oracle card publishes no bounds.'}]
    if name == 'mass healing':
        return [{'source': 'Crystal BP healing helper hypothesis',
                 'revision': PINS['crystal'], 'path': 'data/scripts/lib/register_spells.lua',
                 'source_url': 'https://github.com/zimbadev/crystalserver/blob/'+PINS['crystal']+'/data/scripts/lib/register_spells.lua#L646-L651',
                 'base_power': 200, 'sha256': CRYSTAL_HEALING_HELPER_SHA256, 'authority': 'OtsHypothesisOnly',
                 'conflict': 'Legacy callbacks5.7ML+26..10.43ML+62 do not scale the accepted wiki BP200. Generic Crystal BP healing helper selected as source hypothesis; healing coefficients unmeasured per schema section3. No independent calculator bounds card exists.'}]
    return []


def evidence(name, spell_type, records, texts):
    name=str(name).casefold()
    if not _qualified(name,spell_type,records,texts):
        return {}
    sources=[{'source':source,'revision':PINS[source],'path':spec['file'],
              'sha256':spec['sha256'],'blob':spec['blob'],'authority':'OtsHypothesisOnly'}
             for source in records for spec in [SPECS[name][source]]]
    sources += [{'source':'canary','revision':PINS['canary'],'path':path,'sha256':digest,
                 'authority':'OtsHypothesisOnly'} for path,digest in HELPERS.items()]
    conflicts=[]
    if name in ('energy beam','great energy beam'): conflicts+=CONFLICTS['beam']
    if name=="executioner's throw": conflicts+=CONFLICTS['execution']
    if name=='magic shield': conflicts+=CONFLICTS['shield']
    if name=='chivalrous challenge': conflicts+=CONFLICTS['chivalrous']
    if name=='mass spirit mend': conflicts+=CONFLICTS['mend']
    if name=='strong ice wave': conflicts+=CONFLICTS['ice']
    if name in ('chivalrous challenge','divine dazzle'):
        conflicts.append({'field':'first_target','selected':'attacked valid target before nearest fallback',
                          'authority':'official15.01.4b0877 native chain candidate',
                          'alternative':'pinned OTS nearest-only picker'})
    decisions=['Native domain executor and owner integration remain unadmitted.',
               'Wheel stage snapshot and cooldown mutation authority require owner integration.',
               'Formula inputs must be bound by accepted Formula input authority; source skill is not client input.',
               'Damage/heal percentage modifiers use pinned source full-magnitude semantics; official base-damage wording does not establish a different multiplier phase.']
    if name in ('mass healing','mass spirit mend'):
        decisions+=['Healing filter and paralysis dispel target-set split uses Canary callback ordering hypothesis.',
                    'Literal ravennous hunger boss name is preserved; canonical typo correction requires evidence.']
    if name.startswith('avatar'):
        decisions+=['Outfit stacking/expiry and Forge avatar interaction need accepted StateCondition owner contract.']
    if name in ('energy beam','energy wave','great energy beam'):
        decisions+=['Elemental stance presentation/bonus ordering is an external owner boundary.']
    if name in ('chivalrous challenge','divine dazzle',"executioner's throw"):
        decisions+=['Equal-distance chain ties preserve source iteration hypothesis; deterministic owner order unaccepted.']
    return {'selection_rule':'S24 official/wiki accepted target-date facts; S21 pinned Canary fallback hypotheses; S5 level normalization',
            'source_evidence':sources,'accepted_reference_evidence':deepcopy(REFERENCE_EVIDENCE),
            'formula_reference_evidence':_formula_evidence(name),
            'attributed_conflicts':deepcopy(conflicts),
            'unresolved_decisions':decisions,'runtime_admission':'rejected_until_native_contract_and_owner_integration',
            'authoring_complete':True}
