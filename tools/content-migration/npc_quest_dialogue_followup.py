"""Closed R28 static quest-transcript upgrades, preserving all runtime/economic authoring."""
import copy,hashlib,json
from npc_source_refine import ROOT,build as historical_build,speech,source,values,update
from npc_bulk_enrich import indexed
from npc_bulk_stage import canonical
FROM='g4-npc-provisional-enrichment-r27';TO='g4-npc-provisional-enrichment-r28'
BASIC=('greet','farewell','name','job')
def eligible():
    p=json.loads((ROOT/'docs/agents/evidence/OTV2-20261002-npc-appearance-r27/native-enrichment.json').read_text())
    keys={r['after']['identity']['key'] for r in p['repairs'] if all(
        json.loads(values(r['after'])['quality']).get('dialogue.'+part)=='defaulted' for part in BASIC)}
    if len(keys)!=33:raise ValueError('quest-dialogue closed eligible inventory drifted')
    return keys
def literal_input(quote,actor):
    q=copy.deepcopy(quote)
    if q.get('normalization')=={'from':'Player','to':'|PLAYERNAME|'}:
        if q.get('classification')!='APPROXIMATE_PLACEHOLDER_NORMALIZATION':
            raise ValueError('Player placeholder substitution needs explicit approximation classification')
        proofs=q.get('quote_evidence',[]);reply=q.get('reply',[])
        if len(reply)!=len(proofs) or not reply:raise ValueError('Player normalization lacks literal evidence')
        for actual,proof in zip(reply,proofs):
            raw=proof.get('original_exact_text','')
            if 'Player' not in raw or actual!=raw.replace('Player','|PLAYERNAME|'):
                raise ValueError('unpermitted Player quote rewriting')
        q['reply']=[p['original_exact_text'] for p in proofs]
        q.pop('normalization');q['classification']='LITERAL_SOURCE_QUOTE'
    speech(q,actor)
    return q
def build(baseline,records,exchange_reference=None):
    selected=indexed(records,lambda r:r['key']);allowed=eligible()
    if set(selected)-allowed:raise ValueError('quest dialogue upgrades are restricted to the33 default-basic actors')
    helper_records=[]
    for r in records:
        if set(r)-{'key','name','dialogue'} or not r.get('dialogue'):
            raise ValueError('quest dialogue batch cannot change appearance/role or enable actions')
        q=copy.deepcopy(r)
        q['dialogue']={part:literal_input(quote,r['name']) for part,quote in r['dialogue'].items()}
        helper_records.append(q)
    helper_baseline=copy.deepcopy(baseline)
    if 'project_revision' in helper_baseline:helper_baseline['project_revision']='g4-npc-provisional-enrichment-r23'
    packet=historical_build(helper_baseline,helper_records)
    if packet['profile_repairs']:raise ValueError('dialogue successor cannot repair profiles')
    packet['from_project_revision']=FROM;packet['project_revision']=TO
    for repair in packet['repairs']:
        before,after=repair['before'],repair['after'];key=after['identity']['key']
        if after['kind']=='NPC':
            metadata=json.loads(values(before)['source_metadata'])
            if key in selected:metadata['r28_source_upgrades']=selected[key]
            if exchange_reference and key=='oteryn:npc.a_blue_stone':
                if exchange_reference['key']!=key or len(exchange_reference['prices'])!=5:
                    raise ValueError('exchange reference is restricted to documented BlueStone five prices')
                source(exchange_reference['source'])
                metadata['r28_exchange_reference']={**exchange_reference,'runtime_enabled':False,'trade_enabled':False}
            update(after,{'source_metadata':metadata})
        elif after['kind']=='Dialogue':
            actor=key.replace('oteryn:dialogue.npc.','oteryn:npc.',1);actual=selected[actor]['dialogue']
            for part,quote in actual.items():
                if part in {'greet','farewell'}:after[part]=quote['reply']
                else:next(n for n in after['keywords'] if n['key']==part)['reply']=quote['reply']
            document=json.loads(values(after)['dialogue_source'])
            document.pop('r24_selected');document['r28_selected']=actual
            document['r28_original_role_alignment']=document.pop('r24_original_role_alignment')
            update(after,{'dialogue_source':document})
    return packet
if __name__=='__main__':
    import argparse
    from pathlib import Path
    p=argparse.ArgumentParser(description=__doc__)
    for key in ['baseline','upgrades','output']:p.add_argument('--'+key,type=Path,required=True)
    p.add_argument('--exchange-reference',type=Path)
    a=p.parse_args();exchange=json.loads(a.exchange_reference.read_text()) if a.exchange_reference else None
    packet=build(json.loads(a.baseline.read_text()),json.loads(a.upgrades.read_text()),exchange)
    a.output.write_bytes(canonical(packet));print(hashlib.sha256(a.output.read_bytes()).hexdigest())
