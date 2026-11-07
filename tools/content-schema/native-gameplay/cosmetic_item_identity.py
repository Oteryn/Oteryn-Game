"""Close the exact 44048 DAT/Item-factory/action/loot identity omitted by both XMLs.

No general numeric-ID promotion. Historical bindings remain unchanged. The
pinned loader supplies the explicit zero-weight default; Wiki checks remain
pending until current-Global weight can independently supersede that default.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
PACKET = 'tools/content-schema/native-gameplay/candidate-bindings/srv7-cosmetic-item.json'
SOURCES = {
 'data/items/items.xml':'8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
 'data/items/appearances.dat':'17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
 'src/items/items.cpp':'f39d8fc6c1c81b8ec6f749e1f152dd945472ffd5a36b9cf3cd9a3a64b2f19b1d',
 'src/items/items_definitions.hpp':'0eff3929fca762fada5e31aa5ad069cf8f92713ee8c047c0578bb7ea8612c4cf',
 'src/items/items.hpp':'06d52ee83250ba852fe804c6b006b0aac150ceb26893e9079fa8c93502d209ca',
 'src/items/item.cpp':'e27e622bf4fd9a94d7fffb3a818fd0c5f911e6fbf4b92844320154ded326950b',
 'data/scripts/actions/items/usable_spirit_of_purity_items.lua':'b1972dd042874e48343c652bbc61b3c8b0194d33319965d670ac4a133f176476',
 'data-global/monster/quests/rotten_blood_quest/bakragore.lua':'974df9389e734ca947111cd41af9adec44f531adf0cc53233d5a7a94ab190178',
 'data-global/monster/quests/rotten_blood_quest/murcion.lua':'cdd4e39e5119d6c1952f3850ba4bfd19877f608716f8b166fe39a6f25ca1801a',
}


def candidate(donor):
    sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
    import build_source_item_profiles as source
    from build_spell_item_identities import BINDINGS, binding_index
    raw = {p:(donor/p).read_bytes() for p in SOURCES}
    if any(hashlib.sha256(raw[p]).hexdigest()!=h for p,h in SOURCES.items()):
        raise ValueError('SRV7 generic cosmetic factory source changed')
    xml = source.xml_index(raw['data/items/items.xml'])
    if 44048 in xml:
        raise ValueError('44048 no longer uses the qualified generic Item factory')
    objects = source.engine_items.load_appearance_objects(raw['data/items/appearances.dat'])
    obj = objects[44048]
    expected = {'flags.usable':True,'flags.take':True,'flags.light':True,'light.brightness':3,
        'light.color':179,'flags.market':True,'market.category':15,'market.trade_as_object_id':44048,
        'market.show_as_object_id':44048,'flags.cyclopediaitem':True,'cyclopediaitem.cyclopedia_type':44048}
    if obj['flags']!=expected or obj['name']!='spiritual horseshoe':
        raise ValueError('44048 appearance identity/physics changed')
    checks={
     'src/items/items.cpp':['iType.id = static_cast<uint16_t>(object.id());','iType.movable = object.flags().unmove() == false;',
        'iType.stackable = object.flags().cumulative();','iType.pickupable = object.flags().take();','iType.name = object.name();'],
     'src/items/items_definitions.hpp':['ITEM_TYPE_VALUABLE = 15,','ITEM_TYPE_MAGICFIELD,','ITEM_TYPE_TELEPORT,'],
     'src/items/items.hpp':['int32_t weight = 0;','uint32_t charges = 0;','uint32_t decayTime = 0;'],
     'src/items/item.cpp':['if (it.id != 0)','newItem = std::make_shared<Item>(type, count);'],
     'data/scripts/actions/items/usable_spirit_of_purity_items.lua':['[44048] = { key = "spiritual-horseshoe", count = 4 }','mountId = 217','item:remove(1)'],
     'data-global/monster/quests/rotten_blood_quest/bakragore.lua':['name = "spiritual horseshoe"'],
     'data-global/monster/quests/rotten_blood_quest/murcion.lua':['name = "spiritual horseshoe"'],
    }
    if any(any(text not in raw[path].decode() for text in values) for path,values in checks.items()):
        raise ValueError('44048 explicit producer/loader closure changed')
    bindings=binding_index(BINDINGS.read_bytes())
    if '44048' in bindings:
        raise ValueError('44048 now has a historical binding; remove the local qualification')
    physics=source.policies(raw['data/items/appearances.dat'],raw['data/items/items.xml'],{44048})[0]
    record=source.profile(44048,None,obj,physics,bindings,None)
    if record['attributes'].get('spell_augments') == []:
        del record['attributes']['spell_augments']
    binding={'source_key':'oteryn:source.crystalserver','source_revision':PIN,
        'identity_namespace':'ots/item_server_id','external_id':'44048',
        'target':{'family':'Item','key':'oteryn:item.tibia.i44048','revision':'definition-r1'},'disposition':'EXACT'}
    record['production_binding']=binding
    record['production_definition']={'family':'Item','production_key':'oteryn:item.tibia.i44048','revision_ref':'definition-r1'}
    return {'schema':'OTERYN_SRV7_COSMETIC_ITEM_IDENTITY/v1','family':'Item','source_revision':PIN,
        'bindings':[binding],'sources':[{'path':p,'sha256':h} for p,h in SOURCES.items()],
        'method':'Exact DAT market identity + pinned Items loader/defaults + ordinary Item factory + named boss loot + explicit mount action',
        'xml_present':False,'weight_policy':'Pinned ItemType default 0; Global Wiki weight pending',
        'appearance_flags':expected,'profile':record}


def profile(packet):
    result=copy.deepcopy(packet['profile'])
    result['production_binding_qualification']=copy.deepcopy(packet)
    return result


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--donor',required=True,type=Path)
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    raw=(json.dumps(candidate(args.donor),indent=2,ensure_ascii=False)+'\n').encode()
    output=ROOT/PACKET
    pin=(hashlib.sha256(raw).hexdigest()+'\n').encode()
    if args.check:
        if output.read_bytes()!=raw or output.with_suffix('.sha256').read_bytes()!=pin:
            raise SystemExit('SRV7 cosmetic candidate stale')
    else:
        output.parent.mkdir(parents=True,exist_ok=True);output.write_bytes(raw);output.with_suffix('.sha256').write_bytes(pin)
