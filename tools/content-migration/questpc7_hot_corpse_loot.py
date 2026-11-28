"""Complete the source-pinned quest and raid corpse and loot custody facts."""
import hashlib
import json
import build_full_spell_artifact as source

PATH = 'tools/content-schema/native-gameplay/loot-tables.json'
SOURCES = {'chicken': (6042, 5, 4, 'ba8ad97745a1dbf2009ff6ff6cf21bc2ed8913fd13c442b750d43be8ea6700f3'),
           'bat': (6053, 14, 1, '9b2d9b16634b4aed39d8920f06f33c403de5ea0e70566312363a9890b4308acc')}
# Every active source raid with a Silver Raid Token in its retained native table.
TOKEN_RAID_SOURCES = {
    'furyosa': (18118, 20, 23, '9ce3a8b3c8e1a292403588147a796bae6ab8d24374f8f889dc5ce949409d5923'),
    'hirintror': (7282, 30, 19, 'c79fbae3865066889dcce16f1a0f3b50c087d64ad5dc78edcd9ef9004584a6f7'),
    'ocyakao': (7320, 24, 12, '25db1ec7877df73847a7012b66cdc78b56d7206ddcaadd553fdf6938f55ae543'),
    'the_welter': (18974, 20, 20, 'b422cbf18e3ecc0b8cf4936f27240f160cec645d7023f7309544617638fee4ec'),
    'tyrn': (18970, 15, 30, 'acc78e696ac3d71ff8b41f4ead840c4abf96e1dafe1c2b0e9ccabc68d04c70c9'),
    'weakened_shlorg': (18982, 20, 21, 'f4d92f7d6fe7cbb1b946e00e359693bfc771a81f5e44f976be5a90875d31b28d'),
    'white_pale': (18978, 20, 12, '81f4988f68ab15ef9d1200022681d42255ef40532b837a1d7d25d26943b1644f'),
    'willi_wasp': (5989, 5, 6, 'fd882b4d06f44debc08c2f1db4fccab89b9d200c4d308efc6270b5ced7df78ef'),
    'zushuka': (18265, 30, 32, 'ef26b3d51d24ac200dd515c9c314032417689b873415f24ee2bd468380d9dddd'),
}


def apply(root, generated, profile_path, manifest_path, canonical, *, sources=None):
    document = json.loads(generated.get(PATH, (root / PATH).read_bytes()))
    profiles = {r['production_definition']['production_key']: r for r in json.loads(generated[profile_path])['records']}
    for creature, (item, capacity, count, digest) in ((SOURCES | TOKEN_RAID_SOURCES) if sources is None else sources).items():
        table, = [r for r in document['tables'] if r['identity']['key'] == 'oteryn:loot.creature.' + creature]
        actual = hashlib.sha256(json.dumps(table, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        if actual != digest or len(table['entries']) != count or any(e.get('children') for e in table['entries']):
            raise ValueError('QUESTPC7_HOT_LOOT_SOURCE_DRIFT')
        binding, = [r for r in document['creature_loot'] if r['creature']['key'] == 'oteryn:creature.' + creature]
        if binding['loot'] != table['identity']: raise ValueError('QUESTPC7_HOT_LOOT_BINDING_DRIFT')
        corpse = f'oteryn:item.tibia.i{item}'
        selected = {corpse} | {e['item']['key'] for e in table['entries']}
        found = set()
        for facts in document['items']:
            key = facts['item']['key']
            if key not in selected: continue
            if key in found: raise ValueError('QUESTPC7_HOT_LOOT_DUPLICATE')
            found.add(key)
            if key == corpse:
                if source.native_corpse_capacity(facts['item'], profiles[key]) != capacity:
                    raise ValueError('QUESTPC7_HOT_CORPSE_CAPACITY_DRIFT')
                facts.update(materializable=True, container_capacity=capacity, stackable=False)
            elif not facts['materializable']:
                observed = source.native_loot_yield_facts(facts['item'], profiles[key])
                if observed is None: raise ValueError('QUESTPC7_HOT_LOOT_YIELD_NOT_ADMITTED:' + key)
                facts.update(observed)
        if found != selected: raise ValueError('QUESTPC7_HOT_LOOT_CLOSURE_MISSING')
    raw = canonical(document); generated[PATH] = raw
    manifest = json.loads(generated[manifest_path]); manifest['loot_tables']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
