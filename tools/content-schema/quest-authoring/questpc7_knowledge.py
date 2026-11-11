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
    rules=[{'family':family,'tier':3,
        'any_completed_quests':['oteryn:quest.heart_of_destruction_quest'],'sources':sources}
        for family in [1,11,12]]
    return {'schema':1,'rules':extend_forgotten(root,inputs,pin,rules)}


# Insert after existing Heart rules are constructed; data-only extension to shared engine.
def extend_forgotten(root, inputs, pin, rules):
    import xml.etree.ElementTree as ET
    path='content/quests/definitions/quests-00100-00199.json';raw=(root/path).read_bytes()
    index,d=next((i,r['definition']) for i,r in enumerate(json.loads(raw)['records']) if r['definition']['identity']['key']=='oteryn:quest.forgotten_knowledge_quest')
    if not any(r['kind']=='access' and 'imbu' in r['name'].lower() for r in d['oteryn_recipe']['payload']['recipe']['reward_intents']):raise ValueError('FORGOTTEN_KNOWLEDGE_REWARD_DRIFT')
    xml=inputs['data/XML/imbuements.xml'];digest=hashlib.sha256(xml).hexdigest()
    if digest!='7e4ae3be28e0b3e740e856e8cec2c8c9ff01e7f9ab4b2b8d3b970b51ca1df8fd':raise ValueError('FORGOTTEN_IMBUEMENTS_SOURCE_DRIFT')
    families={1:'Strike',3:'Lich Shroud',4:'Snake Skin',5:'Cloud Fabric',6:'Dragon Hide',7:'Demon Presence',8:'Quara Scale',10:'Swiftness',11:'Vampirism',12:'Void',14:'Chop',15:'Bash',16:'Precision',17:'Punch',18:'Epiphany',19:'Blockade',20:'Slash'}
    source_rules={r.get('name'):r for r in ET.fromstring(xml) if r.get('base')=='3'}
    for family,name in sorted(families.items()):
        row=source_rules[name]
        if int(row.get('storage','0')) not in [50488,50490,50492,50494,50496,50498,50501]:raise ValueError('FORGOTTEN_KNOWLEDGE_STORAGE_DRIFT:'+name)
        line=next(i for i,s in enumerate(xml.decode().splitlines(),1) if f'name="{name}" base="3"' in s)
        source=[f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}',f'crystal:{pin}:data/XML/imbuements.xml:{line}:sha256:{digest}']
        existing=next((r for r in rules if r['family']==family and r['tier']==3),None)
        if existing is None:rules.append(dict(family=family,tier=3,any_completed_quests=['oteryn:quest.forgotten_knowledge_quest'],sources=source))
        else:
            existing['any_completed_quests'].append('oteryn:quest.forgotten_knowledge_quest');existing['sources']+=source
    return sorted(rules,key=lambda r:(r['family'],r['tier']))
