"""Normalize the current Messenger stock without a duplicate rendered-tree alias.

Crystal201189f8 messenger_of_santa.lua55-66 supplies12exact goods; the current
Wiki62185 stock also sells bluepresent6510 for20. Wiki43048's rendered2812tree
is the same package whose actual pinned construction-kit item is10208.
"""
import argparse
import hashlib
import json
from pathlib import Path
import summer_npcs

ROOT=Path(__file__).resolve().parents[2]
KEY='oteryn:service.trade.messenger_of_santa'
PIN='acc9994e2ad8868696bc10ef08070108e2baba92'
WIKI_SHA='36eaf3275dd982ead0c27380b158e4a77bbbec69b00694d38b887bf2e53ec64c'
SOURCE='oteryn:source.tibiawiki.srv7_messenger_shop'
BATCH='srv7-seasonal-messenger-shop-r1'
CURRENT={10208:50,6387:10,6488:40,6501:45,6502:25,6503:25,6504:25,6505:20,6506:70,6507:60,6508:80,6509:20,6510:20}

def normalized(offers):
 current={int(row['item']['key'].rsplit('i',1)[1]):row['unit_price'] for row in offers if row['direction']=='SellToPlayer'}
 old=dict(CURRENT);old.pop(6510);old[2812]=50
 if len(offers)!=13 or current not in (old,CURRENT):raise ValueError('MESSENGER_STOCK_CONCURRENT_SOURCE_CONFLICT')
 return [{'direction':'SellToPlayer','item':{'family':'Item','key':f'oteryn:item.tibia.i{i}','revision':'definition-r1'},'unit_price':price} for i,price in sorted(CURRENT.items(),key=lambda row:f'oteryn:item.tibia.i{row[0]}')]

def source_data():
 data=json.loads(Path(__file__).with_suffix('.json').read_bytes())
 if data['schema']!='OTERYN_SRV7_CURRENT_MESSENGER_STOCK/v1' or data['current_wiki']['mirror_revision']!=PIN or data['current_wiki']['sha256']!=WIKI_SHA or {r['source_id']:r['price'] for r in data['offers']}!=CURRENT:
  raise ValueError('MESSENGER_PINNED_CURRENT_STOCK_INPUT')
 return data

def files():
 source_data()
 docs={relative:json.loads((ROOT/'content/world'/relative).read_bytes()) for relative in ['definitions/declarations.json','provenance/sources.json','provenance/imports.json']}
 rows=[r for r in docs['definitions/declarations.json']['records'] if r.get('identity',{}).get('key')==KEY]
 if len(rows)!=1:raise ValueError('MESSENGER_ONE_NATIVE_SERVICE')
 rows[0]['offers']=normalized(rows[0]['offers'])
 sources=docs['provenance/sources.json']
 summer_npcs.append_exact(sources['sources'],{'key':SOURCE,'revision':PIN,'sha256':WIKI_SHA,'import_batch_id':BATCH,'evidence':'Derived'},lambda r:(r['key'],r['revision']))
 summer_npcs.append_exact(sources['source_identity_bindings'],{'source_key':SOURCE,'source_revision':PIN,'identity_namespace':'mediawiki/page_id','external_id':'62185','target':{'family':'NPC','key':'oteryn:npc.messenger_of_santa','revision':'definition-r1'},'disposition':'EXACT'},lambda r:(r['source_key'],r['source_revision'],r['identity_namespace'],r['external_id'],r['target']['family']))
 batches=docs['provenance/imports.json']['batches'];batches[:]=[r for r in batches if r['batch_id']!=BATCH]
 batches.append({'batch_id':BATCH,'source_repository':'Lavovskii/grimorio','source_revision':PIN,'source_artifact_sha256':WIKI_SHA,'access_disposition':'PENDING','source_generation_profile':'OTERYN_NPC_SHOP_REFERENCE/v1','importer':'OTERYN_SEASONAL_MESSENGER_SHOP/v1','mapper':'OTERYN_SEASONAL_MESSENGER_SHOP/v1','mapper_revision':'srv7-current-stock-r1','mapper_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'candidates':[],'reimport_states':[]})
 return summer_npcs.canonical_files({p:summer_npcs.dump(d) for p,d in docs.items()})

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');a=p.parse_args()
 for path,raw in files().items():
  if a.check:
   if Path(path).read_text()!=raw:raise ValueError('MESSENGER_CANONICAL_DRIFT:'+path)
  else:Path(path).write_text(raw)
 print('SRV7_MESSENGER_CURRENT_SOURCE13 blue6510=true package10208=true duplicate2812=false')
if __name__=='__main__':main()
