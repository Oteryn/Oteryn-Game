"""Regenerate the existing closed Creature shard includes after owned canonical admission."""
import json
import re


def refresh(root):
    index=json.loads((root/'content/creatures/definitions/index.json').read_bytes())
    paths=list(index['shards'])
    assert all(p.startswith('content/creatures/definitions/creatures-') for p in paths)
    rows=[r for p in paths for r in json.loads((root/p).read_bytes())['records']]
    assert len(rows)==index['record_count']
    assert len({r['definition']['identity']['key'] for r in rows})==len(rows)
    target=root/'crates/gameplay/content/src/content/project/bestiary.rs';text=target.read_text()
    body=f'const SHARDS: [(&str, &[u8]); {len(paths)}] = [\n'
    for path in paths:
        body+=f'    (\n        "{path}",\n        include_bytes!(\n            "../../../../../../{path}"\n        ),\n    ),\n'
    body+='];'
    text,count=re.subn(r'const SHARDS: \[\(&str, &\[u8\]\); \d+\] = \[.*?\n\];',lambda _:body,text,flags=re.S)
    assert count==1
    text,count=re.subn(r'const CREATURE_COUNT: usize = [\d_]+;',f'const CREATURE_COUNT: usize = {len(rows):_};',text)
    assert count==1
    text,count=re.subn(r'assert_eq!\(limits\(\).max_reference_records, [\d_]+\);',f'assert_eq!(limits().max_reference_records, {len(rows):_});',text)
    assert count==1
    if target.read_text()!=text:target.write_text(text)
    packet=json.loads((root/'tools/content-schema/quest-authoring/questpc7-thaian-authoring.json').read_bytes())
    reference_count=len(json.loads((root/'content/world/definitions/reference.json').read_bytes())['records'])
    for path in ('crates/gameplay/content/src/content/npc_catalogue.rs','apps/game-server/examples/materialize_content_world_project_v2.rs'):
        target=root/path;text=target.read_text()
        text,count=re.subn(r'max_reference_records: [^,\n]+,',f'max_reference_records: {reference_count:_},',text)
        assert count==1
        imports=len(json.loads((root/'content/world/provenance/sources.json').read_bytes())['sources'])
        text=re.sub(r'max_import_records: (?:25 \+ 4|\d+),',f'max_import_records: {imports},',text)
        if target.read_text()!=text:target.write_text(text)

    # The closed legacy cohort includes Cake; Thaian adds its exact declaration.
    assert len(rows) == 1864 + sum(r['identity']['family'] == 'Creature' for r in packet['records']) == 1865
    for path in ('tools/content-migration/validate_world_project_v2_to_tree.py',
                 'tools/content-migration/test_world_project_v2_to_tree.py'):
        target = root / path
        text = target.read_text().replace("counts['Creature'] == 1863", "counts['Creature'] == 1865")
        text = text.replace('"Creature": 1863, "Encounter"', '"Creature": 1865, "Encounter"')
        if target.read_text() != text: target.write_text(text)
