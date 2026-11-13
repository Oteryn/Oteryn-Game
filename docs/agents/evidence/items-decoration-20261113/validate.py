"""Validate compact Decoration literals, custody, ordered scope and source regeneration."""
import collections, hashlib, json, pathlib, re
from compare_sources import generate
HERE = pathlib.Path(__file__).resolve().parent

def main():
    rows = []; proofs = {}; texts = layouts = pages = 0
    expected_sources = {1: '148fc064cb276b1b74189cc85a9f8efb5d3e0157da523f8ba5e23a0897145fc6', 181: '7430c341ed574887af6d4094a5185a6c323b98fae9637864a49744c8bfed533d'}
    allowed = {'capture_key', 'capture_id', 'ui_entry_ordinal', 'fullName', 'name_hash_is_item_identity', 'properties', 'proofs', 'saved_basic_pages', 'description_receipt', 'description_display_lines_receipt'}
    for path in sorted(HERE.glob('observations-*.json')):
        packet = json.loads(path.read_text()); coverage = packet['coverage']; items = packet['items']
        first = coverage['first_physical_ui_ordinal']; last = coverage['last_physical_ui_ordinal']
        assert packet['category'] == 'Decoration'
        assert packet['source_receipt']['sha256'] == expected_sources[first]
        assert [r['ui_entry_ordinal'] for r in items] == list(range(first, last + 1))
        assert len(items) == coverage['records'] and coverage['missing_physical_ordinals_in_cutoff'] == []
        assert not any(coverage[k] for k in ('category_end_observed', 'trade_panels_qualified', 'animation_qualified'))
        packet_texts = packet_layouts = 0; packet_proofs = set()
        for row in items:
            assert set(row) <= allowed
            assert row['capture_key'] == f"items-sequence-decoration:{row['capture_id']}"
            assert row['name_hash_is_item_identity'] is False
            assert 'Description' not in row['properties'] and 'description_display_lines' not in row
            assert isinstance(row['fullName'], str) and row['fullName']
            assert row['saved_basic_pages'] >= 1
            pages += row['saved_basic_pages']
            for proof in row['proofs']:
                key = proof['private_source_key']; h = proof['sha256']
                assert key.startswith(f"items-sequence-decoration/cards/{row['capture_id']:05d}-")
                assert not pathlib.PurePosixPath(key).is_absolute() and '..' not in pathlib.PurePosixPath(key).parts
                assert re.fullmatch('[0-9a-f]{64}', h)
                if key in proofs: assert proofs[key] == h
                proofs[key] = h; packet_proofs.add(key)
            if 'description_receipt' in row:
                r = row['description_receipt']; packet_texts += 1
                assert r['field_name'] == 'Description' and r['state'] == 'OBSERVED_VALUE_RETAINED_PRIVATELY'
                assert re.fullmatch('[0-9a-f]{64}', r['sha256_utf8'])
                assert r['bytes'] >= r['characters'] >= 0
            if 'description_display_lines_receipt' in row:
                r = row['description_display_lines_receipt']; packet_layouts += 1
                assert r['field_name'] == 'description_display_lines' and r['state'] == 'OBSERVED_LAYOUT_RETAINED_PRIVATELY'
                assert re.fullmatch('[0-9a-f]{64}', r['canonical_json_sha256'])
                assert r['line_count'] == len(r['line_character_lengths']) and r['canonical_utf8_bytes'] >= 2
        assert packet_texts == packet['field_export_policy']['private_text_receipts']
        assert packet_layouts == packet['field_export_policy']['private_display_layout_receipts']
        assert len(packet_proofs) == packet['private_png_hash_checks']
        assert sum(r['saved_basic_pages'] for r in items) == coverage['saved_basic_pages']
        texts += packet_texts; layouts += packet_layouts; rows += items
    rows.sort(key=lambda r: r['ui_entry_ordinal'])
    assert [r['ui_entry_ordinal'] for r in rows] == list(range(1,225))
    assert len({r['capture_key'] for r in rows}) == len(rows)
    assert len(proofs) == 897 and pages == 225 and texts == layouts == 81
    by = {r['ui_entry_ordinal']: r for r in rows}
    assert by[1]['fullName'] == 'ancient bonelord santa' and by[1]['properties']['Weight'] == '6.50 oz'
    assert by[7]['fullName'] == by[8]['fullName'] and 'description_receipt' not in by[7] and 'description_receipt' in by[8]
    assert len({by[n]['fullName'] for n in range(130,137)}) == 1
    assert len({by[n]['description_receipt']['sha256_utf8'] for n in range(130,137)}) == 7
    assert by[170]['fullName'] == by[171]['fullName'] == by[172]['fullName']
    for n in range(208,213): assert 'Augments' in by[n]['properties'] and 'Skill Boost' not in by[n]['properties']
    comparison = json.loads((HERE/'source-candidate-comparison.json').read_text())
    assert comparison == generate()
    assert len(comparison['items']) == len(rows)
    assert all(r['binding_status'] == 'UNQUALIFIED_NAME_CORRESPONDENCE' for r in comparison['items'])
    backlog = json.loads((HERE/'uncompared-field-backlog.json').read_text()); expected = collections.defaultdict(list)
    for row in rows:
        for name in row['properties']:
            if name not in ('Weight', 'Tradeable In Market'): expected[name].append(row['ui_entry_ordinal'])
    assert backlog['fields'] == [{'field':n, 'observations':len(v), 'physical_ordinals':v, 'source_route_status':'NOT_YET_QUALIFIED_FOR_COMPARISON'} for n,v in sorted(expected.items())]
    assert backlog['private_narrative_receipts'] == texts
    for name, receipt in json.loads((HERE/'packet-integrity.json').read_text())['files'].items():
        raw = (HERE/name).read_bytes(); assert len(raw) == receipt['bytes'] and hashlib.sha256(raw).hexdigest() == receipt['sha256']
    for path in HERE.iterdir():
        assert path.suffix not in ('.png','.zip','.crt','.key')
        if path.suffix == '.json': assert not any(s in path.read_text() for s in ('/home/mole/','Bearer ','bridge-access.json','server.key','192.168.1.8'))
    print(json.dumps({'passed':True,'records':len(rows),'physical_ordinals':[1,224],'private_proof_hashes':len(proofs),'saved_basic_pages':pages,'private_description_receipts':texts,'private_layout_receipts':layouts,'uncompared_fields':sorted(expected),'source_receipts':len(comparison['source_receipts']),'category_end_observed':False,'overall':'PARTIAL'}))

if __name__ == '__main__': main()
