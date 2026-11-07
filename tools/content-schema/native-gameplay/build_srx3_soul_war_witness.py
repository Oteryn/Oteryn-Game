#!/usr/bin/env python3
"""Generate the bounded W11 callback witness from canonical native source inputs.

No WorldProject is fabricated. The actual native loader's complete source digest
binds the bounded records to the full canonical project; exact typed equality at
runtime binds every consumed Creature/Behavior/Ability/Effect/Formula record.
Crystal callback bytes are read-only compatibility evidence, never executed.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
CANARY = "47dfd51f45280a59a1d3e50ba7edd573d7234446"
KEYS = ("goshnar_s_greed", "greedbeast", "soulsnatcher", "weak_soul",
        "strong_soul", "powerful_soul", "soul_sphere", "goshnar_s_malice",
        "soul_cage", "dreadful_harvester")
CALLBACKS = {'data-global/monster/quests/soul_war/goshnars_greed.lua': 'e9c11a26a1c18f42947b8a5b37ba9dcea8ca5b0322dfb2fc047c3d2e33b07ad0', 'data-global/monster/quests/soul_war/greedbeast.lua': 'c2500e72847896d8db62eda89a2674e4a7443cd9b4db9a98d35b46443477d158', 'data-global/monster/quests/soul_war/weak_soul.lua': '91d0f2a5f5d82a63f72e71260f0d1d70571e2d6a41b3e07a71c136fede4414a6', 'data-global/monster/quests/soul_war/strong_soul.lua': 'c8baadc57f446ab7ba972ae4d93ce92c3eba1e3b4db4c47c6c2b19d81b48cbbb', 'data-global/monster/quests/soul_war/powerful_soul.lua': '86e23cad8a54abc6dde2adb32644ddb29f6d6efab8ee137c36f86acd7bfb5f2e', 'data-global/monster/quests/soul_war/soulsnatcher.lua': 'd852709c53c6099051b8009bb88a23283ece21ed9ebb9b905b43f59b5e63155b', 'data-global/monster/quests/soul_war/soul_sphere.lua': '6a986338ec930ed02bbecc6da59230a2355b50a11091b59f4489f1210b2ef2dc', 'data-global/lib/quests/soul_war.lua': '89c0024914b0593b9b7d2bfb535a24025a552f54b158319697cdf56a66375e75', 'data-global/scripts/quests/soul_war/soul_war_mechanics.lua': '9e1107dc41abc16b4f56e795180c1c17c917d8e73754547c443f070ed3167916'}
# Reviewed callback participant bodies; rebinding a whole-project SHA cannot
# silently change the qualified source schedules or numerical closure.
REVIEWED_PROFILES = {'oteryn:creature.goshnar_s_greed': 'a34f3727a81bd4e9081be79f94ab9ba0612007a526e0dd38f78a7beb3df9f6e6', 'oteryn:creature.greedbeast': '6b68327edac1000527bc14ef8dfa012c16f1f3e5995fe9a0cd9e4ed26adf48eb', 'oteryn:creature.soulsnatcher': 'b38d2658acfcdc6f61363594fac3c39ee87d9b291c94bd4d7e50885fff8c5f42', 'oteryn:creature.weak_soul': 'a94e228b2079465abad5b53df29cd40e04df4d97bc7d8045c71d88ba9c4410a6', 'oteryn:creature.strong_soul': 'f547124cd0a8bb46adc960efa057e5e8fc6e59a76cc8e1c9207541529b5f2b57', 'oteryn:creature.powerful_soul': 'df149c323b2e562c3d526b9161af87c4d53314b800a8a4a0072c6aebfcbe5b9e', 'oteryn:creature.soul_sphere': '5d0fad5e2e7375b3c18d8053ed198f8655208e5e76107aa2278c7f251d10a8ba'}

REVIEWED_PROFILES.update({
    'oteryn:creature.goshnar_s_malice': 'f5e07266408035e8b18ff3ce02b456547ef0ec3965ea3c23371a638e328b50d0',
    'oteryn:creature.soul_cage': '4579634468a6b255888432ee9d69e700f075da3168374314791713ff787744b6',
    'oteryn:creature.dreadful_harvester': '0980c40d5d2cb951f5bfc52c6604c5f9a6bb25f16adb5505b3267d22c7f57e0d',
})
CALLBACKS['data-global/monster/quests/soul_war/goshnars_malice.lua'] = '2394dae37135ca27aea35459e7879e95a566a5c7b68bd5491e9bc625f84839ec'
CALLBACKS['data-global/monster/quests/soul_war/soul_cage.lua'] = '3514e57d4d6cc95a8e51f98495d8d56095c5a9a50f24f2b9cc1254da44f59490'
CALLBACKS['data-global/monster/quests/soul_war/dreadful_harvester.lua'] = 'c716b140a444501bc741047c6828c30b7ca7e76f519b0efc2833a0045ba6ae01'

# SW4 bounded participant closure; exact canonical bodies and Canary identities.
KEYS += ('mirror_image', 'druid_s_apparition', 'knight_s_apparition', 'monk_s_apparition', 'paladin_s_apparition', 'sorcerer_s_apparition')
REVIEWED_PROFILES.update({'oteryn:creature.druid_s_apparition': 'cf37a7285aa6e134d6f90842567212827f4fc7efac4637699d518f164a663182', 'oteryn:creature.knight_s_apparition': 'bcc07f4d293dda4adc5afb9c38c84b051132b4563fe01ebf317b1315bc0bde04', 'oteryn:creature.monk_s_apparition': '538d2cba6b3dcaadd85c7c6a128b7b437ad5ff2f9d844aba091dd978fa54a634', 'oteryn:creature.paladin_s_apparition': '96c3584153afb91c67531eb42ea4c9fe0cde2daae020ebfde8d73a89557f9dce', 'oteryn:creature.sorcerer_s_apparition': '4b4b1a1b818ae60c9f589f09df7c882dc703475c1aa0ae49fe95801c59b75d75', 'oteryn:creature.mirror_image': '3133ec776d4655e202e116ebfd0c9f2af09a395f239f992119389b67d04ad4d8'})
CALLBACKS['data-global/monster/quests/soul_war/mirror_image.lua'] = '9500f7a6b157c5bf84ac68536811e37a4bb36542b7d0cd47db4c15ea36495af4'
CALLBACKS["data-global/monster/quests/soul_war/normal_monsters/druid's_apparition.lua"] = 'bbf27786e559fedd1bada108bd606b67895a97df43d6f01e5c61fe98ec1732d1'
CALLBACKS["data-global/monster/quests/soul_war/normal_monsters/knight's_apparition.lua"] = 'cdc4fe5f4092fd5ff2a2477de10f3a279cbd87955fc4f9fba5a6fa2f3d2eefda'
CALLBACKS["data-global/monster/quests/soul_war/normal_monsters/monk's_apparition.lua"] = '924385010008a9157872d090073bb7dd237b8eafe67440b8b81eac3f2651c1a7'
CALLBACKS["data-global/monster/quests/soul_war/normal_monsters/paladin's_apparition.lua"] = '426a0931a9dfa70ab7c79771e6ad8132e5e38d23dd93369e4e53ba4040d1c1d3'
CALLBACKS["data-global/monster/quests/soul_war/normal_monsters/sorcerer's_apparition.lua"] = '0230f650868a4201f49bdd2084615f270faa24fda0b5263c36c134e630146caa'

# Source World first-taint participants and Combat/PlayerThink callback owner.
KEYS += ('bony_sea_devil', 'brachiodemon', 'branchy_crawler', 'cloak_of_terror', 'many_faces', 'spiteful_spitter')
REVIEWED_PROFILES.update({'oteryn:creature.bony_sea_devil': 'f2395e28fb3ba5398b3ffc8275700d7c859f9496140b265c3bf8511c23e29d1a', 'oteryn:creature.brachiodemon': 'da5cc83edd16a425d9917d2478621532c4ae49cb585057f5637d2923721bcdc7', 'oteryn:creature.branchy_crawler': '9db881f23b8b0bea9a5535de28d4bb7ebafd35a666f0d741872d31f5bb4bcceb', 'oteryn:creature.cloak_of_terror': '0177a85bca09e92137fabbbed76c37173f1bdbe0ba43f88e14cc0e07c9e1d40c', 'oteryn:creature.many_faces': 'c6403158b794df6d3cd2d7af8747da0abe7f452514b11f772cc3d709a85e5f83', 'oteryn:creature.spiteful_spitter': '9b275bb6f155907515a31ac522d1c3b82378dc908e24ea53522bc81b24b6c02c'})
CALLBACKS['data-global/monster/quests/soul_war/normal_monsters/bony_sea_devil.lua'] = '90ba74bd1eee3c552ca62fe4a6c82d9bd0477d0b824e8278ace4e429ce20f540'
CALLBACKS['data-global/monster/quests/soul_war/normal_monsters/brachiodemon.lua'] = '64f42388be10a378704ff7615e6ed0bd3f716fa8f8a383acb46f4ea418771186'
CALLBACKS['data-global/monster/quests/soul_war/normal_monsters/branchy_crawler.lua'] = '01e75364bae48acb69344a84304beeab41ebf2528a9f336b6b87675251e0f302'
CALLBACKS['data-global/monster/quests/soul_war/normal_monsters/furious_crater/cloak_of_terror.lua'] = '2fca76ce2fcdc41edb87152d385ceefb8bb6bfd34f995eb0e7fc3a62add3b78b'
CALLBACKS['data-global/monster/quests/soul_war/normal_monsters/many_faces.lua'] = '5334231a5065d6a6ee0d98b0fbd92583339e83f94a3233a0fed8da90fc94e97c'
CALLBACKS['data-global/monster/quests/soul_war/spiteful_spitter.lua'] = '9fc035a2a17539121fc7cd5564970cd1ba5da7ac10a7a10998a0081c1cb31d3a'
CALLBACKS['data-global/scripts/quests/soul_war/eventcallback_on_combat_taint.lua'] = 'eedb610c7cc8b08ef61424517ca3f4dabadf2fe95c224bcbf238538c09918d88'

# Effective final FourthTaint event registrations: fourteen realm monsters.
KEYS += ('capricious_phantom', 'distorted_phantom', 'courage_leech', 'vibrant_phantom', 'infernal_demon', 'infernal_phantom', 'mould_phantom', 'rotten_golem', 'turbulent_elemental')
REVIEWED_PROFILES.update({'oteryn:creature.capricious_phantom': '493510a9bb791e88a8f5613bc3819d5278356877f186226853795f4aa495cc3d', 'oteryn:creature.distorted_phantom': 'e156a4871107375f7d0a390dd4ab2c7efe4a6bca76a0d337a4efc7af9b3095b3', 'oteryn:creature.courage_leech': '81b443ce9b5a072c5e3c1e04c296dfc1a85c5d9b04aaabfc2b665250c3e9f888', 'oteryn:creature.vibrant_phantom': '921755cbe5794e85122ff26f57ef4c54ace58366d3f052e4e0522d766e5f68dd', 'oteryn:creature.infernal_demon': 'b61a86bb5eba20fcefce230383e6679b35ad9d8468fc55bc514bc9b1cf860153', 'oteryn:creature.infernal_phantom': 'e741870fbca4da9b2ea5ebad4b029d4f18154b57f7b0683abb3cb7991116734b', 'oteryn:creature.mould_phantom': '619e89125a2aa6e393185509421103b1879e740c7f685fab2c57517a3a0f53fc', 'oteryn:creature.rotten_golem': '987bd8bc87be4fa893353de06c729312a7031f1825870b11033fec77dbca8899', 'oteryn:creature.turbulent_elemental': 'a36f7be1628c7fdcb4a8ec327f11ac368d297bab2310e3787c2bf8def7816a6a'})
CALLBACKS.update({'data-global/monster/quests/soul_war/normal_monsters/capricious_phantom.lua': 'eb969c6d5ee5ba34d7f664a09e2bd2a9c63692cd15f0c6d4726fff2352a82b0f', 'data-global/monster/quests/soul_war/normal_monsters/distorted_phantom.lua': '2b192bb6905f04d54e465850a35eaf4f6413c109f7239fd2e040f449141fde06', 'data-global/monster/quests/soul_war/normal_monsters/furious_crater/courage_leech.lua': 'b526c138a084017fca13cfa41ecab7621e7655e3677183fe71a46200efa864f0', 'data-global/monster/quests/soul_war/normal_monsters/furious_crater/vibrant_phantom.lua': '988d04119545416af195a5e190db07a3de6545215d60329195ae7a8e176df591', 'data-global/monster/quests/soul_war/normal_monsters/infernal_demon.lua': 'e7e579673b88242d8563c493083b2e27714f9342fb0c82b5eee902a4f7412ee1', 'data-global/monster/quests/soul_war/normal_monsters/infernal_phantom.lua': '2062ac9de68efee63b56d21b5712999c35f0ea2dae79c40c6a793edca757cc32', 'data-global/monster/quests/soul_war/normal_monsters/mould_phantom.lua': 'baf5b67c2f1dcdc0803e6161c8c9d7491d3cb0f7dfe47121f3cf645c6b13bcf3', 'data-global/monster/quests/soul_war/normal_monsters/rotten_golem.lua': 'a74071a649dcdeaba636b4fb22815ac3b5eed76be07f0c62ce8cac73ddeb56a5', 'data-global/monster/quests/soul_war/normal_monsters/turbulent_elemental.lua': 'f0e18007ae9886f6793c97ac44c318bc23e99453a6cab703067a1dd0779f08f2'})

CALLBACKS['data-global/scripts/creaturescripts/monster/mirror_image_transform.lua'] = '32b662ad8bfcc3b6afe557a99ad4e58eb39392dbf1bfaa725f04190b5ca3a34b'

# Remaining source-qualified encounter participants, reviewed against pinned Lua.
KEYS += ('goshnar_s_spite', 'weeping_soul', 'goshnar_s_hatred', 'ashes_of_burning_hatred', 'spark_of_burning_hatred', 'flame_of_burning_hatred', 'blaze_of_burning_hatred', 'symbol_of_hatred', 'goshnar_s_cruelty', 'a_greedy_eye', 'poor_soul', 'goshnar_s_megalomania_purple', 'goshnar_s_megalomania_green', 'goshnar_s_megalomania_blue', 'aspect_of_power', 'hazardous_phantom', 'dreadful_disruptor')
REVIEWED_PROFILES.update({'oteryn:creature.goshnar_s_spite': '7c07642f6019a10161f00551c626d374aeb4d09d08960c8768e49308806ff0fb', 'oteryn:creature.weeping_soul': 'cd84fb70ea8f5d07c759a715d9a182cabf4a0ba1ca11bb95f6d296ed073e86db', 'oteryn:creature.goshnar_s_hatred': '59aa9be23e56c8dac112c2c39131507024558f092ff0412a7d6cf42f9a7080a8', 'oteryn:creature.ashes_of_burning_hatred': 'a2505d998bdd9855db47896a7b90758af364af72f17339756cc2a0c27ef0e19f', 'oteryn:creature.spark_of_burning_hatred': 'e2e243ee150f4983c50065939b34232650d5a363a1b01d5bedf4ee5995930c0f', 'oteryn:creature.flame_of_burning_hatred': 'ead874a435aaa1696f3953bc44f0190fc3a4721a8edf23dbf2abc57089fec96d', 'oteryn:creature.blaze_of_burning_hatred': 'c68277ff0927186eec0750c885f94f102285c893b5d7c4a70fc8cc8d81274dbf', 'oteryn:creature.symbol_of_hatred': 'ebf287070255ef9daf86c1d6e46283a3a3f6b7204b5602d991a06b4d0ea34706', 'oteryn:creature.goshnar_s_cruelty': 'd4ee2198ce417df9be2101b73047cf64d23dfe568a7eaca5eee88d437bd38da6', 'oteryn:creature.a_greedy_eye': 'd13f23478c157176893e298f775d1556af17fa87337fce6e3c210212d606a29a', 'oteryn:creature.poor_soul': 'c83937032d5430d6d8baa3f73b7aaaeafa7b085bbccc7e5a64aae6ec533b9fe2', 'oteryn:creature.goshnar_s_megalomania_purple': 'b5dbc9b93939952331c86c5dd2bfde2cd0ffcf702838659daa3a4b337d297cc6', 'oteryn:creature.goshnar_s_megalomania_green': 'a1add91364a178e37c5e81b503e09604095ecfd783bc990cad0092bbc03db392', 'oteryn:creature.goshnar_s_megalomania_blue': 'b47d9cc8b289deec0d63344695ff20533b053615cca57194f56ca243be993503', 'oteryn:creature.aspect_of_power': 'e8f4764c0d2fb4d0238979c7d8b5ca1f0c43a3119b3cfadcea43c5e20cdd14fd', 'oteryn:creature.hazardous_phantom': 'ad0d09d997766fc9850cbc216e27a3cd1065da42549e3141f7c13faac52b58b5', 'oteryn:creature.dreadful_disruptor': 'dc6c52384f001af134fad80b8b7f64bdf050a746edb108093d66396aefa43ddf'})
EXTERNAL_IDS = {'goshnar_s_spite': 'quests/soul_war/goshnars_spite', 'weeping_soul': 'quests/soul_war/weeping_soul', 'goshnar_s_hatred': 'quests/soul_war/goshnars_hatred', 'ashes_of_burning_hatred': 'quests/soul_war/normal_monsters/burning_hatred/ashes_of_burning_hatred', 'spark_of_burning_hatred': 'quests/soul_war/normal_monsters/burning_hatred/spark_of_burning_hatred', 'flame_of_burning_hatred': 'quests/soul_war/normal_monsters/burning_hatred/flame_of_burning_hatred', 'blaze_of_burning_hatred': 'quests/soul_war/normal_monsters/burning_hatred/blaze_of_burning_hatred', 'symbol_of_hatred': 'quests/soul_war/normal_monsters/burning_hatred/symbol_of_hatred', 'goshnar_s_cruelty': 'quests/soul_war/goshnars_cruelty', 'a_greedy_eye': 'quests/soul_war/normal_monsters/furious_crater/a_greedy_eye', 'poor_soul': 'quests/soul_war/normal_monsters/furious_crater/poor_soul', 'goshnar_s_megalomania_purple': "quests/soul_war/goshnar's_megalomania_purple", 'goshnar_s_megalomania_green': "quests/soul_war/goshnar's_megalomania_green", 'goshnar_s_megalomania_blue': "quests/soul_war/goshnar's_megalomania_blue", 'aspect_of_power': 'quests/soul_war/aspect_of_power', 'hazardous_phantom': 'undeads/hazardous_phantom', 'dreadful_disruptor': 'quests/soul_war/dreadful_disruptor'}
CALLBACKS.update({'data-global/monster/quests/soul_war/goshnars_spite.lua': '50c678660280d5f455b97e90a8c976bc022f3120b616f64b7e799fddf6264fd0', 'data-global/monster/quests/soul_war/weeping_soul.lua': '900ca29c72c64313b0315ce9fd42d93bc6c933d039a68b3d0c5dfb67e2ba2f04', 'data-global/monster/quests/soul_war/goshnars_hatred.lua': '2f5c8e63b499eddca333e39421b651607f2992afbf13873dc5e0fdd498511fef', 'data-global/monster/quests/soul_war/normal_monsters/burning_hatred/ashes_of_burning_hatred.lua': '510556a30b9b9b40c3035f1ef3c99733b8d38c317efe52121d749598308168f6', 'data-global/monster/quests/soul_war/normal_monsters/burning_hatred/spark_of_burning_hatred.lua': '3efe638127dfd357f32b23a89f33e33941603ddf45f9f65b7b78e3688462a0e9', 'data-global/monster/quests/soul_war/normal_monsters/burning_hatred/flame_of_burning_hatred.lua': '36b43b904f5d4691440643b7ac0c08562b41617cb3105f3206f7c9de02445e2d', 'data-global/monster/quests/soul_war/normal_monsters/burning_hatred/blaze_of_burning_hatred.lua': 'e4df78075fa078cecc890dafef68d4ce35b545ea998b12dd9f5512e531b42283', 'data-global/monster/quests/soul_war/normal_monsters/burning_hatred/symbol_of_hatred.lua': '3aad1b0b58dbdfd7d39bd7020b960da7cddee736c0c9325a7ee065818d053643', 'data-global/monster/quests/soul_war/goshnars_cruelty.lua': '5f71c7170921b537ba5bd397b70805c00cb3457da97f08a915383b3e34e58c0b', 'data-global/monster/quests/soul_war/normal_monsters/furious_crater/a_greedy_eye.lua': 'f150dd14d1f90bfaa9377117556cef93b035e9dfedad47bc66c7b6a1f8a6a4ee', 'data-global/monster/quests/soul_war/normal_monsters/furious_crater/poor_soul.lua': '58c2d9b4e5d2989f5e927c5d5248e77d3e414d3a70b0103cb6eb2dee32da978d', "data-global/monster/quests/soul_war/goshnar's_megalomania_purple.lua": 'cd2cae8c72c72a46064ec084c5db1684a4223e4150d3670f154a88bcb2589b2b', "data-global/monster/quests/soul_war/goshnar's_megalomania_green.lua": '073de955c173424a484e50885c2538749725fd437e3499382d3dc6c5199761f0', "data-global/monster/quests/soul_war/goshnar's_megalomania_blue.lua": 'd1ba4b9e8303d804ceaa75b0c4d24f9bf3181ed7ad6736861ba5ff67b7a64848', 'data-global/monster/quests/soul_war/aspect_of_power.lua': '0192030dc0c2a176a9f7e30d32943a4c6ff6b43f9828a37245075e922f50bee6', 'data-global/monster/quests/soul_war/hazardous_phantom.lua': '70201a5829a62c603decc9316d6bb282d0f74ff17c2f3dc7317aa32a607aca17', 'data-global/monster/quests/soul_war/dreadful_disruptor.lua': '3743ad3bfe451927878ff4a8245ae1da6ed29c2e7ccfcaa6b81706014737524a'})

# Complete Megalomania native child closure, including source corpse33984.
KEYS += ('necromantic_focus', 'lesser_splinter_of_madness', 'greater_splinter_of_madness', 'mighty_splinter_of_madness')
REVIEWED_PROFILES.update({'oteryn:creature.necromantic_focus': '159f8149111d45f1f00763f1b5869de82dc450cf528f24640c652b674ca286e1', 'oteryn:creature.lesser_splinter_of_madness': 'ed2917d145b4268e1bbe3d5b5240c6883e7b964a196ce96c529660b4ab5e601c', 'oteryn:creature.greater_splinter_of_madness': '5d7fb0ba2106df3ff2486026ddba6c3a20a900088cf8ad64e630406ae77a4132', 'oteryn:creature.mighty_splinter_of_madness': 'f1aa0934d6723cbd6b2811679cfd0f90c616febcd15c1acaa86494ba5e22f267'})
EXTERNAL_IDS.update({'necromantic_focus': 'quests/soul_war/normal_monsters/megalomania_room/necromantic_focus', 'lesser_splinter_of_madness': 'quests/soul_war/normal_monsters/megalomania_room/lesser_splinter_of_madness', 'greater_splinter_of_madness': 'quests/soul_war/normal_monsters/megalomania_room/greater_splinter_of_madness', 'mighty_splinter_of_madness': 'quests/soul_war/normal_monsters/megalomania_room/mighty_splinter_of_madness'})
CALLBACKS.update({'data-global/monster/quests/soul_war/normal_monsters/megalomania_room/necromantic_focus.lua': 'a064c450c8a973530fcefe0104806ff126f37586d67b02794803a5217a7e8188', 'data-global/monster/quests/soul_war/normal_monsters/megalomania_room/lesser_splinter_of_madness.lua': '9893e70705a8d517a4b7c20b07e9d73f888bfab54601d9fff235059c399a615c', 'data-global/monster/quests/soul_war/normal_monsters/megalomania_room/greater_splinter_of_madness.lua': '2e57d8357687d4c69e2dc5fa01ddb9a72a3dd7a5b5eb7febf79a99e449b4d3a3', 'data-global/monster/quests/soul_war/normal_monsters/megalomania_room/mighty_splinter_of_madness.lua': '3918ebb53db6790edf666efc8fe25372940953beb148f149b65a8cb0b61ff050'})

OUTPUT = ROOT / "tools/content-schema/native-gameplay/srx3-soul-war-witness.json"


def build():
    source = json.loads((ROOT / "content/creatures/definitions/spell-native-profiles.json").read_bytes())
    digest = source["source_definitions_sha256"]
    if len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
        raise ValueError("native full source digest missing")
    records = []
    bindings = []
    for slug in KEYS:
        key = "oteryn:creature." + slug
        matches = [row for row in source["records"] if row["profile"]["target"]["key"] == key]
        if len(matches) != 1:
            raise ValueError("missing or duplicated native participant: " + key)
        record = matches[0]
        actual = hashlib.sha256(json.dumps(record, sort_keys=True, separators=(",", ":"),
                                           ensure_ascii=False).encode()).hexdigest()
        if actual != REVIEWED_PROFILES[key]:
            raise ValueError("native Soul War body changed; source qualification review required: " + key)
        if record["profile"]["target"]["revision"] != "definition-r1" or not record.get("behavior"):
            raise ValueError("changed native source participant: " + key)
        canonical = [row for path in (ROOT / "content/creatures/definitions").glob("creatures-*.json")
                     for row in json.loads(path.read_bytes())["records"]
                     if row["definition"]["identity"]["key"] == key]
        if len(canonical) != 1:
            raise ValueError("canonical participant absent or duplicated: " + key)
        file = {"goshnar_s_greed": "goshnars_greed",
                "goshnar_s_malice": "goshnars_malice",
                "cloak_of_terror":"normal_monsters/furious_crater/cloak_of_terror",
                **{name:"normal_monsters/"+name for name in ('bony_sea_devil', 'brachiodemon', 'branchy_crawler', 'cloak_of_terror', 'many_faces', 'spiteful_spitter') if name not in ("cloak_of_terror","spiteful_spitter")},
                **{name: "normal_monsters/" + ("furious_crater/" if name in ("courage_leech", "vibrant_phantom") else "") + name for name in ('capricious_phantom', 'distorted_phantom', 'courage_leech', 'vibrant_phantom', 'infernal_demon', 'infernal_phantom', 'mould_phantom', 'rotten_golem', 'turbulent_elemental')},
                **{name: "normal_monsters/" + name.replace("_s_", "'s_")
                   for name in KEYS if name.endswith("_apparition")}}.get(slug, slug)
        wanted = {"disposition": "EXACT", "external_id": EXTERNAL_IDS.get(slug, "quests/soul_war/" + file),
                  "identity_namespace": "canary/monster-file", "source_key": "oteryn:source.canary",
                  "source_revision": CANARY, "target": record["profile"]["target"]}
        if wanted not in canonical[0]["source_bindings"]:
            raise ValueError("source binding changed: " + key)
        bindings.append(wanted)
        records.append(record)
    return {"schema": "OTERYN_SRX3_SOUL_WAR_CALLBACK_WITNESS/v1",
            "mechanics_source_revision": PIN, "source_definitions_sha256": digest,
            "source_bindings": bindings, "callbacks": CALLBACKS, "records": records}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--source-root", type=Path)
    args = parser.parse_args()
    if args.source_root is not None:
        for path, expected in CALLBACKS.items():
            if hashlib.sha256((args.source_root / path).read_bytes()).hexdigest() != expected:
                raise ValueError("pinned callback changed: " + path)
    raw = (json.dumps(build(), sort_keys=True, indent=2, ensure_ascii=False) + "\n").encode()
    if args.check:
        if OUTPUT.read_bytes() != raw:
            raise SystemExit("stale W11 source witness; run this generator")
    else:
        OUTPUT.write_bytes(raw)


if __name__ == "__main__":
    main()
