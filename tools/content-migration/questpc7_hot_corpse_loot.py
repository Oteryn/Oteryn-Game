"""Complete only the source-pinned Chicken/Bat corpse and loot custody facts."""
import hashlib
import json
import build_full_spell_artifact as source

PATH = 'tools/content-schema/native-gameplay/loot-tables.json'
SOURCES = {'chicken': (6042, 5, 4, 'ba8ad97745a1dbf2009ff6ff6cf21bc2ed8913fd13c442b750d43be8ea6700f3'),
           'bat': (6053, 14, 1, '9b2d9b16634b4aed39d8920f06f33c403de5ea0e70566312363a9890b4308acc')}


def apply(root, generated, profile_path, manifest_path, canonical, *, sources=None):
    document = json.loads(generated.get(PATH, (root / PATH).read_bytes()))
    profiles = {r['production_definition']['production_key']: r for r in json.loads(generated[profile_path])['records']}
    for creature, (item, capacity, count, digest) in (SOURCES if sources is None else sources).items():
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
