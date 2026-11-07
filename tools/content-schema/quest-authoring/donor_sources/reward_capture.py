"""Lossless donor component witnesses, not Native or inferred quest semantics.

Existing lexical masking is reused; full donor bytes remain authoritative.
Literal argument syntax is a candidate fact, not recipient/charge/count semantics.
"""
import argparse
import gzip
import io
import hashlib
import json
from pathlib import Path
import re
import sys

METHODS = {'addItem': 'reward_call', 'addItemEx': 'reward_call', 'createItem': 'item_constructor',
 'createItemEx': 'item_constructor', 'doPlayerAddItem': 'reward_call', 'doPlayerAddItemEx': 'reward_call',
 'addMoney': 'reward_call', 'addExperience': 'reward_call', 'addAchievement': 'reward_call',
 'addOutfit': 'reward_call', 'addOutfitAddon': 'reward_call', 'addMount': 'reward_call',
 'removeItem': 'item_dependency', 'getItemCount': 'item_dependency', 'getItemById': 'item_dependency',
 'getId': 'item_identity_query', 'getUniqueId': 'key_predicate', 'getActionId': 'key_predicate',
 'setStorageValue': 'storage_effect', 'getStorageValue': 'storage_predicate',
 'setPlayerStorageValue': 'storage_effect', 'getPlayerStorageValue': 'storage_predicate',
 'getStorage': 'storage_predicate', 'setStorage': 'storage_effect',
 'setBossCooldown': 'cooldown', 'getBossCooldown': 'cooldown', 'time': 'time_expression',
 'random': 'random_expression', 'getLevel': 'requirement', 'isPremium': 'requirement',
 'getVocation': 'requirement', 'getSex': 'requirement', 'hasOutfit': 'requirement'}
TABLE = re.compile(r'(?<![\w])([A-Za-z_]\w*)\s*=\s*\{')
CALL = re.compile(r'(?<![\w])([A-Za-z_]\w*(?:[.:][A-Za-z_]\w*)*)\s*\(')
PIN = {'canary': ('opentibiabr/canary', '04b83b512114bfd888000d6e1433ed8ecaec7c5b'),
       'crystalserver': ('zimbadev/crystalserver', '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'),
       'crystal-summer': ('zimbadev/crystalserver', '00ce02a57ca5a12e48f32a3476e37471167e4c3f')}


def digest(b): return hashlib.sha256(b).hexdigest()


def canonical(x): return json.dumps(x, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def close_paren(masked, opening):
    depth = 0
    for pos in range(opening, len(masked)):
        if masked[pos] == '(': depth += 1
        elif masked[pos] == ')':
            depth -= 1
            if depth == 0: return pos + 1
    return None


def argument_expressions(raw, masked):
    spans=[];start=0;depth=0
    for pos,c in enumerate(masked):
        if c in '({[':depth+=1
        elif c in ')}]':depth-=1
        elif c==',' and depth==0:spans.append((start,pos));start=pos+1
    spans.append((start,len(raw)))
    return [raw[a:b].strip() for a,b in spans] if raw.strip() else []


def capture_lua(blob, source, path, mask_code):
    text = blob.decode('utf-8', 'surrogateescape');masked=mask_code(text);rows=[]
    for match in CALL.finditer(masked):
        qualified=match.group(1);method=re.split('[.:]',qualified)[-1]
        if method not in METHODS:continue
        opening=masked.find('(',match.start(),match.end());end=close_paren(masked,opening);complete=end is not None
        if end is None:end=len(text)
        a,b=match.start(),end;line=text.count('\n',0,a)+1;last=text.count('\n',0,b)+1
        line_start=text.rfind('\n',0,a)+1;line_end=text.find('\n',b);line_end=len(text) if line_end<0 else line_end
        raw=text[a:b];args=argument_expressions(text[opening+1:b-1],masked[opening+1:b-1])if complete else []
        literal_candidates=[{'argument_index':n,'raw':arg,'integer':int(arg)}for n,arg in enumerate(args)if re.fullmatch(r'\d+',arg)]
        rows.append({'source':source,'path':path,'blob_sha256':digest(blob),'category':METHODS[method],
          'callee':qualified,'syntax_role':'declaration' if re.search(r'\bfunction\s*$',masked[max(0,masked.rfind('\n',0,a)+1):a]) else 'call_candidate','line_start':line,'line_end':last,'char_start':a,'char_end':b,
          'byte_start':len(text[:a].encode('utf-8','surrogateescape')),'byte_end':len(text[:b].encode('utf-8','surrogateescape')),
          'raw_expression':raw,'raw_expression_sha256':digest(raw.encode('utf-8','surrogateescape')),
          'source_line_span':text[line_start:line_end],
          'source_line_span_sha256':digest(text[line_start:line_end].encode('utf-8','surrogateescape')),
          'argument_expressions':args,'literal_integer_syntax_candidates':literal_candidates,
          'capture_complete':complete,'syntax_item_id_argument':(1 if method in ('doPlayerAddItem','doPlayerAddItemEx') else 0) if method in ('addItem','createItem','createItemEx','doPlayerAddItem','doPlayerAddItemEx','removeItem','getItemCount','getItemById') else None,'semantic_decoded':False,'native_bound':False,
          'unresolved_semantics':['callee_receiver_authority','guard_and_control_flow','argument_quantity_subtype_or_effect','helper_and_dynamic_dependencies']})
    for match in TABLE.finditer(masked):
        label=match.group(1)
        if 'reward' not in label.casefold() and label.casefold() not in ('items','questitems'):continue
        opening=masked.find('{',match.start(),match.end());depth=0;end=None
        for pos in range(opening,len(masked)):
            if masked[pos]=='{':depth+=1
            elif masked[pos]=='}':
                depth-=1
                if depth==0:end=pos+1;break
        if end is None:continue
        a,b=match.start(),end;raw=text[a:b];line=text.count('\n',0,a)+1
        rows.append({'source':source,'path':path,'blob_sha256':digest(blob),'category':'reward_table_candidate',
          'callee':label,'syntax_role':'table_assignment_candidate','line_start':line,'line_end':text.count('\n',0,b)+1,'char_start':a,'char_end':b,
          'byte_start':len(text[:a].encode('utf-8','surrogateescape')),'byte_end':len(text[:b].encode('utf-8','surrogateescape')),
          'raw_expression':raw,'raw_expression_sha256':digest(raw.encode('utf-8','surrogateescape')),
          'source_line_span':raw,'source_line_span_sha256':digest(raw.encode('utf-8','surrogateescape')),
          'argument_expressions':[],'literal_integer_syntax_candidates':[],'syntax_item_id_argument':None,
          'capture_complete':True,'semantic_decoded':False,'native_bound':False,
          'unresolved_semantics':['table_role_and_owner','helper_and_dynamic_dependencies','branch_or_random_selection','quantity_vs_subtype']})
    return rows


def verify_component(blob, component):
    raw = blob[component['byte_start']:component['byte_end']]
    if raw != component['raw_expression'].encode('utf-8','surrogateescape'):
        raise ValueError('raw expression byte span differs')
    if digest(raw) != component['raw_expression_sha256'] or digest(blob) != component['blob_sha256']:
        raise ValueError('raw expression or source blob digest differs')
    if blob.count(b'\n',0,component['byte_start'])+1 != component['line_start']:
        raise ValueError('source line differs')
    if component['semantic_decoded'] is not False or component['native_bound'] is not False:
        raise ValueError('lexical capture cannot claim semantic or Native qualification')
    return True


def main():
    ap=argparse.ArgumentParser();ap.add_argument('--repo-root',type=Path,required=True);ap.add_argument('--corpus-manifest',type=Path,required=True);ap.add_argument('--out',type=Path,required=True);args=ap.parse_args()
    sys.path.insert(0,str(args.repo_root/'tools/content-schema/quest-authoring'));from lua_writers import mask_code
    manifest=json.loads(args.corpus_manifest.read_text());files=manifest['files'];inventory=[];components=[]
    for row in files:
        if not row['path'].endswith('.lua'):continue
        cache=Path(row.get('cache_path',row.get('blob_path')))
        if not cache.is_absolute():cache=args.corpus_manifest.parent/cache
        blob=cache.read_bytes();assert digest(blob)==row['sha256'];assert hashlib.sha1(b'blob '+str(len(blob)).encode()+b'\0'+blob).hexdigest()==row['git_blob_sha1']
        repository,revision=PIN[row['source']];assert row['revision']==revision and row['repository']==repository
        found=capture_lua(blob,row['source'],row['path'],mask_code);components.extend(found)
        inventory.append({k:row[k]for k in ('source','repository','revision','path','sha256','git_blob_sha1')}|{'component_count':len(found),'lossless_source_available':True,'semantic_decoded':False,'native_bound':False})
    packet={'schema':'OTERYN_DONOR_REWARD_COMPONENT_CAPTURE/v1','scope':'PINNED_DONOR_SOURCE_ONLY_NO_WIKI_OR_CHOSEN_OTERYN_FACTS',
      'source_manifest_sha256':digest(args.corpus_manifest.read_bytes()),'files':inventory,'components':components,
      'summary':{'files_checked':len(inventory),'components_captured':len(components),'semantic_decoded':0,'native_bound':0}}
    packet['summary']['semantic_scope']='No expression semantic or Native qualification performed by this lexical capture; zero counts do not measure existing server implementation.'
    args.out.parent.mkdir(parents=True,exist_ok=True);raw=(json.dumps(packet,ensure_ascii=True,indent=2)+'\n').encode()
    if args.out.suffix=='.gz':
        out=io.BytesIO()
        with gzip.GzipFile(fileobj=out,mode='wb',filename='',mtime=0,compresslevel=9)as stream:stream.write(raw)
        args.out.write_bytes(out.getvalue())
    else:args.out.write_bytes(raw)
    print(packet['summary'])


if __name__=='__main__':main()
