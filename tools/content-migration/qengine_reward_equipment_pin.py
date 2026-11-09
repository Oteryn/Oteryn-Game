"""Generate the exact QENGINE reward identity witness from its source-owned IDs."""
import re

import qengine_reward_item_profiles as producer

IDS = producer.IDS


def refresh(source, packet):
    rows = packet.get("records", [])
    keys = [r["production_definition"]["production_key"] for r in rows]
    expected = {f"oteryn:item.tibia.i{i}" for i in producer.IDS}
    if (
        packet.get("schema") != "OTERYN_QENGINE_REWARD_ITEM_PROFILES/v1"
        or packet.get("producer") != "tools/content-migration/qengine_reward_item_profiles.py"
        or packet.get("sources") != [
            {"path": producer.XML, "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446", "sha256": producer.XML_SHA},
            {"path": producer.CLIENT, "sha256": producer.CLIENT_SHA},
        ]
        or len(keys) != len(expected)
        or set(keys) != expected
    ):
        raise ValueError("QENGINE_REWARD_SOURCE_COHORT")
    marker = r'    // QENGINE2 identities: generated begin\n.*?    // QENGINE2 identities: generated end\n'
    pattern = marker if '// QENGINE2 identities: generated begin' in source else r'^\s*assert_eq!\(added_keys\.len\(\), [0-9_]+\);\n'
    identities = '\n'.join(f'            "{key}",' for key in sorted(expected))
    block = f'''    // QENGINE2 identities: generated begin
    assert_eq!(added_keys.len(), {len(expected)});
    assert_eq!(
        added_keys,
        std::collections::BTreeSet::from([
{identities}
        ])
    );
    // QENGINE2 identities: generated end
'''
    result, count = re.subn(pattern, lambda _: block, source, flags=re.S | re.M)
    if count != 1:
        raise ValueError('QENGINE_REWARD_IDENTITY_WITNESS_SHAPE')
    result = re.sub(
        r'\("qengine-reward-item-profiles\.json", [0-9_]+\)',
        f'("qengine-reward-item-profiles.json", {len(expected)})',
        result,
    )
    result = re.sub(r'QENGINE2 adds exactly the [0-9]+ independently captured physical quest Items\.',
                    f'QENGINE2 adds exactly the {len(expected)} independently captured physical quest Items.', result)
    return result.replace(
        '    // QENGINE2 captures 53 shared Items plus seven portable QUESTPC4 Items.',
        "    // QENGINE2: generated witness of the producer's explicit IDS set.",
    )
