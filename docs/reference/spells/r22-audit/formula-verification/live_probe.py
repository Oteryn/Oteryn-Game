import concurrent.futures,copy,hashlib,json,pathlib,time,urllib.request
p=pathlib.Path(__file__).parent
base=json.load(open('/workspace/spells-r18/implementation-handoff-r18/calculator-vectors/fixtures/baseline.json'))
selected=[c for c in base['cases'] if c['family']=='level-grid' and c['formulaInputs']['level'] in (1,1000,1101)]
def probe(case):
 req=urllib.request.Request('https://tibiatools.io/api/v1/damage',data=json.dumps(case['request']).encode(),headers={'Content-Type':'application/json','User-Agent':'Mozilla/5.0'})
 try:
  with urllib.request.urlopen(req,timeout=30) as r:
   raw=r.read();status=r.status
  res=json.loads(raw)
  (p/'live-responses'/f"{case['caseId']}.json").write_bytes(raw)
  result=copy.deepcopy(case);result['response']=res
  return result,{'case_id':case['caseId'],'http_status':status,'response_sha256':hashlib.sha256(raw).hexdigest(),'bytes':len(raw),'same_as_pinned_recording':res==case['response']}
 except Exception as e:return None,{'case_id':case['caseId'],'error':str(e)}
(p/'live-responses').mkdir(exist_ok=True)
with concurrent.futures.ThreadPoolExecutor(max_workers=5) as ex:results=list(ex.map(probe,selected))
out=copy.deepcopy(base);out['cases']=[r for r,e in results if r];out['source']={'url':'https://tibiatools.io/api/v1/damage','commit':None,'execution':'Live normal HTTP POST 2026-10-02; exact requests from pinned baseline; metadata lookup independently recorded at pinned eeed345c','live_revision_unpublished':True,'metadata_commit':'eeed345c86a76eb00f5ff7b2f0e0b49927799bf1'}
(p/'live-reference.json').write_text(json.dumps(out,indent=2)+'\n');(p/'live-probe-receipts.json').write_text(json.dumps({'requests':len(selected),'receipts':[e for r,e in results]},indent=2)+'\n')
print(json.dumps({'requests':len(selected),'success':len(out['cases']),'same_recordings':sum(e.get('same_as_pinned_recording',False) for r,e in results)}))
