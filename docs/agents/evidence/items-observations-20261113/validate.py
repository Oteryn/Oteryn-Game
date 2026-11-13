"""Validate bounded observation packet, provenance, privacy and source comparison."""
import collections,hashlib,json,pathlib,re
from compare_sources import generate
HERE=pathlib.Path(__file__).resolve().parent


def read(name):return json.loads((HERE/name).read_text())


def main():
    cp=read('creature-products-basic-1-134-repo-ready.json');items=cp['items']
    assert len(items)==134 and [item['ui_entry_ordinal']for item in items]==list(range(1,135))
    assert cp['coverage']['status']=='PARTIAL'and cp['coverage']['category_end_observed']is False
    assert cp['coverage']['trade_panels_qualified']is False and cp['coverage']['animation_qualified']is False
    proofs={proof['private_source_key']:proof['sha256']for item in items for proof in item['proofs']}
    assert len(proofs)==404
    for key,value in proofs.items():
        assert not pathlib.PurePosixPath(key).is_absolute()and '..'not in pathlib.PurePosixPath(key).parts
        assert re.fullmatch('[0-9a-f]{64}',value)
    armors=read('armors-basic-1-33-qualified-repo-ready.json');assert len(armors['items'])==33
    assert [item['capture_id']for item in armors['items']]==list(range(1,34))
    assert all(item['global_physical_ui_ordinal']is None and item['global_physical_ui_ordinal_status']=='UNKNOWN'for item in armors['items'])
    armor_proofs={proof['private_source_key']:proof['sha256']for item in armors['items']for proof in item['proofs']}
    assert len(armor_proofs)==141
    trade=read('trade-initial-panel-audit.json');assert len(trade['items'])==176
    counts=collections.Counter(item['category']for item in trade['items']);assert counts=={'boots':67,'containers':109}
    assert len({(item['category'],item['ui_entry_ordinal'])for item in trade['items']})==176
    checklist=read('trade-recapture-checklist-batch-20261113.json');assert len(checklist['items'])==22
    assert sum(bool(item['vertical_panels'])for item in checklist['items'])==19
    assert sum(len(item['vertical_panels'])for item in checklist['items'])==21
    assert sum(item['horizontal_clipped']for item in checklist['items'])==1
    assert sum(item['npc_exact_text_unknown']for item in checklist['items'])==2
    custody=read('custody-validation.json');assert custody['proof_bytes_verified_on_owner_ubuntu']['combined_unique']==580
    assert read('additional-custody-validation.json')['proof_bytes_verified_on_owner_ubuntu']==141
    assert custody['calibration']['all_ordinals_preserved']and custody['calibration']['ordinals_audited']==134
    for name,receipt in read('packet-integrity.json')['files'].items():
        raw=(HERE/name).read_bytes();assert hashlib.sha256(raw).hexdigest()==receipt['sha256']and len(raw)==receipt['bytes'],name
    comparison=read('source-candidate-comparison.json');assert comparison==generate()
    assert len(comparison['items'])==167
    assert all(item['binding_status']=='UNQUALIFIED_NAME_CORRESPONDENCE'for item in comparison['items'])
    # Block private machine paths/configuration and original binary media. The
    # packet contains only names, literal observed fields and custody references.
    for path in HERE.iterdir():
        if path.suffix=='.json':
            raw=path.read_text()
            assert not any(value in raw for value in ('/home/mole/','Bearer ','bridge-access.json','server.key','192.168.1.8'))
        assert path.suffix not in ('.png','.apng','.zip','.crt','.key')
    print(json.dumps({'passed':True,'creature_products_records':134,'creature_products_proof_receipts':404,
        'armors_requalified_records':33,'armors_proof_receipts':141,'trade_initial_cards':176,'vertical_retake_cards':19,
        'vertical_panels':21,'horizontal_labels_pending':1,'npc_names_pending':2,'source_candidate_records':167,
        'source_receipts':len(comparison['source_receipts'])}))


if __name__=='__main__':main()
