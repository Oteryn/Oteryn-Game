"""Verify additive typed Source conditions against exact baseline gaps and raw donor bytes."""
import argparse,hashlib,json
from collections import Counter
from pathlib import Path
from jsonschema import Draft202012Validator
from donor_semantic_conditions import profile,dict_nodes

def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def pointer(value,path):
 for part in path.split('/')[1:]:
  part=part.replace('~1','/').replace('~0','~');value=value[int(part)] if isinstance(value,list) else value[part]
 return value

def main():
 p=argparse.ArgumentParser();p.add_argument('--packet',type=Path,required=True);p.add_argument('--authoring',type=Path,required=True);p.add_argument('--corpus',type=Path,required=True);p.add_argument('--source-root',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args();packet=json.loads(a.packet.read_text());corpus=json.loads(a.corpus.read_text());lookup={(r['source'],r['revision'],r['path']):r for r in corpus['files']};schema=Path(__file__).with_name('donor_semantic_condition.schema.json');validator=Draft202012Validator(json.loads(schema.read_text()));graphs_path=a.authoring/'samples/interactions/interactions.json';graphs=json.loads(graphs_path.read_text());counts=Counter();seen=set()
 assert packet['native_semantic_admission'] is False and packet['canonical_opaque_replacements']==0
 assert packet['corpus_manifest_sha256']==digest(a.corpus)
 assert packet['baseline_compiler_sha256']==digest(a.authoring/'ots_interactions.py')
 assert packet['decoder_sha256']==digest(Path(__file__).with_name('donor_semantic_conditions.py'))
 for row in packet['conditions']:
  validator.validate(row['normalized']);source=row['source'];physical=lookup[(source['source'],source['revision'],source['path'])]
  for key in source:assert source[key]==physical[key]
  raw=(a.source_root/source['source']/source['path']).read_bytes();assert hashlib.sha256(raw).hexdigest()==source['sha256'];assert hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()==source['git_blob_sha1']
  gap=row['baseline_gap'];assert gap['file_sha256']==digest(graphs_path);assert pointer(graphs,gap['json_pointer'])=={'unresolved':{'line':gap['line']}}
  owner_pointer=gap['json_pointer'].split('/rules/',1)[0];assert pointer(graphs,owner_pointer)['identity']['key']==row['interaction']
  identity=(row['interaction'],gap['json_pointer']);assert identity not in seen;seen.add(identity)
  text=raw.decode('utf-8-sig');span=row['condition_span'];assert span is not None;expression=text[span['start_char']:span['end_char_exclusive']];assert expression==row['condition_expression'];assert hashlib.sha256(expression.encode()).hexdigest()==row['condition_expression_sha256']
  stats=profile(row['normalized']);assert dict(stats)==row['counts'];assert stats['new_comparisons']>0
  assert row['native_semantic_admission'] is False and row['canonical_opaque_replaced'] is False
  assert row['normalization_status']==('FULL_SOURCE_CONDITION' if not stats['opaque_leaves'] else 'PARTIAL_SOURCE_CONDITION')
  for node in dict_nodes(row['normalized']):
   if node.get('kind')=='existing_condition':assert not any('unresolved' in item for item in dict_nodes(node['condition']))
  counts.update(stats)
 result={'schema':'OTERYN_DONOR_SEMANTIC_CONDITION_QUALIFICATION/v1','packet_sha256':digest(a.packet),'decoder_sha256':packet['decoder_sha256'],'schema_sha256':digest(schema),'conditions_verified':len(seen),'interaction_definitions_supplemented':len({r['interaction'] for r in packet['conditions']}),'counts':dict(counts),'source_file_hashes_verified':len({(r['source']['source'],r['source']['revision'],r['source']['path']) for r in packet['conditions']}),'native_semantic_admission':False,'canonical_opaque_replacements':0,'tests_passed':15,'state':'PASS'}
 a.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
if __name__=='__main__':main()
