"""Validate provenance and reject invented defaults or identity collapse."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import unittest
import extract

HERE = Path(__file__).resolve().parent


def load(name):
    return json.loads((HERE / name).read_text(encoding='utf-8'))


def validate(packet, private_root=None):
    rows = packet['records']
    assert len(rows) == len({r['appearance_object_id'] for r in rows}) == 82
    assert sum(bool(r['npc_rows']) for r in rows) == 43
    assert sum(len(r['npc_rows']) for r in rows) == 457
    assert sum(bool(r['description']) for r in rows) == 18
    assert sum(r['clothes_slot'] is not None for r in rows) == 7
    assert all(r['weight'] == {'status':'UNKNOWN_NOT_IN_APPEARANCE_SCHEMA','value':None} for r in rows)
    assert all(r['canonical_item_binding'] == 'UNQUALIFIED_APPEARANCE_ID_NOT_SERVER_ID' for r in rows)
    assert all(not r['unmapped_flag_numbers'] and not set(range(60,65)).intersection(r['present_flag_numbers']) for r in rows)
    baking = next(r for r in rows if r['appearance_object_id'] == 3464)
    assert [(r['name'],r['location'],r['sale_price'],r['buy_price']) for r in baking['npc_rows']] == [
        ('Chephan','Venore City',20,0),('Maro','Rathleton City',20,0)]
    assert next(r for r in rows if r['appearance_object_id'] == 31913)['record_sha256'] != baking['record_sha256']
    schema = load('compiled-field-map.json')
    flags = {f['number']:f for m in schema['messages'] if m['name']=='AppearanceFlags' for f in m['fields']}
    assert [flags[n]['name'] for n in range(60,65)] == ['imbueable','proficiency','restrict_to_vocation','minimum_level','weapon_type']
    assert load('gui-queue.json')['tools_count'] == len(load('gui-queue.json')['items']) == 82
    if private_root:
        root = Path(private_root)
        provenance = load('provenance.json')
        for receipt in provenance['private_source_receipts']:
            raw = (root / receipt['path']).read_bytes()
            assert len(raw) == receipt['bytes'] and hashlib.sha256(raw).hexdigest() == receipt['sha256']
        raw = (root / 'work/client-15-33' / ('appearances-'+packet['appearance_sha256']+'.dat')).read_bytes()
        census = json.loads((root / 'outputs/items-client-assets-audit/appearance-object-census.json').read_text(encoding='utf-8'))
        assert extract.extract(raw,census) == packet


class Controls(unittest.TestCase):
    def test_weight_default_rejected(self):
        packet = copy.deepcopy(load('tools82.json'))
        packet['records'][0]['weight']['value'] = 0
        with self.assertRaises(AssertionError): validate(packet)

    def test_server_id_promotion_rejected(self):
        packet = copy.deepcopy(load('tools82.json'))
        packet['records'][0]['canonical_item_binding'] = 'SERVER_ID_VERIFIED'
        with self.assertRaises(AssertionError): validate(packet)

    def test_npc_direction_or_pair_substitution_rejected(self):
        packet = copy.deepcopy(load('tools82.json'))
        row = next(r for r in packet['records'] if r['appearance_object_id']==3464)
        row['npc_rows'][0]['sale_price'],row['npc_rows'][0]['buy_price'] = 0,20
        with self.assertRaises(AssertionError): validate(packet)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--private-root')
    parser.add_argument('--test',action='store_true')
    args = parser.parse_args()
    if args.test:
        unittest.main(argv=['validate'])
    else:
        validate(load('tools82.json'),args.private_root)
        print('PASS:82Tools/457NPC/18descriptions/7slots; schema and no-default/ID boundaries'+('; private sources reproduced' if args.private_root else ''))
