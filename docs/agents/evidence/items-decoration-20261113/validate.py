"""Validate compact Decoration literals, custody, ordered scope and source regeneration."""
import collections, hashlib, json, pathlib, re
from compare_sources import generate
HERE = pathlib.Path(__file__).resolve().parent

def main():
    rows = []; proofs = {}; texts = layouts = pages = 0
    expected_sources = {1: '148fc064cb276b1b74189cc85a9f8efb5d3e0157da523f8ba5e23a0897145fc6', 181: '7430c341ed574887af6d4094a5185a6c323b98fae9637864a49744c8bfed533d', 225: '1b2581b2b7d727c68ab9a69c03d0d04f7138d2af78fd98479f9630fd8e19b18c'}
    allowed = {'capture_key', 'capture_id', 'ui_entry_ordinal', 'fullName', 'name_hash_is_item_identity', 'properties', 'proofs', 'saved_basic_pages', 'description_receipt', 'description_display_lines_receipt'}
    for path in sorted(HERE.glob('observations-*.json')):
        packet = json.loads(path.read_text()); coverage = packet['coverage']; items = packet['items']
        first = coverage['first_physical_ui_ordinal']; last = coverage['last_physical_ui_ordinal']
        assert packet['category'] == 'Decoration'
        assert packet['source_receipt']['sha256'] == expected_sources[first]
        assert [r['ui_entry_ordinal'] for r in items] == list(range(first, last + 1))
        assert len(items) == coverage['records'] and coverage['missing_physical_ordinals_in_cutoff'] == []
        assert coverage['category_end_observed'] == (first == 225)
        assert not any(coverage[k] for k in ('trade_panels_qualified', 'animation_qualified'))
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
    assert [r['ui_entry_ordinal'] for r in rows] == list(range(1,463))
    assert len({r['capture_key'] for r in rows}) == len(rows)
    assert len(proofs) == 1849 and pages == 463 and texts == layouts == 135
    by = {r['ui_entry_ordinal']: r for r in rows}
    assert by[1]['fullName'] == 'ancient bonelord santa' and by[1]['properties']['Weight'] == '6.50 oz'
    assert by[7]['fullName'] == by[8]['fullName'] and 'description_receipt' not in by[7] and 'description_receipt' in by[8]
    assert len({by[n]['fullName'] for n in range(130,137)}) == 1
    assert len({by[n]['description_receipt']['sha256_utf8'] for n in range(130,137)}) == 7
    assert by[170]['fullName'] == by[171]['fullName'] == by[172]['fullName']
    for n in range(208,213): assert 'Augments' in by[n]['properties'] and 'Skill Boost' not in by[n]['properties']
    assert by[235]['properties']['Total Expire Time'] == 'unknown'
    assert by[236]['properties']['Skill Boost'] == 'hard drinking'
    assert by[458]['fullName'] == by[459]['fullName'] and by[458]['capture_id'] != by[459]['capture_id']
    assert by[462]['fullName'] == 'yellow tapestry' and by[462]['properties']['Weight'] == '10.00 oz'
    assert by[462]['properties']['Tradeable In Market'] == 'yes'
    union = json.loads((HERE/'reader-coverage-union.json').read_text())
    assert union['physical_ui_ordinals'] == {'first':1,'last':462,'records':462,'gaps':[],'duplicates':[]}
    assert union['distinct_original_basic_png'] == pages and union['original_proof_hashes_freshly_verified'] == len(proofs)
    assert union['private_Description_values'] == texts
    assert union['prior_published_1_224_compact_rows_unchanged'] is True
    assert sum(x['records'] for x in union['immutable_inputs']) == 462
    assert len(union['immutable_inputs']) == 5
    assert union['overall_static_audit_complete'] is False and union['trade_or_animation_complete'] is False
    assert union['duplicate_row_qualification']['physical_ordinals'] == [458,459]
    assert union['duplicate_row_qualification']['witness_sha256_freshly_verified'] is True
    assert union['duplicate_row_qualification']['witness_receipt']['sha256'] == '07849576092d1dfa8bf59050de3e7861a153888f81265c44f4338ef008e0bce8'
    assert union['duplicate_row_qualification']['witness_receipt']['freshly_verified'] is True
    basic = {k for k in proofs if re.search(r'-basic-(?:top|\d+)\.png$',k)}
    assert len(basic) == pages
    end = json.loads((HERE/'category-end-receipt.json').read_text())
    assert end['source_receipt']['sha256'] == '69ac3ce174bca885cd5b391b0485e1a95f738e944fe37cd3f87dd92819eeca37'
    assert end['captured_records'] == 462 and end['captured_physical_ui_range'] == [1,462]
    assert end['category_end_observed'] is True and end['final_entry_ui_ordinal'] == end['final_capture_id'] == 462
    assert end['end_stability_checks_before'] == end['end_stability_checks_after'] == 2
    assert len(end['end_proof_receipts']) == 2
    assert end['finalization_receipt']['sha256'] == '5ef64ea7c274d7ba126695d5fd7840a28b453a0b152716cf1f35c056ffe6b94b'
    trade = json.loads((HERE/'trade-initial-panel-coverage.json').read_text())
    assert trade['source_receipt']['sha256'] == '6789526aa54238ce65b38f454854c0a29ffdfe815e586e73583e8ac6d8e47d29'
    assert trade['physical_ui_range'] == [1,50] and trade['initial_panels'] == 100
    assert trade['initial_panel_states'] == {'OBSERVED_EMPTY_PANEL':73,'OBSERVED_NONEMPTY_INITIAL_PANEL':27}
    assert trade['vertical_retake_ordinals'] == [46] and trade['horizontal_retake_ordinals'] == []
    assert trade['trade_panels_complete'] is False and trade['npc_lists_complete'] is False
    assert trade['transactions_or_conversations'] == trade['gui_input_actions'] == 0
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
    print(json.dumps({'passed':True,'records':len(rows),'physical_ordinals':[1,462],'private_proof_hashes':len(proofs),'saved_basic_pages':pages,'private_description_receipts':texts,'private_layout_receipts':layouts,'uncompared_fields':sorted(expected),'source_receipts':len(comparison['source_receipts']),'category_end_observed':True,'overall':'PARTIAL'}))

if __name__ == '__main__': main()
