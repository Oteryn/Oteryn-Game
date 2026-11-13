"""Check ongoing compact CP facts, corrected capture mapping and pinned comparison."""
import collections,hashlib,json,pathlib,re
from compare_sources import generate
HERE=pathlib.Path(__file__).resolve().parent


def read(name):return json.loads((HERE/name).read_text())


def main():
    observations=[];proofs={};private_text=0;private_layouts=0
    for path in sorted(HERE.glob('observations-*.json')):
        packet=json.loads(path.read_text());rows=packet['items'];coverage=packet['coverage']
        assert packet['category']=='Creature Products'
        assert len(rows)==coverage['records']
        assert sorted(row['ui_entry_ordinal']for row in rows)==list(range(coverage['first_physical_ui_ordinal'],coverage['last_physical_ui_ordinal']+1))
        assert coverage['missing_physical_ordinals_in_cutoff']==[]
        assert coverage['category_end_observed']is False and coverage['trade_panels_qualified']is False
        assert coverage['animation_qualified']is False
        assert re.fullmatch('[0-9a-f]{64}',packet['source_receipt']['sha256'])
        for row in rows:
            assert row['capture_id']==int(row['capture_key'].rsplit(':',1)[1])
            assert row['name_hash_is_item_identity']is False
            assert 'Description'not in row['properties']
            for proof in row['proofs']:
                key=proof['private_source_key'];h=proof['sha256']
                assert key.startswith(f"items-sequence-creature-products/cards/{row['capture_id']:05d}-")
                assert '..'not in pathlib.PurePosixPath(key).parts and not pathlib.PurePosixPath(key).is_absolute()
                assert re.fullmatch('[0-9a-f]{64}',h)
                if key in proofs:assert proofs[key]==h
                proofs[key]=h
            assert 'description_display_lines'not in row
            if 'description_display_lines_receipt'in row:
                layout=row['description_display_lines_receipt'];private_layouts+=1
                assert layout['field_name']=='description_display_lines'and layout['state']=='OBSERVED_LAYOUT_RETAINED_PRIVATELY'
                assert re.fullmatch('[0-9a-f]{64}',layout['canonical_json_sha256'])
                assert layout['line_count']>=0 and layout['canonical_utf8_bytes']>=2
            if 'description_receipt'in row:
                receipt=row['description_receipt'];private_text+=1
                assert receipt['field_name']=='Description'and receipt['state']=='OBSERVED_VALUE_RETAINED_PRIVATELY'
                assert re.fullmatch('[0-9a-f]{64}',receipt['sha256_utf8'])
                assert receipt['bytes']>=receipt['characters']>=0
        assert sum('description_receipt'in row for row in rows)==packet['field_export_policy']['private_text_receipts']
        assert len({proof['private_source_key']for row in rows for proof in row['proofs']})==packet['private_png_hash_checks']
        if 'reader_batches'in packet:
            assert sum(batch['records']for batch in packet['reader_batches'])==len(rows)
            assert packet['review_reuse_policy']['previous_immutable_native_reads_reused']+packet['review_reuse_policy']['new_native_reads_this_resume']==len(rows)
            assert all(re.fullmatch('[0-9a-f]{64}',batch['sha256'])for batch in packet['reader_batches'])
        observations+=rows
    observations.sort(key=lambda row:row['ui_entry_ordinal']);ordinals=[row['ui_entry_ordinal']for row in observations]
    assert ordinals==list(range(135,max(ordinals)+1))and len({row['capture_key']for row in observations})==len(observations)
    by={row['ui_entry_ordinal']:row for row in observations}
    assert by[197]['capture_id']==246 and by[197]['fullName']=='crown'
    assert by[197]['properties']['Weight']=='0.90 oz'and by[197]['properties']['Tradeable In Market']=='yes'
    assert by[237]['capture_id']==247 and by[237]['fullName']=='deepling breaktime snack'
    assert by[237]['properties']['Weight']=='2.40 oz'and by[237]['properties']['Tradeable In Market']=='yes'
    assert by[163]['fullName']==by[164]['fullName']=='chitinous mouth'
    assert by[222]['fullName']==by[223]['fullName']=='dark bell'
    if 500 in by:
        assert by[352]['fullName']==by[353]['fullName']=='frozen claw'
        assert by[434]['fullName']==by[435]['fullName']
        assert by[315]['properties']['Armor']=='1'and by[315]['properties']['Body Position']=='head'
        assert by[315]['properties']['Classification']=='4'and by[315]['properties']['Weight']=='8.50 oz'
        assert by[352]['properties']['Body Position']=='finger'and by[393]['properties']['Body Position']=='extra slot'
    if 980 in by:
        assert len(observations)==846 and len(proofs)==3386 and private_text==113 and private_layouts==97
        assert by[594]['fullName']=='nettle spit'and by[594]['properties']['Weight']=='0.00 oz'
        assert by[572]['properties']['Body Position']=='extra slot'
        assert by[624]['properties']['Armor']=='1'and by[624]['properties']['Body Position']=='body'
        assert by[725]['properties']['Attack']=='5'and by[725]['properties']['Defence']=='1'
        assert by[729]['properties']['Attack']=='12'and by[729]['properties']['Defence']=='10'
        assert by[729]['properties']['Classification']=='3'
        assert by[872]['fullName']==by[873]['fullName']
        assert by[980]['fullName']=='zaogun shoulderplates'and by[980]['properties']['Weight']=='1.03 oz'
        assert by[980]['properties']['Tradeable In Market']=='yes'
        union=read('reader-coverage-union.json');end=read('category-end-receipt.json')
        assert union['physical_ui_ordinals']=={'first':1,'last':980,'records':980,'gaps':[],'duplicates':[]}
        assert union['saved_basic_pages']==union['distinct_original_basic_png']==984
        assert union['original_proof_hashes_freshly_verified']==3790
        assert union['count_correction']['understated_by']==2 and union['count_correction']['correct_basic_pages']==984
        assert union['source_receipt']['sha256']=='bb3fd2f70edb815f1ee73842859929f58f275861ae1de03cf203e6d3bb48a941'
        assert sum(row['records']for row in union['immutable_inputs'])==980
        assert sum(row['proofs']for row in union['immutable_inputs'])==3790
        assert union['immutable_inputs'][0]['sha256']=='9b4dd2cc5fba5fe15f94d74be6d02c418bc0284da10ffc645157c7be0f137b71'
        packets={packet['coverage']['first_physical_ui_ordinal']:packet for packet in [json.loads(p.read_text())for p in sorted(HERE.glob('observations-*.json'))]}
        for source in union['immutable_inputs'][1:]:
            assert packets[source['first_physical_ui_ordinal']]['source_receipt']['sha256']==source['sha256']
        assert [r['extra_page_ordinals']for r in union['original_capture_index_receipt']['ranges']]==[[113,116],[],[315],[729]]
        basic_proofs={key for key in proofs if re.search(r'-basic-(?:top|\d+)\.png$',key)}
        assert len(basic_proofs)==848 and 848+136==984
        assert end['category_end_observed']is True and end['captured_records']==980
        assert end['captured_physical_ui_range']==[1,980]
        assert end['end_stability_checks_before']==end['end_stability_checks_after']==2
        assert end['final_entry_ui_ordinal']==end['final_capture_id']==980
        assert len(end['end_proof_receipts'])==2
        assert all(re.fullmatch('[0-9a-f]{64}',p['sha256'])for p in end['end_proof_receipts'])
    repair=read('gap-correction.json');assert repair['repairs']==[{key:by[n][key]for key in ('ui_entry_ordinal','capture_id','fullName','properties')}for n in(197,237)]
    for name,receipt in read('packet-integrity.json')['files'].items():
        raw=(HERE/name).read_bytes();assert len(raw)==receipt['bytes']and hashlib.sha256(raw).hexdigest()==receipt['sha256'],name
    comparison=read('source-candidate-comparison.json');assert comparison==generate()
    assert len(comparison['items'])==len(observations)
    assert all(row['binding_status']=='UNQUALIFIED_NAME_CORRESPONDENCE'for row in comparison['items'])
    backlog=read('uncompared-field-backlog.json')
    expected=collections.defaultdict(list)
    for item in observations:
        for name in item['properties']:
            if name not in('Weight','Tradeable In Market'):expected[name].append(item['ui_entry_ordinal'])
    assert backlog['fields']==[{'field':name,'observations':len(ordinals),'physical_ordinals':ordinals,'source_route_status':'NOT_YET_QUALIFIED_FOR_COMPARISON'}for name,ordinals in sorted(expected.items())]
    assert backlog['private_narrative_receipts']==private_text
    for path in HERE.iterdir():
        assert path.suffix not in('.png','.apng','.zip','.crt','.key')
        if path.suffix=='.json':assert not any(text in path.read_text()for text in('/home/mole/','Bearer ','bridge-access.json','server.key','192.168.1.8'))
    print(json.dumps({'passed':True,'physical_ordinal_cutoff':[min(ordinals),max(ordinals)],'records':len(observations),
        'private_png_receipts':len(proofs),'observed_private_description_receipts':private_text,'observed_private_display_layout_receipts':private_layouts,'uncompared_literal_fields':sorted(expected),
        'gap_repairs_verified':[197,237],'duplicate_name_rows_preserved':[[163,164],[222,223]],
        'source_receipts':len(comparison['source_receipts']),'ordered_basic_reader_category_complete':980 in by,'global_basic_pages':984 if 980 in by else None,'global_card_proof_references':3790 if 980 in by else None,'overall_audit':'PARTIAL'}))


if __name__=='__main__':main()
