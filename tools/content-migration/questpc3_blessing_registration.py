#!/usr/bin/env python3
"""Restore canonical blessing family registration after additive native-profile merges."""
import argparse,json
from pathlib import Path
from world_project_v2_to_tree import ROOT,write_registry

def refresh(check=False):
 index_path='content/services/blessings/index.json';index=json.loads((ROOT/index_path).read_bytes())
 rows=[r for shard in index['shards'] for r in json.loads((ROOT/shard).read_bytes())['records']]
 if len(rows)!=index['record_count']:raise ValueError('Blessing index differs from canonical rows')
 documents={p:json.loads((ROOT/p).read_bytes()) for p in ('content/manifest.json','content/content.lock.json')}
 manifest=documents['content/manifest.json'];manifest['families']['Service.Blessing']={'records':len(rows),'index':index_path}
 paths={r['path'] for r in manifest['managed_files'] if not r['path'].startswith('content/services/blessings/')}
 manifest['managed_files']=[{'path':p} for p in sorted(paths|{index_path,*index['shards']})]
 documents['content/content.lock.json']['family_counts']['Service.Blessing']=len(rows)
 for path,value in documents.items():
  if check:
   if json.loads((ROOT/path).read_bytes())!=value:raise ValueError('Blessing registration drift: '+path)
  else:write_registry(path,value)
 print('Blessing registration canonical records='+str(len(rows)))
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');refresh(p.parse_args().check)
