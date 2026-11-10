"""Recognize only exact approved contact additions; retain the frozen base cohort checks."""
import json
from pathlib import Path

PACKET=Path(__file__).resolve().parents[1]/'content-schema/npc-authoring/samples/native-contact-definitions.json'


def qualified_counts(npcs, bindings, packet=None):
    packet=json.loads(PACKET.read_text()) if packet is None else packet
    rows=packet.get('records',[])
    if packet.get('schema')!='OTERYN_NATIVE_CONTACT_NPCS/v1' or not 1<=len(rows)<=16:
        raise ValueError('native contact packet schema/count')
    keys=[r['declaration']['identity']['key'] for r in rows]
    if len(set(keys))!=len(keys) or any(r['declaration'].get('kind')!='NPC' for r in rows):
        raise ValueError('native contact packet identity')
    for row in rows:
        identity=dict(family='NPC',**row['declaration']['identity'])
        if not row['source_bindings'] or any(b['target']!=identity
                or b['source_key']!=packet['source']['key']
                or b['source_revision']!=packet['source']['revision'] for b in row['source_bindings']):
            raise ValueError('native contact packet binding target/source')
    present=[n for n in npcs if n['identity']['key'] in keys]
    attached=[b for b in bindings if b['target']['key'] in keys]
    if not present and not attached:
        return 0,0
    expected=[r['declaration'] for r in rows]
    expected_bindings=[b for r in rows for b in r['source_bindings']]
    ordered=lambda records:sorted(json.dumps(r,sort_keys=True,separators=(',',':')) for r in records)
    if ordered(present)!=ordered(expected) or ordered(attached)!=ordered(expected_bindings):
        raise ValueError('native contact addition differs from the exact approved packet')
    return len(expected),len(expected_bindings)
