"""Validate ordered Helmets Basic custody and reproduce bounded source candidates."""
import collections,hashlib,json,pathlib,re
from compare_sources import generate,field,compare_extended
HERE=pathlib.Path(__file__).resolve().parent
def read(name):return json.loads((HERE/name).read_text(encoding='utf-8'))
def hash64(v):return isinstance(v,str) and re.fullmatch('[0-9a-f]{64}',v)
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
    for name,v in read('packet-integrity.json')['files'].items():
        raw=(HERE/name).read_bytes();assert len(raw)==v['bytes']and hashlib.sha256(raw).hexdigest()==v['sha256']
    for path in HERE.iterdir():
        assert path.suffix not in('.png','.zip','.crt','.key')
        if path.suffix=='.json':
            text=path.read_text(encoding='utf-8')
            assert not any(v in text for v in('/home/mole/','Bearer ','bridge-access.json','server.key'))
    print(json.dumps({'passed':True,'records':128,'non_narrative_literals':899,'private_proof_hashes':652,'Basic_PNG':268,'Description_receipts':28,'layout_receipts':10,'candidate_classes':comparison['counts']['name_candidate_classes'],'NPC_trade_animation_semantic_parity':'PARTIAL','overall':'PARTIAL'}))
if __name__=='__main__':main()
