"""Source-only membership handoff for the six remaining Summer-bound Items."""
import json
import re
import tarfile
from pathlib import Path
import questpc2_rotten_item_facts as previous
shared = previous.shared
OUTPUT = 'tools/content-schema/quest-authoring/samples/questpc2-rotten-membership-facts.json'
IDS = frozenset({43961, 43962, 43964, 43966, 43967, 43968})

def capture(root):
    verified = previous.capture(root)
    declaration = json.loads((root / previous.DIRECTORY / 'corpus-input.json').read_bytes())
    with tarfile.open(root / previous.DIRECTORY / declaration['path']) as stream:
        manifest = json.load(stream.extractfile('corpus-manifest.json'))
        source = next(r for r in manifest['files'] if r['revision'] == previous.REVISION
                      and r['path'] == 'data/items/items.xml')
        raw = stream.extractfile(source['cache_path']).read()
    if previous.hashlib.sha256(raw).hexdigest() != previous.XML_SHA:
        raise ValueError('Rotten membership source XML pin differs')
    xml = shared.source.xml_index(raw)
    bindings = json.loads((root / shared.BINDINGS).read_bytes())['bindings']
    records = []
    for i in sorted(IDS):
        matches = [r for r in bindings if r['external_id'] == str(i)]
        if len(matches) != 1 or matches[0]['source_revision'] != previous.REVISION:
            raise ValueError('Rotten membership exact binding differs')
        start = re.search(rb'<item\b[^>]*\bid="' + str(i).encode() + rb'"', raw).start()
        records.append(dict(source_item_id=i, name=xml[i].get('name'),
                            source_line=raw[:start].count(b'\n') + 1,
                            identity_binding=matches[0]))
    return dict(schema='OTERYN_ROTTEN_BLOOD_ITEM_MEMBERSHIP_FACTS/v1',
                native_admission=False, use_effects_admitted=False,
                source=verified['source'], items=records)

if __name__ == '__main__':
    root = Path(__file__).resolve().parents[3]
    (root / OUTPUT).write_text(json.dumps(capture(root), sort_keys=True, indent=2) + '\n')
