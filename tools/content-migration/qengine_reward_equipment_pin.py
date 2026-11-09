"""Generate the exact QENGINE reward identity witness from its source-owned IDs."""
import re

from qengine_reward_item_profiles import IDS


def refresh(source, packet):
    expected = {f'oteryn:item.tibia.i{i}' for i in IDS}
    keys = [r['production_definition']['production_key'] for r in packet['records']]
    if len(keys) != len(expected) or set(keys) != expected:
        raise ValueError('QENGINE_REWARD_COHORT_IDENTITIES')
    marker = r'    // QENGINE2 identities: generated begin\n.*?    // QENGINE2 identities: generated end\n'
    pattern = marker if '// QENGINE2 identities: generated begin' in source else r'    assert_eq!\(added_keys\.len\(\), [0-9_]+\);\n'
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
    result, count = re.subn(pattern, lambda _: block, source, flags=re.S)
    if count != 1:
        raise ValueError('QENGINE_REWARD_IDENTITY_WITNESS_SHAPE')
    result = re.sub(
        r'\("qengine-reward-item-profiles\.json", [0-9_]+\)',
        f'("qengine-reward-item-profiles.json", {len(expected)})',
        result,
    )
    return result.replace(
        '    // QENGINE2 captures 53 shared Items plus seven portable QUESTPC4 Items.',
        "    // QENGINE2: generated witness of the producer's explicit IDS set.",
    )
