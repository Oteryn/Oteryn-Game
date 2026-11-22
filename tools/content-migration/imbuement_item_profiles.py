"""PARITY-IMBUE-FORGE-WIRE-1: imbuement slots of native item policies from pinned Canary items.xml.

Canary is the value source and Crystal the cross-check: a native record whose source id
declares `imbuementslot` gets its slot count and allowed families; any disagreement with
the committed Crystal facts stops generation. Every other record keeps its imbuement state.

    python3 tools/content-migration/imbuement_item_profiles.py --stage-crystal <crystal items.xml>
rewrites the Crystal facts; register_spell_families.py [--check] applies the overlay.
"""
import argparse
import hashlib
import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

from native_profile_composition import encode_native_profile

CANARY = 'imports/canary/items-xml/items.xml'
CANARY_SHA256 = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
CRYSTAL_FACTS = 'imports/crystalserver/facts/items-imbuement-slots.json'
CRYSTAL_REVISION = 'ff7ede593c69d4c658b382c97443e8155926924a'
CRYSTAL_SHA256 = 'c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb'
AUDIT_FACTS = 'imports/crystalserver/facts/items-imbuement-parity-201189f8.json'
AUDIT_SOURCE = {
    'path': 'data/items/items.xml',
    'repository': 'zimbadev/crystalserver',
    'revision': '201189f871d23597b82caf88fd611454a2ddefb8',
    'sha256': '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
}
PROFILES = 'content/items/definitions/spell-native-profiles.json'
# items.xml `imbuementslot` child keys (Canary src/items/functions/item/item_parse.cpp) to
# the content family names.
FAMILIES = {
    'critical hit': 'CRITICAL_HIT', 'elemental damage': 'ELEMENTAL_DAMAGE',
    'elemental protection death': 'PROTECTION_DEATH', 'elemental protection earth': 'PROTECTION_EARTH',
    'elemental protection energy': 'PROTECTION_ENERGY', 'elemental protection fire': 'PROTECTION_FIRE',
    'elemental protection holy': 'PROTECTION_HOLY', 'elemental protection ice': 'PROTECTION_ICE',
    'increase capacity': 'INCREASE_CAPACITY', 'increase speed': 'INCREASE_SPEED',
    'life leech': 'LIFE_LEECH', 'mana leech': 'MANA_LEECH', 'paralysis deflection': 'PARALYSIS_REMOVAL',
    'paralysis removal': 'PARALYSIS_REMOVAL',  # Crystal's name for the same boots category.
    'skillboost axe': 'SKILL_AXE', 'skillboost club': 'SKILL_CLUB', 'skillboost distance': 'SKILL_DISTANCE',
    'skillboost fist': 'SKILL_FIST', 'skillboost magic level': 'SKILL_MAGIC_LEVEL',
    'skillboost shielding': 'SKILL_SHIELDING', 'skillboost sword': 'SKILL_SWORD',
}
# ReferenceImbuementFamily wire order.
ORDER = ['CRITICAL_HIT', 'ELEMENTAL_DAMAGE', 'PROTECTION_DEATH', 'PROTECTION_EARTH', 'PROTECTION_ENERGY',
         'PROTECTION_FIRE', 'PROTECTION_HOLY', 'PROTECTION_ICE', 'INCREASE_CAPACITY', 'INCREASE_SPEED',
         'LIFE_LEECH', 'MANA_LEECH', 'PARALYSIS_REMOVAL', 'SKILL_AXE', 'SKILL_CLUB', 'SKILL_DISTANCE',
         'SKILL_FIST', 'SKILL_MAGIC_LEVEL', 'SKILL_SHIELDING', 'SKILL_SWORD']
TIERS = {'2': 'Two', '3': 'Three', '10': 'Ten'}


def slots(xml_bytes, ids):
    """`{id: {'slot_count': n, 'families': {family: tier}}}` for the wanted ids that declare slots."""
    found = {}
    for item in ET.fromstring(xml_bytes).iter('item'):
        if 'id' in item.attrib:
            item_ids = [int(item.attrib['id'])]
        elif 'fromid' in item.attrib:
            item_ids = list(range(int(item.attrib['fromid']), int(item.attrib['toid']) + 1))
        else:
            continue
        wanted = [i for i in item_ids if i in ids]
        for attribute in item.findall('attribute'):
            if attribute.get('key', '').lower() != 'imbuementslot' or not wanted:
                continue
            families = {}
            for child in attribute.findall('attribute'):
                family = FAMILIES[child.get('key').lower()]
                if family in families or child.get('value') not in TIERS:
                    raise ValueError(f'IMBUEMENT_ITEM_SLOT {wanted}')
                families[family] = child.get('value')
            for i in wanted:
                if i in found:
                    raise ValueError(f'IMBUEMENT_ITEM_DUPLICATE {i}')
                found[i] = {'slot_count': int(attribute.get('value')), 'families': families}
    return found


def native_ids(document):
    return {int(r['production_binding']['external_id']) for r in document['records']
            if r.get('production_binding') and r['production_binding']['external_id'].isdigit()}


def crystal_facts(xml_bytes, ids):
    if hashlib.sha256(xml_bytes).hexdigest() != CRYSTAL_SHA256:
        raise ValueError('IMBUEMENT_CRYSTAL_SOURCE')
    return {
        'evidence': 'OtsHypothesisOnly',
        'extracted_by': 'tools/content-migration/imbuement_item_profiles.py --stage-crystal',
        'schema': 'OTERYN_CRYSTAL_ITEM_IMBUEMENT_SLOT_FACTS/v1',
        'scope': 'imbuementslot of the native item policy source ids',
        'source': {'path': 'data/items/items.xml', 'repository': 'zimbadev/crystalserver',
                   'revision': CRYSTAL_REVISION, 'sha256': CRYSTAL_SHA256},
        'records': {str(i): row for i, row in sorted(slots(xml_bytes, ids).items())},
    }


def admit(root, generated, profile_path, manifest_path, order, canonical):
    source = (root / CANARY).read_bytes()
    if hashlib.sha256(source).hexdigest() != CANARY_SHA256:
        raise ValueError('IMBUEMENT_CANARY_SOURCE')
    document = json.loads(generated[profile_path])
    canary = slots(source, native_ids(document))
    crystal = json.loads((root / CRYSTAL_FACTS).read_bytes())
    if (crystal['source']['revision'] != CRYSTAL_REVISION
            or crystal['source']['sha256'] != CRYSTAL_SHA256
            or any(crystal['records'].get(str(i)) != row for i, row in canary.items())):
        raise ValueError('IMBUEMENT_CRYSTAL_CROSS_CHECK')
    for record in document['records']:
        binding = record.get('production_binding')
        row = binding and binding['external_id'].isdigit() and canary.get(int(binding['external_id']))
        if not row:
            continue
        if record['semantics']['imbuement'] != {'state': 'UNKNOWN'} or not 1 <= row['slot_count'] <= 3:
            raise ValueError(f"IMBUEMENT_ITEM_PROFILE {binding['external_id']}")
        record['semantics']['imbuement'] = {'state': 'KNOWN', 'value': {
            'slot_count': {'state': 'KNOWN', 'value': row['slot_count']},
            'allowed_family_tiers': {'state': 'KNOWN', 'value': [
                {'family': family, 'tier': TIERS[row['families'][family]]}
                for family in ORDER if family in row['families']]},
            'excluded_families': {'state': 'KNOWN', 'value': []},
        }}
    document['records'].sort(key=order)
    raw = encode_native_profile(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


def complete_boots(root, generated, profile_path, manifest_path, order, canonical):
    """Later shop/loot cohorts add boots after the original three Item profiles.

    Fill only their missing imbuement allowance, from the same pinned item donors.
    Canary adds Swiftness to boots of haste and Vibrancy to depth calcei. Galthen
    loot boots use Canary's tier three families, not Crystal's OT tier ten.
    Retain these explicit donor differences, as their existing owners require.
    """
    raw = (root / CANARY).read_bytes()
    if hashlib.sha256(raw).hexdigest() != CANARY_SHA256:
        raise ValueError('IMBUEMENT_CANARY_SOURCE')
    document = json.loads(generated[profile_path])
    source = slots(raw, native_ids(document))
    boots = {i: row for i, row in source.items() if 'PARALYSIS_REMOVAL' in row['families']}
    crystal = json.loads((root / CRYSTAL_FACTS).read_bytes())
    for item, row in boots.items():
        expected = row
        if item == 3079:
            expected = {'slot_count': 1, 'families': {'PARALYSIS_REMOVAL': '3'}}
        elif item == 13997:
            expected = {'slot_count': 1, 'families': {}}
        elif item in (44648, 44649):
            expected = {'slot_count': 1, 'families': {'INCREASE_SPEED': '10'}}
        if crystal['records'].get(str(item)) != expected:
            raise ValueError(f'IMBUEMENT_BOOT_CROSS_CHECK {item}')
    for record in document['records']:
        binding = record.get('production_binding')
        row = binding and binding['external_id'].isdigit() and boots.get(int(binding['external_id']))
        if not row:
            continue
        expected = {'state': 'KNOWN', 'value': {
            'slot_count': {'state': 'KNOWN', 'value': row['slot_count']},
            'allowed_family_tiers': {'state': 'KNOWN', 'value': [
                {'family': family, 'tier': TIERS[row['families'][family]]}
                for family in ORDER if family in row['families']]},
            'excluded_families': {'state': 'KNOWN', 'value': []},
        }}
        existing = record['semantics']['imbuement']
        unknown_families = {'state': 'KNOWN', 'value': {
            'slot_count': expected['value']['slot_count'],
            'allowed_family_tiers': {'state': 'UNKNOWN'},
            'excluded_families': {'state': 'UNKNOWN'},
        }}
        if existing not in ({'state': 'UNKNOWN'}, unknown_families, expected):
            raise ValueError(f"IMBUEMENT_BOOT_PROFILE {binding['external_id']}")
        record['semantics']['imbuement'] = expected
    document['records'].sort(key=order)
    encoded = encode_native_profile(document)
    generated[profile_path] = encoded
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(encoded).hexdigest()
    generated[manifest_path] = canonical(manifest)


def apply_crystal_audit(root, *providers, require_all=True):
    """Apply the two audited cohorts to their already admitted Item owners."""
    facts = json.loads((root / AUDIT_FACTS).read_bytes())
    if (facts['schema'] != 'OTERYN_CRYSTAL_ITEM_IMBUEMENT_SLOT_FACTS/v1'
            or facts['source'] != AUDIT_SOURCE):
        raise ValueError('IMBUEMENT_AUDIT_SOURCE')
    rows = facts['records']
    seen = set()
    for document in providers:
        for record in document['records']:
            key = record['production_definition']['production_key']
            item_id = key.removeprefix('oteryn:item.tibia.i')
            if item_id not in rows:
                continue
            binding = record['production_binding']
            if (item_id in seen or not record['admission']['materializable']
                    or not binding or binding['external_id'] != item_id
                    or binding['source_key'] != 'oteryn:source.crystalserver'
                    or binding['identity_namespace'] != 'ots/item_server_id'
                    or binding['disposition'] != 'EXACT'):
                raise ValueError(f'IMBUEMENT_AUDIT_ITEM_BINDING {key}')
            seen.add(item_id)
            row = rows[item_id]
            families = row['families']
            if not 1 <= row['slot_count'] <= 3 or not families or families.keys() - set(ORDER):
                raise ValueError(f'IMBUEMENT_AUDIT_ITEM_PROFILE {key}')
            # XML children form the complete allowlist. An unlisted family is
            # ineligible without a separate exclusion overriding the allowlist.
            record['semantics']['imbuement'] = {'state': 'KNOWN', 'value': {
                'slot_count': {'state': 'KNOWN', 'value': row['slot_count']},
                'allowed_family_tiers': {'state': 'KNOWN', 'value': [
                    {'family': family, 'tier': TIERS[families[family]]}
                    for family in ORDER if family in families]},
                'excluded_families': {'state': 'KNOWN', 'value': []},
            }}
    if require_all and (missing := rows.keys() - seen):
        raise ValueError(f'IMBUEMENT_AUDIT_MISSING_ITEMS {sorted(missing)}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--stage-crystal', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    ids = native_ids(json.loads((root / PROFILES).read_bytes()))
    canary = slots((root / CANARY).read_bytes(), ids)
    # Preserve the original three-profile cross-check and qualify the admitted boots.
    ids = {2854, 3264, 3349} | {i for i, row in canary.items() if 'PARALYSIS_REMOVAL' in row['families']}
    facts = crystal_facts(args.stage_crystal.read_bytes(), ids)
    (root / CRYSTAL_FACTS).write_text(json.dumps(facts, indent=2, sort_keys=True) + '\n')
    return 0


if __name__ == '__main__':
    sys.exit(main())
