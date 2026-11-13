"""Validate full ordered Food Basic custody and bounded source candidates."""
import collections,hashlib,json,pathlib,re
from compare_sources import generate
HERE=pathlib.Path(__file__).resolve().parent

def read(name):return json.loads((HERE/name).read_text())
def hash64(value):return isinstance(value,str)and re.fullmatch('[0-9a-f]{64}',value)

def main():
    packet=read('observations-1-169.json');rows=packet['items'];coverage=packet['coverage']
    assert packet['category']=='Food'and packet['classification']=='PROVEN'
    assert packet['source_receipt']['sha256']=='64f29437dbbac3c95f9e22b6137c066ba27bb567f920b9453e224726bfc902c6'
    assert [r['ui_entry_ordinal']for r in rows]==list(range(1,170))
    assert len({r['capture_key']for r in rows})==169
    assert coverage['first_physical_ui_ordinal']==1 and coverage['last_physical_ui_ordinal']==169 and coverage['records']==169
    assert coverage['category_end_observed']is True and coverage['missing_physical_ordinals_in_cutoff']==[]
    assert coverage['trade_panels_qualified']is False and coverage['animation_qualified']is False
    proofs={};texts=layouts=pages=0
    allowed={'capture_key','capture_id','ui_entry_ordinal','fullName','name_hash_is_item_identity','properties','proofs','saved_basic_pages','description_receipt','description_display_lines_receipt'}
    for r in rows:
        assert set(r)<=allowed
        assert r['capture_key']==f"items-sequence-food:{r['capture_id']}"
        assert r['name_hash_is_item_identity']is False and isinstance(r['fullName'],str)and r['fullName']
        assert set(r['properties'])=={'Weight','Tradeable In Market'}
        assert 'Description'not in r['properties']and'description_display_lines'not in r
        assert r['saved_basic_pages']==1;pages+=r['saved_basic_pages']
        for proof in r['proofs']:
            key=proof['private_source_key'];h=proof['sha256']
            assert key.startswith(f"items-sequence-food/cards/{r['capture_id']:05d}-")and hash64(h)
            assert not pathlib.PurePosixPath(key).is_absolute()and'..'not in pathlib.PurePosixPath(key).parts
            if key in proofs:assert proofs[key]==h
            proofs[key]=h
        if 'description_receipt'in r:
            receipt=r['description_receipt'];texts+=1
            assert receipt['field_name']=='Description'and receipt['state']=='OBSERVED_VALUE_RETAINED_PRIVATELY'
            assert hash64(receipt['sha256_utf8'])and receipt['bytes']>=receipt['characters']>=0
        if 'description_display_lines_receipt'in r:
            receipt=r['description_display_lines_receipt'];layouts+=1
            assert receipt['field_name']=='description_display_lines'and receipt['state']=='OBSERVED_LAYOUT_RETAINED_PRIVATELY'
            assert hash64(receipt['canonical_json_sha256'])and receipt['canonical_utf8_bytes']>=2
            assert receipt['line_count']==len(receipt['line_character_lengths'])
    assert len(proofs)==packet['private_png_hash_checks']==676 and pages==coverage['saved_basic_pages']==169
    assert texts==layouts==39
    assert texts==packet['field_export_policy']['private_text_receipts']and layouts==packet['field_export_policy']['private_display_layout_receipts']
    assert len({k for k in proofs if re.search(r'-basic-(?:top|\d+)\.png$',k)})==169
    by={r['ui_entry_ordinal']:r for r in rows}
    assert by[28]['fullName']==by[29]['fullName']
    assert by[32]['fullName']==by[33]['fullName']and by[32]['properties']['Weight']=='2.40 oz'and by[33]['properties']['Weight']=='0.40 oz'
    assert len({by[n]['fullName']for n in range(45,50)})==1
    assert len({by[n]['fullName']for n in range(87,91)})==1
    assert [by[n]['properties']['Weight']for n in range(87,91)]==['1.00 oz','1.00 oz','1.00 oz','0.70 oz']
    assert len({by[n]['description_receipt']['sha256_utf8']for n in range(87,91)})==4
    assert by[146]['fullName']==by[147]['fullName']
    assert by[158]['fullName']!=by[159]['fullName']
    assert by[169]['fullName']=='zaoan sauce'and by[169]['properties']['Weight']=='4.00 oz'and by[169]['properties']['Tradeable In Market']=='yes'
    assert 'description_receipt'in by[169]
    union=read('reader-coverage-union.json');end=read('category-end-receipt.json')
    assert union['physical_ui_ordinals']=={'first':1,'last':169,'records':169,'gaps':[],'duplicates':[]}
    assert union['distinct_original_basic_png']==169 and union['original_proof_hashes_freshly_verified']==676
    assert union['private_Description_values']==union['private_display_layouts']==39
    assert union['index_Basic_pathset_equals_reader_Basic_proofset']is True
    assert union['historical_pending_reader_receipts_preserved']is True
    assert union['trade_or_animation_complete']is False and union['overall_static_audit_complete']is False
    assert [r['records']for r in union['immutable_inputs']]==[168,1]
    assert union['immutable_inputs']==packet['immutable_reader_inputs']
    assert union['immutable_inputs'][1]['sha256']=='e92c600ef78fadd6ebefda8386cbc2ac98e12c362be1a595cf5ec135776d7ae9'
    assert end['captured_records']==end['final_entry_ui_ordinal']==end['final_capture_id']==169 and end['captured_physical_ui_range']==[1,169]
    assert end['source_receipt']['sha256']=='eb121888770a35d0f863a0960d5cef5efe0a4601b26ae6bcfd9ade5181467540'
    assert end['end_stability_checks_before']==end['end_stability_checks_after']==2
    assert len(end['end_proof_receipts'])==2 and all(hash64(x['sha256'])for x in end['end_proof_receipts'])
    journal=end['dynamic_capture_journal'];scope=journal['restored_scope']
    assert journal['source_receipt']['sha256']=='a34086ed34e7861448847ef9a78f0da8aaf91a58746348dc4c59c2c9190fe8a3'
    assert journal['stage']=='POST_END_CONFIRMED'and journal['capture_attempts']==1 and journal['expected_next']==169
    assert journal['source_index_sha_after']==union['original_index_receipt']['sha256']
    assert journal['native_source_sha256']==scope['native_source_sha256']=='a67da64696153f1e3540d70706ff4b59c7ec3d87b46b9c4c7a8dfb742c9a2903'
    assert hash64(scope['actual_fresh_frame_sha256'])and scope['raw_full_frame_equal_not_required']is True
    assert scope['physical_offset_pixels']==5960 and scope['stable_end_checks']==2 and scope['geometry']==[698,536]
    assert end['observed_final_row']=={'physical_ui_ordinal':169,'source_y':660,'source_offset_pixels':5960,'physical_row_pitch_pixels':36,'identity_claim':False}
    root=end['root_closed_world_qualification'];assert root['source_receipt']['sha256']=='149a58e9115b62e0de4f6b8cf6946860b2df4be1c65ec9bcfda4a176378210cf'
    assert root['capture_attempts']==1 and root['cards']==169 and root['historical_native_reader_final_pending']is True
    assert hash64(root['closed_world_proof_receipt']['sha256'])and isinstance(root['actual_frame_ns'],int)
    assert end['historical_completion_reader_qualification']=='PENDING_SNAPSHOT_PRESERVED_UNCHANGED'
    comparison=read('source-candidate-comparison.json');assert comparison==generate()and len(comparison['items'])==169
    assert all(r['binding_status']=='UNQUALIFIED_NAME_CORRESPONDENCE'for r in comparison['items'])
    backlog=read('uncompared-field-backlog.json');assert backlog['fields']==[]and backlog['private_narrative_receipts']==39
    for name,receipt in read('packet-integrity.json')['files'].items():
        raw=(HERE/name).read_bytes();assert len(raw)==receipt['bytes']and hashlib.sha256(raw).hexdigest()==receipt['sha256']
    for path in HERE.iterdir():
        assert path.suffix not in('.png','.zip','.crt','.key')
        if path.suffix=='.json':assert not any(x in path.read_text()for x in('/home/mole/','Bearer ','bridge-access.json','server.key','192.168.1.8'))
    print(json.dumps({'passed':True,'records':169,'saved_basic_pages':169,'private_proof_hashes':676,'private_description_receipts':39,'private_layout_receipts':39,'source_receipts':len(comparison['source_receipts']),'dynamic_tail_capture_attempts':1,'root_fresh_world_receipt_verified':True,'NPC_trade_animation_semantic_parity':'PARTIAL','overall':'PARTIAL'}))

if __name__=='__main__':main()
