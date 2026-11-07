"""Source-qualified shrine learning rules; ordinary recipe/item allowances stay separate."""
import hashlib
import json


def build(root, inputs, pin):
    path='content/quests/definitions/quests-00100-00199.json'
    raw=(root/path).read_bytes()
    index,definition=next((i,r['definition']) for i,r in enumerate(json.loads(raw)['records'])
        if r['definition']['identity']['key']=='oteryn:quest.heart_of_destruction_quest')
    recipe=definition['oteryn_recipe']['payload']['recipe']
    if not any(r['kind']=='access' and r['name']=='Powerful imbuement knowledge' for r in recipe['reward_intents']):
        raise ValueError('HEART_IMBUEMENT_KNOWLEDGE_RECIPE_DRIFT')
    source='data-global/npc/yana.lua';body=inputs[source]
    for name in [b'Powerful Strike',b'Powerful Void',b'Powerful Vampirism']:
        if name not in body:raise ValueError('HEART_YANA_KNOWLEDGE_SOURCE_DRIFT')
    sources=[f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}',
        f'crystal:{pin}:{source}:185-187:sha256:{hashlib.sha256(body).hexdigest()}']
    return {'schema':1,'rules':[{'family':family,'tier':3,
        'any_completed_quests':['oteryn:quest.heart_of_destruction_quest'],'sources':sources}
        for family in [1,11,12]]}
