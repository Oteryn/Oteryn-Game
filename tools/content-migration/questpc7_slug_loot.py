"""Qualify only Slug's pinned corpse/loot closure; preserve other owners."""
import hashlib
import json
import build_full_spell_artifact as source

PATH = 'tools/content-schema/native-gameplay/loot-tables.json'
TABLE_SHA = '33d8e78abf88d73215915d918746f31f6c6cf1566dfc91bd5f1af7cd51352c3c'


def apply(root, generated, profile_path, manifest_path, canonical):
    document=json.loads(generated.get(PATH,(root/PATH).read_bytes()))
    table,=[r for r in document['tables'] if r['identity']['key']=='oteryn:loot.creature.slug']
    digest=hashlib.sha256(json.dumps(table,sort_keys=True,separators=(',',':')).encode()).hexdigest()
    if digest!=TABLE_SHA or len(table['entries'])!=2 or any(e.get('children') for e in table['entries']):
        raise ValueError('QUESTPC7_SLUG_LOOT_SOURCE_DRIFT')
    binding,=[r for r in document['creature_loot'] if r['creature']['key']=='oteryn:creature.slug']
    if binding['loot']!=table['identity']:raise ValueError('QUESTPC7_SLUG_LOOT_BINDING_DRIFT')
    profiles={r['production_definition']['production_key']:r for r in json.loads(generated[profile_path])['records']}
    corpse='oteryn:item.tibia.i12526';selected={corpse}|{e['item']['key'] for e in table['entries']};found=set()
    for facts in document['items']:
        key=facts['item']['key']
        if key not in selected:continue
        if key in found:raise ValueError('QUESTPC7_SLUG_LOOT_DUPLICATE')
        found.add(key)
        if key==corpse:
            capacity=source.native_corpse_capacity(facts['item'],profiles[key])
            if capacity!=10:raise ValueError('QUESTPC7_SLUG_CORPSE_CAPACITY_DRIFT')
            facts.update(materializable=True,container_capacity=capacity,stackable=False)
        elif not facts['materializable']:
            observed=source.native_loot_yield_facts(facts['item'],profiles[key])
            if observed is None:raise ValueError('QUESTPC7_SLUG_LOOT_YIELD_NOT_ADMITTED:'+key)
            facts.update(observed)
    if found!=selected:raise ValueError('QUESTPC7_SLUG_LOOT_CLOSURE_MISSING')
    raw=canonical(document);generated[PATH]=raw
    manifest=json.loads(generated[manifest_path]);manifest['loot_tables']['sha256']=hashlib.sha256(raw).hexdigest()
    generated[manifest_path]=canonical(manifest)
