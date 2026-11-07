"""Compact independently reviewed source witnesses; never uses compiled guards as an oracle."""
import json,hashlib,pathlib,copy,argparse
a=argparse.ArgumentParser(description=__doc__);a.add_argument("--review",type=pathlib.Path,required=True);a.add_argument("--program",type=pathlib.Path,required=True);a.add_argument("--output",type=pathlib.Path,required=True);a.add_argument("--check",action="store_true");a.add_argument("--primal-witness",type=pathlib.Path,help="Actual pinned Lua normal/invalid branch execution witness");args=a.parse_args()
d=json.loads(args.review.read_bytes());p=json.loads(args.program.read_bytes());linked={(n['npc'],t['id']):t for n in p['npcs']for t in n['transitions']};cases=[]
for v in d['vectors']:
 if (v['npc'],v['transition']) not in linked:continue
 for negative in [False,True]:
  n=v['negative'] if negative else v
  facts=copy.deepcopy(n['before']);message=n['message'];expected=n['expected_transition']
  # A selected integer is obtained from the actual message, never independent stale facts.
  change=n.get('changed_fact',{})
  if negative and change.get('bucket')=='selected':message=str(change['after'])
  for k in ['selected','money','bank']:facts.pop(k,None)
  facts={k:x for k,x in facts.items()if x!={}}
  cases.append(dict(npc=v['npc'],source_transition=v['transition'],negative=negative,message=message,topic=n['topic'],before=facts,expected=expected))
if args.primal_witness:
 witness=json.loads(args.primal_witness.read_bytes())
 source=next(n['source'] for n in p['npcs'] if n['npc']=='oteryn:npc.gnomadness')
 if witness['source_path']!=source['path'] or witness['source_sha256']!=source['sha256'] or witness['source_revision']!='201189f871d23597b82caf88fd611454a2ddefb8':raise ValueError('PRIMAL_SOURCE_WITNESS_BINDING')
 captured={row['message']:row for row in witness['cases']}
 for message,reply,topic in [('5','Your hazard level has been set to 5. Good luck!',1),('not a number',"I'm sorry, I don't understand. What hazard level would you like to set?",2)]:
  row=captured[message]
  if row['source_events']!=[{'kind':'reply','text':reply}] or row['source_topic_after']!=topic:raise ValueError('PRIMAL_SOURCE_BRANCH_WITNESS')
 if ('oteryn:npc.gnomadness','ripptor-select') not in linked or ('oteryn:npc.gnomadness','ripptor-select-invalid') not in linked:raise ValueError('PRIMAL_COMPLETE_BRANCHES_NOT_LINKED')
 changed=[row for row in cases if row['npc']=='oteryn:npc.gnomadness' and row['source_transition']=='ripptor-prize' and row['negative'] and row['message']=='5']
 if len(changed)!=1:raise ValueError('PRIMAL_SOURCE_NEGATIVE_CASE_IDENTITY')
 if changed[0]['expected'] not in (None,'ripptor-select'):raise ValueError('PRIMAL_SOURCE_NEGATIVE_CONFLICT')
 changed[0]['expected']='ripptor-select'
assert len(cases)==904
out=dict(schema='OTERYN_NPC_SOURCE_POLICY_CASES/v1',source_input_sha256=d['source_input_sha256'],source_trace_sha256=hashlib.sha256(args.review.read_bytes()).hexdigest(),qualification='Independent pinned Lua traces and explicit current-Tibia differences are retained in the source review packet. Before facts exercise policy only, not earned runtime prerequisites or composed external owner completion.',cases=cases)
if args.primal_witness:out['source_trace_sha256']=hashlib.sha256(args.review.read_bytes()+args.primal_witness.read_bytes()).hexdigest()
raw=(json.dumps(out,separators=(',',':'))+'\n').encode();path=args.output
if args.check:
 if path.read_bytes()!=raw:raise ValueError('NPC_SOURCE_POLICY_CASES_DRIFT')
else:path.write_bytes(raw)
print('cases',len(cases),'bytes',len(raw))
