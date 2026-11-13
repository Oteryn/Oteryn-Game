"""Validate ordered Legs Basic custody, source candidates and client-only sprites."""
import collections,hashlib,json,pathlib,re
from compare_sources import generate,field,compare_extended,projected,ROUTES
HERE=pathlib.Path(__file__).resolve().parent
def read(name):return json.loads((HERE/name).read_text(encoding='utf-8'))
def hash64(v):return isinstance(v,str) and re.fullmatch('[0-9a-f]{64}',v)
def validate_client_appearance(observations):
    receipt=read('client-appearance-receipt.json')
    assert set(receipt)=={'schema','classification','source_receipt','current_DAT_sha256','private_program_receipt','decoder_sources','counts','sheet_checks','verification','rows','boundaries'}
    assert receipt['schema']=='OTERYN_LEGS_CLIENT_APPEARANCE_RECEIPT/v1'
    assert receipt['classification']=='DERIVED_CLIENT_NAMESPACE_CANDIDATES'
    source=receipt['source_receipt'];assert set(source)=={'filename','bytes','sha256'}
    assert source['sha256']=='de6760ccd85a540008428af9c68f4199194eac1f62bfe74c94d1378a909a9640'
    assert source['bytes']>0 and source['filename']=='items-legs-appearance-binding-takeover-20261113.json'
    assert receipt['current_DAT_sha256']=='aa086346daa9234d721e4170848095ad83f3ee9d28ea9e62c1bd06216016174e'
    program=receipt['private_program_receipt'];assert set(program)=={'filename','sha256'}
    assert program['sha256']=='12c0758ce1640143e46d3536647001b44e57c492afcf9b83d18f46adf15c42ad'
    assert len(receipt['decoder_sources'])==8
    for decoder in receipt['decoder_sources']:
        assert set(decoder)=={'repository_path','commit','sha256','git_blob_sha1'}
        assert decoder['commit']=='83be72ad403ebc221dadba4b161c85fa1d47219a'
        assert hash64(decoder['sha256'])and re.fullmatch('[0-9a-f]{40}',decoder['git_blob_sha1'])
        assert decoder['repository_path'].startswith('tools/')and'..'not in pathlib.PurePosixPath(decoder['repository_path']).parts
    assert len(receipt['sheet_checks'])==31
    for sheet in receipt['sheet_checks']:
        assert set(sheet)=={'file','packed_sha256','packed_size','packed_manifest_matches','decoded_bmp_sha256','filename_hash_matches','bpp','compression'}
        assert hash64(sheet['packed_sha256'])and hash64(sheet['decoded_bmp_sha256'])
        assert sheet['file']=='sprites-'+sheet['decoded_bmp_sha256']+'.bmp.lzma'
        assert sheet['packed_manifest_matches']is sheet['filename_hash_matches']is True
        assert sheet['packed_size']>0 and sheet['bpp']==32 and sheet['compression']in(0,3)
    verification=receipt['verification']
    assert verification=={'private_input_files_hash_verified':73,'decoder_files_hash_verified':8,'sprite_slot_metrics_independently_recomputed':142,'named_candidate_sets_reconciled_to_census':66,'projection':'EXPLICIT_KEY_WHITELIST_ONLY','descriptions':'HASH_LENGTH_RECEIPTS_ONLY'}
    def text_seal(value):
        if value is None:return
        assert set(value)=={'state','sha256_utf8','bytes','characters'}
        assert value['state']=='RETAINED_PRIVATELY'and hash64(value['sha256_utf8'])
        assert value['bytes']>=value['characters']>=0
    rows=receipt['rows'];assert len(rows)==66
    assert [r['ui_entry_ordinal']for r in rows]==list(range(1,67))
    classes=collections.Counter();sprites=set();slots=0
    for row,observation in zip(rows,observations):
        assert set(row)=={'ui_entry_ordinal','capture_key','status','retained_appearance_object_ids','server_id','fullName','icon_proof','observed_description_receipt','candidate_evaluations'}
        assert row['capture_key']==observation['capture_key']and row['fullName']==observation['fullName']
        assert row['server_id']is None
        icon=row['icon_proof'];assert set(icon)=={'private_source_key','sha256'}
        assert icon['private_source_key']==f"items-sequence-legs/cards/{row['ui_entry_ordinal']:05d}-icon.png"
        assert hash64(icon['sha256'])
        text_seal(row['observed_description_receipt'])
        sealed=observation.get('description_receipt')
        if sealed:
            assert all(row['observed_description_receipt'][k]==sealed[k]for k in ('sha256_utf8','bytes','characters'))
        else:assert row['observed_description_receipt']is None
        eligible=[]
        for candidate in row['candidate_evaluations']:
            assert set(candidate)=={'appearance_object_id','cyclopedia_type','market_trade_as_object_id','market_show_as_object_id','market_category','record_sha256','description_status','unknown_object_field_numbers','opaque_exact_any_known_sprite','best_mean_rgb_error','source_description_receipt','frame_groups'}
            for label in ('appearance_object_id','cyclopedia_type','market_trade_as_object_id','market_show_as_object_id','market_category'):
                assert candidate[label]is None or isinstance(candidate[label],int)and candidate[label]>=0
            assert hash64(candidate['record_sha256'])
            assert candidate['description_status']in('exact_normalized','different_observed_text','not_comparable_field_absent')
            assert isinstance(candidate['unknown_object_field_numbers'],list)and all(isinstance(n,int)for n in candidate['unknown_object_field_numbers'])
            text_seal(candidate['source_description_receipt'])
            all_metrics=[]
            for group in candidate['frame_groups']:
                assert set(group)=={'group_index','known_sprite_slot_metrics'}
                assert isinstance(group['group_index'],int)and group['group_index']>=0
                for slot,metric in enumerate(group['known_sprite_slot_metrics']):
                    assert set(metric)=={'slot','sprite_id','offset','opaque_pixels','opaque_exact_pixels','opaque_mean_absolute_rgb_error','opaque_max_rgb_error','comparable'}
                    assert metric['slot']==slot and isinstance(metric['sprite_id'],int)and metric['sprite_id']>0
                    assert metric['comparable']is True
                    assert len(metric['offset'])==2 and all(isinstance(n,int)and n>=0 for n in metric['offset'])
                    assert 0<=metric['opaque_exact_pixels']<=metric['opaque_pixels']and metric['opaque_pixels']>0
                    assert 0<=metric['opaque_mean_absolute_rgb_error']<=255 and 0<=metric['opaque_max_rgb_error']<=255
                    sprites.add(metric['sprite_id']);slots+=1;all_metrics.append(metric)
            exact=any(m['opaque_max_rgb_error']==0 for m in all_metrics)
            assert candidate['opaque_exact_any_known_sprite']is exact
            assert candidate['best_mean_rgb_error']==min(m['opaque_mean_absolute_rgb_error']for m in all_metrics)
            if exact and candidate['description_status']!='different_observed_text':eligible.append(candidate['appearance_object_id'])
        assert eligible==row['retained_appearance_object_ids']
        expected='unique_client_appearance_candidate'if len(eligible)==1 else'client_appearance_candidate_set'if len(eligible)>1 else'unresolved'
        assert row['status']==expected;classes[expected]+=1
    assert slots==142 and len(sprites)==139
    assert dict(classes)=={'unique_client_appearance_candidate':66}
    assert receipt['counts']=={**dict(classes),'unresolved':0,'observations':66,'decoded_sheets':31,'decoded_unique_sprites':139}
    assert isinstance(receipt['boundaries'],list)and all(isinstance(b,str)for b in receipt['boundaries'])
def main():
    packet=read('observations-1-66.json');rows=packet['items'];coverage=packet['coverage']
    assert packet['schema']=='OTERYN_COMPACT_PRIVATE_ITEMS_OBSERVATIONS/v1'
    assert packet['classification']=='PROVEN'and packet['category']=='Legs'and packet['noSourceID']is True
    assert packet['source_receipt']['sha256']=='94b060dab5ef0836264eb8a45855926d54e6ac6abe3ed9e5f059aeb6fc6f2996'
    assert [r['ui_entry_ordinal']for r in rows]==list(range(1,67))
    assert len({r['capture_key']for r in rows})==66
    assert coverage['category_end_observed']is True and coverage['trade_panels_qualified']is coverage['animation_qualified']is False
    proofs={};pages=texts=layouts=fields=0
    allowed={'capture_key','capture_id','ui_entry_ordinal','fullName','name_hash_is_item_identity','properties','proofs','saved_basic_pages','description_receipt','description_display_lines_receipt'}
    for r in rows:
        assert set(r)<=allowed and r['capture_key']==f"items-sequence-legs:{r['capture_id']}"
        assert r['name_hash_is_item_identity']is False and isinstance(r['fullName'],str)and r['fullName']
        assert 'Description'not in r['properties']and'description_display_lines'not in r
        assert all(isinstance(v,str)and v for v in r['properties'].values())
        fields+=len(r['properties']);pages+=r['saved_basic_pages']
        assert r['saved_basic_pages']==sum(bool(re.search(r'-basic-(?:top|\d+)\.png$',p['private_source_key']))for p in r['proofs'])
        for proof in r['proofs']:
            key=proof['private_source_key'];h=proof['sha256']
            assert key.startswith(f"items-sequence-legs/cards/{r['capture_id']:05d}-")
            assert hash64(h)and not pathlib.PurePosixPath(key).is_absolute()and'..'not in pathlib.PurePosixPath(key).parts
            if key in proofs:assert proofs[key]==h
            proofs[key]=h
        if'description_receipt'in r:
            v=r['description_receipt'];texts+=1
            assert v['field_name']=='Description'and v['state']=='OBSERVED_VALUE_RETAINED_PRIVATELY'
            assert hash64(v['sha256_utf8'])and v['bytes']>=v['characters']>=0
        if'description_display_lines_receipt'in r:
            v=r['description_display_lines_receipt'];layouts+=1
            assert v['state']=='OBSERVED_LAYOUT_RETAINED_PRIVATELY'and hash64(v['canonical_json_sha256'])
            assert v['canonical_utf8_bytes']>=2 and v['line_count']==len(v['line_character_lengths'])
    assert len(proofs)==packet['private_png_hash_checks']==351
    assert pages==coverage['saved_basic_pages']==153 and fields==507 and texts==layouts==6
    assert packet['field_export_policy']['private_text_receipts']==packet['field_export_policy']['private_display_layout_receipts']==6
    assert len({k for k in proofs if re.search(r'-basic-(?:top|\d+)\.png$',k)})==153
    by={r['ui_entry_ordinal']:r for r in rows}
    assert by[66]['fullName']=='Zaoan legs'and by[66]['properties']=={'Armor':'8','Protection':'physical +2%','Weight':'66.00 oz','Tradeable In Market':'yes','Body Position':'legs','Classification':'2'}
    union=read('reader-coverage-union.json')
    assert union['physical_ui_ordinals']=={'first':1,'last':66,'records':66,'gaps':[],'duplicates':[]}
    assert union['distinct_original_basic_png']==153 and union['original_proof_hashes_freshly_verified']==351
    assert union['index_Basic_pathset_equals_reader_Basic_proofset']is True
    assert union['manual_current_scope']==[1,66]and union['reused_without_manual_reread_scope']==[]
    assert union['trade_or_animation_complete']is union['overall_static_audit_complete']is False
    assert [r['records']for r in union['immutable_inputs']]==[20,20,20,5,1]
    assert union['immutable_inputs']==packet['immutable_reader_inputs']
    end=read('category-end-receipt.json');journal=read('tail-journal-receipt.json')
    assert end['source_receipt']['sha256']=='a1f9c769df9b9d0c6f2daf897586aab0dbb36d083c31c2c2321872965f3beaff'
    assert end['receipt']['cards']==end['receipt']['final_entry_ui_ordinal']==66
    assert end['receipt']['end_stability_checks_before']==end['receipt']['end_stability_checks_after']==2
    assert journal['source_receipt']['sha256']==end['receipt']['tail_journal_receipt']['sha256']=='a3af96c51c7c4f1eb953d4de0b1fcacd4e701e89cc10d2b0122128395838819a'
    j=journal['receipt'];assert j['stage']=='POST_END_CONFIRMED'and j['capture_attempts']==1
    assert j['source_index_sha_after']==union['index_receipt']['sha256']
    assert j['restored_scope_proof']==end['receipt']['restored_scope_proof']and len(j['end_proof_receipts'])==2
    assert j['restored_scope_proof']['state']=='QUALIFIED_RESTORED_SCOPE'
    comparison=read('source-candidate-comparison.json');assert comparison==generate()
    assert comparison['source_commit']=='9b7552e07f241a423a10d8304726c2de94489c74'
    assert len(comparison['items'])==66 and all(r['binding_status']=='UNQUALIFIED_NAME_CORRESPONDENCE'for r in comparison['items'])
    assert comparison['counts']['canonical_records']==34111 and comparison['counts']['selected_membership']==6090
    assert comparison['counts']['name_candidate_classes']=={'one_name_candidate':64,'no_name_candidate':2}
    actual={(r['ui_entry_ordinal'],label):(r,c,d)for r in comparison['items']for c in r['candidates']for label,d in c['fields'].items()if d['comparison']=='CANDIDATE_FIELD_DIFFERENT'}
    rereads=read('native-difference-reread-receipts.json')['rows']
    assert {(r['ordinal'],r['field'])for r in rereads}==set(actual)=={(28,'Mantra'),(29,'Armor'),(29,'Mantra'),(34,'Mantra')}
    for reread in rereads:
        key=(reread['ordinal'],reread['field']);r,c,d=actual[key];observation=by[reread['ordinal']]
        assert reread['capture_key']==r['capture_key']and reread['native_observed_literal']==d['client_literal']==observation['properties'][reread['field']]
        assert projected(d['canonical'],ROUTES[reread['field']][1])==reread['canonical_name_candidate_literal']
        assert c['selected_membership']is False and reread['selected_status']=='NOT_SELECTED'
        assert reread['native_proof']in observation['proofs']
    for state in ('UNKNOWN','NOT_APPLICABLE','CONFLICT'):
        snapshot=field({'container':{'state':state}},['container','capacity'])
        assert snapshot['resolution']=='ancestor_'+state
        assert compare_extended('0',snapshot,'integer')=='SOURCE_UNQUALIFIED_OR_UNKNOWN'
    assert compare_extended('0',{'resolution':'PRESENT','source':{'state':'KNOWN','value':0}},'integer')=='CANDIDATE_FIELD_EQUAL'
    assert compare_extended('no',{'resolution':'PRESENT','source':{'state':'KNOWN','value':False}},'market')=='CANDIDATE_FIELD_EQUAL'
    for label,record in read('field-presence-and-display-coverage.json')['Basic'].items():
        observed=[r['ui_entry_ordinal']for r in rows if label in r['properties']or label=='Description'and'description_receipt'in r]
        assert record['physical_ordinals']==observed
        assert record['not_emitted_ordinals']==[n for n in range(1,67)if n not in observed]
    validate_client_appearance(rows)
    for name,v in read('packet-integrity.json')['files'].items():
        raw=(HERE/name).read_bytes();assert len(raw)==v['bytes']and hashlib.sha256(raw).hexdigest()==v['sha256']
    for path in HERE.iterdir():
        assert path.suffix not in('.png','.zip','.crt','.key')
        if path.suffix=='.json':assert not any(v in path.read_text(encoding='utf-8')for v in('/home/mole/','Bearer ','bridge-access.json','server.key'))
    print(json.dumps({'passed':True,'records':66,'Basic_PNG':153,'card_proofs':351,'non_narrative_literals':507,'Description_receipts':6,'layout_receipts':6,'client_only_appearance_candidates':66,'known_sprites':139,'recomputed_sprite_slots':142,'canonical_candidate_differences':4,'overall':'PARTIAL'}))
if __name__=='__main__':main()
