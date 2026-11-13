"""Validate exact Gold Basic absence/display states and bounded recovery custody."""
import hashlib,json,pathlib,re
from compare_sources import generate
HERE=pathlib.Path(__file__).resolve().parent

def read(name):return json.loads((HERE/name).read_text())
def h64(v):return isinstance(v,str)and re.fullmatch('[0-9a-f]{64}',v)

def main():
    packet=read('observations-1-3.json');rows=packet['items'];proofs={}
    assert packet['category']=='Gold'and packet['source_receipt']['sha256']=='c735fe9b3d4e73b0e10e653dc1d97b50ea85b2667ea635792260f6e4373035a5'
    assert [r['ui_entry_ordinal']for r in rows]==[1,2,3]and len({r['capture_key']for r in rows})==3
    assert [r['fullName']for r in rows]==['crystal coin','gold coin','platinum coin']
    assert [r['properties']for r in rows]==[{'Weight':'0.10 oz'},{'Weight':'0.01 oz'},{'Weight':'0.10 oz'}]
    expected_presence={'Weight':'OBSERVED_VALUE','Description':'ABSENT_FROM_OBSERVED_BASIC_FIELDS','Tradeable In Market':'ABSENT_FROM_OBSERVED_BASIC_FIELDS'}
    for r in rows:
        assert r['capture_key']==f"items-sequence-gold:{r['capture_id']}"and r['name_hash_is_item_identity']is False
        assert r['not_displayed_in_basic']==['Description','Tradeable In Market']and r['basic_field_presence']==expected_presence
        assert 'description_receipt'not in r and'description_display_lines_receipt'not in r and r['saved_basic_pages']==1
        for proof in r['proofs']:
            key=proof['private_source_key'];value=proof['sha256'];assert h64(value)
            assert key.startswith(f"items-sequence-gold/cards/{r['capture_id']:05d}-")and not pathlib.PurePosixPath(key).is_absolute()and'..'not in pathlib.PurePosixPath(key).parts
            if key in proofs:assert proofs[key]==value
            proofs[key]=value
        display=r['display_facts'];assert display['average_market_price_display_number']=='0'
        assert display['display_suffix']=='gold-colored coin glyph'and display['loot_value_source_selected']=='NPC Buy Value'
        assert isinstance(display['skip_when_quick_looting_checked'],bool)
    assert [r['display_facts']['resulting_value_display_number']for r in rows]==['10,000','1','100']
    assert [r['display_facts']['skip_when_quick_looting_checked']for r in rows]==[False,True,False]
    assert len(proofs)==packet['private_png_hash_checks']==12
    assert len({k for k in proofs if re.search(r'-basic-(?:top|\d+)\.png$',k)})==3
    assert packet['coverage']['category_end_observed']is True and packet['coverage']['missing_physical_ordinals_in_cutoff']==[]
    assert packet['coverage']['animation_qualified']is False and packet['coverage']['full_trade_lists_qualified']is False
    assert packet['field_export_policy']['private_text_receipts']==packet['field_export_policy']['private_display_layout_receipts']==0
    presence=read('field-presence-and-display-coverage.json');assert presence['Basic']['Tradeable In Market']['state']=='ABSENT_FROM_OBSERVED_BASIC_FIELDS'
    assert presence['Basic']['Description']['state']=='ABSENT_FROM_OBSERVED_BASIC_FIELDS'
    assert {r['field']for r in presence['display_fields']}==set(rows[0]['display_facts'])
    assert all(r['comparison']=='UNCOMPARED_CLIENT_DISPLAY_ONLY'for r in presence['display_fields'])
    trade=read('trade-initial-panel-coverage.json');assert trade['source_receipt']['sha256']=='d523e5f4b7892be1c7ff0313d16e0eb7e4582979413e0b4904458fee3f94a537'
    assert trade['initial_panels']==trade['observed_empty_panels']==6 and trade['observed_nonempty_panels']==0
    assert trade['global_NPC_absence_qualified']is False and trade['transactions_or_conversations']==0
    assert trade['original_Basic_proofs_freshly_verified']==3
    for r in trade['items']:
        assert r['panels']=={'Sell To':'OBSERVED_EMPTY_PANEL','Buy From':'OBSERVED_EMPTY_PANEL'}
        assert proofs[r['proof']['private_source_key']]==r['proof']['sha256']
    end=read('category-end-receipt.json');assert end['source_receipt']['sha256']=='02fb4d410e664eabe75ed9ff38299e1c372d3667405dc7c30eb344bcdcb45923'
    assert end['captured_records']==3 and end['ordinals']==[1,2,3]
    assert end['origin_stability_checks']==end['end_stability_checks_before']==end['end_stability_checks_after']==2
    assert end['all_entries_visible_in_origin_frame']is True and end['no_scrollbar_geometry_inferred']is True
    assert len(end['end_proof_receipts'])==2 and all(h64(r['sha256'])for r in end['end_proof_receipts'])
    v2=end['V2_recovery'];assert v2['journal_stage']=='POST_END_CONFIRMED'and v2['cards']==v2['capture_attempts']==3 and v2['gaps']==[]
    assert v2['journal_receipt']['sha256']=='39bff569b689e00620f2ca1420ca11c590a6ba14275311251f4341a91fef8bc7'
    assert v2['source_receipt']['sha256']=='b3dcd2c6e981dd5f7c629002d06129603e93913a8e01d48a8983d02263f80f41'
    assert v2['status']=='GOLD_CAPTURE_COMPLETE_KNOWN_WORLD_VERIFIED'and h64(v2['closed_world_proof_receipt']['sha256'])
    assert v2['historical_reader_qualification_pending']is True
    repair=end['private_local_collector_repair'];assert repair['source_receipt']['sha256']=='e23b5167024647089d5df5f4fe15e6488356e24d93f6a6523a1a841eea0298ab'
    assert repair['old_sha256']=='0cf7a4cea17d67f00e3736415f5c4f6d039cec47c103dba33bab3e8a5c408d91'
    assert repair['new_sha256']=='83f7639bebf13496740b0f7e85a1b5c69d5f337b215139d0b135092396258432'
    assert repair['prior_index_exists']is False and repair['prior_card_pngs']==0 and repair['compile']=='PASS'
    assert repair['historical_actual_recovery']=='PENDING'and end['historical_reader_qualification']=='PENDING_SNAPSHOT_PRESERVED_UNCHANGED'
    assert end['overall_static_audit_complete']is False
    result=read('source-candidate-comparison.json');assert result==generate()and result['scope']['compared_fields']==['Weight']
    assert result['scope']['narrative_text_retained_privately_with_hash_receipts']is False and result['scope']['narrative_text_presence']=='ABSENT_FROM_OBSERVED_BASIC_FIELDS'
    assert all(r['binding_status']=='UNQUALIFIED_NAME_CORRESPONDENCE'for r in result['items'])
    assert all(set(c['fields'])=={'Weight'}for r in result['items']for c in r['candidates'])
    assert read('uncompared-field-backlog.json')['private_narrative_receipts']==0
    for name,receipt in read('packet-integrity.json')['files'].items():
        raw=(HERE/name).read_bytes();assert len(raw)==receipt['bytes']and hashlib.sha256(raw).hexdigest()==receipt['sha256']
    for path in HERE.iterdir():
        assert path.suffix not in('.png','.zip','.crt','.key')
        if path.suffix=='.json':assert not any(s in path.read_text()for s in('/home/mole/','Bearer ','bridge-access.json','server.key','192.168.1.8'))
    print(json.dumps({'passed':True,'Gold_cards':3,'Basic_pages':3,'private_card_proofs':12,'Description':'ABSENT_FROM_OBSERVED_BASIC_FIELDS','Market':'ABSENT_FROM_OBSERVED_BASIC_FIELDS_NOT_COMPARED','observed_empty_initial_NPC_panels':6,'global_NPC_absence':False,'source_receipts':len(result['source_receipts']),'V1_persisted_records':0,'V2_capture_attempts':3,'overall':'PARTIAL'}))

if __name__=='__main__':main()
