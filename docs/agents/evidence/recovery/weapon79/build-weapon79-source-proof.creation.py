import json,sys,hashlib,pathlib
R=pathlib.Path('/workspace/pr1437-world-owner-completion');S=pathlib.Path('/workspace/audit-continuation');sys.path.insert(0,str(R/'tools/content-schema/item-authoring'))
import lower_wiki_stack_default_packet as base
import lower_wiki_movable_packet as movable
from engine_items import decode_appearance_object,protobuf_fields
import appearance_membership
sha=base.sha;load=lambda p:json.loads(p.read_bytes())
proposal=load(S/'weapon79-two-property-itemauthoring-owner-proposal.json')
assert sha((S/'weapon79-two-property-itemauthoring-owner-proposal.json').read_bytes())=='9bbbaace7b2fcdfc539cf3abcb58018bf4bd0cd7397fabff3e00c5c63b2c8f82'
gp='docs/agents/evidence/OTV2-20261001-item-movable-source-qualification-v1.json';g=load(R/gp);index,pages=movable.own_index(g)
identity=load(R/g['identity_bridge']['path']);revisions=identity['bridge']['source_revisions'];bindings=base.exact_bindings(load(R/base.BINDINGS)['bindings'],revisions)
_,manifests=appearance_membership.load_admitted(out_dir=R/'imports/official/appearance-membership');members={k:{v[0]:v for v in m['entries']} for k,m in manifests.items()}
objects={}
for tag,raw in protobuf_fields(base.checked(R,base.CLIENT,base.CLIENT_SHA)):
 if tag==1:
  obj=decode_appearance_object(raw);assert obj['id'] not in objects;objects[obj['id']]=obj|{'object_sha256':sha(raw)}
records={}
for shard in load(R/'content/items/index.json')['shards']:
 for row in load(R/shard)['records']:
  key=row['definition']['identity']['key'];assert key not in records;records[key]=row
witnesses={};qualified=[]
for row in proposal['qualified_records']:
 iid=row['item_id'];target=row['full_target'];bound=bindings[target['key']];record=records[target['key']];obj=objects[iid];tokens=[]
 assert record['definition']['semantics']['weapon']==row['preserve_current_native_weapon_group']
 for pid in sorted(index[iid]):
  page=pages[pid]
  for box in page['own_objects']:
   if not any(str(iid) in __import__('re').findall(r'\d+',v) for v in box['raw_itemid_values']):continue
   token=f"{pid}:{box['box_index']}";w=g['competing_own_infobox_witnesses'][token];witnesses[token]=w;tokens.append(token)
 source={'source_item_id':iid,'target':target,'binding':bound,'official_name':obj['name'],'official_object_sha256':obj['object_sha256'],'current_membership':members['client-15.30'][iid],'binding_membership':members['crystal-donor-00ce02a5' if bound['source_revision'].startswith('00ce') else 'crystal-ff7ede5'][iid],'own_tokens':tokens,'field':row['source_field'],'value':row['proposed_typed_value'],'native_weapon_guard':record['definition']['semantics']['weapon'],'native_name_guard':movable.leaf(record['definition'],'presentation.name')}
 qualified.append(source)
proof={'schema':'OTERYN_ITEM_WEAPON_METADATA_SOURCE_QUALIFICATION/v1','authoring_baseline':'2ec1f34f05fd89abbb041f313b0852c331ae2b53','qualification_cutoff':proposal['cutoff'],'source_frame_policy':'COMPACT_OWN_INFOBOX_FIELD_WITNESS','fullarticle_content_sha256_semantics':'DECLARED_CAPTURE_COORDINATE_NOT_RECOMPUTED; own raw template independently hashed','historical_proposal_sha256':'9bbbaace7b2fcdfc539cf3abcb58018bf4bd0cd7397fabff3e00c5c63b2c8f82','independent_peer_sha256':'7459bb72f07abc02ab549d5691562e33b133c35f8ae91277f236767cc77f9404','global_own_index_bridge':{'path':gp,'sha256':sha((R/gp).read_bytes())},'source_revisions':revisions,'client_path':base.CLIENT,'client_sha256':base.CLIENT_SHA,'input_digests':{p:sha((R/p).read_bytes()) for p in [base.DECODER,base.PARSER,base.BINDINGS,base.WIKI]},'competing_own_infobox_witnesses':witnesses,'records':qualified,'counts':{'items':79,'fields':79,'atk_mod':54,'hit_chance':25,'native_name_unknown':4},'preserved_holds':proposal['holds']}
p=S/'weapon79-2ec-source-qualification-checkpoint.json';p.write_text(json.dumps(proof,sort_keys=True,separators=(',',':'))+'\n');print(str(p),p.stat().st_size,sha(p.read_bytes()))
