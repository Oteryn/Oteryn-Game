"""Validate ordered Helmets Basic custody and reproduce bounded source candidates."""
import collections,hashlib,json,pathlib,re
from compare_sources import generate,field,compare_extended
HERE=pathlib.Path(__file__).resolve().parent
def read(name):return json.loads((HERE/name).read_text(encoding='utf-8'))
def hash64(v):return isinstance(v,str) and re.fullmatch('[0-9a-f]{64}',v)
def validate_client_appearance(observations):
    receipt=read('client-appearance-receipt.json')
    assert set(receipt)=={'schema','classification','source_receipt','current_DAT_sha256','private_program_receipt','decoder_sources','counts','sheet_checks','verification','rows','boundaries'}
    assert receipt['schema']=='OTERYN_HELMETS_CLIENT_APPEARANCE_RECEIPT/v1'
    assert receipt['classification']=='DERIVED_CLIENT_NAMESPACE_CANDIDATES'
    source=receipt['source_receipt'];assert set(source)=={'filename','bytes','sha256'}
    assert source['sha256']=='6add45e583f17fd6e6a1f75a32a882bf6791fbd291e585e84d7eefca8913d6ea'
    assert source['bytes']>0 and source['filename']=='items-helmets-appearance-binding-takeover-20261113.json'
    assert receipt['current_DAT_sha256']=='aa086346daa9234d721e4170848095ad83f3ee9d28ea9e62c1bd06216016174e'
    program=receipt['private_program_receipt'];assert set(program)=={'filename','sha256'}
    assert program['sha256']=='dfe8ed1eae2520b57d94a34e41158a96c10a9c383c4b27b676869853b6c3a024'
    assert len(receipt['decoder_sources'])==8
    for decoder in receipt['decoder_sources']:
        assert set(decoder)=={'repository_path','commit','sha256','git_blob_sha1'}
        assert decoder['commit']=='83be72ad403ebc221dadba4b161c85fa1d47219a'
        assert hash64(decoder['sha256'])and re.fullmatch('[0-9a-f]{40}',decoder['git_blob_sha1'])
        assert decoder['repository_path'].startswith('tools/')and'..'not in pathlib.PurePosixPath(decoder['repository_path']).parts
    assert len(receipt['sheet_checks'])==56
    for sheet in receipt['sheet_checks']:
        assert set(sheet)=={'file','packed_sha256','packed_size','packed_manifest_matches','decoded_bmp_sha256','filename_hash_matches','bpp','compression'}
        assert hash64(sheet['packed_sha256'])and hash64(sheet['decoded_bmp_sha256'])
        assert sheet['file']=='sprites-'+sheet['decoded_bmp_sha256']+'.bmp.lzma'
        assert sheet['packed_manifest_matches']is sheet['filename_hash_matches']is True
        assert sheet['packed_size']>0 and sheet['bpp']==32 and sheet['compression']in(0,3)
    verification=receipt['verification']
    assert verification=={'private_input_files_hash_verified':135,'decoder_files_hash_verified':8,'sprite_slot_metrics_independently_recomputed':299,'named_candidate_sets_reconciled_to_census':128,'projection':'EXPLICIT_KEY_WHITELIST_ONLY','descriptions':'HASH_LENGTH_RECEIPTS_ONLY'}
    def text_seal(value):
        if value is None:return
        assert set(value)=={'state','sha256_utf8','bytes','characters'}
        assert value['state']=='RETAINED_PRIVATELY'and hash64(value['sha256_utf8'])
        assert value['bytes']>=value['characters']>=0
    rows=receipt['rows'];assert len(rows)==128
    assert [r['ui_entry_ordinal']for r in rows]==list(range(1,129))
    classes=collections.Counter();sprites=set();slots=0
    for row,observation in zip(rows,observations):
        assert set(row)=={'ui_entry_ordinal','capture_key','status','retained_appearance_object_ids','server_id','fullName','icon_proof','observed_description_receipt','candidate_evaluations'}
        assert row['capture_key']==observation['capture_key']and row['fullName']==observation['fullName']
        assert row['server_id']is None
        icon=row['icon_proof'];assert set(icon)=={'private_source_key','sha256'}
        assert icon['private_source_key']==f"items-sequence-helmets-hats/cards/{row['ui_entry_ordinal']:05d}-icon.png"
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
    assert slots==299 and len(sprites)==272
    assert rows[28]['retained_appearance_object_ids']==[13995,14258]
    assert rows[54]['retained_appearance_object_ids']==[5460,11585]
    assert rows[78]['retained_appearance_object_ids']==[139]and rows[79]['retained_appearance_object_ids']==[875]
    assert dict(classes)=={'unique_client_appearance_candidate':126,'client_appearance_candidate_set':2}
    assert receipt['counts']=={**dict(classes),'unresolved':0,'observations':128,'decoded_sheets':56,'decoded_unique_sprites':272}
    assert isinstance(receipt['boundaries'],list)and all(isinstance(b,str)for b in receipt['boundaries'])
def main():
    p=read('observations-1-128.json');rows=p['items'];coverage=p['coverage']
    assert p['schema']=='OTERYN_COMPACT_PRIVATE_ITEMS_OBSERVATIONS/v1'
    assert p['classification']=='PROVEN' and p['category']=='Helmets and Hats'
    assert p['source_receipt']['sha256']=='4e0ebece746ce49d91908a13af83087306f179073d8c11fb12b93bc2698eded2'
    assert [r['ui_entry_ordinal']for r in rows]==list(range(1,129))
    assert len({r['capture_key']for r in rows})==128
    assert coverage['category_end_observed']is True
    assert coverage['trade_panels_qualified']is coverage['animation_qualified']is False
    proofs={};texts=layouts=pages=fields=0
    allowed={'capture_key','capture_id','ui_entry_ordinal','fullName','name_hash_is_item_identity','properties','proofs','saved_basic_pages','description_receipt','description_display_lines_receipt'}
    for r in rows:
        assert set(r)<=allowed
        assert r['capture_key']==f"items-sequence-helmets-hats:{r['capture_id']}"
        assert r['name_hash_is_item_identity']is False
        assert isinstance(r['fullName'],str)and r['fullName']
        assert 'Description'not in r['properties']and'description_display_lines'not in r
        assert all(isinstance(v,str)and v for v in r['properties'].values())
        fields+=len(r['properties']);pages+=r['saved_basic_pages']
        assert r['saved_basic_pages']==sum(bool(re.search(r'-basic-(?:top|\d+)\.png$',v['private_source_key']))for v in r['proofs'])
        for proof in r['proofs']:
            key=proof['private_source_key'];h=proof['sha256']
            assert key.startswith(f"items-sequence-helmets-hats/cards/{r['capture_id']:05d}-")
            assert hash64(h)and not pathlib.PurePosixPath(key).is_absolute()and'..'not in pathlib.PurePosixPath(key).parts
            if key in proofs:assert proofs[key]==h
            proofs[key]=h
        if'description_receipt'in r:
            v=r['description_receipt'];texts+=1
            assert v['state']=='OBSERVED_VALUE_RETAINED_PRIVATELY'and v['field_name']=='Description'
            assert hash64(v['sha256_utf8'])and v['bytes']>=v['characters']>=0
        if'description_display_lines_receipt'in r:
            v=r['description_display_lines_receipt'];layouts+=1
            assert v['state']=='OBSERVED_LAYOUT_RETAINED_PRIVATELY'
            assert hash64(v['canonical_json_sha256'])and v['canonical_utf8_bytes']>=2
            assert v['line_count']==len(v['line_character_lengths'])
    assert len(proofs)==p['private_png_hash_checks']==652
    assert pages==coverage['saved_basic_pages']==268 and fields==899
    assert texts==p['field_export_policy']['private_text_receipts']==28
    assert layouts==p['field_export_policy']['private_display_layout_receipts']==10
    assert len({k for k in proofs if re.search(r'-basic-(?:top|\d+)\.png$',k)})==268
    by={r['ui_entry_ordinal']:r for r in rows}
    assert by[10]['fullName']=='brass helmet'and by[10]['properties']['Armor']=='3'
    assert by[79]['fullName']==by[80]['fullName']=='mining helmet'
    assert by[79]['capture_key']!=by[80]['capture_key']
    assert by[79]['description_receipt']['sha256_utf8']!=by[80]['description_receipt']['sha256_utf8']
    assert by[128]['fullName']=='Zaoan helmet'and by[128]['properties']['Weight']=='66.00 oz'
    union=read('reader-coverage-union.json')
    assert union['physical_ui_ordinals']=={'first':1,'last':128,'records':128,'gaps':[],'duplicates':[]}
    assert union['distinct_original_basic_png']==268 and union['original_proof_hashes_freshly_verified']==652
    assert union['index_Basic_pathset_equals_reader_Basic_proofset']is True
    assert union['manual_current_scope']==[51,128]and union['reused_without_manual_reread_scope']==[1,50]and union['native_reread_exception']==[10]
    assert union['trade_or_animation_complete']is union['overall_static_audit_complete']is False
    correction=read('reader-correction-receipt.json')
    assert correction['source_receipt']['sha256']=='f10548cee67cdd8d42f6828b1743de97970f62b0344af230f94d924c375e9487'
    assert correction['receipt']['correction']=={'field':'properties.Armor','old':'7','new':'3'}
    end=read('category-end-receipt.json')
    assert end['source_receipt']['sha256']=='84fb3e0c5a28bdfdd814a35751d22aa47a2ff76f2ae3f268552021a7cd14bf63'
    assert end['receipt']['cards']==end['receipt']['final_entry_ui_ordinal']==128
    assert end['receipt']['end_stability_checks_before']==end['receipt']['end_stability_checks_after']==2
    assert hash64(end['receipt']['tail_journal_receipt']['sha256'])
    assert end['receipt']['restored_scope_proof']['state']=='QUALIFIED_RESTORED_SCOPE'
    journal=read('tail-journal-receipt.json')
    assert journal['source_receipt']['sha256']==end['receipt']['tail_journal_receipt']['sha256']
    assert journal['receipt']['stage']=='POST_END_CONFIRMED'and journal['receipt']['capture_attempts']==1
    assert journal['receipt']['source_index_sha_after']==union['index_receipt']['sha256']
    assert journal['receipt']['restored_scope_proof']==end['receipt']['restored_scope_proof']
    assert len(journal['receipt']['end_proof_receipts'])==2
    comparison=read('source-candidate-comparison.json');assert comparison==generate()
    assert len(comparison['items'])==128
    assert all(r['binding_status']=='UNQUALIFIED_NAME_CORRESPONDENCE'for r in comparison['items'])
    assert comparison['counts']['canonical_records']==34111 and comparison['counts']['selected_membership']==6090
    # Preserve zero, false, N/A and unknown ancestors rather than silently coercing them.
    for state in ('UNKNOWN','NOT_APPLICABLE','CONFLICT'):
        snap=field({'container':{'state':state}},['container','capacity'])
        assert snap['resolution']=='ancestor_'+state
        assert compare_extended('0',snap,'integer')=='SOURCE_UNQUALIFIED_OR_UNKNOWN'
    assert compare_extended('0',{'resolution':'PRESENT','source':{'state':'KNOWN','value':0}},'integer')=='CANDIDATE_FIELD_EQUAL'
    assert compare_extended('no',{'resolution':'PRESENT','source':{'state':'KNOWN','value':False}},'market')=='CANDIDATE_FIELD_EQUAL'
    presence=read('field-presence-and-display-coverage.json')['Basic']
    for label,record in presence.items():
        observed=[r['ui_entry_ordinal']for r in rows if label in r['properties']or label=='Description'and'description_receipt'in r]
        assert record['physical_ordinals']==observed
        assert record['not_emitted_ordinals']==[n for n in range(1,129)if n not in observed]
    validate_client_appearance(rows)
    for name,v in read('packet-integrity.json')['files'].items():
        raw=(HERE/name).read_bytes();assert len(raw)==v['bytes']and hashlib.sha256(raw).hexdigest()==v['sha256']
    for path in HERE.iterdir():
        assert path.suffix not in('.png','.zip','.crt','.key')
        if path.suffix=='.json':
            text=path.read_text(encoding='utf-8')
            assert not any(v in text for v in('/home/mole/','Bearer ','bridge-access.json','server.key'))
    print(json.dumps({'passed':True,'records':128,'non_narrative_literals':899,'private_proof_hashes':652,'Basic_PNG':268,'Description_receipts':28,'layout_receipts':10,'candidate_classes':comparison['counts']['name_candidate_classes'],'NPC_trade_animation_semantic_parity':'PARTIAL','overall':'PARTIAL'}))
if __name__=='__main__':main()
