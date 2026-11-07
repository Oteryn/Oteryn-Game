from pathlib import Path
import json,hashlib,collections
root=Path(__file__).resolve().parents[3];base=root/'tools/content-schema/native-gameplay'; raw=(root/'content/cosmetics/npc-acquisitions.json').read_bytes(); p=json.loads(raw); bindings={b['storage']:b for b in p['bindings']}
def q(s):return "'"+s.replace("'","''")+"'"
def j(v):return q(json.dumps(v,separators=(',',':')))+'::jsonb'
def arr(v):return 'ARRAY['+','.join(q(x) for x in v)+']::text[]'
quest_bindings=json.loads((base/'npc-quest-endpoint-bindings.json').read_bytes())['rows']
quest_sources={row['source_key'] for row in quest_bindings}
if len(quest_sources)!=23:raise ValueError('finite Quest source closure')
quest_rows=[]
sha=hashlib.sha256(raw).hexdigest();out=[f"INSERT INTO game_npc_source_programs VALUES(decode('{sha}','hex'),decode('{raw.hex()}','hex'));\n"]
for npc in p['npcs']:
 for t in npc['transitions']:
  readonly_ids={'oteryn:npc.vescu':{'assassin-returner-greet','assassin-drunken-beginner-greet','assassin-sober-refusal-greet'},'oteryn:npc.bron':{'barbarian-remorse-greet','barbarian-default-greet'},'oteryn:npc.a_majestic_warwolf':{'druid-unqualified-refusal-greet','druid-qualified-greet'},'oteryn:npc.bozo':{'jester-male-greet','jester-female-greet'}}
  registered_readonly=(t.get('trigger')=='greet' and t['id'] in readonly_ids.get(npc['npc'],set()) and t['effects']==([{'kind':'close_conversation'}] if t['id'] in {'assassin-sober-refusal-greet','druid-unqualified-refusal-greet'} else []))
  if not t['effects'] and not registered_readonly:continue
  writable={};foreign={};costs=collections.Counter();price=0
  enabled=t.get('trigger','say')=='say' and all(e['kind'] in {'set_track','consume_item','grant_look_addon','magic_effect','achievement','achievement_progress','close_conversation'} and (e['kind']!='set_track' or e['storage'] in npc['owned_tracks']) for e in t['effects'])
  ajax_fire = (npc['npc']=='oteryn:npc.ajax' and t['source_key']=='oteryn:npc-cosmetic/ajax/barbarian-fire'
   and [e for e in t['effects'] if e['kind']=='condition_damage']==[{'kind':'condition_damage','condition':'FIRE','delayed':True,'ticks':10,'interval_ms':2000,'damage_per_tick':10}]
   and all(e['kind'] in {'condition_damage','set_track','close_conversation'} for e in t['effects'])
   and all(e['storage'] in npc['owned_tracks'] for e in t['effects'] if e['kind']=='set_track'))
  ajax_greet = (npc['npc']=='oteryn:npc.ajax' and t['source_key']=='oteryn:npc-cosmetic/ajax/barbarian-greet'
   and t.get('trigger')=='greet' and t['effects']==[{'kind':'set_track','storage':'Storage.Quest.U7_8.BarbarianOutfits.BarbarianAddon','value':{'op':'literal','value':3}}])
  royal = (npc['npc'] in {'oteryn:npc.king_tibianus','oteryn:npc.queen_eloise','oteryn:npc.emperor_kruzak'}
   and t['id'] in {'golden-buy-armor','golden-buy-helmet','golden-buy-boots'}
   and [e for e in t['effects'] if e['kind']=='consume_money_bank']==[{'kind':'consume_money_bank','gold':500000000 if t['id']=='golden-buy-armor' else 250000000}]
   and all(e['kind'] in {'consume_money_bank','grant_look_addon','magic_effect','set_track','mint_store_decoration_kit'} for e in t['effects'])
   and all(e['storage'] in npc['owned_tracks'] for e in t['effects'] if e['kind']=='set_track'))
  sandra_output=(npc['npc']=='oteryn:npc.sandra' and t['id'] in {'belt-ticket-283','belt-ticket-284','belt-ticket-285'}
   and t['effects']==[{'kind':'consume_item','item':int(t['id'].rsplit('-',1)[1]),'count':100},{'kind':'mint_item','item':5957,'count':1}])
  quest_output={'oteryn:npc-cosmetic/bozo/jester-addon-1':895,'oteryn:npc-cosmetic/bozo/jester-addon-2':894}
  quest_leaf=(t['source_key'] in quest_sources
   and all(e['kind'] in {'set_track','consume_item','grant_look_addon','magic_effect','achievement','achievement_progress','close_conversation','mint_item','text_message'} for e in t['effects'])
   and all(e['kind']!='mint_item' or (e['item']==quest_output.get(t['source_key']) and e['count']==1) for e in t['effects']))
  vescu_greet=(npc['npc']=='oteryn:npc.vescu' and t.get('trigger')=='greet'
   and t['id'] in {'assassin-returner-greet','assassin-drunken-beginner-greet','assassin-sober-refusal-greet'}
   and t['effects']==([{'kind':'close_conversation'}] if t['id']=='assassin-sober-refusal-greet' else []))
  primal=(npc['npc']=='oteryn:npc.gnomadness' and t['id'] in {'ripptor-prize','ripptor-select'}
   and all(e['kind'] in {'set_hazard_level','set_kv','grant_mount','mint_item','achievement','text_message'} for e in t['effects'])
   and all(e['kind']!='grant_mount' or e['id']==202 for e in t['effects'])
   and all(e['kind']!='mint_item' or (e['item']==39546 and e['count']==1) for e in t['effects']))
  irmana_paid=(npc['npc']=='oteryn:npc.irmana' and t['id'] in {'noble-female-1-buy','noble-female-2-buy','noble-male-1-buy','noble-male-2-buy'}
   and [e for e in t['effects'] if e['kind']=='consume_money']==[{'kind':'consume_money','gold':150000}]
   and all(e['kind'] in {'consume_money','set_track','grant_look_addon','magic_effect','achievement','achievement_progress','close_conversation'} for e in t['effects'])
   and all(e['storage'] in npc['owned_tracks'] for e in t['effects'] if e['kind']=='set_track'))
  stampor=(npc['npc']=='oteryn:npc.chondur' and t['source_key']=='oteryn:npc-cosmetic/chondur/stampor-spirit'
   and t['effects']==[{'kind':'consume_item','item':12312,'count':50},{'kind':'consume_item','item':12314,'count':30},{'kind':'consume_item','item':12313,'count':100},{'kind':'grant_mount','id':11},{'kind':'magic_effect','id':14}])
  enabled = enabled or ajax_fire or ajax_greet or royal or sandra_output or quest_leaf or vescu_greet or primal or registered_readonly or irmana_paid or stampor
  for e in t['effects']:
   if e['kind']=='set_track':
    b=bindings[e['storage']];expr=dict(e['value'])
    if 'storage' in expr:expr['storage']=bindings[expr['storage']]['key']
    target=writable if e['storage'] in npc['owned_tracks'] else foreign
    target[b['key']]={'quest':b['quest'],'value':expr}
   elif e['kind']=='consume_item':costs[e['item']]+=e['count']
   elif e['kind'] in ('consume_money','consume_money_bank'):price+=e['gold']
  if foreign:
   if t['source_key'] not in quest_sources:raise ValueError('unreviewed foreign source write')
   expected=next(row for row in quest_bindings if row['source_key']==t['source_key'])
   if set(foreign)!={row['binding']['key'] for row in expected['matches']}:raise ValueError('foreign effect closure drift')
   quest_rows.append('INSERT INTO game_npc_source_quest_policies VALUES('+','.join([f"decode('{sha}','hex')",q(t['source_key']),j(foreign)])+');\n')
  rows=[{'family':'Item','key':f'oteryn:item.tibia.i{i}','revision':'definition-r1','count':count} for i,count in sorted(costs.items(),key=lambda pair:f'oteryn:item.tibia.i{pair[0]}')]
  service=f"oteryn:service.npc-cosmetic.{npc['npc'].split('.')[-1]}.{t['id']}" if price else None
  grant_keys=[g['key'] for g in t['grants']]
  if stampor:grant_keys.append('oteryn:appearance-grant/npc/chondur/stampor/mount')
  if primal and t['id']=='ripptor-prize':
   grant_keys.append('oteryn:appearance-grant/npc/gnomadness/noxious_ripptor/mount')
  out.append('INSERT INTO game_npc_source_policies VALUES('+','.join([f"decode('{sha}','hex')",q(t['source_key']),q(npc['npc']),arr(sorted({b['key'] for b in p['bindings']})),j(writable),arr(grant_keys),j(rows),j(t['effects']),q(service) if service else 'NULL',str(price) if price else 'NULL','TRUE' if enabled else 'FALSE'])+');\n')
(base/'npc-source-program-policy.sql').write_text(''.join(out)); print(sha,len(out)-1)

if len(quest_rows)!=23:raise ValueError('incomplete foreign policy closure')
(base/'npc-source-quest-policy.sql').write_text(''.join(quest_rows))
