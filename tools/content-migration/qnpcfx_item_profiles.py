"""Promote source-qualified physical inputs for the durable NPC executor.

Refresh with --source-root pointing at Crystal 201189f's pinned items.xml.
Qualification belongs to the existing batch-three importer and native merger.
"""
import argparse
import json
import re
from pathlib import Path

import quest_batch3_item_profiles as batch
import qengine_reward_item_profiles as shared

PATH = 'tools/content-schema/native-gameplay/qnpcfx-item-profiles.json'
SCHEMA = 'OTERYN_QNPCFX_ITEM_PROFILES/v1'
KEYS = {'oteryn:item.tibia.i52636'} | {f'oteryn:item.tibia.i{i}' for i in range(7528, 7533)} | {'oteryn:item.tibia.i919'}


def capture(source_root):
    packet = batch.capture(source_root, additional_inputs=(
        'content/quests/npc-callbacks/qnpcfx-koshei.json',
        'content/quests/npc-callbacks/qnpcfx-m10.json',
    ))
    packet['records'] = [r for r in packet['records']
                         if r['production_definition']['production_key'] in KEYS]
    packet['classification'] = [r for r in packet['classification'] if r['key'] in KEYS]
    # The amulet's resistance attributes belong to the existing equipment
    # importer, not the plain-portable whitelist. Keep that owner's full facts.
    equipment_keys = {'oteryn:item.tibia.i7532', 'oteryn:item.tibia.i919'}
    equipment = shared.capture(item_ids={7532, 919})
    packet['records'] = [r for r in packet['records']
                         if r['production_definition']['production_key'] not in equipment_keys]
    packet['records'].extend(equipment['records'])
    packet['records'].sort(key=lambda r: int(r['production_definition']['production_key'].rsplit('i', 1)[1]))
    for row in packet['classification']:
        if row['key'] in equipment_keys:
            row.update(classification='QUALIFIED_NATIVE_EQUIPMENT', reason=None,
                       profile_source=equipment['sources'])
    packet['sources'].extend(equipment['sources'])
    packet.update(schema=SCHEMA, producer='tools/content-migration/qnpcfx_item_profiles.py',
                  runtime_enabled=True,
                  decision='QNPCFX: production NPC effect qualification',
                  purpose='Physical custody for source-authored NPC hand-ins; no custom use effects.')
    return packet


def cohort_receipt(source, packet):
    """The explicit source key set is independent of the generated native union."""
    keys = {r['production_definition']['production_key'] for r in packet['records']}
    if keys != KEYS or len(packet['records']) != len(KEYS):
        raise ValueError('QNPCFX_COHORT_IDENTITY_SET')
    pattern = r'    // QNPCFX:[^\n]*\n(?:    //[^\n]*\n)*    \("QNPCFX", [0-9_]+\),'
    block = ('    // QNPCFX: generated from qnpcfx_item_profiles.py KEYS and pinned source proofs.\n'
             '    // Koshei pieces/amulet, Hallowed Axe and tender venison; existing equipment owner.\n'
             f'    ("QNPCFX", {len(KEYS)}),')
    result, count = re.subn(pattern, lambda _: block, source)
    if count != 1:
        raise ValueError('QNPCFX_COHORT_RECEIPT_SHAPE')
    return result



def profile_identity_receipt(source, packet):
    """Generate an exact identity witness for every independently declared NPC item."""
    # Reuse the explicit-key admission; the native union cannot supply its own expectation.
    cohort_receipt('    // QNPCFX: witness.\n    ("QNPCFX", 7),', packet)
    marker = r'    // QNPCFX identities: generated begin\n.*?    // QNPCFX identities: generated end\n'
    legacy = (r'    assert_eq!\(npc_additions\.len\(\), [0-9_]+\);\n'
              r'    assert_eq!\(\n        npc_additions\[0\]\["production_definition"\]\["production_key"\],\n'
              r'        "oteryn:item.tibia.i52636"\n    \);\n')
    keys = '\n'.join(f'            "{key}",' for key in sorted(KEYS))
    block = f'''    // QNPCFX identities: generated begin
    assert_eq!(npc_additions.len(), {len(KEYS)});
    let npc_keys = npc_additions
        .iter()
        .map(|row| {{
            row["production_definition"]["production_key"]
                .as_str()
                .ok_or("NPC profile identity")
        }})
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    assert_eq!(
        npc_keys,
        std::collections::BTreeSet::from([
{keys}
        ])
    );
    for key in npc_keys {{
        let profile = profiles
            .records
            .iter()
            .find(|row| row.production_definition.production_key == key)
            .ok_or_else(|| format!("missing NPC source profile {{key}}"))?;
        assert!(profile.admission.materializable, "{{key}}");
        assert_eq!(profile.attributes.speed_bonus, Some(0), "{{key}}");
    }}
    // QNPCFX identities: generated end
'''
    pattern = marker if '// QNPCFX identities: generated begin' in source else legacy
    result, count = re.subn(pattern, lambda _: block, source, flags=re.S)
    if count != 1:
        raise ValueError('QNPCFX_IDENTITY_WITNESS_SHAPE')
    return result


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PATH).read_bytes())
    records = packet['records']
    if (packet['schema'] != SCHEMA or len(records) != len(KEYS)
            or {r['production_definition']['production_key'] for r in records} != KEYS
            or len(packet['classification']) != len(KEYS)
            or {r['key'] for r in packet['classification']} != KEYS
            or any(r['classification'] not in ('QUALIFIED_PLAIN_PORTABLE', 'EXISTING_NATIVE', 'QUALIFIED_NATIVE_EQUIPMENT')
                   for r in packet['classification'])):
        raise ValueError('QNPCFX_ITEM_QUALIFICATION')
    shared.admit_records(root, generated, profile_path, manifest_path, order, canonical, records)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, required=True)
    args = parser.parse_args()
    (batch.ROOT / PATH).write_bytes(batch.grizzly.encode(capture(args.source_root)))
