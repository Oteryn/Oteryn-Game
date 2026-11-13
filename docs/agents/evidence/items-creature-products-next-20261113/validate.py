"""Check ongoing compact CP facts, corrected capture mapping and pinned comparison."""
import collections,hashlib,json,pathlib,re
from compare_sources import generate
HERE=pathlib.Path(__file__).resolve().parent


def read(name):return json.loads((HERE/name).read_text())


def main():
    observations=[];proofs={};private_text=0
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
            if 'description_receipt'in row:
                receipt=row['description_receipt'];private_text+=1
                assert receipt['field_name']=='Description'and receipt['state']=='OBSERVED_VALUE_RETAINED_PRIVATELY'
                assert re.fullmatch('[0-9a-f]{64}',receipt['sha256_utf8'])
                assert receipt['bytes']>=receipt['characters']>=0
        assert sum('description_receipt'in row for row in rows)==packet['field_export_policy']['private_text_receipts']
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
    repair=read('gap-correction.json');assert repair['repairs']==[{key:by[n][key]for key in ('ui_entry_ordinal','capture_id','fullName','properties')}for n in(197,237)]
    for name,receipt in read('packet-integrity.json')['files'].items():
        raw=(HERE/name).read_bytes();assert len(raw)==receipt['bytes']and hashlib.sha256(raw).hexdigest()==receipt['sha256'],name
    comparison=read('source-candidate-comparison.json');assert comparison==generate()
    assert len(comparison['items'])==len(observations)
    assert all(row['binding_status']=='UNQUALIFIED_NAME_CORRESPONDENCE'for row in comparison['items'])
    for path in HERE.iterdir():
        assert path.suffix not in('.png','.apng','.zip','.crt','.key')
        if path.suffix=='.json':assert not any(text in path.read_text()for text in('/home/mole/','Bearer ','bridge-access.json','server.key','192.168.1.8'))
    print(json.dumps({'passed':True,'physical_ordinal_cutoff':[min(ordinals),max(ordinals)],'records':len(observations),
        'private_png_receipts':len(proofs),'observed_private_description_receipts':private_text,
        'gap_repairs_verified':[197,237],'duplicate_name_rows_preserved':[[163,164],[222,223]],
        'source_receipts':len(comparison['source_receipts']),'overall_audit':'PARTIAL'}))


if __name__=='__main__':main()
